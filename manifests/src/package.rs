use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, LazyLock},
};
use validator::{Validate, ValidationError};

use crate::config::NewForNamed;

#[allow(clippy::declare_interior_mutable_const)]
const PACKAGE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new("[a-z]([a-z0-9\\-]*)[a-z0-9]").expect("package re"));

#[allow(clippy::declare_interior_mutable_const)]
const TAG_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new("[a-z]([a-z0-9\\-]*)[a-z0-9]").expect("tag re"));

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(untagged)]
pub enum PathOrText {
    Path { path: PathBuf },
    Text(String),
}

impl PathOrText {
    pub fn text(
        &self,
        fs: &Arc<dyn kintsu_fs::FileSystem>,
    ) -> crate::Result<String> {
        Ok(match self {
            Self::Path { path } => fs.read_to_string_sync(path)?,
            Self::Text(text) => text.clone(),
        })
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
#[derive(serde::Deserialize, serde::Serialize, Clone, validator::Validate)]
pub struct PackageMeta {
    /// The name of the package
    #[validate(length(min = 2, max = 128), custom(function = validate_name))]
    pub name: String,

    /// A short description of the package
    #[serde(default)]
    pub description: Option<PathOrText>,

    /// The version of the package
    #[validate(custom(function = super::version::Version::valid_for_package))]
    pub version: super::version::Version,

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

    /// Keywords associated with the package
    #[serde(default)]
    #[validate(custom(function = validate_keywords))]
    pub keywords: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, validator::Validate, Default)]
pub struct FileConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(serde::Deserialize, validator::Validate, serde::Serialize, Clone)]
pub struct PackageManifest {
    #[validate(nested)]
    pub package: PackageMeta,

    #[serde(default)]
    pub files: FileConfig,

    #[serde(default = "BTreeMap::new")]
    pub dependencies: BTreeMap<String, Dependency>,
}

impl PackageManifest {
    pub fn resolve(
        &mut self,
        fs: Arc<dyn kintsu_fs::FileSystem>,
    ) -> crate::Result<()> {
        if let Some(desc) = &mut self.package.description {
            *desc = PathOrText::Text(desc.text(&fs)?);
        }

        if let Some(license) = &mut self.package.license {
            *license = PathOrText::Text(license.text(&fs)?);
        }

        if let Some(readme) = &mut self.package.readme {
            *readme = PathOrText::Text(readme.text(&fs)?);
        }

        Ok(())
    }
}

impl NewForNamed for PackageManifest {
    const NAME: &str = "schema.toml";
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(untagged)]
pub enum Dependency {
    Git {
        git: String,
        #[serde(rename = "ref")]
        git_ref: String,
        rev: String,
    },
    Path {
        path: PathBuf,
    },
    Remote {
        name: String,
        version: crate::version::Version,
        registry: Option<String>,
    },
}

#[cfg(test)]
mod test {
    use validator::Validate;

    use crate::version::Version;

    #[test_case::test_case("abc_types", "0.1.0", "https://github.com/abc/foo.git"; "valid name with underscore")]
    #[test_case::test_case("abc-types", "0.1.0", "https://github.com/abc/foo.git"; "valid name with dash")]
    #[test_case::test_case("abc", "0.1.0", "https://github.com/abc/foo.git"; "simple name")]
    #[test_case::test_case("abc", "0.1.0.rc0", "https://github.com/abc/foo.git"; "version with rc")]
    fn test_pkg_validate_ok(
        name: &str,
        version: &str,
        homepage: &str,
    ) {
        let p = super::PackageMeta {
            name: name.into(),
            description: None,
            version: Version::parse(version).unwrap(),
            authors: vec![],
            homepage: Some(homepage.into()),
            keywords: vec![],
            license: None,
            readme: None,
        };
        p.validate().unwrap();
    }

    #[test_case::test_case("a", "0.1.0", "https://github.com/abc/foo.git", vec![], "name: Validation error: length"; "name too short")]
    #[test_case::test_case("a".repeat(129).as_str(),
        "0.1.0", "https://github.com/abc/foo.git", vec![],
        "name: Validation error: length"; "name too long")]
    #[test_case::test_case("abc_types!", "0.1.0", "https://github.com/abc/foo.git", vec![], "name: package name must be provided without spaces or special characters"; "invalid character in name")]
    #[test_case::test_case("abc_types", "0.1.0", "not-a-url", vec![], "homepage: Validation error: url [{\"value\": String(\"not-a-url\")}]"; "invalid homepage url")]
    #[test_case::test_case("abc", "0.1", "https://github.com/abc/foo.git", vec![], "version: Validation error:"; "version without patch")]
    #[test_case::test_case("abc", "0.1.0", "https://github.com/abc/foo.git", vec![" foo".into()], "keywords: Validation error:"; "keyword prefixed with space")]
    #[test_case::test_case("abc", "0.1.0", "https://github.com/abc/foo.git", vec!["foo bar".into()], "keywords: Validation error:"; "keyword with space inside")]
    #[test_case::test_case("abc", "0.1.0", "https://github.com/abc/foo.git", vec!["".into()], "keywords: Validation error:"; "empty keyword")]

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
            version: Version::parse(version).unwrap(),
            authors: vec![],
            homepage: Some(homepage.into()),
            keywords,
            license: None,
            readme: None,
        };
        let err = p.validate().unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains(expect),
            "expected error to contain '{expect}', found '{msg}'"
        );
    }
}
