use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use kintsu_manifests::{
    config::NewForNamed,
    lock::{LockedDependencyRef, LockedPackage, LockedSource, Lockfile, Lockfiles},
    version::{Version, VersionExt, VersionSerde},
};
use tokio::sync::RwLock;

use super::{state::SharedCompilationState, utils::normalize_import_to_package_name};

pub struct LockfileManager;

impl LockfileManager {
    pub async fn prune_compatible_versions(state: &Arc<RwLock<SharedCompilationState>>) {
        let mut state = state.write().await;

        let mut version_groups: BTreeMap<String, Vec<(String, Version)>> = BTreeMap::new();

        for (pkg_name, metadata) in &state.resolved_metadata {
            version_groups
                .entry(pkg_name.clone())
                .or_default()
                .push((pkg_name.clone(), metadata.version.clone()));
        }

        let mut to_remove = Vec::new();
        for versions in version_groups.values_mut() {
            if versions.len() <= 1 {
                continue;
            }

            versions.sort_by(|a, b| b.1.cmp(&a.1));

            let (highest_name, highest_version) = &versions[0];
            for (other_name, other_version) in versions.iter().skip(1) {
                if other_version.is_compatible(highest_version) && other_name != highest_name {
                    to_remove.push(other_name.clone());
                }
            }
        }

        for pkg_name in to_remove {
            state.resolved_metadata.remove(&pkg_name);
            state.dependencies.remove(&pkg_name);
            state.loaded_versions.remove(&pkg_name);
        }
    }

    pub async fn write_lockfile(
        state: &Arc<RwLock<SharedCompilationState>>,
        // resolver: &PathPackageResolver,
        fs: Arc<dyn kintsu_fs::FileSystem>,

        root_path: &PathBuf,
        root_package_name: &str,
        root_version: Version,
    ) -> crate::Result<()> {
        Self::prune_compatible_versions(state).await;

        let state_read = state.read().await;

        let mut packages = BTreeMap::new();

        for (pkg_name, metadata) in &state_read.resolved_metadata {
            let mut dependencies = BTreeMap::new();

            for dep_name in &metadata.dependencies {
                if let Some(dep_meta) = state_read.resolved_metadata.get(dep_name) {
                    let provides: Vec<String> = dep_meta.provides.iter().cloned().collect();
                    dependencies.insert(
                        dep_name.clone(),
                        LockedDependencyRef {
                            version: VersionSerde(dep_meta.version.clone()),
                            provides,
                            chain: vec![pkg_name.clone(), dep_name.clone()],
                        },
                    );
                }
            }

            let pkg_name_kebab = normalize_import_to_package_name(pkg_name);
            let key = format!("{}@{}", pkg_name_kebab, metadata.version);

            packages.insert(
                key,
                LockedPackage {
                    name: pkg_name_kebab,
                    version: VersionSerde(metadata.version.clone()),
                    checksum: metadata.checksum.clone(),
                    source: metadata.source.clone(),
                    dependencies,
                },
            );
        }

        let root_checksum = super::resolver::ResolvedDependency {
            version: root_version.clone(),
            fs: fs.clone(),
            path: root_path.clone(),
            mutability: super::resolver::DependencyMutability::Mutable,
        }
        .compute_content_hash()
        .await?;

        let root_pkg_name = normalize_import_to_package_name(root_package_name);

        let mut root_dependencies = BTreeMap::new();
        for (pkg_name, metadata) in &state_read.resolved_metadata {
            let provides: Vec<String> = metadata.provides.iter().cloned().collect();
            root_dependencies.insert(
                pkg_name.clone(),
                LockedDependencyRef {
                    version: VersionSerde(metadata.version.clone()),
                    provides,
                    chain: vec![root_pkg_name.clone(), pkg_name.clone()],
                },
            );
        }

        let root = LockedPackage {
            name: root_pkg_name,
            version: VersionSerde(root_version),
            checksum: root_checksum,
            source: LockedSource::Path {
                path: PathBuf::from("."),
            },
            dependencies: root_dependencies,
        };

        let lockfile = Lockfile { root, packages };

        let lockfiles = Lockfiles::V1(lockfile);
        let lockfile_content = NewForNamed::dump::<PathBuf>(&lockfiles)?;

        fs.write(
            &<Lockfiles as NewForNamed>::path(root_path),
            lockfile_content.into_bytes(),
        )
        .await?;

        Ok(())
    }
}
