use std::{
    collections::{BTreeSet, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use kintsu_fs::FileSystem;
use kintsu_manifests::{
    lock::LockedSource,
    package::{Dependency, PackageManifest},
    version::{Version, VersionExt, VersionSerde},
};
use tokio::sync::RwLock;

use crate::ctx::{
    SchemaCtx,
    cache::{CacheKey, CachedSchema, SchemaCache},
    compile::resolver::PackageResolver,
    registry::TypeRegistry,
};

use super::{
    coordinator::{CoordinatorState, dependency_coordinator, dependency_worker},
    progress::CompilationProgress,
    resolver::ResolvedDependency,
    state::{ResolvedMetadata, SharedCompilationState},
    utils::{normalize_import_to_package_name, normalize_package_to_import_name},
};

#[derive(Clone)]
pub(super) struct ParentContext {
    pub path: PathBuf,
    pub manifest: Arc<PackageManifest>,
}

#[derive(Clone)]
pub(super) struct CompilationTask {
    pub package_name: String,
    pub dependency_chain: Vec<String>,
    pub parent_context: ParentContext,
}

pub(super) enum DependencyTaskResult {
    Loaded(DependencyResult),
    AlreadyLoaded,
}

pub(super) struct DependencyResult {
    pub package_name: String,
    pub schema: Arc<SchemaCtx>,
    pub version: Version,
    pub dependency_chain: Vec<String>,
    pub resolved_path: PathBuf,
}

pub struct DependencyLoader;

impl DependencyLoader {
    pub async fn load_dependencies_parallel(
        root: &SchemaCtx,
        state: Arc<RwLock<SharedCompilationState>>,
        resolver: Arc<dyn super::resolver::PackageResolver>,
        cache: SchemaCache,
        type_registry: TypeRegistry,
        root_path: PathBuf,
        max_concurrent_tasks: usize,
        progress: &CompilationProgress,
    ) -> crate::Result<()> {
        let coord_state = CoordinatorState::new(state.clone(), progress.clone());

        let (task_tx, task_rx) = tokio::sync::mpsc::unbounded_channel::<CompilationTask>();
        let (result_tx, result_rx) =
            tokio::sync::mpsc::unbounded_channel::<Result<DependencyTaskResult, crate::Error>>();
        let (completion_tx, mut completion_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

        let mut seen_packages = HashSet::new();
        let mut initial_tasks = Vec::new();

        let root_context = ParentContext {
            path: root_path.clone(),
            manifest: Arc::new(root.package.clone()),
        };

        for ns_ctx in root.namespaces.values() {
            for import in &ns_ctx.lock().await.imports {
                let pkg_name = import.value.as_ref_context().package.clone();
                if !root.namespaces.contains_key(&pkg_name)
                    && seen_packages.insert(pkg_name.clone())
                {
                    initial_tasks.push(CompilationTask {
                        package_name: pkg_name,
                        dependency_chain: vec![root.package.package.name.clone()],
                        parent_context: root_context.clone(),
                    });
                }
            }
        }

        for task in initial_tasks {
            coord_state.increment_pending();
            if task_tx.send(task).is_err() {
                return Err(crate::Error::InternalError {
                    message: "Failed to send initial task: channel closed".into(),
                });
            }
        }

        if coord_state.pending_count() == 0 {
            tracing::info!("No dependencies to load, skipping parallel loading");
            return Ok(());
        }

        tracing::info!("Seeded {} initial tasks", coord_state.pending_count());

        let task_tx_for_coordinator = task_tx;

        // Spawn coordinator FIRST to ensure it's ready to receive results
        // before any worker can send them
        let coordinator_handle = tokio::spawn(dependency_coordinator(
            result_rx,
            task_tx_for_coordinator,
            completion_tx,
            coord_state.clone(),
        ));

        let shared_task_rx = Arc::new(tokio::sync::Mutex::new(task_rx));

        let mut worker_handles = Vec::new();
        for worker_id in 0..max_concurrent_tasks.max(1) {
            let task_rx = shared_task_rx.clone();
            let result_tx = result_tx.clone();
            let coord_state = coord_state.clone();
            let resolver = resolver.clone();
            let cache = cache.clone();
            let registry = type_registry.clone();

            let handle = tokio::spawn(dependency_worker(
                worker_id,
                task_rx,
                result_tx,
                coord_state,
                resolver,
                cache,
                registry,
            ));

            worker_handles.push(handle);
        }

        drop(result_tx);

        tracing::info!("Waiting for dependency loading to complete");

        if completion_rx.recv().await.is_none() {
            tracing::error!("Completion channel closed without signal");
            return Err(crate::Error::InternalError {
                message: "Completion channel closed without signal".into(),
            });
        }

        tracing::debug!("Completion signal received, waiting for workers");

        for (i, handle) in worker_handles.into_iter().enumerate() {
            if let Err(e) = handle.await {
                tracing::error!("Worker {} failed: {}", i, e);
                return Err(crate::Error::InternalError {
                    message: format!("Worker {} panicked: {}", i, e),
                });
            }
        }

        tracing::debug!("All workers exited, waiting for coordinator");

        if let Err(e) = coordinator_handle.await {
            tracing::error!("Coordinator failed: {}", e);
            return Err(crate::Error::InternalError {
                message: format!("Coordinator panicked: {}", e),
            });
        }

        tracing::info!("Dependency loading complete");

        if let Some(error) = coord_state.take_first_error().await {
            return Err(error);
        }

        Ok(())
    }

    pub(super) async fn process_dependency_task(
        task: CompilationTask,
        state: Arc<RwLock<SharedCompilationState>>,
        resolver: Arc<dyn PackageResolver>,
        cache: SchemaCache,
        registry: TypeRegistry,
    ) -> crate::Result<DependencyTaskResult> {
        let dep_name = &task.package_name;
        let parent_path = &task.parent_context.path;
        let parent_manifest = &task.parent_context.manifest;

        {
            let mut state_write = state.write().await;

            /* TODO: this should be revised since we check for circular dependencies earlier */
            // if state_write.processing_set.contains(dep_name) {
            //     let mut chain = task.dependency_chain.clone();
            //     chain.push(dep_name.clone());
            //     return Err(crate::Error::CircularDependency { chain });
            // }

            if state_write
                .loaded_versions
                .contains_key(dep_name)
            {
                return Ok(DependencyTaskResult::AlreadyLoaded);
            }

            state_write
                .processing_set
                .insert(dep_name.clone());
        }

        let dep = parent_manifest
            .dependencies
            .get(&normalize_import_to_package_name(dep_name))
            .ok_or_else(|| {
                crate::Error::InternalError {
                    message: format!("Dependency '{}' not found in parent manifest", dep_name),
                }
            })?;

        let resolved = resolver.resolve(parent_path, dep_name, dep)?;

        let use_version = Self::resolve_version(&state, dep_name, &resolved).await?;

        let cache_key = Self::build_cache_key(dep_name, &use_version, &resolved).await?;

        let content_hash = cache_key.content_hash.clone().unwrap();

        Self::validate_checksum(&state, dep_name, &use_version, &content_hash).await;

        let dep_schema = Self::load_or_cache_schema(
            &cache,
            &cache_key,
            &use_version,
            &resolved,
            registry.clone(),
        )
        .await?;

        if let Some(dep_lockfile) =
            Self::load_dependency_lockfile(resolved.fs.as_ref(), &resolved.path).await
        {
            Self::merge_dependency_lockfile(&state, dep_lockfile).await;
        }

        let source = Self::build_locked_source(parent_manifest, dep_name, &resolved.path);

        let mut provides = BTreeSet::new();
        for ns_name in dep_schema.namespaces.keys() {
            provides.insert(ns_name.clone());
        }

        let (_, dependency_names) = Self::collect_transitive_deps(
            &dep_schema,
            &resolved.path,
            &state,
            &task.dependency_chain,
            dep_name,
        )
        .await;

        {
            let mut state_write = state.write().await;
            state_write.resolved_metadata.insert(
                dep_name.clone(),
                ResolvedMetadata {
                    version: use_version.clone(),
                    checksum: content_hash.clone(),

                    source,
                    provides,
                    dependencies: dependency_names,
                },
            );
        }

        Ok(DependencyTaskResult::Loaded(DependencyResult {
            package_name: dep_name.clone(),
            schema: dep_schema,
            version: use_version,
            dependency_chain: task.dependency_chain.clone(),
            resolved_path: resolved.path.clone(),
        }))
    }

    async fn resolve_version(
        state: &Arc<RwLock<SharedCompilationState>>,
        dep_name: &str,
        resolved: &ResolvedDependency,
    ) -> crate::Result<Version> {
        let state_read = state.read().await;
        let mut candidate_version = resolved.version.clone();

        if let Some(existing_version) = state_read.loaded_versions.get(dep_name) {
            if !existing_version.is_compatible(&candidate_version) {
                return Err(crate::Error::VersionIncompatibility {
                    package: dep_name.to_string(),
                    required: candidate_version.to_string(),
                    found: existing_version.to_string(),
                });
            }
            if existing_version > &candidate_version {
                candidate_version = existing_version.clone();
            }
        }

        if let Some(lockfile) = &state_read.lockfile {
            let pkg_name_kebab = normalize_import_to_package_name(dep_name);
            for locked_pkg in lockfile.packages.values() {
                if locked_pkg.name == pkg_name_kebab {
                    let locked_version = &locked_pkg.version;
                    if locked_version.is_compatible(&candidate_version)
                        && locked_version.0 > candidate_version
                    {
                        candidate_version = locked_version.0.clone();
                    }
                }
            }
        }

        Ok(candidate_version)
    }

    async fn validate_checksum(
        state: &Arc<RwLock<SharedCompilationState>>,
        dep_name: &str,
        use_version: &Version,
        checksum: &str,
    ) {
        let invalidation_reason = {
            let state_read = state.read().await;
            if let Some(lockfile) = &state_read.lockfile {
                let pkg_name_kebab = normalize_import_to_package_name(dep_name);
                let key = format!("{}@{}", pkg_name_kebab, use_version);
                if let Some(locked_pkg) = lockfile.packages.get(&key) {
                    if locked_pkg.checksum != checksum {
                        Some(format!(
                            "checksum mismatch for package {}: expected '{}', got '{}'",
                            dep_name, locked_pkg.checksum, checksum
                        ))
                    } else {
                        None
                    }
                } else {
                    // Package is not in lockfile - this is a new dependency
                    Some(format!("new dependency '{}' not in lockfile", dep_name))
                }
            } else {
                None
            }
        };

        if let Some(reason) = invalidation_reason {
            let mut state_write = state.write().await;
            state_write.lockfile_invalidated = true;
            tracing::warn!("{}. lockfile will be regenerated.", reason);
        }
    }

    // todo: move this to ResolvedPackage and use package fs
    async fn build_cache_key(
        package_name: &str,
        version: &Version,
        resolved: &ResolvedDependency,
    ) -> crate::Result<CacheKey> {
        let content_hash = resolved.compute_content_hash().await?;

        Ok(CacheKey::new(
            package_name.to_string(),
            version.clone(),
            Some(content_hash),
        ))
    }

    async fn load_or_cache_schema(
        cache: &SchemaCache,
        cache_key: &CacheKey,
        use_version: &Version,
        resolved: &ResolvedDependency,
        registry: TypeRegistry,
    ) -> crate::Result<Arc<SchemaCtx>> {
        if let Some(cached) = cache.get(cache_key).await {
            if !cached.version.is_compatible(use_version) {
                Self::load_schema_fresh(resolved, registry).await
            } else {
                Ok(cached.schema)
            }
        } else {
            let schema = Self::load_schema_fresh(resolved, registry).await?;
            cache
                .insert(
                    cache_key.clone(),
                    CachedSchema::new(schema.clone(), use_version.clone()),
                )
                .await;
            Ok(schema)
        }
    }

    async fn load_schema_fresh(
        resolved: &ResolvedDependency,
        registry: TypeRegistry,
    ) -> crate::Result<Arc<SchemaCtx>> {
        let schema = SchemaCtx::from_path(resolved.fs.as_ref(), &resolved.path, registry).await?;
        Ok(Arc::new(schema))
    }

    async fn load_dependency_lockfile(
        fs: &dyn FileSystem,
        dep_path: &Path,
    ) -> Option<kintsu_manifests::lock::Lockfile> {
        use kintsu_manifests::{config::NewForNamed, lock::Lockfiles};
        match Lockfiles::new_for_opt(fs, dep_path) {
            Ok(Some(Lockfiles::V1(lockfile))) => Some(lockfile),
            _ => None,
        }
    }

    async fn merge_dependency_lockfile(
        state: &Arc<RwLock<SharedCompilationState>>,
        dep_lockfile: kintsu_manifests::lock::Lockfile,
    ) {
        let mut state_write = state.write().await;

        for (key, locked_pkg) in dep_lockfile.packages {
            let pkg_name_snake = normalize_package_to_import_name(&locked_pkg.name);

            if let Some(existing_version) = state_write
                .loaded_versions
                .get(&pkg_name_snake)
                && existing_version >= &locked_pkg.version
            {
                continue;
            }

            if let Some(our_lockfile) = &mut state_write.lockfile {
                let should_add = our_lockfile
                    .packages
                    .get(&key)
                    .map(|existing| locked_pkg.version > existing.version)
                    .unwrap_or(true);

                if should_add {
                    our_lockfile.packages.insert(key, locked_pkg);
                }
            }
        }
    }

    fn build_locked_source(
        root_package: &PackageManifest,
        dep_name: &str,
        resolved_path: &Path,
    ) -> LockedSource {
        match &root_package
            .dependencies
            .get(&normalize_import_to_package_name(dep_name))
        {
            Some(Dependency::Path(path)) => {
                LockedSource::Path {
                    path: path.path.clone(),
                }
            },
            Some(Dependency::Git(git)) => {
                LockedSource::Git {
                    url: git.git.clone(),
                    git_ref: git.git_ref.clone(),
                }
            },
            Some(Dependency::PathWithRemote(path)) => {
                LockedSource::Registry {
                    // todo: this should be the api download url from the remote config
                    url: path
                        .remote
                        .registry
                        .clone()
                        .unwrap_or_else(|| "https://registry.kintsu.dev".to_string()),
                }
            },
            Some(Dependency::Remote(remote)) => {
                LockedSource::Registry {
                    // todo: this should be the api download url from the remote config
                    url: remote
                        .registry
                        .clone()
                        .unwrap_or_else(|| "https://registry.kintsu.dev".to_string()),
                }
            },
            None => {
                LockedSource::Path {
                    path: resolved_path.to_path_buf(),
                }
            },
        }
    }

    pub(super) async fn collect_transitive_deps(
        dep_schema: &Arc<SchemaCtx>,
        dep_path: &Path,
        state: &Arc<RwLock<SharedCompilationState>>,
        dependency_chain: &[String],
        dep_name: &str,
    ) -> (Vec<CompilationTask>, Vec<String>) {
        let mut transitive_deps = Vec::new();
        let mut dependency_names = Vec::new();
        let mut new_chain = dependency_chain.to_vec();
        new_chain.push(dep_name.to_string());

        let parent_context = ParentContext {
            path: dep_path.to_path_buf(),
            manifest: Arc::new(dep_schema.package.clone()),
        };

        for ns_ctx in dep_schema.namespaces.values() {
            for nested_import in &ns_ctx.lock().await.imports {
                let object = nested_import.value.as_ref_context();
                let state_read = state.read().await;
                if !dep_schema
                    .namespaces
                    .contains_key(&object.package)
                    && !state_read
                        .loaded_versions
                        .contains_key(&object.package)
                {
                    drop(state_read);

                    dependency_names.push(object.package.to_string());
                    transitive_deps.push(CompilationTask {
                        package_name: object.package.to_string(),
                        dependency_chain: new_chain.clone(),
                        parent_context: parent_context.clone(),
                    });
                }
            }
        }

        (transitive_deps, dependency_names)
    }
}
