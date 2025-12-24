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
        meta::{ErrorMeta, ItemMeta, ItemMetaItem, VersionMeta},
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

    pub namespace: FromNamedSource<NamespaceDef>,

    pub imports: Vec<FromNamedSource<RefOrItemContext>>,

    pub children: BTreeMap<NamedItemContext, FromNamedSource<NamespaceChild>>,

    pub(crate) registry: TypeRegistry,

    pub resolved_versions: BTreeMap<String, Spanned<u32>>,

    pub resolved_errors: BTreeMap<String, Spanned<String>>,
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
            imports: Vec::new(),
            children: Default::default(),
            resolved_versions: Default::default(),
            resolved_errors: Default::default(),
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
            return Err(crate::Error::EmptyFileList);
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
                    let err = crate::Error::NamespaceMismatch {
                        expected: required.borrow_string().to_string(),
                        found: ctx
                            .namespace
                            .value
                            .def
                            .name
                            .borrow_string()
                            .to_string(),
                    }
                    .with_source(path.clone(), source);
                    return Err(err);
                }

                namespace_ctx = Some(ctx);
            }
        }

        namespace_ctx.ok_or_else(|| crate::Error::FailedToCreateNamespaceCtx)
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

    pub(super) fn from_ast_stream(
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

        Self::extract_meta_items(&[&ast.module_meta], &path, &mut version, &mut error)?;

        let ast_p = path.clone();
        let ast_s = source.clone();

        for item in ast.nodes {
            match item.value {
                Items::Namespace(ns_def) => {
                    Self::extract_meta_items(&ns_def.meta(), &path, &mut version, &mut error)
                        .map_err(|err| err.with_source(ast_p.clone(), ast_s.clone()))?;

                    if let Some(ref existing) = namespace {
                        if ns_def.def.name.borrow_string()
                            != existing.value.def.name.borrow_string()
                        {
                            let err = crate::Error::ns_dir(
                                &existing.value.def.name,
                                &ns_def.def.name,
                                path.clone(),
                            );
                            return Err(err.with_source(path, source));
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
                            crate::Error::NsNotDeclared
                                .with_source(path.clone(), Arc::clone(&source))
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
                            crate::Error::NsNotDeclared
                                .with_source(path.clone(), Arc::clone(&source))
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
                            crate::Error::NsNotDeclared
                                .with_source(path.clone(), Arc::clone(&source))
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
                            crate::Error::NsNotDeclared
                                .with_source(path.clone(), Arc::clone(&source))
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
                            crate::Error::NsNotDeclared
                                .with_source(path.clone(), Arc::clone(&source))
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
                            crate::Error::NsNotDeclared
                                .with_source(path.clone(), Arc::clone(&source))
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
            crate::Error::NsNotDeclared.with_source(path.clone(), Arc::clone(&source))
        })?;

        Ok(Self {
            ctx,
            sources,
            comments: vec![module_comments.with_source(path.clone())],
            error,
            version,
            namespace,
            imports,
            children,
            registry,
            resolved_versions: BTreeMap::new(),
            resolved_errors: BTreeMap::new(),
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
        )?;

        for item in ast.nodes {
            match item.value {
                Items::Namespace(ns_def) => {
                    if ns_def.def.name.borrow_string()
                        != self.namespace.value.def.name.borrow_string()
                    {
                        let err = crate::Error::ns_dir(
                            &self.namespace.value.def.name,
                            &ns_def.def.name,
                            path.clone(),
                        );
                        return Err(err.with_source(path, source));
                    }

                    *found_namespace_decl = true;
                    Self::extract_meta_items(
                        &ns_def.meta(),
                        &path,
                        &mut self.version,
                        &mut self.error,
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
        crate::utils::insert_unique_ident(
            namespace_name,
            children,
            qual,
            type_tag,
            constructor().with_source(path.to_path_buf()),
        )
        .map_err(|e| {
            let span = name.span();
            e.with_span(span.start, span.end)
                .with_source(path.to_path_buf(), Arc::clone(source))
        })
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
            let err = crate::Error::DuplicateNamespace {
                name: name.borrow_string().to_string(),
            }
            .with_span(span.start, span.end);
            return Err(err.with_source(path.to_path_buf(), Arc::clone(source)));
        }

        children.insert(qual_name, child.with_source(path.to_path_buf()));
        Ok(())
    }

    fn extract_meta_items(
        meta_vec: &[&Spanned<ItemMeta>],
        path: &Path,
        version: &mut Option<FromNamedSource<VersionMeta>>,
        error: &mut Option<FromNamedSource<ErrorMeta>>,
    ) -> crate::Result<()> {
        for meta_spanned in meta_vec {
            for meta_item in &meta_spanned.value.meta {
                match meta_item {
                    ItemMetaItem::Version(v) => {
                        if version.is_some() {
                            return Err(crate::Error::DuplicateMetaAttribute {
                                attribute: "version".to_string(),
                                path: path.to_path_buf(),
                            });
                        }
                        *version = Some(v.clone().with_source(path.to_path_buf()));
                    },
                    ItemMetaItem::Error(e) => {
                        if error.is_some() {
                            return Err(crate::Error::DuplicateMetaAttribute {
                                attribute: "error".to_string(),
                                path: path.to_path_buf(),
                            });
                        }
                        *error = Some(e.clone().with_source(path.to_path_buf()));
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

        tracing::debug!(
            total_children = self.children.len(),
            "TypeResolver integration complete"
        );

        Ok(())
    }
}
