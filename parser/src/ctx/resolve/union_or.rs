//! Union Or Resolution Phase (Phase 3.5)
//!
//! Resolves `Type::UnionOr` binary compositions into merged struct types.
//! This phase runs after `resolve_type_aliases` and before `validate_unions`.
//!
//! **Spec references:** RFC-0016, implement-specs.md Phase 2

use std::collections::BTreeMap;

use crate::{
    Token,
    ast::{
        comment::CommentStream,
        one_of::AnonymousOneOf,
        strct::Arg,
        ty::{PathOrIdent, Type},
    },
    ctx::{common::NamespaceChild, resolve::TypeResolver},
    defs::Spanned,
    tokens::{KwOneofToken, Repeated, RepeatedItem, ToTokens},
};

/// Represents a field in the merged result with potential type conflicts
#[derive(Clone)]
struct MergedField {
    /// Original field definition
    arg: Spanned<Arg>,
    /// Types seen for this field across operands (for conflict detection)
    types: Vec<Type>,
    /// Separator token
    sep: Option<Spanned<Token![,]>>,
}

impl TypeResolver {
    /// Resolve union or types (Phase 3.5)
    ///
    /// Converts `A &| B` compositions into merged structs with oneof for conflicts.
    /// Per RFC-0016: non-conflicting fields pass through, conflicts become oneof.
    pub(super) async fn resolve_union_or(&mut self) -> crate::Result<()> {
        tracing::debug!("resolve_union_or: starting phase 3.5");

        let ns = self.namespace.lock().await;

        // Find all type aliases that contain UnionOr
        let union_or_aliases: Vec<_> = ns
            .children
            .iter()
            .filter_map(|(ctx, child)| {
                if let NamespaceChild::Type(type_def) = &child.value {
                    if contains_union_or(&type_def.def.value.ty.value) {
                        return Some((
                            ctx.name.borrow_string().clone(),
                            type_def.def.value.ty.clone(),
                            child.source.clone(),
                        ));
                    }
                }
                None
            })
            .collect();

        drop(ns);

        for (alias_name, type_expr, _source) in union_or_aliases {
            tracing::debug!("resolve_union_or: processing alias '{}'", alias_name);

            let operands = flatten_union_or(&type_expr.value);
            tracing::debug!(
                "resolve_union_or: flattened {} operands for '{}'",
                operands.len(),
                alias_name
            );

            // Validate and merge operands
            let merged_type = self
                .merge_union_or_operands(&operands)
                .await?;

            // Store the resolved type
            self.resolution
                .resolved_aliases
                .insert(alias_name, Spanned::call_site(merged_type));
        }

        tracing::debug!("resolve_union_or: phase 3.5 complete");
        Ok(())
    }

    /// Merge operands from a union or expression into a single type
    async fn merge_union_or_operands(
        &self,
        operands: &[Spanned<Type>],
    ) -> crate::Result<Type> {
        let ns = self.namespace.lock().await;

        let mut merged_fields: BTreeMap<String, MergedField> = BTreeMap::new();

        for operand in operands {
            let fields = self
                .extract_struct_fields(&operand.value, &ns)
                .await?;

            for (field_name, (arg, sep)) in fields {
                merged_fields
                    .entry(field_name)
                    .and_modify(|existing| {
                        // Track multiple types for conflict detection
                        existing.types.push(arg.value.typ.clone());
                    })
                    .or_insert_with(|| {
                        MergedField {
                            arg: arg.clone(),
                            types: vec![arg.value.typ.clone()],
                            sep,
                        }
                    });
            }
        }

        // Build the merged struct's fields
        let result_fields: Vec<RepeatedItem<Arg, Token![,]>> = merged_fields
            .into_values()
            .map(|field| {
                let final_type = if field.types.len() > 1 {
                    // Multiple types → create oneof
                    build_oneof_type(&field.types)
                } else {
                    field.arg.value.typ.clone()
                };

                RepeatedItem {
                    value: Spanned::call_site(Arg {
                        comments: CommentStream::default(),
                        name: field.arg.value.name.clone(),
                        sep: field.arg.value.sep.clone(),
                        typ: final_type,
                    }),
                    sep: field.sep,
                }
            })
            .collect();

        // Return as anonymous struct type
        Ok(Type::Struct {
            ty: Spanned::call_site(crate::ast::anonymous::AnonymousStruct {
                brace: crate::tokens::Brace::call_site(),
                fields: Spanned::call_site(Repeated {
                    values: result_fields,
                }),
            }),
        })
    }

    /// Extract fields from a type that should be a struct
    async fn extract_struct_fields(
        &self,
        typ: &Type,
        ns: &crate::ctx::NamespaceCtx,
    ) -> crate::Result<Vec<(String, (Spanned<Arg>, Option<Spanned<Token![,]>>))>> {
        match typ {
            Type::Struct { ty } => {
                Ok(ty
                    .value
                    .fields
                    .value
                    .values
                    .iter()
                    .map(|item| {
                        (
                            item.value.name.borrow_string().clone(),
                            (item.value.clone(), item.sep.clone()),
                        )
                    })
                    .collect())
            },
            Type::Ident { to } => {
                let ident_name = match to {
                    PathOrIdent::Ident(ident) => ident.borrow_string().clone(),
                    PathOrIdent::Path(_) => {
                        return Err(crate::Error::UnionOperandMustBeStruct {
                            found_type: "path reference".to_string(),
                            operand_name: to.to_string(),
                        });
                    },
                };

                // Check resolved aliases first
                if let Some(resolved) = self
                    .resolution
                    .resolved_aliases
                    .get(&ident_name)
                {
                    return Box::pin(self.extract_struct_fields(&resolved.value, ns)).await;
                }

                // Then check namespace children
                let child_ctx = ns
                    .ctx
                    .item(Spanned::call_site(crate::tokens::IdentToken::new(
                        ident_name.clone(),
                    )));

                if let Some(child) = ns.children.get(&child_ctx) {
                    match &child.value {
                        NamespaceChild::Struct(struct_def) => {
                            Ok(struct_def
                                .def
                                .value
                                .args
                                .values
                                .iter()
                                .map(|item| {
                                    (
                                        item.value.name.borrow_string().clone(),
                                        (item.value.clone(), item.sep.clone()),
                                    )
                                })
                                .collect())
                        },
                        NamespaceChild::Type(type_def) => {
                            // Follow type alias
                            Box::pin(self.extract_struct_fields(&type_def.def.value.ty.value, ns))
                                .await
                        },
                        other => {
                            Err(crate::Error::UnionOperandMustBeStruct {
                                found_type: other.type_name(),
                                operand_name: ident_name,
                            })
                        },
                    }
                } else {
                    Err(crate::Error::UndefinedType { name: ident_name })
                }
            },
            Type::Paren { ty, .. } => Box::pin(self.extract_struct_fields(&ty.value, ns)).await,
            other => {
                Err(crate::Error::UnionOperandMustBeStruct {
                    found_type: other.type_name(),
                    operand_name: "<expression>".to_string(),
                })
            },
        }
    }
}

/// Check if a type contains any UnionOr nodes
fn contains_union_or(typ: &Type) -> bool {
    match typ {
        Type::UnionOr { .. } => true,
        Type::Paren { ty, .. } => contains_union_or(&ty.value),
        Type::Array { ty } => {
            match &ty.value {
                crate::ast::array::Array::Unsized { ty, .. }
                | crate::ast::array::Array::Sized { ty, .. } => contains_union_or(&ty.value),
            }
        },
        _ => false,
    }
}

/// Flatten a UnionOr tree into a list of operands (left-to-right per RFC-0016)
fn flatten_union_or(typ: &Type) -> Vec<Spanned<Type>> {
    match typ {
        Type::UnionOr { lhs, rhs, .. } => {
            let mut result = flatten_union_or(&lhs.value);
            result.extend(flatten_union_or(&rhs.value));
            result
        },
        Type::Paren { ty, .. } => flatten_union_or(&ty.value),
        other => vec![Spanned::call_site(other.clone())],
    }
}

/// Build a oneof type from multiple conflicting field types
fn build_oneof_type(types: &[Type]) -> Type {
    // Deduplicate types by their token representation
    let mut seen = std::collections::HashSet::new();
    let cfg = crate::fmt::FormatConfig::default();
    let unique_types: Vec<_> = types
        .iter()
        .filter(|t| {
            let mut printer = crate::fmt::Printer::new(&cfg);
            t.write(&mut printer);
            seen.insert(printer.buf)
        })
        .cloned()
        .collect();

    if unique_types.len() == 1 {
        return unique_types[0].clone();
    }

    let variants: Vec<RepeatedItem<Type, Token![|]>> = unique_types
        .iter()
        .enumerate()
        .map(|(i, t)| {
            RepeatedItem {
                value: Spanned::call_site(t.clone()),
                sep: if i < unique_types.len() - 1 {
                    Some(Spanned::call_site(crate::tokens::toks::PipeToken::new()))
                } else {
                    None
                },
            }
        })
        .collect();

    Type::OneOf {
        ty: Spanned::call_site(AnonymousOneOf {
            kw: Spanned::call_site(KwOneofToken::new()),
            variants: Spanned::call_site(Repeated { values: variants }),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_union_or() {
        // Simple ident - no union or
        let ident_type = Type::Ident {
            to: PathOrIdent::Ident(Spanned::call_site(crate::tokens::IdentToken::new(
                "Foo".to_string(),
            ))),
        };
        assert!(!contains_union_or(&ident_type));
    }

    #[test]
    fn test_flatten_single_operand() {
        let ident_type = Type::Ident {
            to: PathOrIdent::Ident(Spanned::call_site(crate::tokens::IdentToken::new(
                "Foo".to_string(),
            ))),
        };
        let result = flatten_union_or(&ident_type);
        assert_eq!(result.len(), 1);
    }
}
