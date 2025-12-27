//! Package errors (KPK) - [ERR-0011](https://docs.kintsu.dev/specs/err/ERR-0011)
//! Errors related to package manifests, lockfiles, and dependencies.

define_domain_errors! {
    /// Package errors (KPK domain)
    /// https://docs.kintsu.dev/specs/err/ERR-0011
    pub enum PackageError {
        /// KPK0001: Manifest parse error
        ManifestParseError {
            code: (PK, Syntax, 1),
            message: "failed to parse kintsu.toml: {reason}",
            help: "fix the TOML syntax error",
            fields: { reason: String },
        },

        /// KPK3001: Duplicate dependency
        DuplicateDependency {
            code: (PK, Conflict, 1),
            message: "duplicate dependency '{name}' in manifest",
            help: "remove the duplicate dependency declaration",
            fields: { name: String },
        },

        /// KPK4001: Manifest not found
        ManifestNotFound {
            code: (PK, Missing, 1),
            message: "kintsu.toml not found in {directory}",
            help: "run 'kintsu init' to create a new package",
            fields: { directory: String },
        },

        /// KPK4002: Lockfile not found
        LockfileNotFound {
            code: (PK, Missing, 2),
            message: "kintsu.lock not found but dependencies are specified",
            help: "run 'kintsu install' to resolve and lock dependencies",
        },

        /// KPK6001: Dependency version mismatch
        DependencyVersionMismatch {
            code: (PK, Compatibility, 1),
            message: "dependency version conflict: {package} requires {required}, but {other} requires {other_required}",
            help: "update to a version satisfying both constraints",
            fields: { package: String, required: String, other: String, other_required: String },
        },

        /// KPK6002: Lockfile out of date
        LockfileOutOfDate {
            code: (PK, Compatibility, 2),
            message: "lockfile is out of date with manifest",
            help: "run 'kintsu install' to update the lockfile",
            severity: Warning,
        },

        /// Generic manifest error (for wrapping kintsu_manifests::Error)
        ManifestError {
            code: (PK, Internal, 1),
            message: "{reason}",
            fields: { reason: String },
        },
    }
}

use crate::builder::{ErrorBuilder, Unspanned};

impl PackageError {
    pub fn parse_error(reason: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::ManifestParseError {
            reason: reason.into(),
            span: None,
        })
    }

    pub fn duplicate_dep(name: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::DuplicateDependency {
            name: name.into(),
            span: None,
        })
    }

    pub fn manifest_not_found(directory: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::ManifestNotFound {
            directory: directory.into(),
            span: None,
        })
    }

    pub fn lockfile_not_found() -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::LockfileNotFound { span: None })
    }

    pub fn version_mismatch(
        package: impl Into<String>,
        required: impl Into<String>,
        other: impl Into<String>,
        other_required: impl Into<String>,
    ) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::DependencyVersionMismatch {
            package: package.into(),
            required: required.into(),
            other: other.into(),
            other_required: other_required.into(),
            span: None,
        })
    }

    pub fn lockfile_outdated() -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::LockfileOutOfDate { span: None })
    }

    pub fn manifest_error(reason: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::ManifestError {
            reason: reason.into(),
            span: None,
        })
    }

    /// Alias for manifest_error, used for version-related errors.
    pub fn version_error(reason: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        Self::manifest_error(reason)
    }
}
