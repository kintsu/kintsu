//! Union Or Resolution Phase (Phase 3.5)
//!
//! Resolves `Type::UnionOr` binary compositions into merged struct types.
//! This phase runs after `resolve_type_aliases` and before `validate_unions`.
//!
//! **Spec references:** RFC-0016

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
    arg: Spanned<Arg>,
    types: Vec<Type>,
    sep: Option<Spanned<Token![,]>>,
}

fn types_are_equivalent(
    a: &Type,
    b: &Type,
) -> bool {
    let cfg = crate::fmt::FormatConfig::default();
    let mut printer_a = crate::fmt::Printer::new(&cfg);
    let mut printer_b = crate::fmt::Printer::new(&cfg);
    a.write(&mut printer_a);
    b.write(&mut printer_b);
    printer_a.buf == printer_b.buf
}

fn operand_display_name(typ: &Type) -> String {
    match typ {
        Type::Ident { to } => to.to_string(),
        Type::Struct { .. } => "<anonymous>".to_string(),
        Type::Paren { ty, .. } => operand_display_name(&ty.value),
        _ => "<type>".to_string(),
    }
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
                if let NamespaceChild::Type(type_def) = &child.value
                    && contains_union_or(&type_def.def.value.ty.value)
                {
                    return Some((
                        ctx.name.borrow_string().clone(),
                        type_def.def.value.ty.clone(),
                        child.source.clone(),
                    ));
                }
                None
            })
            .collect();

        drop(ns);

        for (alias_name, type_expr, source_path) in union_or_aliases {
            tracing::debug!("resolve_union_or: processing alias '{}'", alias_name);

            let operands = flatten_union_or(&type_expr.value);
            tracing::debug!(
                "resolve_union_or: flattened {} operands for '{}'",
                operands.len(),
                alias_name
            );

            // Get source content for error reporting
            let ns = self.namespace.lock().await;
            let source_content = ns.sources.get(&source_path).cloned();
            drop(ns);

            // Validate and merge operands
            let merged_type = self
                .merge_union_or_operands(&operands, &source_path, source_content.as_ref())
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
        source_path: &std::path::Path,
        source_content: Option<&std::sync::Arc<String>>,
    ) -> crate::Result<Type> {
        let ns = self.namespace.lock().await;

        let mut merged_fields: BTreeMap<String, MergedField> = BTreeMap::new();

        for operand in operands {
            let current_operand_name = operand_display_name(&operand.value);

            let fields = self
                .extract_struct_fields_spanned(operand, &ns, source_path, source_content)
                .await?;

            for (field_name, (arg, sep)) in fields {
                merged_fields
                    .entry(field_name.clone())
                    .and_modify(|existing| {
                        let new_type = &arg.value.typ;
                        let existing_type = existing
                            .types
                            .last()
                            .expect("at least one type");
                        let is_same = types_are_equivalent(existing_type, new_type);

                        let span = crate::Span::new(arg.span().start, arg.span().end);

                        // Get type strings for error messages
                        let cfg = crate::fmt::FormatConfig::default();
                        let existing_type_str = {
                            let mut printer = crate::fmt::Printer::new(&cfg);
                            existing_type.write(&mut printer);
                            printer.buf
                        };
                        let new_type_str = {
                            let mut printer = crate::fmt::Printer::new(&cfg);
                            new_type.write(&mut printer);
                            printer.buf
                        };

                        if is_same {
                            let mut warning: kintsu_errors::CompilerError =
                                crate::UnionError::field_shadowed(
                                    &field_name,
                                    &current_operand_name,
                                    &existing_type_str,
                                )
                                .at(span)
                                .build();
                            if let Some(content) = source_content {
                                warning = warning
                                    .with_source_arc(source_path.to_path_buf(), content.clone());
                            }
                            kintsu_events::emit_warning(warning);
                        } else {
                            let mut warning: kintsu_errors::CompilerError =
                                crate::UnionError::field_conflict(
                                    &field_name,
                                    &existing_type_str,
                                    &new_type_str,
                                )
                                .at(span)
                                .build();
                            if let Some(content) = source_content {
                                warning = warning
                                    .with_source_arc(source_path.to_path_buf(), content.clone());
                            }
                            kintsu_events::emit_warning(warning);
                        }

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

    /// Extract fields from a spanned type, attaching span to any errors
    async fn extract_struct_fields_spanned(
        &self,
        typ: &Spanned<Type>,
        ns: &crate::ctx::NamespaceCtx,
        source_path: &std::path::Path,
        source_content: Option<&std::sync::Arc<String>>,
    ) -> crate::Result<Vec<(String, (Spanned<Arg>, Option<Spanned<Token![,]>>))>> {
        self.extract_struct_fields_inner(&typ.value, typ.span(), ns, source_path, source_content)
            .await
    }

    /// Extract fields from a type that should be a struct
    async fn extract_struct_fields_inner(
        &self,
        typ: &Type,
        operand_span: &crate::defs::span::RawSpan,
        ns: &crate::ctx::NamespaceCtx,
        source_path: &std::path::Path,
        source_content: Option<&std::sync::Arc<String>>,
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
                        let err: crate::Error = crate::UnionError::non_struct_operand(
                            to.to_string(),
                            "path reference".to_string(),
                        )
                        .at(crate::Span::new(operand_span.start, operand_span.end))
                        .build()
                        .into();
                        return Err(err.with_source_arc_if(
                            source_path.to_path_buf(),
                            source_content.cloned(),
                        ));
                    },
                };

                // Check resolved aliases first
                if let Some(resolved) = self
                    .resolution
                    .resolved_aliases
                    .get(&ident_name)
                {
                    return Box::pin(self.extract_struct_fields_inner(
                        &resolved.value,
                        operand_span,
                        ns,
                        source_path,
                        source_content,
                    ))
                    .await;
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
                            Box::pin(self.extract_struct_fields_inner(
                                &type_def.def.value.ty.value,
                                operand_span,
                                ns,
                                source_path,
                                source_content,
                            ))
                            .await
                        },
                        other => {
                            let err: crate::Error = crate::UnionError::non_struct_operand(
                                ident_name,
                                other.type_name(),
                            )
                            .at(crate::Span::new(operand_span.start, operand_span.end))
                            .build()
                            .into();
                            Err(err.with_source_arc_if(
                                source_path.to_path_buf(),
                                source_content.cloned(),
                            ))
                        },
                    }
                } else {
                    let err: crate::Error = crate::ResolutionError::undefined_type(ident_name)
                        .at(crate::Span::new(operand_span.start, operand_span.end))
                        .build()
                        .into();
                    Err(err.with_source_arc_if(source_path.to_path_buf(), source_content.cloned()))
                }
            },
            Type::Paren { ty, .. } => {
                Box::pin(self.extract_struct_fields_inner(
                    &ty.value,
                    operand_span,
                    ns,
                    source_path,
                    source_content,
                ))
                .await
            },
            other => {
                let err: crate::Error = crate::UnionError::non_struct_operand(
                    "<expression>".to_string(),
                    other.type_name(),
                )
                .at(crate::Span::new(operand_span.start, operand_span.end))
                .build()
                .into();
                Err(err.with_source_arc_if(source_path.to_path_buf(), source_content.cloned()))
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
