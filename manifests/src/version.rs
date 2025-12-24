//! Semantic versioning for kintsu packages.
//!
//! This module provides cargo-flavored semver versioning using the `semver` crate.
//! It supports both exact versions (`Version`) and version requirements (`VersionReq`)
//! for specifying dependency constraints like `>= 1.0.0, < 2.0.0`.
//!
//! All versions must be valid semver format (e.g., `1.0.0`, `1.0.0-rc.0`, `1.0.0-alpha.1+build`).

use std::fmt::Display;

use serde::{Deserialize, Serialize, de::Visitor};
use validator::ValidationError;

// Re-export semver types
pub use semver::{Version, VersionReq};

#[derive(thiserror::Error, Debug)]
pub enum VersionError {
    #[error("invalid semver: {0}")]
    SemverError(#[from] semver::Error),

    #[error("expected {expect}, found '{given}'")]
    Expected { expect: &'static str, given: String },
}

/// Parse a version string in strict semver format.
pub fn parse_version(input: &str) -> Result<Version, VersionError> {
    if input.is_empty() {
        return Err(VersionError::Expected {
            expect: "version string",
            given: input.into(),
        });
    }

    Version::parse(input).map_err(VersionError::from)
}

/// Parse a version requirement string for dependency specifications.
///
/// Supports:
/// - Exact versions: `1.0.0`
/// - Ranges: `>= 1.0.0, < 2.0.0`
/// - Caret requirements: `^1.0.0` (default cargo behavior)
/// - Tilde requirements: `~1.0.0`
/// - Wildcards: `1.*`, `1.0.*`
pub fn parse_version_req(input: &str) -> Result<VersionReq, VersionError> {
    if input.is_empty() {
        return Err(VersionError::Expected {
            expect: "version requirement string",
            given: input.into(),
        });
    }

    VersionReq::parse(input).map_err(VersionError::from)
}

/// Extension trait for Version to provide kintsu-specific functionality
pub trait VersionExt {
    /// Check if this is a stable release (no pre-release identifiers)
    fn is_stable(&self) -> bool;

    /// Validate that this version is fully qualified for package publishing
    fn valid_for_package(&self) -> Result<(), ValidationError>;

    /// Check if two versions are compatible (same major.minor.patch, compatible pre-release)
    fn is_compatible(
        &self,
        other: &Self,
    ) -> bool;
}

impl VersionExt for Version {
    fn is_stable(&self) -> bool {
        self.pre.is_empty()
    }

    fn valid_for_package(&self) -> Result<(), ValidationError> {
        // semver::Version always has major, minor, patch - no validation needed
        Ok(())
    }

    fn is_compatible(
        &self,
        other: &Self,
    ) -> bool {
        self.major == other.major && self.minor == other.minor && self.patch == other.patch
    }
}

/// Wrapper for serde serialization/deserialization of Version
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionSerde(pub Version);

impl From<Version> for VersionSerde {
    fn from(v: Version) -> Self {
        Self(v)
    }
}

impl From<VersionSerde> for Version {
    fn from(v: VersionSerde) -> Self {
        v.0
    }
}

impl std::ops::Deref for VersionSerde {
    type Target = Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for VersionSerde {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for VersionSerde {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>, {
        struct V;
        impl Visitor<'_> for V {
            type Value = VersionSerde;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                write!(
                    formatter,
                    "semver version string (e.g., '1.0.0', '1.0.0-rc.0')"
                )
            }

            fn visit_str<E>(
                self,
                v: &str,
            ) -> Result<Self::Value, E>
            where
                E: serde::de::Error, {
                parse_version(v)
                    .map(VersionSerde)
                    .map_err(|err| serde::de::Error::custom(err))
            }
        }

        deserializer.deserialize_str(V)
    }
}

impl Serialize for VersionSerde {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer, {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl PartialOrd for VersionSerde {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionSerde {
    fn cmp(
        &self,
        other: &Self,
    ) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

/// Wrapper for serde serialization/deserialization of VersionReq
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionReqSerde(pub VersionReq);

impl From<VersionReq> for VersionReqSerde {
    fn from(v: VersionReq) -> Self {
        Self(v)
    }
}

impl From<VersionReqSerde> for VersionReq {
    fn from(v: VersionReqSerde) -> Self {
        v.0
    }
}

impl std::ops::Deref for VersionReqSerde {
    type Target = VersionReq;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for VersionReqSerde {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl VersionReqSerde {
    /// Check if this version requirement only allows stable versions.
    /// Returns false if the requirement might match pre-release versions.
    pub fn is_stable(&self) -> bool {
        self.0
            .comparators
            .iter()
            .all(|c| c.pre.is_empty())
    }
}

impl<'de> Deserialize<'de> for VersionReqSerde {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>, {
        struct V;
        impl Visitor<'_> for V {
            type Value = VersionReqSerde;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                write!(
                    formatter,
                    "version requirement (e.g., '1.0.0', '>= 1.0.0', '^1.0.0', '~1.0.0')"
                )
            }

            fn visit_str<E>(
                self,
                v: &str,
            ) -> Result<Self::Value, E>
            where
                E: serde::de::Error, {
                parse_version_req(v)
                    .map(VersionReqSerde)
                    .map_err(|err| serde::de::Error::custom(err))
            }
        }

        deserializer.deserialize_str(V)
    }
}

impl Serialize for VersionReqSerde {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer, {
        serializer.serialize_str(&self.0.to_string())
    }
}

mod db {
    use super::*;

    impl sea_orm::sea_query::ValueType for VersionSerde {
        fn type_name() -> String {
            "Version".to_owned()
        }

        fn array_type() -> sea_orm::sea_query::ArrayType {
            sea_orm::sea_query::ArrayType::String
        }

        fn column_type() -> sea_orm::ColumnType {
            sea_orm::ColumnType::String(sea_orm::sea_query::StringLen::N(64))
        }

        fn try_from(
            v: sea_orm::sea_query::Value
        ) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
            if let sea_orm::sea_query::Value::String(Some(s)) = v {
                parse_version(&s)
                    .map(VersionSerde)
                    .map_err(|e| {
                        tracing::error!("Failed to parse Version from string: {}", e);
                        sea_orm::sea_query::ValueTypeErr
                    })
            } else {
                Err(sea_orm::sea_query::ValueTypeErr)
            }
        }
    }

    impl sea_orm::TryGetable for VersionSerde {
        fn try_get_by<I: sea_orm::ColIdx>(
            res: &sea_orm::QueryResult,
            index: I,
        ) -> Result<Self, sea_orm::TryGetError> {
            let s: String = sea_orm::TryGetable::try_get_by(res, index)?;
            parse_version(&s)
                .map(VersionSerde)
                .map_err(|e| {
                    tracing::error!("Failed to parse Version from database string: {}", e);
                    sea_orm::TryGetError::DbErr(sea_orm::DbErr::Type(format!("{}", e)))
                })
        }
    }

    impl From<VersionSerde> for sea_orm::Value {
        fn from(v: VersionSerde) -> Self {
            sea_orm::Value::String(Some(v.0.to_string()))
        }
    }

    impl sea_orm::sea_query::ValueType for VersionReqSerde {
        fn type_name() -> String {
            "VersionReq".to_owned()
        }

        fn array_type() -> sea_orm::sea_query::ArrayType {
            sea_orm::sea_query::ArrayType::String
        }

        fn column_type() -> sea_orm::ColumnType {
            sea_orm::ColumnType::String(sea_orm::sea_query::StringLen::N(128))
        }

        fn try_from(
            v: sea_orm::sea_query::Value
        ) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
            if let sea_orm::sea_query::Value::String(Some(s)) = v {
                parse_version_req(&s)
                    .map(VersionReqSerde)
                    .map_err(|e| {
                        tracing::error!("Failed to parse VersionReq from string: {}", e);
                        sea_orm::sea_query::ValueTypeErr
                    })
            } else {
                Err(sea_orm::sea_query::ValueTypeErr)
            }
        }
    }

    impl sea_orm::TryGetable for VersionReqSerde {
        fn try_get_by<I: sea_orm::ColIdx>(
            res: &sea_orm::QueryResult,
            index: I,
        ) -> Result<Self, sea_orm::TryGetError> {
            let s: String = sea_orm::TryGetable::try_get_by(res, index)?;
            parse_version_req(&s)
                .map(VersionReqSerde)
                .map_err(|e| {
                    tracing::error!("Failed to parse VersionReq from database string: {}", e);
                    sea_orm::TryGetError::DbErr(sea_orm::DbErr::Type(format!("{}", e)))
                })
        }
    }

    impl From<VersionReqSerde> for sea_orm::Value {
        fn from(v: VersionReqSerde) -> Self {
            sea_orm::Value::String(Some(v.0.to_string()))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test_case::test_case("0.1.0"; "stable version")]
    #[test_case::test_case("1.2.3"; "full version")]
    #[test_case::test_case("0.1.0-rc.0"; "rc prerelease")]
    #[test_case::test_case("0.1.0-alpha.1"; "alpha prerelease")]
    #[test_case::test_case("0.1.0-beta.2"; "beta prerelease")]
    #[test_case::test_case("1.0.0+build.123"; "build metadata")]
    #[test_case::test_case("1.0.0-rc.1+build"; "prerelease with build")]
    pub fn test_version_parse_ok(src: &str) {
        let ver = parse_version(src).unwrap();
        assert_eq!(src, ver.to_string());
    }

    #[test_case::test_case(""; "empty string")]
    #[test_case::test_case("abc"; "non-numeric")]
    #[test_case::test_case("1.2"; "missing patch")]
    #[test_case::test_case("0.1.0rc0"; "legacy format rejected")]
    #[test_case::test_case("0.1.0alpha1"; "legacy alpha rejected")]
    fn test_version_parse_errors(src: &str) {
        let result = parse_version(src);
        assert!(result.is_err(), "expected error for '{src}'");
    }

    #[test_case::test_case("1.0.0" => "^1.0.0"; "exact version becomes caret")]
    #[test_case::test_case("^1.0.0" => "^1.0.0"; "caret requirement")]
    #[test_case::test_case("~1.0.0" => "~1.0.0"; "tilde requirement")]
    #[test_case::test_case(">= 1.0.0" => ">=1.0.0"; "gte requirement")]
    #[test_case::test_case(">= 1.0.0, < 2.0.0" => ">=1.0.0, <2.0.0"; "range requirement")]
    pub fn test_version_req_parse(src: &str) -> String {
        let req = parse_version_req(src).unwrap();
        req.to_string()
    }

    #[test]
    fn test_version_req_matches() {
        let req = parse_version_req(">= 1.0.0, < 2.0.0").unwrap();
        let v1 = parse_version("1.5.0").unwrap();
        let v2 = parse_version("2.0.0").unwrap();
        let v3 = parse_version("0.9.0").unwrap();

        assert!(req.matches(&v1), "1.5.0 should match >= 1.0.0, < 2.0.0");
        assert!(
            !req.matches(&v2),
            "2.0.0 should not match >= 1.0.0, < 2.0.0"
        );
        assert!(
            !req.matches(&v3),
            "0.9.0 should not match >= 1.0.0, < 2.0.0"
        );
    }

    #[test]
    fn test_version_is_stable() {
        let stable = parse_version("1.0.0").unwrap();
        let prerelease = parse_version("1.0.0-rc.0").unwrap();

        assert!(stable.is_stable());
        assert!(!prerelease.is_stable());
    }

    #[test]
    fn test_serde_roundtrip() {
        let v = VersionSerde(parse_version("1.0.0-rc.0").unwrap());
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#""1.0.0-rc.0""#);

        let parsed: VersionSerde = serde_json::from_str(&json).unwrap();
        assert_eq!(v, parsed);
    }

    #[test]
    fn test_version_req_serde_roundtrip() {
        let v = VersionReqSerde(parse_version_req(">= 1.0.0, < 2.0.0").unwrap());
        let json = serde_json::to_string(&v).unwrap();

        let parsed: VersionReqSerde = serde_json::from_str(&json).unwrap();
        assert_eq!(v, parsed);
    }
}
