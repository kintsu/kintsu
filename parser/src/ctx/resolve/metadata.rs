use std::{path::PathBuf, sync::Arc};

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
            let source_path = child.source.clone();
            let source_content = ns.sources.get(&source_path).cloned();

            let item_version = match &child.value {
                NamespaceChild::Struct(struct_def) => {
                    Self::extract_version_from_meta_validated(
                        &struct_def.meta,
                        &source_path,
                        &source_content,
                    )?
                },
                NamespaceChild::Enum(enum_def) => {
                    Self::extract_version_from_meta_validated(
                        &enum_def.meta,
                        &source_path,
                        &source_content,
                    )?
                },
                NamespaceChild::OneOf(oneof_def) => {
                    Self::extract_version_from_meta_validated(
                        &oneof_def.meta,
                        &source_path,
                        &source_content,
                    )?
                },
                NamespaceChild::Type(type_def) => {
                    Self::extract_version_from_meta_validated(
                        &type_def.meta,
                        &source_path,
                        &source_content,
                    )?
                },
                NamespaceChild::Error(error_def) => {
                    Self::extract_version_from_meta_validated(
                        &error_def.meta,
                        &source_path,
                        &source_content,
                    )?
                },
                NamespaceChild::Operation(op_def) => {
                    Self::extract_version_from_meta_validated(
                        &op_def.meta,
                        &source_path,
                        &source_content,
                    )?
                },
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

    /// Extract version from meta, validating there's at most one version attribute.
    /// Returns KMT3001 error if duplicate version attributes found.
    fn extract_version_from_meta_validated(
        meta_vec: &[Spanned<crate::ast::items::CommentOrMeta>],
        source_path: &PathBuf,
        source_content: &Option<Arc<String>>,
    ) -> crate::Result<Option<Spanned<u32>>> {
        let mut versions: Vec<(Spanned<u32>, crate::Span)> = Vec::new();

        for meta_or_comment in meta_vec {
            if let crate::ast::items::CommentOrMeta::Meta(meta_spanned) = &meta_or_comment.value {
                for item in &meta_spanned.value.meta {
                    if let crate::ast::meta::ItemMetaItem::Version(version_meta) = item {
                        let version = version_meta.version_spanned();
                        let raw_span = version_meta.span();
                        let span = crate::Span::new(raw_span.start, raw_span.end);
                        versions.push((version, span));
                    }
                }
            }
        }

        if versions.len() > 1 {
            let values: Vec<usize> = versions
                .iter()
                .map(|(v, _)| v.value as usize)
                .collect();
            let (_, second_span) = &versions[1];
            let err = crate::MetadataError::version_conflict(values)
                .at(*second_span)
                .build();

            return if let Some(source) = source_content {
                Err(err
                    .with_source_arc(source_path.clone(), Arc::clone(source))
                    .into())
            } else {
                Err(err.into())
            };
        }

        Ok(versions.into_iter().next().map(|(v, _)| v))
    }

    fn default_version() -> Spanned<u32> {
        Spanned::call_site(1)
    }

    pub(super) async fn resolve_error_types(&mut self) -> crate::Result<()> {
        let ns = self.namespace.lock().await;

        // Extract namespace-level error attribute and validate it exists
        let namespace_error = if let Some(e) = ns.error.as_ref() {
            let error_name = e.error_name().to_string();
            let span = crate::Span::new(e.value.span().start, e.value.span().end);

            // Validate that the error type exists - check local namespace and imports
            if !Self::error_type_exists(&error_name, &ns) {
                let err: crate::Error = crate::MetadataError::invalid_error_attr(format!(
                    "'{}' is not a defined error type",
                    error_name
                ))
                .at(span)
                .build()
                .into();

                let source_path = e.source.clone();
                let source_content = ns.sources.get(&source_path).cloned();
                return Err(err.with_source_arc_if(source_path, source_content));
            }

            Some(Spanned::new(
                e.value.span().start,
                e.value.span().end,
                error_name,
            ))
        } else {
            None
        };

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
                                // Get span from operation name
                                let raw_span = item_ctx.name.span.span();
                                let span = crate::Span::new(raw_span.start, raw_span.end);

                                // Get source info for error context
                                let source_path = child.source.clone();
                                let source_content = ns.sources.get(&source_path).cloned();

                                let err: crate::Error = crate::TypeDefError::missing_error_type(
                                    item_ctx.name.borrow_string(),
                                )
                                .at(span)
                                .build()
                                .into();

                                return Err(err.with_source_arc_if(source_path, source_content));
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

    /// Check if an error type name exists in the namespace or imports.
    /// Returns true if the name corresponds to an `error` definition.
    fn error_type_exists(
        name: &str,
        ns: &crate::ctx::NamespaceCtx,
    ) -> bool {
        // Check local namespace children for an Error type
        for (item_ctx, child) in &ns.children {
            if item_ctx.name.borrow_string() == name {
                return matches!(child.value, NamespaceChild::Error(_));
            }
        }

        // Check imports for the error type
        for import in &ns.imports {
            match &import.value {
                crate::ctx::RefOrItemContext::Ref(ref_ctx) => {
                    // Check if the last segment of the namespace path matches
                    if let Some(last_segment) = ref_ctx.namespace.last()
                        && last_segment == name
                    {
                        // Found in imports - we assume the type exists
                        // Full validation would require looking up the imported namespace
                        return true;
                    }
                },
                crate::ctx::RefOrItemContext::Item(item_ctx) => {
                    if item_ctx.name.borrow_string() == name {
                        return true;
                    }
                },
            }
        }

        false
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
