use std::{collections::BTreeMap, path::PathBuf};

use validator::Validate;

use crate::config::NewForNamed;

const LOCKFILE_NAME: &str = "schema.lock";

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum Lockfiles {
    V1(Lockfile),
}

impl Validate for Lockfiles {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        Ok(())
    }
}

impl NewForNamed for Lockfiles {
    const NAME: &str = "schema.lock.toml";
}

impl Lockfiles {
    pub fn exists(
        fs: &dyn kintsu_fs::FileSystem,
        project_root: PathBuf,
    ) -> bool {
        fs.exists_sync(&project_root.join(LOCKFILE_NAME))
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Lockfile {
    pub root: LockedPackage,
    pub packages: BTreeMap<String, LockedPackage>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq, Eq)]
pub struct LockedPackage {
    pub name: String,
    pub version: super::version::Version,
    pub checksum: String,
    pub source: LockedSource,
    #[serde(default)]
    pub dependencies: BTreeMap<String, LockedDependencyRef>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq, Eq)]
pub struct LockedDependencyRef {
    pub version: super::version::Version,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chain: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LockedSource {
    Git {
        url: String,
        #[serde(rename = "ref")]
        git_ref: String,
    },
    Path {
        path: PathBuf,
    },
    Registry {
        url: String,
    },
}

impl Lockfile {
    pub fn new(root: LockedPackage) -> Self {
        Self {
            root,
            packages: BTreeMap::new(),
        }
    }

    pub fn add_package(
        &mut self,
        package: LockedPackage,
    ) {
        let key = format!("{}@{}", package.name, package.version);
        self.packages.insert(key, package);
    }

    pub fn get_package(
        &self,
        name: &str,
        version: &super::version::Version,
    ) -> Option<&LockedPackage> {
        let key = format!("{}@{}", name, version);
        self.packages.get(&key)
    }
}

impl LockedPackage {
    pub fn new(
        name: String,
        version: super::version::Version,
        checksum: String,
        source: LockedSource,
    ) -> Self {
        Self {
            name,
            version,
            checksum,
            source,
            dependencies: BTreeMap::new(),
        }
    }

    pub fn add_dependency(
        &mut self,
        name: String,
        dep_ref: LockedDependencyRef,
    ) {
        self.dependencies.insert(name, dep_ref);
    }
}

impl LockedDependencyRef {
    pub fn new(version: super::version::Version) -> Self {
        Self {
            version,
            provides: Vec::new(),
            chain: Vec::new(),
        }
    }

    pub fn add_provides(
        &mut self,
        ident: String,
    ) {
        if !self.provides.contains(&ident) {
            self.provides.push(ident);
        }
    }

    pub fn with_chain(
        mut self,
        chain: Vec<String>,
    ) -> Self {
        self.chain = chain;
        self
    }
}
