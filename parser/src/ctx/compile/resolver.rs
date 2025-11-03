use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
};

use kintsu_fs::{FileSystem, physical::Physical};
use kintsu_manifests::{
    config::NewForNamed,
    package::{Dependency, PackageManifest},
    version::Version,
};

use super::utils::normalize_import_to_package_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DependencyMutability {
    #[allow(dead_code)]
    Immutable,
    Mutable,
}

#[derive(Clone)]
pub struct ResolvedDependency {
    pub fs: Arc<dyn FileSystem>,
    pub path: PathBuf,
    pub mutability: DependencyMutability,
    pub version: Version,
}

pub struct PackageResolver {
    pub(crate) fs: Arc<dyn FileSystem>,
}

impl PackageResolver {
    pub fn new() -> Self {
        Self {
            fs: Arc::new(Physical),
        }
    }

    pub fn with_fs(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    pub fn resolve(
        &self,
        root_path: &Path,
        manifest: &PackageManifest,
        dep_name: &str,
    ) -> crate::Result<ResolvedDependency> {
        let package_name = normalize_import_to_package_name(dep_name);

        let dep = manifest
            .dependencies
            .get(&package_name)
            .ok_or_else(|| {
                crate::Error::UnresolvedDependency {
                    name: format!("{} (normalized from import '{}')", package_name, dep_name),
                }
            })?;

        match dep {
            Dependency::Path { path } => {
                let resolved_path = root_path.join(path);

                // Load the dependency's manifest to get its version
                let dep_manifest = PackageManifest::new(self.fs.as_ref(), &resolved_path)
                    .map_err(crate::Error::ManifestError)?;

                let version = Version::parse(&dep_manifest.package.version.to_string())?;

                Ok(ResolvedDependency {
                    fs: self.fs.clone(),
                    path: resolved_path,
                    mutability: DependencyMutability::Mutable,
                    version,
                })
            },
            Dependency::Git { .. } | Dependency::Remote { .. } => {
                // TODO: Implement git and remote dependency resolution
                Err(crate::Error::UnresolvedDependency {
                    name: format!("{} (git/remote dependencies not yet supported)", dep_name),
                })
            },
        }
    }

    /* TODO: this should be revised, DefaultHasher is NOT deterministic */
    pub async fn compute_content_hash(
        &self,
        path: &Path,
    ) -> crate::Result<String> {
        let schema_dir = path.join("schema");
        let include = vec![format!("{}/**/*.ks", schema_dir.display())];
        let exclude: Vec<String> = vec![];

        let files = self.fs.find_glob(&include, &exclude)?;

        let mut hasher = DefaultHasher::new();

        let mut sorted_files = files;
        sorted_files.sort();

        for file in sorted_files {
            let content = self.fs.read_to_string(&file).await?;
            file.to_string_lossy().hash(&mut hasher);
            content.hash(&mut hasher);
        }

        Ok(format!("{:x}", hasher.finish()))
    }
}

impl Default for PackageResolver {
    fn default() -> Self {
        Self::new()
    }
}
