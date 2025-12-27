//! Metadata errors (KMT) - [ERR-0008](https://docs.kintsu.dev/specs/err/ERR-0008)
//! Errors related to version attributes, error attributes, and other metadata.

define_domain_errors! {
    /// Metadata errors (KMT domain)
    /// https://docs.kintsu.dev/specs/err/ERR-0008
    pub enum MetadataError {
        /// KMT2001: Invalid version value
        InvalidVersionValue {
            code: (MT, Validation, 1),
            message: "invalid version value: expected positive integer, found {value}",
            help: "version must be a positive integer (e.g., #[version(1)])",
            fields: { value: String },
        },

        /// KMT2002: Invalid error attribute
        InvalidErrorAttribute {
            code: (MT, Validation, 2),
            message: "invalid error attribute: {reason}",
            help: "error attribute must reference a valid error type",
            fields: { reason: String },
        },

        /// KMT3001: Version conflict
        VersionConflict {
            code: (MT, Conflict, 1),
            message: "version attribute conflict: values={values}",
            help: "an item can only have one version attribute",
            fields: { values: String },
        },

        /// KMT3002: Duplicate metadata attribute
        DuplicateMetaAttribute {
            code: (MT, Conflict, 2),
            message: "{attribute} attribute is declared multiple times in {path}",
            help: "each metadata attribute type can only appear once",
            fields: { attribute: String, path: String },
        },

        /// KMT6001: Version incompatibility
        VersionIncompatibility {
            code: (MT, Compatibility, 1),
            message: "version incompatibility: {package} requires version {required}, but found {found}",
            help: "update the dependency version to match requirements",
            fields: { package: String, required: String, found: String },
        },
    }
}

use crate::builder::{ErrorBuilder, Unspanned};

impl MetadataError {
    pub fn invalid_version(value: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::InvalidVersionValue {
            value: value.into(),
            span: None,
        })
    }

    pub fn invalid_error_attr(reason: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::InvalidErrorAttribute {
            reason: reason.into(),
            span: None,
        })
    }

    pub fn version_conflict(
        values: impl IntoIterator<Item = usize>
    ) -> ErrorBuilder<Unspanned, Self> {
        let values = values
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        ErrorBuilder::new(Self::VersionConflict {
            values: format!("[{values}]"),
            span: None,
        })
    }

    pub fn duplicate_attribute(
        attribute: impl Into<String>,
        path: impl Into<String>,
    ) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::DuplicateMetaAttribute {
            attribute: attribute.into(),
            path: path.into(),
            span: None,
        })
    }

    pub fn version_incompatibility(
        package: impl Into<String>,
        required: impl Into<String>,
        found: impl Into<String>,
    ) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::VersionIncompatibility {
            package: package.into(),
            required: required.into(),
            found: found.into(),
            span: None,
        })
    }
}
