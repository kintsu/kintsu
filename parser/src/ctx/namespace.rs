use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    SpannedToken,
    ast::{
        AstStream,
        comment::CommentStream,
        items::{Item, Items, NamespaceDef, UseDef},
        meta::{ErrorMeta, ItemMeta, ItemMetaItem, TagAttribute, VersionMeta},
    },
    ctx::{
        RefOrItemContext,
        paths::{NamedItemContext, RefContext},
    },
    defs::Spanned,
    tokens::{LexingError, ToTokens},
};

use super::{
    common::{FromNamedSource, NamespaceChild, WithSource},
    registry::TypeRegistry,
};

pub struct NamespaceCtx {
    pub ctx: super::paths::RefContext,

    pub sources: BTreeMap<PathBuf, Arc<String>>,

    pub comments: Vec<FromNamedSource<CommentStream>>,

    pub error: Option<FromNamedSource<ErrorMeta>>,

    pub version: Option<FromNamedSource<VersionMeta>>,

    /// Namespace-level tagging default from `#![tag(...)]` per SPEC-0016 Phase 4
    pub tag: Option<FromNamedSource<TagAttribute>>,

    pub namespace: FromNamedSource<NamespaceDef>,

    pub imports: Vec<FromNamedSource<RefOrItemContext>>,

    pub children: BTreeMap<NamedItemContext, FromNamedSource<NamespaceChild>>,

    pub(crate) registry: TypeRegistry,

    pub resolved_versions: BTreeMap<String, Spanned<u32>>,

    pub resolved_errors: BTreeMap<String, Spanned<String>>,

    /// Resolved type aliases (e.g., UnionOr resolved to Struct)
    pub resolved_aliases: BTreeMap<String, Spanned<crate::ast::ty::Type>>,
}

impl NamespaceCtx {
    // pub(crate) fn report_error<T>(
    //     &self,
    //     err: crate::Error,
    //     from: &FromNamedSource<T>,
    // ) -> miette::Report {
    //     err.with_source(
    //         from.source=.clone(),
    //         Arc::clone(self.sources.get(&from.source).unwrap()),
    //     )
    //     .to_report(None, None, None)
    // }

    pub async fn from_empty(
        ctx: &RefContext,
        registry: TypeRegistry,
        namespace: FromNamedSource<NamespaceDef>,
    ) -> Self {
        Self {
            ctx: ctx.clone(),
            registry,
            namespace,
            sources: Default::default(),
            comments: vec![],
            error: None,
            version: None,
            tag: None,
            imports: Vec::new(),
            children: Default::default(),
            resolved_versions: Default::default(),
            resolved_errors: Default::default(),
            resolved_aliases: Default::default(),
        }
    }

    pub async fn from_file(
        ctx: RefContext,
        fs: &dyn kintsu_fs::FileSystem,
        path: impl AsRef<Path>,
        registry: TypeRegistry,
    ) -> crate::Result<Self> {
        let path = path.as_ref();
        let source = fs.read_to_string(path).await?;

        let source = Arc::new(source);
        let mut tt = crate::tokens::tokenize(&source).map_err(|e: LexingError| {
            crate::Error::from(e).with_source(path.to_path_buf(), Arc::clone(&source))
        })?;

        let ast = AstStream::from_tokens_with(path, &mut tt)?;

        Self::from_ast_stream(ctx, ast, path.to_path_buf(), source, registry)
    }

    pub async fn load_files(
        ref_ctx: RefContext,
        fs: &dyn kintsu_fs::FileSystem,
        paths: &[PathBuf],
        required_namespace: Option<&SpannedToken![ident]>,
        registry: TypeRegistry,
    ) -> crate::Result<Self> {
        if paths.is_empty() {
            return Err(crate::Error::Compiler(
                crate::FilesystemError::empty_file_list()
                    .unlocated()
                    .build(),
            ));
        }

        use futures_util::future::join_all;

        let read_futures: Vec<_> = paths
            .iter()
            .map(|path| {
                async move {
                    let source = fs.read_to_string(path).await?;
                    Ok::<(PathBuf, String), kintsu_fs::Error>((path.clone(), source))
                }
            })
            .collect();

        let file_contents = join_all(read_futures).await;

        let mut namespace_ctx: Option<NamespaceCtx> = None;
        let mut found_namespace_decl = false;

        for result in file_contents {
            let (path, source_str) = result?;
            let source = Arc::new(source_str);

            let mut tt = crate::tokens::tokenize(&source).map_err(|e: LexingError| {
                crate::Error::from(e).with_source(path.clone(), Arc::clone(&source))
            })?;

            let ast = AstStream::from_tokens_with(&path, &mut tt)?;

            if let Some(ctx) = &mut namespace_ctx {
                ctx.merge_ast_stream(ast, path, source, &mut found_namespace_decl)?;
            } else {
                let ctx = Self::from_ast_stream(
                    ref_ctx.clone(),
                    ast,
                    path.clone(),
                    Arc::clone(&source),
                    registry.clone(),
                )?;
                found_namespace_decl = ctx.namespace.source == path;

                if let Some(required) = required_namespace
                    && ctx.namespace.value.def.name.borrow_string() != required.borrow_string()
                {
                    let expected = required.borrow_string().to_string();
                    let found = ctx
                        .namespace
                        .value
                        .def
                        .name
                        .borrow_string()
                        .to_string();
                    return Err(crate::Error::Compiler(
                        crate::NamespaceError::mismatch(&expected, &found)
                            .unlocated()
                            .build()
                            .with_source_arc(path.clone(), source),
                    ));
                }

                namespace_ctx = Some(ctx);
            }
        }

        namespace_ctx.ok_or_else(|| {
            crate::Error::Compiler(
                crate::InternalError::failed_namespace_ctx()
                    .unlocated()
                    .build(),
            )
        })
    }

    fn handle_use(
        ctx: &RefContext,
        imports: &mut Vec<FromNamedSource<RefOrItemContext>>,
        use_def: UseDef,
        path: PathBuf,
    ) {
        imports.extend(
            use_def
                .def
                .value
                .path
                .qualified_paths(ctx.package.clone())
                .into_iter()
                .map(|p| p.with_source(path.clone())),
        );
    }

    pub(crate) fn from_ast_stream(
        ctx: RefContext,
        ast: AstStream,
        path: PathBuf,
        source: Arc<String>,
        registry: TypeRegistry,
    ) -> crate::Result<Self> {
        let mut sources = BTreeMap::new();
        sources.insert(path.clone(), Arc::clone(&source));

        let module_comments = ast.module_comments;

        let mut namespace: Option<FromNamedSource<NamespaceDef>> = None;
        let mut imports = Vec::new();
        let mut children = BTreeMap::new();
        let mut version: Option<FromNamedSource<VersionMeta>> = None;
        let mut error: Option<FromNamedSource<ErrorMeta>> = None;
        let mut tag: Option<FromNamedSource<TagAttribute>> = None;

        Self::extract_meta_items(
            &[&ast.module_meta],
            &path,
            &mut version,
            &mut error,
            &mut tag,
        )
        .map_err(|err| err.with_source(path.clone(), Arc::clone(&source)))?;

        let ast_p = path.clone();
        let ast_s = source.clone();

        for item in ast.nodes {
            match item.value {
                Items::Namespace(ns_def) => {
                    Self::extract_meta_items(
                        &ns_def.meta(),
                        &path,
                        &mut version,
                        &mut error,
                        &mut tag,
                    )
                    .map_err(|err| err.with_source(ast_p.clone(), ast_s.clone()))?;

                    if let Some(ref existing) = namespace {
                        if ns_def.def.name.borrow_string()
                            != existing.value.def.name.borrow_string()
                        {
                            let parent = existing
                                .value
                                .def
                                .name
                                .borrow_string()
                                .to_string();
                            let attempted = ns_def.def.name.borrow_string().to_string();
                            let ns_span = ns_def.def.name.span();
                            return Err(crate::Error::Compiler(
                                crate::NamespaceError::dir_conflict(
                                    path.to_string_lossy().to_string(),
                                    &parent,
                                    &attempted,
                                )
                                .at(crate::Span::new(ns_span.start, ns_span.end))
                                .build()
                                .with_source_arc(path, source),
                            ));
                        }
                    } else {
                        namespace = Some(ns_def.with_source(path.clone()));
                    }
                },
                Items::Use(use_def) => {
                    Self::handle_use(&ctx, &mut imports, use_def, path.clone());
                },
                Items::SpannedNamespace(inner_ns) => {
                    let ctx = ctx.enter(inner_ns.def.name.borrow_string());

                    Self::process_spanned_namespace(
                        ctx,
                        &mut children,
                        inner_ns,
                        &path,
                        &source,
                        registry.clone(),
                    )?;
                },
                Items::OneOf(def) => {
                    let name = def.def.name.clone();
                    let ns_name = namespace
                        .as_ref()
                        .ok_or_else(|| {
                            crate::Error::Compiler(
                                crate::NamespaceError::not_declared()
                                    .at_node(&def.def)
                                    .build()
                                    .with_source_arc(path.clone(), Arc::clone(&source)),
                            )
                        })?
                        .value
                        .def
                        .name
                        .clone();

                    Self::insert_typed_child(
                        &ctx,
                        &mut children,
                        ns_name,
                        &name,
                        || NamespaceChild::OneOf(def),
                        &path,
                        &source,
                        "oneof",
                    )?;
                },
                Items::Enum(def) => {
                    let name = def.def.name().clone();
                    let ns_name = namespace
                        .as_ref()
                        .ok_or_else(|| {
                            crate::Error::Compiler(
                                crate::NamespaceError::not_declared()
                                    .at_node(&def.def)
                                    .build()
                                    .with_source_arc(path.clone(), Arc::clone(&source)),
                            )
                        })?
                        .value
                        .def
                        .name
                        .clone();

                    Self::insert_typed_child(
                        &ctx,
                        &mut children,
                        ns_name,
                        &name,
                        || NamespaceChild::Enum(def),
                        &path,
                        &source,
                        "enum",
                    )?;
                },
                Items::Struct(def) => {
                    let name = def.def.name.clone();
                    let ns_name = namespace
                        .as_ref()
                        .ok_or_else(|| {
                            crate::Error::Compiler(
                                crate::NamespaceError::not_declared()
                                    .at_node(&def.def)
                                    .build()
                                    .with_source_arc(path.clone(), Arc::clone(&source)),
                            )
                        })?
                        .value
                        .def
                        .name
                        .clone();

                    Self::insert_typed_child(
                        &ctx,
                        &mut children,
                        ns_name,
                        &name,
                        || NamespaceChild::Struct(def),
                        &path,
                        &source,
                        "struct",
                    )?;
                },
                Items::Type(def) => {
                    let name = def.def.name.clone();
                    let ns_name = namespace
                        .as_ref()
                        .ok_or_else(|| {
                            crate::Error::Compiler(
                                crate::NamespaceError::not_declared()
                                    .at_node(&def.def)
                                    .build()
                                    .with_source_arc(path.clone(), Arc::clone(&source)),
                            )
                        })?
                        .value
                        .def
                        .name
                        .clone();

                    Self::insert_typed_child(
                        &ctx,
                        &mut children,
                        ns_name,
                        &name,
                        || NamespaceChild::Type(def),
                        &path,
                        &source,
                        "type",
                    )?;
                },
                Items::Error(def) => {
                    let name = def.def.name.clone();
                    let ns_name = namespace
                        .as_ref()
                        .ok_or_else(|| {
                            crate::Error::Compiler(
                                crate::NamespaceError::not_declared()
                                    .at_node(&def.def)
                                    .build()
                                    .with_source_arc(path.clone(), Arc::clone(&source)),
                            )
                        })?
                        .value
                        .def
                        .name
                        .clone();

                    Self::insert_typed_child(
                        &ctx,
                        &mut children,
                        ns_name,
                        &name,
                        || NamespaceChild::Error(def),
                        &path,
                        &source,
                        "error",
                    )?;
                },
                Items::Operation(def) => {
                    let name = def.def.name.clone();
                    let ns_name = namespace
                        .as_ref()
                        .ok_or_else(|| {
                            crate::Error::Compiler(
                                crate::NamespaceError::not_declared()
                                    .at_node(&def.def)
                                    .build()
                                    .with_source_arc(path.clone(), Arc::clone(&source)),
                            )
                        })?
                        .value
                        .def
                        .name
                        .clone();

                    Self::insert_typed_child(
                        &ctx,
                        &mut children,
                        ns_name,
                        &name,
                        || NamespaceChild::Operation(def),
                        &path,
                        &source,
                        "operation",
                    )?;
                },
            }
        }

        let namespace = namespace.ok_or_else(|| {
            crate::Error::Compiler(
                crate::NamespaceError::not_declared()
                    .unlocated()
                    .build()
                    .with_source_arc(path.clone(), Arc::clone(&source)),
            )
        })?;

        Ok(Self {
            ctx,
            sources,
            comments: vec![module_comments.with_source(path.clone())],
            error,
            version,
            tag,
            namespace,
            imports,
            children,
            registry,
            resolved_versions: BTreeMap::new(),
            resolved_errors: BTreeMap::new(),
            resolved_aliases: BTreeMap::new(),
        })
    }

    pub(super) fn merge_ast_stream(
        &mut self,
        ast: AstStream,
        path: PathBuf,
        source: Arc<String>,
        found_namespace_decl: &mut bool,
    ) -> crate::Result<()> {
        self.sources
            .insert(path.clone(), Arc::clone(&source));

        Self::extract_meta_items(
            &[&ast.module_meta],
            &path,
            &mut self.version,
            &mut self.error,
            &mut self.tag,
        )
        .map_err(|err| err.with_source(path.clone(), Arc::clone(&source)))?;

        for item in ast.nodes {
            match item.value {
                Items::Namespace(ns_def) => {
                    if ns_def.def.name.borrow_string()
                        != self.namespace.value.def.name.borrow_string()
                    {
                        let parent = self
                            .namespace
                            .value
                            .def
                            .name
                            .borrow_string()
                            .to_string();
                        let attempted = ns_def.def.name.borrow_string().to_string();
                        let ns_span = ns_def.def.name.span();
                        return Err(crate::Error::Compiler(
                            crate::NamespaceError::dir_conflict(
                                path.to_string_lossy().to_string(),
                                &parent,
                                &attempted,
                            )
                            .at(crate::Span::new(ns_span.start, ns_span.end))
                            .build()
                            .with_source_arc(path, source),
                        ));
                    }

                    *found_namespace_decl = true;
                    Self::extract_meta_items(
                        &ns_def.meta(),
                        &path,
                        &mut self.version,
                        &mut self.error,
                        &mut self.tag,
                    )?;
                },
                Items::Use(use_def) => {
                    Self::handle_use(&self.ctx, &mut self.imports, use_def, path.clone());
                },
                Items::SpannedNamespace(inner_ns) => {
                    let ctx = self
                        .ctx
                        .enter(inner_ns.def.name.borrow_string());

                    Self::process_spanned_namespace(
                        ctx,
                        &mut self.children,
                        inner_ns,
                        &path,
                        &source,
                        self.registry.clone(),
                    )?;
                },
                Items::OneOf(def) => {
                    let name = def.def.name.clone();
                    Self::insert_typed_child(
                        &self.ctx,
                        &mut self.children,
                        self.namespace.value.def.name.clone(),
                        &name,
                        || NamespaceChild::OneOf(def),
                        &path,
                        &source,
                        "oneof",
                    )?;
                },
                Items::Enum(def) => {
                    let name = def.def.value.name().clone();
                    Self::insert_typed_child(
                        &self.ctx,
                        &mut self.children,
                        self.namespace.value.def.name.clone(),
                        &name,
                        || NamespaceChild::Enum(def),
                        &path,
                        &source,
                        "enum",
                    )?;
                },
                Items::Struct(def) => {
                    let name = def.def.name.clone();
                    Self::insert_typed_child(
                        &self.ctx,
                        &mut self.children,
                        self.namespace.value.def.name.clone(),
                        &name,
                        || NamespaceChild::Struct(def),
                        &path,
                        &source,
                        "struct",
                    )?;
                },
                Items::Type(def) => {
                    let name = def.def.name.clone();
                    Self::insert_typed_child(
                        &self.ctx,
                        &mut self.children,
                        self.namespace.value.def.name.clone(),
                        &name,
                        || NamespaceChild::Type(def),
                        &path,
                        &source,
                        "type",
                    )?;
                },
                Items::Error(def) => {
                    let name = def.def.name.clone();
                    Self::insert_typed_child(
                        &self.ctx,
                        &mut self.children,
                        self.namespace.value.def.name.clone(),
                        &name,
                        || NamespaceChild::Error(def),
                        &path,
                        &source,
                        "error",
                    )?;
                },
                Items::Operation(def) => {
                    let name = def.def.name.clone();
                    Self::insert_typed_child(
                        &self.ctx,
                        &mut self.children,
                        self.namespace.value.def.name.clone(),
                        &name,
                        || NamespaceChild::Operation(def),
                        &path,
                        &source,
                        "operation",
                    )?;
                },
            }
        }

        self.comments
            .push(ast.module_comments.with_source(path.clone()));

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_typed_child<F>(
        ctx: &RefContext,
        children: &mut BTreeMap<NamedItemContext, FromNamedSource<NamespaceChild>>,
        namespace_name: SpannedToken![ident],
        name: &SpannedToken![ident],
        constructor: F,
        path: &Path,
        source: &Arc<String>,
        type_tag: &'static str,
    ) -> crate::Result<()>
    where
        F: FnOnce() -> NamespaceChild, {
        let qual = ctx.item(name.clone());
        let sp = name.span();
        let err_span = kintsu_errors::Span::new(sp.start, sp.end);
        crate::utils::insert_unique_ident(
            namespace_name,
            children,
            qual,
            type_tag,
            constructor().with_source(path.to_path_buf()),
            Some(err_span),
        )
        .map_err(|e| e.with_source(path.to_path_buf(), Arc::clone(source)))
    }

    pub fn process_spanned_namespace(
        ctx: RefContext,
        children: &mut BTreeMap<NamedItemContext, FromNamedSource<NamespaceChild>>,
        inner_ns: Item<crate::ast::namespace::SpannedNamespace>,
        path: &Path,
        source: &Arc<String>,
        registry: TypeRegistry,
    ) -> crate::Result<()> {
        let name = inner_ns.def.name.clone();
        let ast = inner_ns.def.value.ast.value;

        let nested_ctx = Self::from_ast_stream(
            ctx.clone(),
            ast,
            path.to_path_buf(),
            Arc::clone(source),
            registry,
        )?;

        let child = NamespaceChild::Namespace(Box::new(nested_ctx));

        let qual_name = ctx.item(name.clone());

        if children.contains_key(&qual_name) {
            let span = name.span();
            return Err(crate::Error::Compiler(
                crate::NamespaceError::duplicate(name.borrow_string())
                    .at(crate::Span::new(span.start, span.end))
                    .build()
                    .with_source_arc(path.to_path_buf(), Arc::clone(source)),
            ));
        }

        children.insert(qual_name, child.with_source(path.to_path_buf()));
        Ok(())
    }

    fn extract_meta_items(
        meta_vec: &[&Spanned<ItemMeta>],
        path: &Path,
        version: &mut Option<FromNamedSource<VersionMeta>>,
        error: &mut Option<FromNamedSource<ErrorMeta>>,
        tag: &mut Option<FromNamedSource<TagAttribute>>,
    ) -> crate::Result<()> {
        for meta_spanned in meta_vec {
            for meta_item in &meta_spanned.value.meta {
                match meta_item {
                    ItemMetaItem::Version(v) => {
                        if version.is_some() {
                            let dup_span = v.span();
                            let span = crate::Span::new(dup_span.start, dup_span.end);
                            return Err(crate::Error::Compiler(
                                crate::MetadataError::duplicate_attribute(
                                    "version",
                                    path.display().to_string(),
                                )
                                .at(span)
                                .build(),
                            ));
                        }
                        *version = Some(v.clone().with_source(path.to_path_buf()));
                    },
                    ItemMetaItem::Error(e) => {
                        if error.is_some() {
                            let dup_span = e.span();
                            let span = crate::Span::new(dup_span.start, dup_span.end);
                            return Err(crate::Error::Compiler(
                                crate::MetadataError::duplicate_attribute(
                                    "error",
                                    path.display().to_string(),
                                )
                                .at(span)
                                .build(),
                            ));
                        }
                        *error = Some(e.clone().with_source(path.to_path_buf()));
                    },
                    ItemMetaItem::Tag(t) => {
                        // Namespace-level tag: #![tag(...)] per SPEC-0016 Phase 4
                        if tag.is_some() {
                            let dup_span = t.span();
                            let span = crate::Span::new(dup_span.start, dup_span.end);
                            return Err(crate::Error::Compiler(
                                crate::MetadataError::duplicate_attribute(
                                    "tag",
                                    path.display().to_string(),
                                )
                                .at(span)
                                .build(),
                            ));
                        }
                        *tag = Some(
                            t.value
                                .clone()
                                .with_source(path.to_path_buf()),
                        );
                    },
                    ItemMetaItem::Rename(_) => {
                        // Rename only valid on variants, not at namespace level - skip
                    },
                }
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, resolution), fields(ns = self.ctx.display()))]
    pub async fn integrate_resolution(
        &mut self,
        resolution: super::resolve::NamespaceResolution,
    ) -> crate::Result<()> {
        tracing::debug!(
            anonymous_structs = resolution.anonymous_structs.len(),
            union_structs = resolution.union_structs.len(),
            versions = resolution.versions.len(),
            errors = resolution.errors.len(),
            "Integrating TypeResolver results"
        );

        for struct_def in resolution.anonymous_structs {
            let name = &struct_def.value.value.def.value.name;
            let item_ctx = self.ctx.item(name.clone());

            tracing::trace!(
                struct_name = name.display(),
                namespace = self.ctx.display(),
                "registering anonymous struct"
            );

            let child = NamespaceChild::Struct(struct_def.value.value);
            self.children
                .insert(item_ctx, child.with_source(struct_def.source));
        }

        for struct_def in resolution.union_structs {
            let name = &struct_def.value.value.def.value.name;
            let item_ctx = self.ctx.item(name.clone());

            tracing::trace!(
                struct_name = name.display(),
                namespace = self.ctx.display(),
                "registering union struct"
            );

            let child = NamespaceChild::Struct(struct_def.value.value);
            self.children
                .insert(item_ctx, child.with_source(struct_def.source));
        }

        self.resolved_versions = resolution.versions;
        self.resolved_errors = resolution.errors;
        self.resolved_aliases = resolution.resolved_aliases;

        tracing::debug!(
            total_children = self.children.len(),
            "TypeResolver integration complete"
        );

        Ok(())
    }
}
