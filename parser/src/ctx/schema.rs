use std::{
    collections::BTreeMap,
    ops::DerefMut,
    path::{Path, PathBuf},
    sync::Arc,
};

use convert_case::{Case, Casing};
use tokio::sync::Mutex;

use crate::{
    SpannedToken,
    ast::{
        AstStream,
        items::{Items, NamespaceDef},
        namespace::Namespace,
    },
    ctx::{WithSource, registry::TypeRegistry},
    defs::{Span, Spanned, Spans},
    tokens::{IdentToken, LexingError, SemiToken},
};

use super::namespace::NamespaceCtx;

pub struct SchemaCtx {
    pub root_path: PathBuf,
    pub package: kintsu_manifests::package::PackageManifest,
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

        let package = kintsu_manifests::package::PackageManifest::new(fs, root_path)?;

        let lib_path = root_path.join("schema").join("lib.ks");

        let lib_source = fs
            .read_to_string(&lib_path)
            .await
            .map_err(|e| {
                // If file doesn't exist, convert to MissingLibError
                match e {
                    kintsu_fs::Error::IoError(_) => crate::Error::MissingLibError,
                    _ => crate::Error::from(e),
                }
            })?;

        let root_ctx =
            super::paths::RefContext::new(package.package.name.to_case(Case::Snake), vec![]);

        let lib_source = Arc::new(lib_source);
        let mut tt = crate::tokens::tokenize(&lib_source).map_err(|e: LexingError| {
            crate::Error::from(e).with_source(lib_path.clone(), Arc::clone(&lib_source))
        })?;

        let lib_ast = AstStream::from_tokens_with(&lib_path, &mut tt)?;

        let mut lib_namespace: Option<SpannedToken![ident]> = None;
        let mut use_statements: Vec<_> = Vec::new();

        let mut namespaces = BTreeMap::new();

        for item in lib_ast.nodes {
            match item.value {
                Items::Namespace(ns) => {
                    if lib_namespace.is_some() {
                        let def_span = ns.def.span();
                        return Err(crate::Error::NsConflict
                            .with_span(def_span.start, def_span.end)
                            .with_source(lib_path, lib_source.clone()));
                    }
                    lib_namespace = Some(ns.def.name.clone());
                },
                Items::Use(use_def) => {
                    use_statements.push(use_def.def.root_ident().to_string());
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
                        return Err(crate::Error::DuplicateNamespace {
                            name: ns_name.clone(),
                        }
                        .with_span(name_span.start, name_span.end)
                        .with_source(lib_path, lib_source.clone()));
                    }

                    namespaces.insert(ns_name, Arc::new(Mutex::new(ns_ctx)));
                },
                _ => {
                    return Err(
                        crate::Error::LibPldInvalidItem.with_source(lib_path, lib_source.clone())
                    );
                },
            }
        }

        let schema_dir = root_path.join("schema");

        for use_name in use_statements {
            let ctx = root_ctx.enter(&use_name);
            let single_file = schema_dir.join(format!("{}.ks", use_name));
            let dir_path = schema_dir.join(&use_name);

            let files_to_load = if fs.exists_sync(&single_file) {
                vec![single_file]
            } else {
                let include = vec![format!("{}/**/*.ks", dir_path.display())];

                fs.find_glob(&include, &package.files.exclude)
                    .map_err(crate::Error::from)?
            };

            if files_to_load.is_empty() {
                return Err(crate::Error::UsePathNotFound {
                    name: use_name.clone(),
                }
                .with_source(lib_path.clone(), Arc::clone(&lib_source)));
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
                return Err(crate::Error::NamespaceMismatch {
                    expected: use_name.clone(),
                    found: ns_name,
                }
                .with_source(lib_path.clone(), Arc::clone(&lib_source)));
            }

            namespaces.insert(use_name, Arc::new(Mutex::new(ns_ctx)));
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
