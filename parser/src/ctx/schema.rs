use std::{
    collections::BTreeMap,
    ops::DerefMut,
    path::{Path, PathBuf},
    sync::Arc,
};

use convert_case::{Case, Casing};
use tokio::sync::Mutex;

use crate::{FilesystemError, MetadataError, NamespaceError, ParsingError};

use crate::{
    SpannedToken,
    ast::{
        AstStream,
        items::{Items, NamespaceDef},
        meta::ItemMetaItem,
        namespace::Namespace,
    },
    ctx::{WithSource, registry::TypeRegistry},
    defs::{Span, Spanned, Spans},
    tokens::{IdentToken, LexingError, SemiToken},
};

use super::namespace::NamespaceCtx;

pub struct SchemaCtx {
    pub root_path: PathBuf,
    pub package: kintsu_manifests::package::PackageManifests,
    pub namespaces: BTreeMap<String, Arc<Mutex<NamespaceCtx>>>,
    pub registry: TypeRegistry,
}

impl SchemaCtx {
    pub async fn from_path(
        fs: &dyn kintsu_fs::FileSystem,
        root_path: impl AsRef<Path>,
        registry: TypeRegistry,
    ) -> crate::Result<Self> {
        use kintsu_manifests::config::NewForNamed;

        let root_path = root_path.as_ref();

        let package = kintsu_manifests::package::PackageManifests::new(fs, root_path)?;

        let lib_path = root_path.join("schema").join("lib.ks");

        let lib_source = fs
            .read_to_string(&lib_path)
            .await
            .map_err(|e| {
                // If file doesn't exist, convert to MissingLibError
                match e {
                    kintsu_fs::Error::IoError(_) => {
                        crate::Error::Compiler(
                            FilesystemError::missing_lib_ks()
                                .unlocated()
                                .build(),
                        )
                    },
                    _ => crate::Error::from(e),
                }
            })?;

        let root_ctx =
            super::paths::RefContext::new(package.package().name.to_case(Case::Snake), vec![]);

        let lib_source = Arc::new(lib_source);
        let mut tt = crate::tokens::tokenize(&lib_source).map_err(|e: LexingError| {
            crate::Error::from(e).with_source(lib_path.clone(), Arc::clone(&lib_source))
        })?;

        let lib_ast = AstStream::from_tokens_with(&lib_path, &mut tt)?;

        // Validate module_meta from lib.ks for duplicate attributes (KMT3002)
        // and extract error attribute for later validation (KMT2002)
        let mut found_version = false;
        let mut found_error: Option<&crate::ast::meta::ErrorMeta> = None;
        let mut found_tag = false;
        for meta_item in &lib_ast.module_meta.meta {
            match meta_item {
                ItemMetaItem::Version(v) => {
                    if found_version {
                        let dup_span = v.span();
                        let span = crate::Span::new(dup_span.start, dup_span.end);
                        return Err(crate::Error::Compiler(
                            MetadataError::duplicate_attribute(
                                "version",
                                lib_path.display().to_string(),
                            )
                            .at(span)
                            .build()
                            .with_source_arc(lib_path, lib_source),
                        ));
                    }
                    found_version = true;
                },
                ItemMetaItem::Error(e) => {
                    if found_error.is_some() {
                        let dup_span = e.span();
                        let span = crate::Span::new(dup_span.start, dup_span.end);
                        return Err(crate::Error::Compiler(
                            MetadataError::duplicate_attribute(
                                "error",
                                lib_path.display().to_string(),
                            )
                            .at(span)
                            .build()
                            .with_source_arc(lib_path, lib_source),
                        ));
                    }
                    found_error = Some(e);
                },
                ItemMetaItem::Tag(t) => {
                    if found_tag {
                        let dup_span = t.span();
                        let span = crate::Span::new(dup_span.start, dup_span.end);
                        return Err(crate::Error::Compiler(
                            MetadataError::duplicate_attribute(
                                "tag",
                                lib_path.display().to_string(),
                            )
                            .at(span)
                            .build()
                            .with_source_arc(lib_path, lib_source),
                        ));
                    }
                    found_tag = true;
                },
                ItemMetaItem::Rename(_) => {
                    // Rename is only valid on variants, not at module level - skip
                },
            }
        }

        let mut lib_namespace: Option<SpannedToken![ident]> = None;
        let mut use_statements: Vec<(String, crate::Span)> = Vec::new();

        let mut namespaces = BTreeMap::new();

        for item in lib_ast.nodes {
            let item_span = {
                let raw = item.span.span();
                crate::Span::new(raw.start, raw.end)
            };
            match item.value {
                Items::Namespace(ns) => {
                    if lib_namespace.is_some() {
                        let def_span = ns.def.span();
                        return Err(crate::Error::Compiler(
                            NamespaceError::conflict()
                                .at(crate::Span::new(def_span.start, def_span.end))
                                .build()
                                .with_source_arc(lib_path, lib_source.clone()),
                        ));
                    }
                    lib_namespace = Some(ns.def.name.clone());
                },
                Items::Use(use_def) => {
                    let def_span = use_def.def.span();
                    use_statements.push((
                        use_def.def.root_ident().to_string(),
                        crate::Span::new(def_span.start, def_span.end),
                    ));
                },
                Items::SpannedNamespace(spanned_ns) => {
                    let ns_name = spanned_ns
                        .def
                        .name
                        .borrow_string()
                        .to_string();

                    let name_span = spanned_ns.def.name.span().clone();

                    let ctx = root_ctx.enter(&ns_name);

                    let ns = NamespaceDef {
                        meta: spanned_ns.meta,
                        def: Namespace {
                            kw: Spanned::call_site(crate::tokens::KwNamespaceToken::new()),
                            name: Spanned::new(
                                name_span.start,
                                name_span.end,
                                IdentToken::new(ns_name.clone()),
                            ),
                        }
                        .with_span(Span::new(name_span.start, name_span.end)),
                        end: Spanned::call_site(SemiToken::new()),
                    };

                    let mut ns_ctx = NamespaceCtx::from_empty(
                        &ctx,
                        registry.clone(),
                        ns.with_source(lib_path.clone()),
                    )
                    .await;

                    let mut redeclared = false;

                    ns_ctx
                        .merge_ast_stream(
                            spanned_ns.def.value.ast.value,
                            lib_path.clone(),
                            lib_source.clone(),
                            &mut redeclared,
                        )
                        .map_err(|e| e.with_source(lib_path.clone(), Arc::clone(&lib_source)))?;

                    if namespaces.contains_key(&ns_name) {
                        return Err(crate::Error::Compiler(
                            NamespaceError::duplicate(&ns_name)
                                .at(crate::Span::new(name_span.start, name_span.end))
                                .build()
                                .with_source_arc(lib_path, lib_source.clone()),
                        ));
                    }

                    namespaces.insert(ns_name, Arc::new(Mutex::new(ns_ctx)));
                },
                _ => {
                    return Err(crate::Error::Compiler(
                        ParsingError::lib_invalid_item()
                            .at(item_span)
                            .build()
                            .with_source_arc(lib_path, lib_source.clone()),
                    ));
                },
            }
        }

        let schema_dir = root_path.join("schema");

        for (use_name, use_span) in use_statements {
            let ctx = root_ctx.enter(&use_name);
            let single_file = schema_dir.join(format!("{}.ks", use_name));
            let dir_path = schema_dir.join(&use_name);

            let files_to_load = if fs.exists_sync(&single_file) {
                vec![single_file]
            } else {
                let include = vec![format!("{}/**/*.ks", dir_path.display())];

                fs.find_glob(&include, &package.files().exclude)
                    .map_err(crate::Error::from)?
            };

            if files_to_load.is_empty() {
                return Err(crate::Error::Compiler(
                    NamespaceError::use_not_found(&use_name)
                        .at(use_span)
                        .build()
                        .with_source_arc(lib_path.clone(), Arc::clone(&lib_source)),
                ));
            }

            let ns_ctx = NamespaceCtx::load_files(ctx, fs, &files_to_load, None, registry.clone())
                .await
                .map_err(|e| e.with_source(lib_path.clone(), Arc::clone(&lib_source)))?;

            let ns_name = ns_ctx
                .namespace
                .value
                .def
                .name
                .borrow_string()
                .to_string();
            if ns_name != use_name {
                return Err(crate::Error::Compiler(
                    NamespaceError::mismatch(&use_name, &ns_name)
                        .at(use_span)
                        .build()
                        .with_source_arc(lib_path.clone(), Arc::clone(&lib_source)),
                ));
            }

            namespaces.insert(use_name, Arc::new(Mutex::new(ns_ctx)));
        }

        // KMT2002: Validate that #![err(ErrorType)] references an existing error type
        if let Some(err_meta) = found_error {
            let error_name = err_meta.error_name().to_string();
            let mut error_type_found = false;

            // Check all loaded namespaces for the error type
            for ns_lock in namespaces.values() {
                let ns = ns_lock.lock().await;
                for (item_ctx, child) in &ns.children {
                    if *item_ctx.name.borrow_string() == error_name
                        && matches!(child.value, crate::ctx::common::NamespaceChild::Error(_))
                    {
                        error_type_found = true;
                        break;
                    }
                }
                if error_type_found {
                    break;
                }
            }

            if !error_type_found {
                let err_span = err_meta.span();
                let span = crate::Span::new(err_span.start, err_span.end);
                return Err(crate::Error::Compiler(
                    MetadataError::invalid_error_attr(format!(
                        "'{}' is not a defined error type",
                        error_name
                    ))
                    .at(span)
                    .build()
                    .with_source_arc(lib_path, lib_source),
                ));
            }
        }

        Ok(Self {
            root_path: root_path.to_path_buf(),
            package,
            namespaces,
            registry,
        })
    }

    pub fn get_namespace(
        &self,
        name: &str,
    ) -> Option<Arc<Mutex<NamespaceCtx>>> {
        self.namespaces.get(name).cloned()
    }

    pub async fn get_namespace_mut<F: FnMut(&mut NamespaceCtx) -> R, R>(
        &mut self,
        name: &str,
        mut f: F,
    ) -> Option<R> {
        match self.namespaces.get(name) {
            Some(lock) => Some(f(lock.lock().await.deref_mut())),
            None => None,
        }
    }
}
