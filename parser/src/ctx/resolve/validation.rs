use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{
    ast::{ty::Type, variadic::Variant},
    defs::Spanned,
    tokens::ToTokens,
};

use super::TypeResolver;

impl TypeResolver {
    pub(super) async fn validate_all_references(&mut self) -> crate::Result<()> {
        tracing::debug!("validate_all_references: starting phase 8");

        let ns = self.namespace.lock().await;

        for child in ns.children.values() {
            let source_path = child.source.clone();
            let source_content = ns.sources.get(&source_path).cloned();

            match &child.value {
                super::super::NamespaceChild::Struct(struct_item) => {
                    // Validate duplicate fields
                    Self::validate_struct_duplicate_fields(
                        struct_item,
                        &source_path,
                        &source_content,
                    )?;

                    // Validate type references
                    for field in &struct_item.def.value.args.values {
                        Self::validate_type_reference(
                            &field.value.typ,
                            &ns,
                            &source_path,
                            &source_content,
                        )?;
                    }
                },
                super::super::NamespaceChild::Operation(op_item) => {
                    if let Some(params) = &op_item.def.value.args {
                        for param in &params.value.values {
                            Self::validate_type_reference(
                                &param.value.typ,
                                &ns,
                                &source_path,
                                &source_content,
                            )?;
                        }
                    }
                    Self::validate_type_reference(
                        &op_item.def.value.return_type,
                        &ns,
                        &source_path,
                        &source_content,
                    )?;
                },
                super::super::NamespaceChild::OneOf(oneof_item) => {
                    for variant in &oneof_item.def.value.variants.values {
                        match &variant.value.value {
                            Variant::Tuple { inner, .. } => {
                                Self::validate_type_reference(
                                    inner,
                                    &ns,
                                    &source_path,
                                    &source_content,
                                )?;
                            },
                            Variant::LocalStruct { inner, .. } => {
                                for field in &inner.value.fields.values {
                                    Self::validate_type_reference(
                                        &field.value.typ,
                                        &ns,
                                        &source_path,
                                        &source_content,
                                    )?;
                                }
                            },
                            Variant::Unit { .. } => {
                                // Unit variants have no type reference to validate
                            },
                        }
                    }
                },
                super::super::NamespaceChild::Error(error_item) => {
                    for variant in &error_item.def.value.variants.values {
                        match &variant.value.value {
                            Variant::Tuple { inner, .. } => {
                                Self::validate_type_reference(
                                    inner,
                                    &ns,
                                    &source_path,
                                    &source_content,
                                )?;
                            },
                            Variant::LocalStruct { inner, .. } => {
                                for field in &inner.value.fields.values {
                                    Self::validate_type_reference(
                                        &field.value.typ,
                                        &ns,
                                        &source_path,
                                        &source_content,
                                    )?;
                                }
                            },
                            Variant::Unit { .. } => {
                                // Unit variants have no type reference to validate
                            },
                        }
                    }
                },
                _ => {},
            }
        }

        tracing::debug!("validate_all_references: phase 8 complete");
        Ok(())
    }

    /// Validates that a struct does not have duplicate field names.
    /// Per ERR-0005: KTY3003 requires span on duplicate field.
    fn validate_struct_duplicate_fields(
        struct_item: &crate::ast::items::StructDef,
        source_path: &PathBuf,
        source_content: &Option<Arc<String>>,
    ) -> crate::Result<()> {
        let struct_name = struct_item.def.value.name.borrow_string();
        let mut seen: HashMap<String, crate::Span> = HashMap::new();

        for field in &struct_item.def.value.args.values {
            let field_name = field.value.name.borrow_string().clone();
            let field_span_raw = field.value.name.span();
            let field_span = crate::Span::new(field_span_raw.start, field_span_raw.end);

            if let Some(first_span) = seen.get(&field_name) {
                // Build error with secondary label pointing to first declaration
                let err = crate::TypeDefError::duplicate_field(
                    &field_name,
                    "struct",
                    struct_name.clone(),
                )
                .at(field_span)
                .build()
                .with_secondary_label(*first_span, "first declaration here");

                return if let Some(source) = source_content {
                    Err(err
                        .with_source_arc(source_path.clone(), Arc::clone(source))
                        .into())
                } else {
                    Err(err.into())
                };
            }
            seen.insert(field_name, field_span);
        }
        Ok(())
    }

    fn validate_type_reference(
        ty: &Type,
        ns: &super::super::NamespaceCtx,
        source_path: &PathBuf,
        source_content: &Option<Arc<String>>,
    ) -> crate::Result<()> {
        match ty {
            Type::Ident { to } => {
                if !ns.registry.is_valid(&ns.ctx, to, ns) {
                    let type_name = match to {
                        crate::ast::ty::PathOrIdent::Ident(name_token) => {
                            name_token.value.borrow_string().clone()
                        },
                        crate::ast::ty::PathOrIdent::Path(path) => {
                            <Spanned<_> as ToTokens>::display(path)
                        },
                    };
                    let span = to.span();

                    tracing::error!("Undefined type reference: {}", type_name);

                    let err = crate::ResolutionError::undefined_type(type_name)
                        .at(span)
                        .build();
                    return if let Some(source) = source_content {
                        Err(err
                            .with_source_arc(source_path.clone(), Arc::clone(source))
                            .into())
                    } else {
                        Err(err.into())
                    };
                }
            },
            Type::Array { ty } => {
                match &ty.value {
                    crate::ast::array::Array::Sized { ty: inner, .. } => {
                        Self::validate_type_reference(inner, ns, source_path, source_content)?;
                    },
                    crate::ast::array::Array::Unsized { ty: inner, .. } => {
                        Self::validate_type_reference(inner, ns, source_path, source_content)?;
                    },
                }
            },
            Type::Union { ty } => {
                for ty_item in &ty.value.types.values {
                    match &ty_item.value.value {
                        crate::ast::union::IdentOrUnion::Ident(union_disc) => {
                            // Validate the identifier reference
                            match union_disc {
                                crate::ast::union::UnionDiscriminant::Ref(path_or_ident) => {
                                    if !ns
                                        .registry
                                        .is_valid(&ns.ctx, path_or_ident, ns)
                                    {
                                        let type_name = match path_or_ident {
                                            crate::ast::ty::PathOrIdent::Ident(name_token) => {
                                                name_token.value.borrow_string().clone()
                                            },
                                            crate::ast::ty::PathOrIdent::Path(path) => {
                                                path.display()
                                            },
                                        };
                                        let span = path_or_ident.span();

                                        tracing::error!(
                                            "Undefined type reference in union: {}",
                                            type_name
                                        );
                                        let err = crate::ResolutionError::undefined_type(type_name)
                                            .at(span)
                                            .build();
                                        return if let Some(source) = source_content {
                                            Err(err
                                                .with_source_arc(
                                                    source_path.clone(),
                                                    Arc::clone(source),
                                                )
                                                .into())
                                        } else {
                                            Err(err.into())
                                        };
                                    }
                                },
                                crate::ast::union::UnionDiscriminant::Anonymous(anon_struct) => {
                                    for field in &anon_struct.fields.values {
                                        Self::validate_type_reference(
                                            &field.value.typ,
                                            ns,
                                            source_path,
                                            source_content,
                                        )?;
                                    }
                                },
                            }
                        },
                        crate::ast::union::IdentOrUnion::Union { inner, .. } => {
                            for nested_item in &inner.value.types.values {
                                match &nested_item.value.value {
                                    crate::ast::union::IdentOrUnion::Ident(nested_disc) => {
                                        if let crate::ast::union::UnionDiscriminant::Ref(nested_ref) =
                                            nested_disc
                                            && !ns.registry.is_valid(&ns.ctx, nested_ref, ns)
                                        {
                                            let type_name = match nested_ref {
                                                crate::ast::ty::PathOrIdent::Ident(name_token) => {
                                                    name_token.value.borrow_string().clone()
                                                },
                                                crate::ast::ty::PathOrIdent::Path(path) => {
                                                    path.display()
                                                },
                                            };
                                            let span = nested_ref.span();

                                            tracing::error!("Undefined type: {}", type_name);
                                            let err =
                                                crate::ResolutionError::undefined_type(type_name)
                                                    .at(span)
                                                    .build();
                                            return if let Some(source) = source_content {
                                                Err(err
                                                    .with_source_arc(
                                                        source_path.clone(),
                                                        Arc::clone(source),
                                                    )
                                                    .into())
                                            } else {
                                                Err(err.into())
                                            };
                                        }
                                    },
                                    crate::ast::union::IdentOrUnion::Union { .. } => {
                                        // todo: implement because this is valid
                                    },
                                }
                            }
                        },
                    }
                }
            },
            Type::Paren { ty, .. } => {
                Self::validate_type_reference(&ty.value, ns, source_path, source_content)?;
            },
            Type::Result { ty, .. } => {
                Self::validate_type_reference(&ty.value, ns, source_path, source_content)?;
            },
            Type::Struct { ty } => {
                for field in &ty.value.fields.values {
                    Self::validate_type_reference(
                        &field.value.typ,
                        ns,
                        source_path,
                        source_content,
                    )?;
                }
            },
            Type::OneOf { ty } => {
                for variant in &ty.value.variants.values {
                    Self::validate_type_reference(&variant.value, ns, source_path, source_content)?;
                }
            },
            Type::UnionOr { lhs, rhs, .. } => {
                Self::validate_type_reference(&lhs.value, ns, source_path, source_content)?;
                Self::validate_type_reference(&rhs.value, ns, source_path, source_content)?;
            },
            Type::TypeExpr { expr } => {
                // Type expressions are validated during Phase 3.6 resolution
                // Here we just validate references within the expression
                Self::validate_type_expr_references(&expr.value, ns, source_path, source_content)?;
            },
            Type::Builtin { .. } => {
                // always valid
            },
        }
        Ok(())
    }

    fn validate_type_expr_references(
        expr: &crate::ast::type_expr::TypeExpr,
        ns: &super::super::NamespaceCtx,
        source_path: &PathBuf,
        source_content: &Option<Arc<String>>,
    ) -> crate::Result<()> {
        use crate::ast::type_expr::{TypeExpr, TypeExprOp};
        match expr {
            TypeExpr::TypeRef { reference } => {
                if !ns.registry.is_valid(&ns.ctx, reference, ns) {
                    let type_name = match reference {
                        crate::ast::ty::PathOrIdent::Ident(name_token) => {
                            name_token.value.borrow_string().clone()
                        },
                        crate::ast::ty::PathOrIdent::Path(path) => {
                            <crate::defs::Spanned<_> as crate::ToTokens>::display(path)
                        },
                    };
                    let span = reference.span();
                    let err = crate::ResolutionError::undefined_type(type_name)
                        .at(span)
                        .build();
                    return if let Some(source) = source_content {
                        Err(err
                            .with_source_arc(source_path.clone(), Arc::clone(source))
                            .into())
                    } else {
                        Err(err.into())
                    };
                }
            },
            TypeExpr::FieldAccess { base, .. } => {
                Self::validate_type_expr_references(&base.value, ns, source_path, source_content)?;
            },
            TypeExpr::Op(op) => {
                match &op.value {
                    TypeExprOp::Pick { target, .. }
                    | TypeExprOp::Omit { target, .. }
                    | TypeExprOp::Partial { target, .. }
                    | TypeExprOp::Required { target, .. }
                    | TypeExprOp::Exclude { target, .. }
                    | TypeExprOp::Extract { target, .. }
                    | TypeExprOp::ArrayItem { target } => {
                        Self::validate_type_expr_references(
                            target,
                            ns,
                            source_path,
                            source_content,
                        )?;
                    },
                }
            },
        }
        Ok(())
    }
}
