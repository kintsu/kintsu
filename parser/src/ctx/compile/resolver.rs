use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
};

use kintsu_fs::FileSystem;
use kintsu_manifests::{
    config::NewForNamed,
    package::{Dependency, GitDependency, PathDependency, RemoteDependency},
    version::Version,
};

pub mod path;
pub use path::PathPackageResolver;

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

impl ResolvedDependency {
    pub async fn compute_content_hash(&self) -> crate::Result<String> {
        let schema_dir = self.path.join("schema");
        let include = vec![format!("{}/**/*.ks", schema_dir.display())];
        let exclude: Vec<String> = vec![];

        let files = self.fs.find_glob(&include, &exclude)?;

        let mut hasher = DefaultHasher::new();

        let mut sorted_files = files;
        sorted_files.sort();

        for file in sorted_files.into_iter() {
            let content = self.fs.read_to_string(&file).await?;

            file.to_string_lossy().hash(&mut hasher);
            content.hash(&mut hasher);
        }

        Ok(format!("{:x}", hasher.finish()))
    }
}

pub trait RemoteResolver {
    fn resolve_remote(
        &self,
        dep_name: &str,
        remote: &RemoteDependency,
    ) -> crate::Result<ResolvedDependency>;
}

pub trait PathResolver {
    fn resolve_path(
        &self,
        dep_name: &str,
        root_path: &Path,
        path: &PathDependency,
    ) -> crate::Result<ResolvedDependency>;
}

pub trait GitResolver {
    fn resolve_git(
        &self,
        dep_name: &str,
        git: &GitDependency,
    ) -> crate::Result<ResolvedDependency>;
}

pub trait PackageResolver: Send + Sync + PathResolver + RemoteResolver + GitResolver {
    fn dependency_as_remote(&self) -> bool {
        false
    }

    fn resolve(
        &self,
        root_path: &Path,
        dep_name: &str,
        dependency: &Dependency,
    ) -> crate::Result<ResolvedDependency> {
        match dependency {
            Dependency::Path(path) => self.resolve_path(dep_name, root_path, path),
            Dependency::Git(git) => self.resolve_git(dep_name, git),
            Dependency::Remote(remote) => self.resolve_remote(dep_name, remote),
            Dependency::PathWithRemote(pwr) => {
                if self.dependency_as_remote() {
                    self.resolve_remote(dep_name, &pwr.remote)
                } else {
                    self.resolve_path(dep_name, root_path, &pwr.path)
                }
            },
        }
    }
}

pub struct Resolver {
    pub path: PathPackageResolver,
}

impl Resolver {
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self {
            path: PathPackageResolver::with_fs(fs),
        }
    }
}

impl PathResolver for Resolver {
    fn resolve_path(
        &self,
        dep_name: &str,
        root_path: &Path,
        path: &PathDependency,
    ) -> crate::Result<ResolvedDependency> {
        self.path
            .resolve_path(dep_name, root_path, path)
    }
}

impl GitResolver for Resolver {
    fn resolve_git(
        &self,
        dep_name: &str,
        git: &GitDependency,
    ) -> crate::Result<ResolvedDependency> {
        todo!()
    }
}

impl RemoteResolver for Resolver {
    fn resolve_remote(
        &self,
        dep_name: &str,
        remote: &RemoteDependency,
    ) -> crate::Result<ResolvedDependency> {
        todo!()
    }
}

impl PackageResolver for Resolver {}
