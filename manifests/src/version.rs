use std::{fmt::Display, num::ParseIntError};

use serde::{Deserialize, Serialize, de::Visitor};
use validator::ValidationError;

#[derive(thiserror::Error, Debug)]
pub enum VersionError {
    #[error(".{{rev,rc,...}} versions are expected to be in the form 'rev.0'. found '{found}'")]
    VariantExpectation { found: String },

    #[error("{0}")]
    ParseIntError(#[from] ParseIntError),

    #[error(
        "'{given}' is unknown. valid variant forms are stable, canary, rc.0, pre.0, beta.0, alpha.0, post.0, and rev.0"
    )]
    UnknownVariant { given: String },

    #[error("expected {expect}, found '{given}'")]
    Expected { expect: &'static str, given: String },
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Variant {
    Stable,
    Canary,
    Rc(usize),
    Pre(usize),
    Alpha(usize),
    Beta(usize),
    Post(usize),
    Rev(usize),
}

impl Display for Variant {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Stable => write!(f, ""),
            Self::Canary => write!(f, "canary"),
            Self::Rc(ver) => write!(f, "rc{ver}"),
            Self::Pre(ver) => write!(f, "pre{ver}"),
            Self::Alpha(ver) => write!(f, "alpha{ver}"),
            Self::Beta(ver) => write!(f, "beta{ver}"),
            Self::Post(ver) => write!(f, "post{ver}"),
            Self::Rev(ver) => write!(f, "rev{ver}"),
        }
    }
}

impl Variant {
    pub fn is_compatible(
        &self,
        other: &Self,
    ) -> bool {
        match (self, other) {
            (Variant::Stable, Variant::Stable) | (Variant::Canary, Variant::Canary) => true,
            (Variant::Stable, Variant::Post(_)) | (Variant::Post(_), Variant::Stable) => true,
            (Variant::Stable, Variant::Rev(_)) | (Variant::Rev(_), Variant::Stable) => true,
            (Variant::Rc(a), Variant::Rc(b)) => a == b,
            (Variant::Pre(a), Variant::Pre(b)) => a == b,
            (Variant::Alpha(a), Variant::Alpha(b)) => a == b,
            (Variant::Beta(a), Variant::Beta(b)) => a == b,
            (Variant::Post(a), Variant::Post(b)) => a == b,
            (Variant::Rev(a), Variant::Rev(b)) => a == b,
            _ => false,
        }
    }
    pub fn parse(s: &str) -> Result<Self, VersionError> {
        if s.is_empty() || s == "stable" {
            return Ok(Self::Stable);
        } else if s == "canary" {
            return Ok(Self::Canary);
        }

        let pos = match s
            .chars()
            .enumerate()
            .map_while(|it| {
                if it.1.is_alphabetic() {
                    Some(it.0)
                } else {
                    None
                }
            })
            .last()
        {
            Some(v) => v,
            None => {
                return Err(VersionError::Expected {
                    expect: "extension characters (rc, rev, ...)",
                    given: s.into(),
                });
            },
        };

        if s.len() == pos + 1 {
            return Err(VersionError::VariantExpectation { found: s.into() });
        }

        let (ty, ver) = (&s[..pos + 1], s[pos + 1..].parse()?);

        Ok(match ty {
            "rc" => Self::Rc(ver),
            "pre" => Self::Pre(ver),
            "alpha" => Self::Alpha(ver),
            "beta" => Self::Beta(ver),
            "post" => Self::Post(ver),
            "rev" => Self::Rev(ver),
            ty => return Err(VersionError::UnknownVariant { given: ty.into() }),
        })
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct Version {
    maj: usize,
    minor: Option<usize>,
    patch: Option<usize>,
    ext: Variant,
}

impl Version {
    pub fn is_compatible(
        &self,
        other: Version,
    ) -> bool {
        let maj = self.maj == other.maj;
        if !maj {
            return false;
        }
        let min = if let Some(minor) = self.minor
            && let Some(other) = other.minor
        {
            minor == other
        } else {
            true
        };

        let patch = if let Some(patch) = self.patch
            && let Some(other) = other.patch
        {
            patch == other
        } else {
            true
        };

        maj && min && patch && self.ext.is_compatible(&other.ext)
    }

    pub fn is_stable(&self) -> bool {
        matches!(self.ext, Variant::Stable)
    }
}

impl Version {
    pub fn parse(target: &str) -> Result<Self, VersionError> {
        if target.is_empty() {
            return Err(VersionError::Expected {
                expect: "major version",
                given: target.into(),
            });
        }

        let mut s = target.split(".");

        let maj = s.next().unwrap().parse()?;

        let minor = match s.next() {
            Some(v) => Some(v.parse()?),
            None => None,
        };

        let (patch, ext) = match s.next() {
            Some(v) => {
                let pos = match v
                    .chars()
                    .enumerate()
                    .map_while(|it| {
                        if it.1.is_numeric() {
                            Some(it.0)
                        } else {
                            None
                        }
                    })
                    .last()
                {
                    Some(v) => v,
                    None => {
                        return Err(VersionError::Expected {
                            expect: "patch version specifier",
                            given: v.into(),
                        });
                    },
                };
                let patch: usize = v[..pos + 1].parse()?;
                Some(if v.len() != pos + 1 {
                    (Some(patch), Variant::parse(&v[pos + 1..])?)
                } else {
                    (Some(patch), Variant::Stable)
                })
            },
            None => None,
        }
        .unwrap_or((None, Variant::Stable));

        Ok(Self {
            maj,
            minor,
            patch,
            ext,
        })
    }

    pub fn valid_for_package(&self) -> Result<(), validator::ValidationError> {
        if self.minor.is_none() {
            return Err(ValidationError::new("version error").with_message(
                "minor version is required in fully qualified package versions".into(),
            ));
        } else if self.patch.is_none() {
            return Err(ValidationError::new("version error").with_message(
                "patch version is required in fully qualified package versions".into(),
            ));
        }

        Ok(())
    }
}

impl TryInto<Version> for String {
    type Error = VersionError;

    fn try_into(self) -> Result<Version, Self::Error> {
        Version::parse(&self)
    }
}

impl Display for Version {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match (self.minor, self.patch) {
            (Some(minor), Some(patch)) => write!(f, "{}.{}.{}{}", self.maj, minor, patch, self.ext),
            (Some(minor), None) => write!(f, "{}.{}", self.maj, minor),
            _ => write!(f, "{}", self.maj),
        }
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>, {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Version;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                write!(formatter, "version string (0, 0.1, 0.1.1, 0.1.0rc0)")
            }

            fn visit_str<E>(
                self,
                v: &str,
            ) -> Result<Self::Value, E>
            where
                E: serde::de::Error, {
                Version::parse(v).map_err(|err| serde::de::Error::custom(err))
            }
        }

        deserializer.deserialize_str(V)
    }
}

impl Serialize for Version {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer, {
        serializer.serialize_str(&format!("{self}"))
    }
}

mod db {
    use super::*;

    impl sea_orm::sea_query::ValueType for Version {
        fn type_name() -> String {
            "Version".to_owned()
        }

        fn array_type() -> sea_orm::sea_query::ArrayType {
            sea_orm::sea_query::ArrayType::String
        }

        fn column_type() -> sea_orm::ColumnType {
            sea_orm::ColumnType::String(sea_orm::sea_query::StringLen::N(32))
        }

        fn try_from(
            v: sea_orm::sea_query::Value
        ) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
            if let sea_orm::sea_query::Value::String(Some(s)) = v {
                Version::parse(&s).map_err(|e| {
                    tracing::error!("Failed to parse Version from string: {}", e);
                    sea_orm::sea_query::ValueTypeErr
                })
            } else {
                Err(sea_orm::sea_query::ValueTypeErr)
            }
        }
    }

    impl sea_orm::TryGetable for Version {
        fn try_get_by<I: sea_orm::ColIdx>(
            res: &sea_orm::QueryResult,
            index: I,
        ) -> Result<Self, sea_orm::TryGetError> {
            let s: String = sea_orm::TryGetable::try_get_by(res, index)?;
            Version::parse(&s).map_err(|e| {
                tracing::error!("Failed to parse Version from database string: {}", e);
                sea_orm::TryGetError::DbErr(sea_orm::DbErr::Type(format!("{}", e)))
            })
        }
    }

    impl From<Version> for sea_orm::Value {
        fn from(v: Version) -> Self {
            sea_orm::Value::String(Some(format!("{}", v)))
        }
    }
}

#[cfg(test)]
mod test {
    #[test_case::test_case("0")]
    #[test_case::test_case("0.1")]
    #[test_case::test_case("0.1.1")]
    #[test_case::test_case("0.1.1rc0")]
    #[test_case::test_case("0.1.1canary")]
    pub fn test_version_rt(src: &str) {
        let ver = super::Version::parse(src).unwrap();
        assert_eq!(src, format!("{ver}"));
    }

    #[test_case::test_case("", "expected major version, found ''"; "empty string")]
    #[test_case::test_case("abc", "invalid digit found in string"; "non-numeric major")]
    #[test_case::test_case("1..2", "cannot parse integer from empty string"; "double dot")]
    #[test_case::test_case("1.a", "invalid digit found in string"; "non-numeric minor")]
    #[test_case::test_case("1.2.a", "expected patch version specifier, found 'a'"; "non-numeric patch")]
    #[test_case::test_case("1.2.3foo", ".{rev,rc,...} versions are expected to be in the form 'rev.0'. found 'foo'"; "unknown variant after patch")]
    #[test_case::test_case("1.2.3rc", ".{rev,rc,...} versions are expected to be in the form 'rev.0'. found 'rc'"; "missing rc number")]
    #[test_case::test_case("1.2.3unknown0", "'unknown' is unknown. valid variant forms are stable, canary, rc.0, pre.0, beta.0, alpha.0, post.0, and rev.0"; "unknown variant")]
    fn test_version_parse_errors(
        src: &str,
        expected_msg: &str,
    ) {
        let err = super::Version::parse(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains(expected_msg),
            "expected error message to contain '{expected_msg}', got '{msg}'"
        );
    }
}
