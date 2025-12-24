#![allow(clippy::declare_interior_mutable_const)]

use crate::config::NewForNamed;
use regex::Regex;
use std::{collections::BTreeMap, path::PathBuf, sync::LazyLock};
use validator::{Validate, ValidationError, ValidationErrors};

const REGISTRY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("[a-z]([a-z0-9\\-]*)[a-z0-9]").expect("registry re"));

const PACKAGE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("[a-z]([a-z0-9\\-]*)[a-z0-9]").expect("package re"));

const TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("[a-z]([a-z0-9\\-]*)[a-z0-9]").expect("tag re"));

static GIT_REF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"refs\/(heads|tags)\/[a-zA-Z0-9\-_]+").unwrap());

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(untagged)]
pub enum PathOrText {
    Path {
        #[cfg_attr(feature = "api", schema(value_type = String, format = "path"))]
        path: PathBuf,
    },
    Text(String),
}

impl PathOrText {
    pub fn text<F: kintsu_fs::FileSystem>(
        &self,
        fs: &F,
    ) -> crate::Result<String> {
        Ok(match self {
            Self::Path { path } => fs.read_to_string_sync(path)?,
            Self::Text(text) => text.clone(),
        })
    }
}

impl PathOrText {
    pub fn text_opt<F: kintsu_fs::FileSystem>(
        this: Option<&Self>,
        fs: &F,
    ) -> crate::Result<Option<String>> {
        match this {
            Some(pot) => Ok(Some(pot.text(fs)?)),
            None => Ok(None),
        }
    }
}

#[allow(clippy::borrow_interior_mutable_const)]
fn validate_name(name: &str) -> Result<(), ValidationError> {
    const ERR_SPEC: &str = "package name must be provided without spaces or special characters";
    if let Some(capt) = PACKAGE_RE.find(name) {
        if capt.as_str().len() != name.len() {
            return Err(ValidationError::new("package name").with_message(ERR_SPEC.into()));
        }
    } else {
        return Err(ValidationError::new("package name").with_message(ERR_SPEC.into()));
    }

    Ok(())
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(serde::Deserialize, serde::Serialize, Clone, validator::Validate)]
pub struct Author {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(email)]
    pub email: Option<String>,
}

fn validate_keyword(keyword: &str) -> Result<(), ValidationError> {
    const ERR_SPEC: &str = "keywords must be provided without spaces or special characters";
    if let Some(capt) = TAG_RE.find(keyword) {
        if capt.as_str().len() != keyword.len() {
            return Err(ValidationError::new("keyword").with_message(ERR_SPEC.into()));
        }
    } else {
        return Err(ValidationError::new("keyword").with_message(ERR_SPEC.into()));
    }
    Ok(())
}

fn validate_keywords(keywords: &Vec<String>) -> Result<(), ValidationError> {
    for keyword in keywords {
        validate_keyword(keyword)?;
    }
    Ok(())
}

/// Package metadata as defined in schema.toml.
/// This package is first deserialized from the manifest file, requiring `.validate()` to be called
/// to ensure all fields are valid. After validation, the `resolve` method should be called
/// to resolve any paths to text content if the package will interact with the registry
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(serde::Deserialize, serde::Serialize, Clone, validator::Validate)]
pub struct PackageMeta {
    /// The name of the package
    #[validate(length(min = 2, max = 128), custom(function = "validate_name"))]
    pub name: String,

    /// A short description of the package
    #[serde(default)]
    pub description: Option<PathOrText>,

    /// The version of the package
    #[cfg_attr(feature = "api", schema(value_type = String, format = "version"))]
    pub version: super::version::VersionSerde,

    /// The authors of the package
    #[serde(default)]
    pub authors: Vec<Author>,

    /// The homepage URL of the package
    #[serde(default)]
    #[validate(url)]
    pub homepage: Option<String>,

    /// The license of the package (text or path to file)
    #[serde(default)]
    pub license: Option<PathOrText>,

    /// The readme of the package (text or path to file)
    #[serde(default)]
    pub readme: Option<PathOrText>,

    /// The repository URL of the package
    #[serde(default)]
    #[validate(url)]
    pub repository: Option<String>,

    /// Keywords associated with the package
    #[serde(default)]
    #[validate(custom(function = validate_keywords))]
    pub keywords: Vec<String>,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(serde::Deserialize, serde::Serialize, Clone, validator::Validate, Default)]
pub struct FileConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
}

pub type NamedDependencies = BTreeMap<String, Dependency>;

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(serde::Deserialize, validator::Validate, serde::Serialize, Clone)]
pub struct PackageManifest {
    #[validate(nested)]
    pub package: PackageMeta,

    #[serde(default)]
    pub files: FileConfig,

    #[serde(default = "BTreeMap::new")]
    pub dependencies: NamedDependencies,
}

impl PackageManifest {
    fn dependency_error(
        name: &str,
        reason: &str,
    ) -> ValidationError {
        ValidationError::new("dependencies")
            .with_message(format!("'dependencies.{name}' is invalid: {reason}").into())
    }

    fn with_dependency_error(
        errors: &mut ValidationErrors,
        name: &str,
        reason: &str,
    ) {
        errors.add("dependencies", Self::dependency_error(name, reason));
    }

    fn validate_remote(
        errors: &mut ValidationErrors,
        package_version: &crate::version::VersionSerde,
        name: &str,
        remote: &mut RemoteDependency,
    ) {
        use crate::version::VersionExt;
        if package_version.is_stable() && !remote.version.is_stable() {
            Self::with_dependency_error(
                errors,
                name,
                "cannot depend on pre-release versions from a stable package",
            );
        }
    }

    pub fn prepare_publish(&mut self) -> Result<(), validator::ValidationErrors> {
        self.validate()?;
        let mut errors = ValidationErrors::new();
        for (name, dep) in self.dependencies.iter_mut() {
            match dep {
                Dependency::Path(_) => {
                    Self::with_dependency_error(
                        &mut errors,
                        name,
                        "path dependencies are not allowed for publishing",
                    );
                },
                Dependency::Git(_) => {
                    Self::with_dependency_error(
                        &mut errors,
                        name,
                        "git dependencies are not allowed for publishing",
                    )
                },
                Dependency::PathWithRemote(pwr) => {
                    Self::validate_remote(
                        &mut errors,
                        &self.package.version,
                        name,
                        &mut pwr.remote,
                    );
                    *dep = Dependency::Remote(pwr.remote.clone());
                },
                Dependency::Remote(remote) => {
                    Self::validate_remote(&mut errors, &self.package.version, name, remote);
                },
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(())
    }

    pub fn resolve<F: kintsu_fs::FileSystem>(
        &mut self,
        fs: &F,
    ) -> crate::Result<()> {
        if let Some(desc) = &mut self.package.description {
            *desc = PathOrText::Text(desc.text(fs)?);
        }

        if let Some(license) = &mut self.package.license {
            *license = PathOrText::Text(license.text(fs)?);
        }

        if let Some(readme) = &mut self.package.readme {
            *readme = PathOrText::Text(readme.text(fs)?);
        }

        Ok(())
    }
}

impl NewForNamed for PackageManifest {
    const NAME: &str = "schema.toml";
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(serde::Deserialize, serde::Serialize, Clone, validator::Validate)]
pub struct GitDependency {
    #[validate(url)]
    #[cfg_attr(feature = "api", schema(value_type = String, format = "url"))]
    pub git: String,

    #[validate(length(min = 1), regex(path = *GIT_REF_RE, message = "git ref must be in the format refs/heads/<branch> or refs/tags/<tag>"))]
    #[cfg_attr(feature = "api", schema(value_type = String, format = "ref"))]
    #[serde(rename = "ref")]
    pub git_ref: String,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(serde::Deserialize, serde::Serialize, Clone, validator::Validate)]
pub struct PathDependency {
    #[cfg_attr(feature = "api", schema(value_type = String, format = "path"))]
    pub path: PathBuf,
}

fn default_registry() -> Option<String> {
    Some("kintsu-public".to_string())
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(serde::Deserialize, serde::Serialize, Clone, validator::Validate)]
pub struct RemoteDependency {
    #[serde(default = "default_registry")]
    #[validate(regex(path = *REGISTRY_RE, message = "registry must be a valid URL"))]
    pub name: Option<String>,

    /// Version requirement for the dependency. Supports semver ranges like:
    /// - `1.0.0` (exact version, treated as ^1.0.0)
    /// - `^1.0.0` (caret requirement)
    /// - `~1.0.0` (tilde requirement)
    /// - `>= 1.0.0, < 2.0.0` (range requirement)
    #[cfg_attr(feature = "api", schema(value_type = String, format = "version"))]
    pub version: crate::version::VersionReqSerde,

    pub registry: Option<String>,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(serde::Deserialize, serde::Serialize, Clone, validator::Validate)]
pub struct PathWithRemote {
    #[serde(flatten)]
    #[validate(nested)]
    pub path: PathDependency,
    #[serde(flatten)]
    #[validate(nested)]
    pub remote: RemoteDependency,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(untagged)]
pub enum Dependency {
    PathWithRemote(PathWithRemote),

    Remote(RemoteDependency),
    Path(PathDependency),

    Git(GitDependency),
}

impl Dependency {
    pub fn version(&self) -> Option<&crate::version::VersionReqSerde> {
        match self {
            Dependency::Git(_) => None,
            Dependency::Path(_) => None,
            Dependency::Remote(dep) => Some(&dep.version),
            Dependency::PathWithRemote(dep) => Some(&dep.remote.version),
        }
    }
}

impl validator::Validate for Dependency {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            Dependency::Git(dep) => dep.validate(),
            Dependency::Path(dep) => dep.validate(),
            Dependency::Remote(dep) => dep.validate(),
            Dependency::PathWithRemote(dep) => dep.validate(),
        }
    }
}

#[cfg(test)]
mod test {
    use validator::Validate;

    use crate::version::{VersionSerde, parse_version};

    #[test_case::test_case("abc-types", "0.1.0", "https://github.com/abc/foo.git"; "valid name with dash")]
    #[test_case::test_case("abc", "0.1.0", "https://github.com/abc/foo.git"; "simple name")]
    #[test_case::test_case("abc", "0.1.0-rc.0", "https://github.com/abc/foo.git"; "version with rc")]
    fn test_pkg_validate_ok(
        name: &str,
        version: &str,
        homepage: &str,
    ) {
        let p = super::PackageMeta {
            name: name.into(),
            description: None,
            version: VersionSerde(parse_version(version).unwrap()),
            authors: vec![],
            homepage: Some(homepage.into()),
            keywords: vec![],
            license: None,
            readme: None,
            repository: None,
        };
        p.validate().unwrap();
    }

    #[test_case::test_case("abc_types", "0.1.0", "https://github.com/abc/foo.git", vec![], "name: package name"; "invalid name with underscore")]
    #[test_case::test_case("a", "0.1.0", "https://github.com/abc/foo.git", vec![], "name: Validation error: length"; "name too short")]
    #[test_case::test_case("a".repeat(129).as_str(),
        "0.1.0", "https://github.com/abc/foo.git", vec![],
        "name: Validation error: length"; "name too long")]
    #[test_case::test_case("abc_types!", "0.1.0", "https://github.com/abc/foo.git", vec![], "name: package name must be provided without spaces or special characters"; "invalid character in name")]
    #[test_case::test_case("abc_types", "0.1.0", "not-a-url", vec![], "homepage: Validation error: url [{\"value\": String(\"not-a-url\")}]"; "invalid homepage url")]
    #[test_case::test_case("abc", "0.1.0", "https://github.com/abc/foo.git", vec![" foo".into()], "keywords: keywords must be provided"; "keyword prefixed with space")]
    #[test_case::test_case("abc", "0.1.0", "https://github.com/abc/foo.git", vec!["foo bar".into()], "keywords: keywords must be provided"; "keyword with space inside")]
    #[test_case::test_case("abc", "0.1.0", "https://github.com/abc/foo.git", vec!["".into()], "keywords: keywords must be provided"; "empty keyword")]

    fn test_pkg_validate_err(
        name: &str,
        version: &str,
        homepage: &str,
        keywords: Vec<String>,
        expect: &str,
    ) {
        let p = super::PackageMeta {
            name: name.into(),
            description: None,
            version: VersionSerde(parse_version(version).unwrap()),
            authors: vec![],
            homepage: Some(homepage.into()),
            keywords,
            license: None,
            readme: None,
            repository: None,
        };
        let err = p.validate().unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains(expect),
            "expected error to contain '{expect}', found '{msg}'"
        );
    }
}
