use std::path::Path;

use convert_case::{Case, Casing};
use kintsu_manifests::{
    package::Dependency,
    version::{Version, VersionSerde},
};
use kintsu_parser::ctx::compile::resolver::{
    DependencyMutability, GitResolver, PackageResolver, PathResolver, RemoteResolver,
    ResolvedDependency,
};
pub struct InternalPackageResolver {
    pre_computed:
        std::collections::HashMap<(String, VersionSerde), kintsu_fs::memory::MemoryFileSystem>,
}

impl InternalPackageResolver {
    pub fn new(
        sources: std::collections::HashMap<
            (String, VersionSerde),
            kintsu_fs::memory::MemoryFileSystem,
        >
    ) -> Self {
        Self {
            pre_computed: sources,
        }
    }
}

impl PathResolver for InternalPackageResolver {
    fn resolve_path(
        &self,
        _: &str,
        _: &Path,
        _: &kintsu_manifests::package::PathDependency,
    ) -> kintsu_parser::Result<ResolvedDependency> {
        unreachable!("validated beforehand that we never have path dependencies here")
    }
}

impl GitResolver for InternalPackageResolver {
    fn resolve_git(
        &self,
        _: &str,
        _: &kintsu_manifests::package::GitDependency,
    ) -> kintsu_parser::Result<ResolvedDependency> {
        unreachable!("validated beforehand that we never have git dependencies here")
    }
}

impl RemoteResolver for InternalPackageResolver {
    fn resolve_remote(
        &self,
        _: &str,
        _: &kintsu_manifests::package::RemoteDependency,
    ) -> kintsu_parser::Result<ResolvedDependency> {
        unreachable!("validated beforehand that we never call this")
    }
}

impl PackageResolver for InternalPackageResolver {
    fn resolve(
        &self,
        _: &Path,
        dep_name: &str,
        dependency: &Dependency,
    ) -> kintsu_parser::Result<ResolvedDependency> {
        let dep_name = dep_name.to_case(Case::Kebab);

        let version_req = dependency.version().ok_or_else(|| {
            kintsu_parser::Error::UnresolvedDependency {
                name: dep_name.to_string(),
            }
        })?;

        // Find a matching version from pre-computed sources
        let (found_version, found_fs) = self
            .pre_computed
            .iter()
            .find(|((name, ver), _)| name == &dep_name && version_req.matches(&ver.0))
            .map(|((_, ver), fs)| (ver.clone(), fs.clone()))
            .ok_or_else(|| {
                kintsu_parser::Error::UnresolvedDependency {
                    name: dep_name.to_string(),
                }
            })?;

        Ok(ResolvedDependency {
            fs: std::sync::Arc::new(found_fs),
            path: "./".into(),
            mutability: DependencyMutability::Immutable,
            version: found_version.0,
        })
    }
}
