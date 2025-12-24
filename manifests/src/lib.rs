use std::path::{Path, PathBuf};

pub mod config;
pub mod lock;
pub mod manager;
pub mod package;
pub mod registry;
pub mod rules;
pub mod version;

use convert_case::{Case, Casing};

pub use crate::config::NewForConfig;
use crate::{config::NewForNamed, package::PackageManifest};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Fs(#[from] kintsu_fs::Error),
    #[error("[{file}] {err}")]
    WithSource { file: PathBuf, err: Box<Self> },
    #[error("config error: {0}")]
    ConfigError(#[from] ::config::ConfigError),
    #[error("validation error: {0}")]
    ValidationError(#[from] ::validator::ValidationError),
    #[error("validation errors: {0}")]
    ValidationErrors(#[from] ::validator::ValidationErrors),
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("{0}")]
    VersionError(#[from] version::VersionError),
    #[error("{0}")]
    SerError(#[from] toml::ser::Error),
    #[error("{0}")]
    DeError(#[from] toml::de::Error),

    #[error("{0}")]
    ManifestError(#[from] InvalidManifest),
}

#[derive(thiserror::Error, Debug, serde::Serialize)]
#[serde(tag = "type", content = "details", rename_all = "snake_case")]
pub enum InvalidManifest {
    #[error("Package license is required in manifest for publication in registries.")]
    PackageMissingLicense,
    #[error("Package readme is required in manifest for publication in registries.")]
    PackageMissingReadme,
    #[error("Package repository is required in manifest for publication in registries.")]
    PackageMissingRepository,

    #[error("Package manifest specifies an unresolved dependency: {name}@{}", version.clone().map(|v| format!("{}", v)).unwrap_or("unknown".into()))]
    UnresolvedDependency {
        name: String,
        #[serde(skip)]
        version: Option<version::VersionReqSerde>,
    },
    #[error("Package manifest contains unresolved dependencies. See sources for details.")]
    UnresolvedDependencies { sources: Vec<InvalidManifest> },
}

impl Error {
    pub fn with_source(
        self,
        file: impl AsRef<Path>,
    ) -> Self {
        Self::WithSource {
            file: file.as_ref().to_path_buf(),
            err: Box::new(self),
        }
    }

    pub fn from_with_source_init<E: Into<Error>>(file: impl AsRef<Path>) -> impl FnOnce(E) -> Self {
        |err| err.into().with_source(file)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn init(
    name: String,
    dir: Option<PathBuf>,
) -> Result<()> {
    use validator::Validate;

    let pkg = package::PackageManifest {
        package: package::PackageMeta {
            name,
            description: None,
            version: version::VersionSerde(version::parse_version("0.1.0")?),
            authors: vec![],
            keywords: vec![],
            homepage: None,
            license: None,
            readme: None,
            repository: None,
        },
        dependencies: Default::default(),
        files: Default::default(),
    };

    pkg.validate()?;

    let dir = dir.unwrap_or_else(|| PathBuf::from(pkg.package.name.clone()));

    let manifest = dir.join(PackageManifest::NAME);

    if !dir.exists() {
        std::fs::create_dir(&dir)?;
    }

    let out = toml::to_string(&pkg)?;
    std::fs::write(manifest, out)?;

    let schema = dir.join("schema/");
    if !schema.exists() {
        std::fs::create_dir(dir)?;
    }

    let lib = schema.join("lib.ks");
    if !lib.exists() {
        std::fs::write(
            lib,
            format!(
                "#![version(1)]\nnamespace {};\n",
                pkg.package.name.to_case(Case::Snake)
            ),
        )?;
    }

    Ok(())
}
