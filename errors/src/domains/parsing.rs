//! Parsing errors (KPR) - [ERR-0003](https://docs.kintsu.dev/specs/err/ERR-0003)
//! Errors during AST construction from token stream.

define_domain_errors! {
    /// Parsing errors (KPR domain)
    /// https://docs.kintsu.dev/specs/err/ERR-0003
    pub enum ParsingError {
        /// KPR0001: Unexpected token
        UnexpectedToken {
            code: (PR, Syntax, 1),
            message: "expected {expected}, found {found}",
            help: "check the syntax at this location",
            fields: { expected: String, found: String },
        },

        /// KPR0002: Unexpected end of file
        UnexpectedEndOfFile {
            code: (PR, Syntax, 2),
            message: "expected {expected}, found end of file",
            help: "check for unclosed braces or incomplete declarations",
            fields: { expected: String },
        },

        /// KPR0003: Expected one of several alternatives
        ExpectedOneOf {
            code: (PR, Syntax, 3),
            message: "expected one of {alternatives}, found {found}",
            help: "the parser expected one of several valid alternatives",
            fields: { alternatives: String, found: String },
        },

        /// KPR0004: Invalid path syntax
        InvalidPath {
            code: (PR, Syntax, 4),
            message: "invalid path '{path}': {reason}",
            help: "paths must use :: separators and valid identifiers",
            fields: { path: String, reason: String },
        },

        /// KPR0005: Unknown attribute
        UnknownAttribute {
            code: (PR, Syntax, 5),
            message: "unknown attribute '{name}'",
            help: "check the attribute name spelling",
            fields: { name: String },
        },

        /// KPR0006: Missing lib.ks file
        MissingLibKs {
            code: (PR, Missing, 6),
            message: "missing lib.ks: every schema must have a schema/lib.ks file",
            help: "create a lib.ks file with namespace declaration",
        },

        /// KPR0007: Multi-segment import in lib.ks
        LibKsMultiSegmentImport {
            code: (PR, Validation, 7),
            message: "lib.ks should only contain single-segment use statements",
            help: "use 'use foo' not 'use foo::bar'",
        },

        /// KPR0008: Invalid item in lib.ks
        LibKsInvalidItem {
            code: (PR, Validation, 8),
            message: "lib.ks should only contain namespace declaration and use statements",
            help: "move type definitions to other files",
        },

        /// KPR0009: Missing namespace in lib.ks
        LibKsMissingNamespace {
            code: (PR, Missing, 9),
            message: "lib.ks must contain a namespace declaration",
            help: "add 'namespace <name>;' to lib.ks",
        },

        /// KPR0010: Empty file list
        EmptyFileList {
            code: (PR, Missing, 10),
            message: "no files provided to load_files",
            help: "provide at least one .ks file",
        },
    }
}

use crate::builder::{ErrorBuilder, Unspanned};

impl ParsingError {
    pub fn unexpected(
        expected: impl Into<String>,
        found: impl Into<String>,
    ) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::UnexpectedToken {
            expected: expected.into(),
            found: found.into(),
            span: None,
        })
    }

    pub fn eof(expected: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::UnexpectedEndOfFile {
            expected: expected.into(),
            span: None,
        })
    }

    pub fn expected_one_of(
        alternatives: impl Into<String>,
        found: impl Into<String>,
    ) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::ExpectedOneOf {
            alternatives: alternatives.into(),
            found: found.into(),
            span: None,
        })
    }

    pub fn invalid_path(
        path: impl Into<String>,
        reason: impl Into<String>,
    ) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::InvalidPath {
            path: path.into(),
            reason: reason.into(),
            span: None,
        })
    }

    pub fn unknown_attribute(name: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::UnknownAttribute {
            name: name.into(),
            span: None,
        })
    }

    pub fn lib_multi_segment_import() -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::LibKsMultiSegmentImport { span: None })
    }

    pub fn lib_invalid_item() -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::LibKsInvalidItem { span: None })
    }

    pub fn lib_missing_namespace() -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::LibKsMissingNamespace { span: None })
    }
}
