use std::{path::Path, sync::Arc};

use kintsu_fs::{FileSystem, physical::Physical};
use kintsu_manifests::{
    config::NewForNamed,
    package::{PackageManifests, PathDependency},
    version::parse_version,
};

use super::*;

pub struct PathPackageResolver {
    pub(crate) fs: Arc<dyn FileSystem>,
}

impl PathPackageResolver {
    pub fn new() -> Self {
        Self {
            fs: Arc::new(Physical),
        }
    }

    pub fn with_fs(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    fn resolve_path_dep(
        &self,
        root_path: &Path,
        dep: &PathDependency,
    ) -> crate::Result<ResolvedDependency> {
        let resolved_path = root_path.join(&dep.path);

        let dep_manifest = PackageManifests::new(self.fs.as_ref(), &resolved_path)?;

        let version = parse_version(&dep_manifest.package().version.to_string())?;

        Ok(ResolvedDependency {
            fs: self.fs.clone(),
            path: resolved_path,
            mutability: DependencyMutability::Mutable,
            version,
        })
    }
}

impl Default for PathPackageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl PathResolver for PathPackageResolver {
    fn resolve_path(
        &self,
        _dep_name: &str,
        root_path: &Path,
        path: &PathDependency,
    ) -> crate::Result<ResolvedDependency> {
        self.resolve_path_dep(root_path, path)
    }
}
