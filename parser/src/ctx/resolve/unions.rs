use super::helpers::{NameContext, UnionRecord, UnionWorkingSet};
use crate::{
    ToTokens,
    ast::{
        array::Array,
        items::{OneOfDef, OperationDef, StructDef},
        ty::{PathOrIdent, Type},
        union::{IdentOrUnion, UnionDiscriminant},
        variadic::Variant,
    },
    ctx::common::{FromNamedSource, NamespaceChild, WithSource},
    defs::Spanned,
    tokens::Brace,
};
use std::sync::Arc;

pub(super) fn identify_from_child(
    name_gen: &mut NameContext,
    child: &FromNamedSource<NamespaceChild>,
) -> crate::Result<Vec<FromNamedSource<UnionRecord>>> {
    let source = child.source.clone();

    match &child.value {
        NamespaceChild::Struct(struct_def) => {
            identify_from_struct_fields(name_gen, struct_def, &source)
        },
        NamespaceChild::Operation(op_def) => identify_from_operation(name_gen, op_def, &source),
        NamespaceChild::OneOf(oneof_def) => identify_from_oneof(name_gen, oneof_def, &source),
        NamespaceChild::Type(alias_def) => {
            if let Type::Union { ty } = &alias_def.def.value.ty.value {
                let record = UnionRecord {
                    union_ref: Arc::new(ty.clone()),
                    context_stack: name_gen.stack.clone(),
                    in_oneof: false,
                    variant_index: None,
                };
                Ok(vec![record.with_source(source)])
            } else {
                identify_from_type(
                    name_gen,
                    &alias_def.def.value.ty.value,
                    &source,
                    false,
                    None,
                )
            }
        },
        _ => Ok(Vec::new()),
    }
}

fn identify_from_struct_fields(
    name_gen: &mut NameContext,
    struct_def: &StructDef,
    source: &std::path::PathBuf,
) -> crate::Result<Vec<FromNamedSource<UnionRecord>>> {
    let mut unions = Vec::new();

    for field in &struct_def.def.value.args.values {
        name_gen.push(field.value.name.borrow_string().clone());
        unions.extend(identify_from_type(
            name_gen,
            &field.value.typ,
            source,
            false,
            None,
        )?);
        name_gen.pop();
    }

    Ok(unions)
}

fn identify_from_operation(
    name_gen: &mut NameContext,
    op_def: &OperationDef,
    source: &std::path::PathBuf,
) -> crate::Result<Vec<FromNamedSource<UnionRecord>>> {
    let mut unions = Vec::new();

    name_gen.push(op_def.def.value.name.borrow_string().clone());

    if let Some(args) = op_def
        .def
        .value
        .args
        .as_ref()
        .map(|ok| &ok.value.values)
    {
        for arg in args {
            name_gen.push(arg.value.name.borrow_string().clone());
            unions.extend(identify_from_type(
                name_gen,
                &arg.value.typ,
                source,
                false,
                None,
            )?);
            name_gen.pop();
        }
    }

    name_gen.push("Result");
    unions.extend(identify_from_type(
        name_gen,
        &op_def.def.value.return_type.value,
        source,
        false,
        None,
    )?);
    name_gen.pop(); // result
    name_gen.pop(); // operation

    Ok(unions)
}

fn identify_from_oneof(
    name_gen: &mut NameContext,
    oneof_def: &OneOfDef,
    source: &std::path::PathBuf,
) -> crate::Result<Vec<FromNamedSource<UnionRecord>>> {
    let mut unions = Vec::new();

    for (idx, variant) in oneof_def
        .def
        .value
        .variants
        .values
        .iter()
        .enumerate()
    {
        match &variant.value.value {
            Variant::Tuple { inner, .. } => {
                unions.extend(identify_from_type(
                    name_gen,
                    inner,
                    source,
                    true,
                    Some(idx + 1), // 1-indexed
                )?);
            },
            Variant::LocalStruct { .. } => {
                // TODO: Handle anonymous struct variants
            },
            Variant::Unit { .. } => {
                // Unit variants have no type to extract unions from
            },
        }
    }

    Ok(unions)
}

fn identify_from_type(
    name_gen: &mut NameContext,
    typ: &Type,
    source: &std::path::PathBuf,
    in_oneof: bool,
    variant_index: Option<usize>,
) -> crate::Result<Vec<FromNamedSource<UnionRecord>>> {
    tracing::debug!("identify_from_type: type_variant={:?}", typ.type_name());
    let mut unions = Vec::new();

    match typ {
        Type::Union { ty } => {
            tracing::debug!(
                "identify_from_type: FOUND UNION, context={:?}",
                name_gen.stack
            );
            let record = UnionRecord {
                union_ref: Arc::new(ty.clone()),
                context_stack: name_gen.stack.clone(),
                in_oneof,
                variant_index,
            };
            unions.push(record.with_source(source.clone()));
        },
        Type::Paren { ty, .. } => {
            tracing::debug!("identify_from_type: unwrapping parenthesized type");
            unions.extend(identify_from_type(
                name_gen,
                ty.value.as_ref(),
                source,
                in_oneof,
                variant_index,
            )?);
        },
        Type::Array { ty } => {
            tracing::debug!("identify_from_type: checking array");
            unions.extend(identify_from_array(
                name_gen,
                &ty.value,
                source,
                in_oneof,
                variant_index,
            )?);
        },
        Type::OneOf { ty } => {
            tracing::debug!("identify_from_type: checking oneof variants");
            for (idx, variant_type) in ty.value.variants.values.iter().enumerate() {
                unions.extend(identify_from_type(
                    name_gen,
                    &variant_type.value.value,
                    source,
                    true,
                    Some(idx + 1),
                )?);
            }
        },
        Type::Struct { ty } => {
            tracing::debug!("identify_from_type: checking struct fields");
            for field in &ty.value.fields.value.values {
                name_gen.push(field.value.name.borrow_string().clone());
                unions.extend(identify_from_type(
                    name_gen,
                    &field.value.typ,
                    source,
                    in_oneof,
                    variant_index,
                )?);
                name_gen.pop();
            }
        },
        _ => {
            tracing::debug!("identify_from_type: no unions in this type");
        },
    }

    Ok(unions)
}

fn identify_from_array(
    name_gen: &mut NameContext,
    array: &Array,
    source: &std::path::PathBuf,
    in_oneof: bool,
    variant_index: Option<usize>,
) -> crate::Result<Vec<FromNamedSource<UnionRecord>>> {
    tracing::debug!(
        "identify_from_array: in_oneof={}, variant_index={:?}",
        in_oneof,
        variant_index
    );
    match array {
        Array::Sized { ty, .. } | Array::Unsized { ty, .. } => {
            tracing::debug!("identify_from_array: element_type={:?}", ty.value.display());
            identify_from_type(name_gen, &ty.value, source, in_oneof, variant_index)
        },
    }
}

#[tracing::instrument(
    target = "type-validation",
    level = "debug",
    skip(union_record, ns, resolved_aliases, source_path, source_content),
    fields(
        record = union_record.context_stack.join("."),
        discriminant = union_record.variant_index.unwrap_or(0),
        ns = ns.ctx.display(),
        source = %source_path.display()
    )
)]
pub(super) async fn validate_union_record(
    union_record: &UnionRecord,
    ns: &super::super::NamespaceCtx,
    resolved_aliases: &std::collections::BTreeMap<String, Spanned<Type>>,
    source_path: &std::path::Path,
    source_content: Option<&std::sync::Arc<String>>,
) -> crate::Result<()> {
    tracing::debug!(
        "validate_union_record: checking union '{}'",
        union_record.generate_name()
    );

    validate_union_operands(
        &union_record.union_ref.value.types,
        ns,
        resolved_aliases,
        source_path,
        source_content,
    )
    .await
}

#[tracing::instrument(
    target = "type-validation",
    level = "debug",
    skip(types, ns, resolved_aliases, source_path, source_content),
    fields(
        ns = ns.ctx.display(),
    )
)]
fn validate_union_operands<'a>(
    types: &'a crate::tokens::Repeated<IdentOrUnion, crate::tokens::AmpToken>,
    ns: &'a super::super::NamespaceCtx,
    resolved_aliases: &'a std::collections::BTreeMap<String, Spanned<Type>>,
    source_path: &'a std::path::Path,
    source_content: Option<&'a std::sync::Arc<String>>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + 'a>> {
    Box::pin(async move {
        for type_item in &types.values {
            validate_union_operand(
                &type_item.value.value,
                ns,
                resolved_aliases,
                source_path,
                source_content,
            )
            .await?;
        }
        Ok(())
    })
}

fn validate_union_operand<'a>(
    operand: &'a IdentOrUnion,
    ns: &'a super::super::NamespaceCtx,
    resolved_aliases: &'a std::collections::BTreeMap<String, Spanned<Type>>,
    source_path: &'a std::path::Path,
    source_content: Option<&'a std::sync::Arc<String>>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + 'a>> {
    Box::pin(async move {
        match operand {
            IdentOrUnion::Ident(discriminant) => {
                let (ident_name, ident_span) = match discriminant {
                    UnionDiscriminant::Anonymous(..) => return Ok(()),
                    UnionDiscriminant::Ref(ty) => {
                        match ty {
                            PathOrIdent::Ident(ident) => {
                                let sp = ident.span.span();
                                (
                                    ident.borrow_string().clone(),
                                    crate::Span::new(sp.start, sp.end),
                                )
                            },
                            PathOrIdent::Path(_) => {
                                // Paths are validated in Phase 8
                                return Ok(());
                            },
                        }
                    },
                };

                // Check if it's a resolved alias
                if let Some(resolved) = resolved_aliases.get(&ident_name) {
                    return validate_resolved_type(
                        &resolved.value,
                        &ident_name,
                        &ident_span,
                        ns,
                        resolved_aliases,
                        source_path,
                        source_content,
                    )
                    .await;
                }

                // Check namespace children
                if let Some(child) = ns
                    .children
                    .get(
                        &ns.ctx
                            .item(Spanned::call_site(crate::tokens::IdentToken::new(
                                ident_name.clone(),
                            ))),
                    )
                {
                    match &child.value {
                        NamespaceChild::Struct(_) => {
                            tracing::debug!(
                                "validate_union_operand: '{}' is valid struct",
                                ident_name
                            );
                            Ok(())
                        },
                        NamespaceChild::Enum(_) => {
                            let err: crate::Error = crate::UnionError::non_struct_operand(
                                ident_name,
                                "enum".to_string(),
                            )
                            .at(ident_span)
                            .build()
                            .into();
                            Err(err.with_source_arc_if(
                                source_path.to_path_buf(),
                                source_content.cloned(),
                            ))
                        },
                        NamespaceChild::OneOf(_) => {
                            let err: crate::Error = crate::UnionError::non_struct_operand(
                                ident_name,
                                "oneof".to_string(),
                            )
                            .at(ident_span)
                            .build()
                            .into();
                            Err(err.with_source_arc_if(
                                source_path.to_path_buf(),
                                source_content.cloned(),
                            ))
                        },
                        NamespaceChild::Type(_) => {
                            Err(crate::InternalError::internal(format!(
                                "Type alias '{}' should have been resolved in Phase 3",
                                ident_name
                            ))
                            .unlocated()
                            .build()
                            .into())
                        },
                        NamespaceChild::Error(_) => {
                            let err: crate::Error = crate::UnionError::non_struct_operand(
                                ident_name,
                                "error".to_string(),
                            )
                            .at(ident_span)
                            .build()
                            .into();
                            Err(err.with_source_arc_if(
                                source_path.to_path_buf(),
                                source_content.cloned(),
                            ))
                        },
                        NamespaceChild::Operation(_) => {
                            let err: crate::Error = crate::UnionError::non_struct_operand(
                                ident_name,
                                "operation".to_string(),
                            )
                            .at(ident_span)
                            .build()
                            .into();
                            Err(err.with_source_arc_if(
                                source_path.to_path_buf(),
                                source_content.cloned(),
                            ))
                        },
                        NamespaceChild::Namespace(_) => {
                            let err: crate::Error = crate::UnionError::non_struct_operand(
                                ident_name,
                                "namespace".to_string(),
                            )
                            .at(ident_span)
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
                        .at(ident_span)
                        .build()
                        .into();
                    Err(err.with_source_arc_if(source_path.to_path_buf(), source_content.cloned()))
                }
            },
            IdentOrUnion::Union { inner, .. } => {
                // Recursively validate nested union
                validate_union_operands(
                    &inner.value.types,
                    ns,
                    resolved_aliases,
                    source_path,
                    source_content,
                )
                .await
            },
        }
    })
}

#[allow(clippy::only_used_in_recursion)]
fn validate_resolved_type<'a>(
    resolved_type: &'a Type,
    type_name: &'a str,
    type_span: &'a crate::Span,
    ns: &'a super::super::NamespaceCtx,
    resolved_aliases: &'a std::collections::BTreeMap<String, Spanned<Type>>,
    source_path: &'a std::path::Path,
    source_content: Option<&'a std::sync::Arc<String>>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + 'a>> {
    Box::pin(async move {
        match resolved_type {
            Type::Struct { .. } => Ok(()),
            Type::Ident { to } => {
                let ident_str = match to {
                    crate::ast::ty::PathOrIdent::Ident(ident) => ident.borrow_string().clone(),
                    crate::ast::ty::PathOrIdent::Path(_) => return Ok(()), // Paths assumed valid
                };
                let inner_span = to.span();
                if let Some(child) = ns
                    .children
                    .get(
                        &ns.ctx
                            .item(Spanned::call_site(crate::tokens::IdentToken::new(
                                ident_str.clone(),
                            ))),
                    )
                {
                    match &child.value {
                        NamespaceChild::Struct(_) => Ok(()),
                        ty => {
                            let err: crate::Error = crate::UnionError::non_struct_operand(
                                type_name.to_string(),
                                ty.type_name(),
                            )
                            .at(*type_span)
                            .build()
                            .into();
                            Err(err.with_source_arc_if(
                                source_path.to_path_buf(),
                                source_content.cloned(),
                            ))
                        },
                    }
                } else {
                    let err: crate::Error = crate::ResolutionError::undefined_type(ident_str)
                        .at(inner_span)
                        .build()
                        .into();
                    Err(err.with_source_arc_if(source_path.to_path_buf(), source_content.cloned()))
                }
            },
            Type::Paren { ty, .. } => {
                validate_resolved_type(
                    &ty.value,
                    type_name,
                    type_span,
                    ns,
                    resolved_aliases,
                    source_path,
                    source_content,
                )
                .await
            },
            ty => {
                let err: crate::Error =
                    crate::UnionError::non_struct_operand(type_name.to_string(), ty.type_name())
                        .at(*type_span)
                        .build()
                        .into();
                Err(err.with_source_arc_if(source_path.to_path_buf(), source_content.cloned()))
            },
        }
    })
}

pub(super) async fn merge_union(
    union_record: &UnionRecord,
    ns: &super::super::NamespaceCtx,
    source_path: &std::path::Path,
) -> crate::Result<FromNamedSource<StructDef>> {
    tracing::debug!(
        "merge_union: processing union '{}'",
        union_record.generate_name()
    );

    let source_content = ns.sources.get(source_path).cloned();
    let mut working_set = UnionWorkingSet::new();

    // Extract union span for secondary labels in warnings
    let union_span = Some(crate::Span::new(
        union_record.union_ref.span().start,
        union_record.union_ref.span().end,
    ));

    for operand in &union_record.union_ref.value.types.values {
        merge_operand(
            &operand.value.value,
            ns,
            source_content.as_ref(),
            &mut working_set,
            union_span,
        )
        .await?;
    }

    let generated_name = union_record.generate_name();
    let source = source_path.to_path_buf();

    let merged_struct =
        working_set.into_struct_def(generated_name.clone(), source, Brace::call_site());

    tracing::debug!("merge_union: generated struct '{}'", generated_name);

    Ok(merged_struct)
}

fn merge_operand<'a>(
    operand: &'a IdentOrUnion,
    ns: &'a super::super::NamespaceCtx,
    source_content: Option<&'a std::sync::Arc<String>>,
    working_set: &'a mut UnionWorkingSet,
    union_span: Option<crate::Span>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + 'a>> {
    Box::pin(async move {
        match operand {
            IdentOrUnion::Ident(discriminant) => {
                match discriminant {
                    UnionDiscriminant::Anonymous(anon) => {
                        let fields = anon
                            .fields
                            .value
                            .values
                            .iter()
                            .map(|item| {
                                crate::tokens::RepeatedItem {
                                    value: item.value.clone(),
                                    sep: item.sep.clone(),
                                }
                            })
                            .collect();

                        working_set.merge_struct_with_warnings(
                            std::path::PathBuf::new(),
                            source_content,
                            "<anonymous>",
                            fields,
                            union_span,
                        );
                    },
                    UnionDiscriminant::Ref(path_or_ident) => {
                        let ident_name = match path_or_ident {
                            PathOrIdent::Ident(ident) => ident.borrow_string().clone(),
                            PathOrIdent::Path(_) => {
                                // Paths are handled later during validation & merging
                                return Ok(());
                            },
                        };

                        if let Some(child) =
                            ns.children
                                .get(&ns.ctx.item(Spanned::call_site(
                                    crate::tokens::IdentToken::new(ident_name.clone()),
                                )))
                            && let NamespaceChild::Struct(struct_def) = &child.value
                        {
                            let fields = struct_def
                                .def
                                .value
                                .args
                                .values
                                .iter()
                                .map(|item| {
                                    crate::tokens::RepeatedItem {
                                        value: item.value.clone(),
                                        sep: item.sep.clone(),
                                    }
                                })
                                .collect();

                            working_set.merge_struct_with_warnings(
                                child.source.clone(),
                                source_content,
                                &ident_name,
                                fields,
                                union_span,
                            );
                        }
                    },
                }
            },
            IdentOrUnion::Union { inner, .. } => {
                for nested_operand in &inner.value.types.values {
                    merge_operand(
                        &nested_operand.value.value,
                        ns,
                        source_content,
                        working_set,
                        union_span,
                    )
                    .await?;
                }
            },
        }

        Ok(())
    })
}
