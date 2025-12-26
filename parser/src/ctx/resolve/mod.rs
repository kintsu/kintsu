pub(super) mod aliases;
pub(super) mod anonymous;
pub(super) mod helpers;
pub(super) mod metadata;
pub(super) mod tagging;
pub(super) mod type_expr;
pub(super) mod union_or;
pub(super) mod unions;
pub(super) mod validation;

#[cfg(test)]
mod phase_tests;

pub(crate) use helpers::UnionRecord;

use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    ast::ty::Type,
    ctx::{SourceSpanned, common::WithSource},
    defs::{Span, Spanned, Spans},
};

use self::helpers::NameContext;

#[derive(Default)]
pub struct NamespaceResolution {
    pub anonymous_structs: Vec<SourceSpanned<crate::ast::items::StructDef>>,
    pub identified_unions: Vec<Spanned<UnionRecord>>,
    pub union_structs: Vec<SourceSpanned<crate::ast::items::StructDef>>,
    pub resolved_aliases: BTreeMap<String, Spanned<Type>>,
    pub versions: BTreeMap<String, Spanned<u32>>,
    pub errors: BTreeMap<String, Spanned<String>>,
}

impl NamespaceResolution {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct ResolvedTypeRef {
    pub original: Arc<Spanned<Type>>,
    pub resolved: Arc<Spanned<Type>>,
    pub path: super::paths::NamedItemContext,
}

impl ResolvedTypeRef {
    pub fn new(
        path: super::paths::NamedItemContext,
        typ: Arc<Spanned<Type>>,
    ) -> Self {
        Self {
            original: typ.clone(),
            resolved: typ.clone(),
            path,
        }
    }

    pub fn with_resolved(
        path: super::paths::NamedItemContext,
        original: Arc<Spanned<Type>>,
        resolved: Arc<Spanned<Type>>,
    ) -> Self {
        Self {
            path,
            original,
            resolved,
        }
    }
}

pub struct TypeResolver {
    namespace: Arc<Mutex<super::NamespaceCtx>>,
    resolution: NamespaceResolution,
}

impl TypeResolver {
    pub fn new(namespace: Arc<Mutex<super::NamespaceCtx>>) -> Self {
        Self {
            namespace,
            resolution: NamespaceResolution::new(),
        }
    }

    pub async fn resolve(mut self) -> crate::Result<NamespaceResolution> {
        // Phase 1: Extract anonymous structs
        self.anonymous_structs().await?;
        // Phase 2: Identify union types
        self.identify_unions().await?;
        // Phase 3: Resolve type aliases
        self.resolve_type_aliases().await?;
        // Phase 3.5: Resolve union or compositions (RFC-0016)
        self.resolve_union_or().await?;
        // Phase 3.6: Resolve type expressions (RFC-0018)
        self.resolve_type_expressions().await?;
        // Phase 4: Validate unions
        self.validate_unions().await?;
        // Phase 4.5: Validate tagging (RFC-0017)
        self.validate_tagging().await?;
        // Phase 5: Merge unions into structs
        self.merge_unions().await?;
        // Phase 6: Resolve versions
        self.resolve_versions().await?;
        // Phase 7: Resolve error types
        self.resolve_error_types().await?;
        // Phase 8: Validate all references
        self.validate_all_references().await?;

        Ok(self.resolution)
    }

    async fn anonymous_structs(&mut self) -> crate::Result<()> {
        let mut name_gen = NameContext::new();

        let ns = self.namespace.lock().await;

        for (child_name, child) in &ns.children {
            name_gen.push(child_name.name.borrow_string().clone());

            let extracted = anonymous::from_child(&mut name_gen, child)?;
            self.resolution
                .anonymous_structs
                .extend(extracted.into_iter().map(|it| {
                    it.value
                        .with_span(Span::CallSite)
                        .with_source(it.source)
                }));

            name_gen.pop();
        }

        Ok(())
    }

    async fn identify_unions(&mut self) -> crate::Result<()> {
        let mut name_gen = NameContext::new();

        let ns = self.namespace.lock().await;

        for (child_name, child) in &ns.children {
            name_gen.push(child_name.name.borrow_string().clone());

            let unions_found = unions::identify_from_child(&mut name_gen, child)?;
            self.resolution.identified_unions.extend(
                unions_found
                    .into_iter()
                    .map(|u| Spanned::call_site(u.value)),
            );

            name_gen.pop();
        }

        Ok(())
    }

    async fn validate_unions(&mut self) -> crate::Result<()> {
        tracing::debug!("validate_unions: starting phase 4");

        let ns = self.namespace.lock().await;

        for union_record in &self.resolution.identified_unions {
            unions::validate_union_record(
                &union_record.value,
                &ns,
                &self.resolution.resolved_aliases,
            )
            .await?;
        }

        tracing::debug!("validate_unions: phase 4 complete");
        Ok(())
    }

    async fn merge_unions(&mut self) -> crate::Result<()> {
        tracing::debug!("merge_unions: starting phase 5");

        let ns = self.namespace.lock().await;

        for union_record in &self.resolution.identified_unions {
            let merged_struct = unions::merge_union(&union_record.value, &ns).await?;
            self.resolution.union_structs.push(
                merged_struct
                    .value
                    .with_span(Span::CallSite)
                    .with_source(merged_struct.source),
            );
        }

        tracing::debug!("merge_unions: phase 5 complete");
        Ok(())
    }
}
