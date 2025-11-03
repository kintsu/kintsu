use crate::{ast::ty::Type, ctx::common::NamespaceChild, defs::Spanned};

use super::TypeResolver;

impl TypeResolver {
    pub(super) async fn resolve_versions(&mut self) -> crate::Result<()> {
        let ns = self.namespace.lock().await;

        let namespace_version = ns
            .version
            .as_ref()
            .map(|v| v.version_spanned());

        for (item_ctx, child) in &ns.children {
            let item_name = item_ctx.name.borrow_string().clone();

            let item_version = match &child.value {
                NamespaceChild::Struct(struct_def) => {
                    Self::extract_version_from_meta(&struct_def.meta)
                },
                NamespaceChild::Enum(enum_def) => Self::extract_version_from_meta(&enum_def.meta),
                NamespaceChild::OneOf(oneof_def) => {
                    Self::extract_version_from_meta(&oneof_def.meta)
                },
                NamespaceChild::Type(type_def) => Self::extract_version_from_meta(&type_def.meta),
                NamespaceChild::Error(error_def) => {
                    Self::extract_version_from_meta(&error_def.meta)
                },
                NamespaceChild::Operation(op_def) => Self::extract_version_from_meta(&op_def.meta),
                NamespaceChild::Namespace(_) => {
                    // Nested namespaces handle their own versions
                    continue;
                },
            };

            let resolved_version = item_version
                .or_else(|| namespace_version.clone())
                .unwrap_or_else(Self::default_version);

            self.resolution
                .versions
                .insert(item_name, resolved_version);
        }

        Ok(())
    }

    fn extract_version_from_meta(
        meta_vec: &[Spanned<crate::ast::items::CommentOrMeta>]
    ) -> Option<Spanned<u32>> {
        for meta_or_comment in meta_vec {
            if let crate::ast::items::CommentOrMeta::Meta(meta_spanned) = &meta_or_comment.value {
                for item in &meta_spanned.value.meta {
                    if let crate::ast::meta::ItemMetaItem::Version(version_meta) = item {
                        return Some(version_meta.version_spanned());
                    }
                }
            }
        }
        None
    }

    fn default_version() -> Spanned<u32> {
        Spanned::call_site(1)
    }

    pub(super) async fn resolve_error_types(&mut self) -> crate::Result<()> {
        let ns = self.namespace.lock().await;

        let namespace_error = ns.error.as_ref().map(|e| {
            let error_name = e.error_name().to_string();
            Spanned::new(e.value.span().start, e.value.span().end, error_name)
        });

        for (item_ctx, child) in &ns.children {
            if let NamespaceChild::Operation(op_def) = &child.value {
                let item_name = item_ctx.name.borrow_string().clone();

                let is_fallible = Self::is_fallible_operation(&op_def.def.value.return_type.value);

                if !is_fallible {
                    // Non-fallible operation, no error type needed
                    continue;
                }

                let operation_error = Self::extract_error_from_meta(&op_def.meta);

                let resolved_error = match operation_error {
                    Some(err) => err,
                    None => {
                        match namespace_error.clone() {
                            Some(err) => err,
                            None => {
                                return Err(crate::Error::MissingErrorType {
                                    operation: item_ctx.name.clone(),
                                });
                            },
                        }
                    },
                };

                self.resolution
                    .errors
                    .insert(item_name, resolved_error);
            }
        }

        Ok(())
    }

    fn is_fallible_operation(return_type: &Type) -> bool {
        match return_type {
            Type::Result { .. } => true,
            Type::Paren { ty, .. } => Self::is_fallible_operation(&ty.value),
            _ => false,
        }
    }

    fn extract_error_from_meta(
        meta_vec: &[Spanned<crate::ast::items::CommentOrMeta>]
    ) -> Option<Spanned<String>> {
        for meta_or_comment in meta_vec {
            if let crate::ast::items::CommentOrMeta::Meta(meta_spanned) = &meta_or_comment.value {
                for item in &meta_spanned.value.meta {
                    if let crate::ast::meta::ItemMetaItem::Error(error_meta) = item {
                        let error_name = error_meta.error_name().to_string();
                        return Some(Spanned::new(
                            error_meta.span().start,
                            error_meta.span().end,
                            error_name,
                        ));
                    }
                }
            }
        }
        None
    }
}
