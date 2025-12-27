use std::collections::{BTreeMap, VecDeque};

use crate::{
    ast::{
        array::Array,
        ty::{PathOrIdent, Type},
        union::{IdentOrUnion, Union, UnionDiscriminant},
    },
    defs::Spanned,
};

use super::{TypeResolver, helpers::AliasEntry};

/// Normalize a cycle to start from lexicographically smallest element.
/// This ensures deterministic error reporting regardless of detection order.
fn normalize_cycle<T: Clone>(
    chain: Vec<String>,
    associated: Vec<T>,
) -> (Vec<String>, Vec<T>) {
    if chain.is_empty() {
        return (chain, associated);
    }

    // Find the index of the lexicographically smallest element
    let min_idx = chain
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(i, _)| i)
        .unwrap_or(0);

    // Rotate both vectors to start from that index
    let mut rotated_chain = chain[min_idx..].to_vec();
    rotated_chain.extend_from_slice(&chain[..min_idx]);

    let mut rotated_associated = associated[min_idx..].to_vec();
    rotated_associated.extend_from_slice(&associated[..min_idx]);

    (rotated_chain, rotated_associated)
}

struct AliasGraph {
    aliases: BTreeMap<String, AliasEntry>,
}

impl AliasGraph {
    fn new() -> Self {
        Self {
            aliases: BTreeMap::new(),
        }
    }

    fn add_alias(
        &mut self,
        name: String,
        target_type: Spanned<Type>,
        source: std::path::PathBuf,
    ) {
        let dependencies = Self::extract_type_dependencies(&target_type.value);

        self.aliases.insert(
            name,
            AliasEntry {
                target_type,
                source,
                dependencies,
            },
        );
    }

    fn extract_type_dependencies(typ: &Type) -> Vec<String> {
        let mut deps = Vec::new();

        match typ {
            Type::Ident { to } => {
                if let PathOrIdent::Ident(ident) = to {
                    deps.push(ident.borrow_string().clone());
                }
                // PathOrIdent::Path is external, not a dependency for our graph
            },
            Type::Array { ty } => {
                match &ty.value {
                    Array::Unsized { ty: element_ty, .. } | Array::Sized { ty: element_ty, .. } => {
                        deps.extend(Self::extract_type_dependencies(&element_ty.value));
                    },
                }
            },
            Type::Union { ty } => {
                for type_item in &ty.value.types.values {
                    match &type_item.value.value {
                        IdentOrUnion::Ident(discriminant) => {
                            if let UnionDiscriminant::Ref(ty) = discriminant {
                                deps.extend(Self::extract_type_dependencies(&Type::Ident {
                                    to: ty.clone(),
                                }));
                            }
                        },
                        IdentOrUnion::Union { inner, .. } => {
                            let nested_type = Type::Union {
                                ty: Spanned::call_site(*inner.value.clone()),
                            };
                            deps.extend(Self::extract_type_dependencies(&nested_type));
                        },
                    }
                }
            },
            Type::UnionOr { lhs, rhs, .. } => {
                deps.extend(Self::extract_type_dependencies(&lhs.value));
                deps.extend(Self::extract_type_dependencies(&rhs.value));
            },
            Type::Paren { ty, .. } => {
                deps.extend(Self::extract_type_dependencies(&ty.value));
            },
            Type::OneOf { ty } => {
                for variant in &ty.value.variants.values {
                    deps.extend(Self::extract_type_dependencies(&variant.value.value));
                }
            },
            Type::Struct { ty } => {
                for field in &ty.value.fields.value.values {
                    deps.extend(Self::extract_type_dependencies(&field.value.typ));
                }
            },
            Type::Builtin { .. } | Type::Result { .. } => {
                // No dependencies
            },
            Type::TypeExpr { expr } => {
                // Type expressions have dependencies on their target types
                // These are handled during Phase 3.6 resolution
                deps.extend(Self::extract_type_expr_dependencies(&expr.value));
            },
        }

        deps
    }

    fn extract_type_expr_dependencies(expr: &crate::ast::type_expr::TypeExpr) -> Vec<String> {
        match expr {
            crate::ast::type_expr::TypeExpr::TypeRef { reference } => {
                if let PathOrIdent::Ident(ident) = reference {
                    vec![ident.borrow_string().clone()]
                } else {
                    vec![]
                }
            },
            crate::ast::type_expr::TypeExpr::FieldAccess { base, .. } => {
                Self::extract_type_expr_dependencies(&base.value)
            },
            crate::ast::type_expr::TypeExpr::Op(op) => {
                Self::extract_type_expr_op_dependencies(&op.value)
            },
        }
    }

    fn extract_type_expr_op_dependencies(op: &crate::ast::type_expr::TypeExprOp) -> Vec<String> {
        match op {
            crate::ast::type_expr::TypeExprOp::Pick { target, .. }
            | crate::ast::type_expr::TypeExprOp::Omit { target, .. }
            | crate::ast::type_expr::TypeExprOp::Partial { target, .. }
            | crate::ast::type_expr::TypeExprOp::Required { target, .. }
            | crate::ast::type_expr::TypeExprOp::Exclude { target, .. }
            | crate::ast::type_expr::TypeExprOp::Extract { target, .. }
            | crate::ast::type_expr::TypeExprOp::ArrayItem { target } => {
                Self::extract_type_expr_dependencies(target)
            },
        }
    }

    fn detect_cycles(&self) -> crate::Result<()> {
        use std::collections::HashSet;

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();
        let mut path_spans = Vec::new();

        for alias_name in self.aliases.keys() {
            if !visited.contains(alias_name) {
                self.detect_cycles_dfs(
                    alias_name,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut path_spans,
                )?;
            }
        }

        Ok(())
    }

    fn detect_cycles_dfs(
        &self,
        current: &str,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
        path: &mut Vec<String>,
        path_spans: &mut Vec<(crate::Span, std::path::PathBuf)>,
    ) -> crate::Result<()> {
        visited.insert(current.to_string());
        rec_stack.insert(current.to_string());
        path.push(current.to_string());

        if let Some(entry) = self.aliases.get(current) {
            // Track span for this alias
            let raw_span = entry.target_type.span.span();
            let span = crate::Span::new(raw_span.start, raw_span.end);
            path_spans.push((span, entry.source.clone()));

            for dep in &entry.dependencies {
                if !self.aliases.contains_key(dep) {
                    // external struct/enum/etc
                    continue;
                }

                if !visited.contains(dep) {
                    self.detect_cycles_dfs(dep, visited, rec_stack, path, path_spans)?;
                } else if rec_stack.contains(dep) {
                    let cycle_start = path.iter().position(|n| n == dep).unwrap();
                    let chain: Vec<String> = path[cycle_start..].to_vec();
                    let spans: Vec<_> = path_spans[cycle_start..].to_vec();

                    // Normalize cycle to start from lexicographically smallest element
                    // for deterministic error reporting
                    let (normalized_chain, normalized_spans) = normalize_cycle(chain, spans);

                    // Use the first alias's span in the normalized cycle
                    let (first_span, first_source) = &normalized_spans[0];
                    let source_content = std::fs::read_to_string(first_source)
                        .ok()
                        .map(std::sync::Arc::new);

                    // Build error with secondary labels for each cycle element
                    let mut err: kintsu_errors::CompilerError =
                        crate::ResolutionError::circular_alias(normalized_chain.clone())
                            .at(*first_span)
                            .build();

                    // Add secondary labels for the rest of the cycle elements
                    for (i, (span, _source)) in normalized_spans.iter().enumerate().skip(1) {
                        let label =
                            format!("cycle element {} of {}", i + 1, normalized_chain.len());
                        err = err.with_secondary_label(*span, label);
                    }

                    let err: crate::Error = err.into();
                    return Err(err.with_source_arc_if(first_source.clone(), source_content));
                }
            }
            path_spans.pop();
        }

        rec_stack.remove(current);
        path.pop();

        Ok(())
    }

    fn topological_sort(&self) -> crate::Result<Vec<String>> {
        let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();
        let mut adj_list: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for alias_name in self.aliases.keys() {
            in_degree.insert(alias_name.clone(), 0);
            adj_list.insert(alias_name.clone(), Vec::new());
        }

        for (alias_name, entry) in &self.aliases {
            for dep in &entry.dependencies {
                if self.aliases.contains_key(dep) {
                    *in_degree.get_mut(alias_name).unwrap() += 1;
                    adj_list
                        .get_mut(dep)
                        .unwrap()
                        .push(alias_name.clone());
                }
            }
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, degree)| **degree == 0_usize)
            .map(|(name, _)| name.clone())
            .collect();

        let mut sorted = Vec::new();

        while let Some(current) = queue.pop_front() {
            sorted.push(current.clone());

            if let Some(dependents) = adj_list.get(&current) {
                for dependent in dependents {
                    let degree = in_degree.get_mut(dependent).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }

        if sorted.len() != self.aliases.len() {
            return Err(crate::ResolutionError::circular_alias(vec![
                "<uncaught error>".to_string(),
            ])
            .unlocated()
            .build()
            .into());
        }

        Ok(sorted)
    }
}

impl TypeResolver {
    pub(super) async fn resolve_type_aliases(&mut self) -> crate::Result<()> {
        tracing::debug!("resolve_type_aliases: starting phase 3");

        let alias_graph = self.build_alias_graph().await?;

        tracing::debug!(
            "resolve_type_aliases: built graph with {} aliases",
            alias_graph.aliases.len()
        );

        alias_graph.detect_cycles()?;

        tracing::debug!("resolve_type_aliases: no cycles detected");

        let topo_order = alias_graph.topological_sort()?;

        tracing::debug!(
            "resolve_type_aliases: resolved {} aliases in topo order",
            topo_order.len()
        );

        for alias_name in &topo_order {
            self.resolve_single_alias(alias_name, &alias_graph)
                .await?;
        }

        tracing::debug!("resolve_type_aliases: phase 3 complete");
        Ok(())
    }

    async fn build_alias_graph(&self) -> crate::Result<AliasGraph> {
        let mut graph = AliasGraph::new();
        let ns = self.namespace.lock().await;

        for (child_name, child) in &ns.children {
            if let crate::ctx::common::NamespaceChild::Type(type_def) = &child.value {
                let alias_name = child_name.name.borrow_string().clone();
                let target_type = type_def.def.value.ty.clone();
                let source = child.source.clone();

                graph.add_alias(alias_name, target_type, source);
            }
        }

        Ok(graph)
    }

    async fn resolve_single_alias(
        &mut self,
        alias_name: &str,
        graph: &AliasGraph,
    ) -> crate::Result<()> {
        tracing::debug!("resolve_single_alias: resolving '{}'", alias_name);

        let alias_entry = graph
            .aliases
            .get(alias_name)
            .ok_or_else(|| -> crate::Error {
                crate::ResolutionError::undefined_type(alias_name.to_string())
                    .unlocated()
                    .build()
                    .into()
            })?;

        let resolved = self
            .resolve_type_deep(&alias_entry.target_type.value, &alias_entry.source, graph)
            .await?;

        self.resolution
            .resolved_aliases
            .insert(alias_name.to_string(), Spanned::call_site(resolved));

        Ok(())
    }

    #[allow(clippy::only_used_in_recursion)]
    fn resolve_type_deep<'a>(
        &'a self,
        typ: &'a Type,
        source: &'a std::path::PathBuf,
        graph: &'a AliasGraph,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<Type>> + 'a>> {
        Box::pin(async move {
            match typ {
                Type::Ident { to } => {
                    let ident_name = match to {
                        PathOrIdent::Ident(ident) => ident.borrow_string().clone(),
                        PathOrIdent::Path(_path) => {
                            return Ok(typ.clone());
                        },
                    };

                    if let Some(resolved) = self
                        .resolution
                        .resolved_aliases
                        .get(&ident_name)
                    {
                        tracing::debug!("resolve_type_deep: '{}' already resolved", ident_name);
                        return Ok(resolved.value.clone());
                    }

                    if let Some(alias_entry) = graph.aliases.get(&ident_name) {
                        tracing::debug!("resolve_type_deep: '{}' is alias, recursing", ident_name);
                        return self
                            .resolve_type_deep(
                                &alias_entry.target_type.value,
                                &alias_entry.source,
                                graph,
                            )
                            .await;
                    }

                    tracing::debug!("resolve_type_deep: '{}' is primitive reference", ident_name);
                    Ok(typ.clone())
                },
                Type::Array { ty } => {
                    let resolved_element = match &ty.value {
                        Array::Unsized { ty: element_ty, .. } => {
                            self.resolve_type_deep(&element_ty.value, source, graph)
                                .await?
                        },
                        Array::Sized { ty: element_ty, .. } => {
                            self.resolve_type_deep(&element_ty.value, source, graph)
                                .await?
                        },
                    };

                    let resolved_array = match &ty.value {
                        Array::Unsized { bracket, .. } => {
                            Array::Unsized {
                                ty: Box::new(Spanned::call_site(resolved_element)),
                                bracket: bracket.clone(),
                            }
                        },
                        Array::Sized { bracket, size, .. } => {
                            Array::Sized {
                                ty: Box::new(Spanned::call_site(resolved_element)),
                                bracket: bracket.clone(),
                                size: size.clone(),
                            }
                        },
                    };

                    Ok(Type::Array {
                        ty: Spanned::call_site(resolved_array),
                    })
                },
                Type::Union { ty } => {
                    let mut resolved_types = Vec::new();

                    for type_item in &ty.value.types.values {
                        let resolved = match &type_item.value.value {
                            IdentOrUnion::Ident(discriminant) => {
                                match discriminant {
                                    UnionDiscriminant::Anonymous(_anon) => {
                                        type_item.value.value.clone()
                                    },
                                    UnionDiscriminant::Ref(path_or_ident) => {
                                        let resolved_ty = self
                                            .resolve_type_deep(
                                                &Type::Ident {
                                                    to: path_or_ident.clone(),
                                                },
                                                source,
                                                graph,
                                            )
                                            .await?;

                                        match resolved_ty {
                                            Type::Ident { to } => {
                                                IdentOrUnion::Ident(UnionDiscriminant::Ref(to))
                                            },
                                            _ => IdentOrUnion::Ident(discriminant.clone()),
                                        }
                                    },
                                }
                            },
                            IdentOrUnion::Union { paren, inner } => {
                                let nested_type = Type::Union {
                                    ty: Spanned::call_site(*inner.value.clone()),
                                };
                                let resolved = self
                                    .resolve_type_deep(&nested_type, source, graph)
                                    .await?;

                                match resolved {
                                    Type::Union {
                                        ty: resolved_union_spanned,
                                    } => {
                                        let span = inner.span.span();
                                        IdentOrUnion::Union {
                                            paren: paren.clone(),
                                            inner: Spanned::new(
                                                span.start,
                                                span.end,
                                                Box::new(resolved_union_spanned.value),
                                            ),
                                        }
                                    },
                                    _ => type_item.value.value.clone(),
                                }
                            },
                        };

                        resolved_types.push(crate::tokens::RepeatedItem {
                            value: Spanned::call_site(resolved),
                            sep: type_item.sep.clone(),
                        });
                    }

                    Ok(Type::Union {
                        ty: Spanned::call_site(Union {
                            types: crate::tokens::Repeated {
                                values: resolved_types,
                            },
                        }),
                    })
                },
                Type::Paren { ty, .. } => {
                    self.resolve_type_deep(&ty.value, source, graph)
                        .await
                },
                Type::UnionOr { lhs, rhs, op } => {
                    let resolved_lhs = self
                        .resolve_type_deep(&lhs.value, source, graph)
                        .await?;
                    let resolved_rhs = self
                        .resolve_type_deep(&rhs.value, source, graph)
                        .await?;

                    Ok(Type::UnionOr {
                        lhs: Spanned::call_site(Box::new(resolved_lhs)),
                        op: op.clone(),
                        rhs: Spanned::call_site(Box::new(resolved_rhs)),
                    })
                },
                Type::OneOf { .. } | Type::Struct { .. } => {
                    // These contain nested types but are themselves primitive
                    // For now, don't recursively resolve their contents
                    // (that would happen in later phases)
                    Ok(typ.clone())
                },
                Type::Builtin { .. } | Type::Result { .. } => {
                    // Already primitive
                    Ok(typ.clone())
                },
                Type::TypeExpr { .. } => {
                    // Type expressions are resolved in Phase 3.6
                    // During alias resolution, pass through unchanged
                    Ok(typ.clone())
                },
            }
        })
    }
}
