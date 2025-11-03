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
            match &child.value {
                super::super::NamespaceChild::Struct(struct_item) => {
                    for field in &struct_item.def.value.args.values {
                        Self::validate_type_reference(&field.value.typ, &ns)?;
                    }
                },
                super::super::NamespaceChild::Operation(op_item) => {
                    if let Some(params) = &op_item.def.value.args {
                        for param in &params.value.values {
                            Self::validate_type_reference(&param.value.typ, &ns)?;
                        }
                    }
                    Self::validate_type_reference(&op_item.def.value.return_type, &ns)?;
                },
                super::super::NamespaceChild::OneOf(oneof_item) => {
                    for variant in &oneof_item.def.value.variants.values {
                        match &variant.value.value {
                            Variant::Tuple { inner, .. } => {
                                Self::validate_type_reference(inner, &ns)?;
                            },
                            Variant::LocalStruct { inner, .. } => {
                                for field in &inner.value.fields.values {
                                    Self::validate_type_reference(&field.value.typ, &ns)?;
                                }
                            },
                        }
                    }
                },
                super::super::NamespaceChild::Error(error_item) => {
                    for variant in &error_item.def.value.variants.values {
                        match &variant.value.value {
                            Variant::Tuple { inner, .. } => {
                                Self::validate_type_reference(inner, &ns)?;
                            },
                            Variant::LocalStruct { inner, .. } => {
                                for field in &inner.value.fields.values {
                                    Self::validate_type_reference(&field.value.typ, &ns)?;
                                }
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

    fn validate_type_reference(
        ty: &Type,
        ns: &super::super::NamespaceCtx,
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

                    tracing::error!("Undefined type reference: {}", type_name);

                    return Err(crate::Error::UndefinedType { name: type_name });
                }
            },
            Type::Array { ty } => {
                match &ty.value {
                    crate::ast::array::Array::Sized { ty: inner, .. } => {
                        Self::validate_type_reference(inner, ns)?;
                    },
                    crate::ast::array::Array::Unsized { ty: inner, .. } => {
                        Self::validate_type_reference(inner, ns)?;
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

                                        tracing::error!(
                                            "Undefined type reference in union: {}",
                                            type_name
                                        );
                                        return Err(crate::Error::UndefinedType {
                                            name: type_name,
                                        });
                                    }
                                },
                                crate::ast::union::UnionDiscriminant::Anonymous(anon_struct) => {
                                    for field in &anon_struct.fields.values {
                                        Self::validate_type_reference(&field.value.typ, ns)?;
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

                                            tracing::error!("Undefined type: {}", type_name);
                                            return Err(crate::Error::UndefinedType {
                                                name: type_name,
                                            });
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
                Self::validate_type_reference(&ty.value, ns)?;
            },
            Type::Result { ty, .. } => {
                Self::validate_type_reference(&ty.value, ns)?;
            },
            Type::Struct { ty } => {
                for field in &ty.value.fields.values {
                    Self::validate_type_reference(&field.value.typ, ns)?;
                }
            },
            Type::OneOf { ty } => {
                for variant in &ty.value.variants.values {
                    Self::validate_type_reference(&variant.value, ns)?;
                }
            },
            Type::Builtin { .. } => {
                // always valid
            },
        }
        Ok(())
    }
}
