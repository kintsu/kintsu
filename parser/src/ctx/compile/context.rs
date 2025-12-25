use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use kintsu_fs::FileSystem;
use kintsu_manifests::{config::NewForNamed, lock::Lockfiles, version::parse_version};
use tokio::sync::RwLock;

use crate::{
    ctx::{SchemaCtx, cache::SchemaCache, registry::TypeRegistry},
    tokens::ToTokens,
};

use super::{
    loader::DependencyLoader, lockfile::LockfileManager, resolver::PackageResolver,
    state::SharedCompilationState,
};

use kintsu_cli_core::ProgressManager;

pub struct CompileCtx {
    pub root: Arc<SchemaCtx>,
    pub root_fs: Arc<dyn FileSystem>,

    type_registry: TypeRegistry,

    pub(super) state: Arc<RwLock<SharedCompilationState>>,
    #[allow(dead_code)]
    pub(super) resolver: Arc<dyn PackageResolver>,
    pub(super) cache: SchemaCache,

    pub(super) root_path: PathBuf,
    pub(super) progress: ProgressManager,
}

impl CompileCtx {
    pub fn type_registry(&self) -> TypeRegistry {
        self.type_registry.clone()
    }

    pub async fn lockfile(&self) -> Option<kintsu_manifests::lock::Lockfile> {
        self.state.read().await.lockfile.clone()
    }

    pub async fn lockfile_invalidated(&self) -> bool {
        self.state.read().await.lockfile_invalidated
    }

    pub async fn get_dependency(
        &self,
        package: &str,
    ) -> Option<Arc<SchemaCtx>> {
        self.state
            .read()
            .await
            .dependencies
            .get(package)
            .cloned()
    }

    pub async fn dependency_names(&self) -> Vec<String> {
        self.state
            .read()
            .await
            .dependencies
            .keys()
            .cloned()
            .collect()
    }

    pub async fn cache_stats(&self) -> (usize, usize) {
        (self.cache.entry_count().await, self.cache.size_deep().await)
    }

    pub async fn should_write_lockfile(&self) -> bool {
        let state = self.state.read().await;
        state.lockfile_invalidated | state.lockfile.is_none()
    }

    pub async fn finalize(&self) -> crate::Result<()> {
        if self.should_write_lockfile().await {
            let root_version = parse_version(
                &self
                    .root
                    .package
                    .package()
                    .version
                    .to_string(),
            )?;
            LockfileManager::write_lockfile(
                &self.state,
                self.root_fs.clone(),
                &self.root_path,
                &self.root.package.package().name,
                root_version,
            )
            .await?;
            tracing::debug!(
                "Lockfile written to {}",
                self.root_path
                    .join(Lockfiles::NAME)
                    .display()
            );
        } else {
            tracing::debug!("Lockfile unchanged, skipping write");
        }

        tracing::info!(
            entries = {
                let (entries, size) = self.cache_stats().await;
                format!("{} entries, size: {} bytes", entries, size)
            },
            "compilation stats",
        );

        #[cfg(debug_assertions)]
        {
            println!("Registered Types:\n{}", self.hierarchy());
        }

        Ok(())
    }

    pub fn hierarchy(&self) -> String {
        let mut result = String::new();

        for (it, _, _) in self.type_registry().all_types() {
            result.push_str(&format!("- {}\n", it.display()));
        }

        result
    }
}

impl CompileCtx {
    fn default_resolver(fs: Arc<dyn FileSystem>) -> Arc<dyn PackageResolver> {
        Arc::new(super::resolver::Resolver::new(fs))
    }

    pub async fn from_entry_point(entry_path: impl AsRef<Path>) -> crate::Result<Self> {
        Self::from_entry_point_with_progress(entry_path, false).await
    }

    pub async fn from_entry_point_with_progress(
        entry_path: impl AsRef<Path>,
        show_progress: bool,
    ) -> crate::Result<Self> {
        Self::from_entry_point_with_config(entry_path, num_cpus::get(), show_progress).await
    }

    pub async fn with_fs(
        fs: Arc<dyn FileSystem>,
        entry_path: impl AsRef<Path>,
    ) -> crate::Result<Self> {
        Self::with_fs_and_config(
            fs.clone(),
            Self::default_resolver(fs.clone()),
            entry_path,
            num_cpus::get(),
            false,
        )
        .await
    }

    pub async fn with_fs_and_config(
        fs: Arc<dyn FileSystem>,
        resolver: Arc<dyn super::resolver::PackageResolver>,
        entry_path: impl AsRef<Path>,
        max_concurrent_tasks: usize,
        show_progress: bool,
    ) -> crate::Result<Self> {
        let progress = ProgressManager::new(show_progress);
        let registry = TypeRegistry::new();

        let pb = progress.add_spinner("Initializing");
        pb.set_message("root schema");

        let entry_path_ref = entry_path.as_ref();
        let root_path = entry_path_ref.to_path_buf();
        let root =
            Arc::new(SchemaCtx::from_path(fs.as_ref(), entry_path_ref, registry.clone()).await?);

        pb.finish_with_message("root schema");

        let cache = SchemaCache::new();

        let existing_lockfile = Lockfiles::new_for_opt(fs.as_ref(), &root_path)?.map(|lockfiles| {
            match lockfiles {
                Lockfiles::V1(lockfile) => lockfile,
            }
        });

        let mut initial_state = SharedCompilationState::new();
        initial_state.lockfile = existing_lockfile;

        let state = Arc::new(RwLock::new(initial_state));

        let ctx = Self {
            root,
            type_registry: registry.clone(),
            state: state.clone(),
            resolver: resolver.clone(),
            root_fs: fs.clone(),
            cache: cache.clone(),
            root_path: root_path.clone(),
            progress: progress.clone(),
        };

        DependencyLoader::load_dependencies_parallel(
            &ctx.root,
            state.clone(),
            resolver.clone(),
            cache.clone(),
            registry.clone(),
            root_path.clone(),
            max_concurrent_tasks,
            &progress,
        )
        .await?;

        super::schema_compiler::SchemaCompiler::compile_all(&ctx).await?;

        progress.finish();

        Ok(ctx)
    }

    pub async fn from_entry_point_with_cache(entry_path: impl AsRef<Path>) -> crate::Result<Self> {
        Self::from_entry_point_with_config(entry_path, num_cpus::get(), false).await
    }

    pub async fn from_entry_point_with_config(
        entry_path: impl AsRef<Path>,
        max_concurrent_tasks: usize,
        show_progress: bool,
    ) -> crate::Result<Self> {
        let progress = ProgressManager::new(show_progress);
        let registry = TypeRegistry::new();

        let pb = progress.add_spinner("Initializing");
        pb.set_message("root schema");

        let fs = Arc::new(kintsu_fs::physical::Physical);
        let resolver = Arc::new(super::resolver::Resolver::new(fs.clone()));

        let entry_path_ref = entry_path.as_ref();
        let root_path = entry_path_ref.to_path_buf();

        let root =
            Arc::new(SchemaCtx::from_path(fs.as_ref(), entry_path_ref, registry.clone()).await?);

        pb.finish_with_message(format!("completed {}", root.package.package().name));

        let cache = SchemaCache::new();

        let existing_lockfile = Lockfiles::new_for_opt(fs.as_ref(), &root_path)?.map(|lockfiles| {
            match lockfiles {
                Lockfiles::V1(lockfile) => lockfile,
            }
        });

        let mut initial_state = SharedCompilationState::new();
        initial_state.lockfile = existing_lockfile;

        let state = Arc::new(RwLock::new(initial_state));

        let ctx = Self {
            root,
            root_fs: fs,
            type_registry: registry.clone(),
            state: state.clone(),
            resolver: resolver.clone(),
            cache: cache.clone(),
            root_path: root_path.clone(),
            progress: progress.clone(),
        };

        DependencyLoader::load_dependencies_parallel(
            &ctx.root,
            state.clone(),
            resolver.clone(),
            cache.clone(),
            registry.clone(),
            root_path.clone(),
            max_concurrent_tasks,
            &progress,
        )
        .await?;

        super::schema_compiler::SchemaCompiler::compile_all(&ctx).await?;

        progress.finish();

        Ok(ctx)
    }
}
