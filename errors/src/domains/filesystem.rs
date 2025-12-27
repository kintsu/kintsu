//! Filesystem errors (KFS) - [ERR-0013](https://docs.kintsu.dev/specs/err/ERR-0013)
//! Errors related to file operations and I/O during compilation.

define_domain_errors! {
    /// Filesystem errors (KFS domain)
    /// https://docs.kintsu.dev/specs/err/ERR-0013
    pub enum FilesystemError {
        /// KFS2001: Invalid glob pattern
        InvalidGlobPattern {
            code: (FS, Validation, 1),
            message: "glob pattern error: {reason}",
            help: "fix the glob pattern syntax",
            fields: { reason: String },
        },

        /// KFS2002: Empty file list
        EmptyFileList {
            code: (FS, Validation, 2),
            message: "no files provided to load_files",
            help: "ensure schema directory contains .ks files",
        },

        /// KFS4001: File not found
        FileNotFound {
            code: (FS, Missing, 1),
            message: "file not found: {path}",
            help: "check that the file exists at the specified path",
            fields: { path: String },
        },

        /// KFS4002: Missing lib.ks
        MissingLibKs {
            code: (FS, Missing, 2),
            message: "missing lib.ks: every schema must have a schema/lib.ks file",
            help: "create schema/lib.ks with your namespace declaration",
        },

        /// KFS9001: IO error
        IoError {
            code: (FS, Internal, 1),
            message: "io error: {reason}",
            fields: { reason: String },
        },

        /// KFS9002: Permission denied
        PermissionDenied {
            code: (FS, Internal, 2),
            message: "permission denied: {path}",
            help: "check file permissions",
            fields: { path: String },
        },
    }
}

use crate::builder::{ErrorBuilder, Unspanned};

impl FilesystemError {
    pub fn invalid_glob(reason: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::InvalidGlobPattern {
            reason: reason.into(),
            span: None,
        })
    }

    pub fn empty_file_list() -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::EmptyFileList { span: None })
    }

    pub fn file_not_found(path: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::FileNotFound {
            path: path.into(),
            span: None,
        })
    }

    pub fn missing_lib_ks() -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::MissingLibKs { span: None })
    }

    pub fn io_error(reason: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::IoError {
            reason: reason.into(),
            span: None,
        })
    }

    pub fn permission_denied(path: impl Into<String>) -> ErrorBuilder<Unspanned, Self> {
        ErrorBuilder::new(Self::PermissionDenied {
            path: path.into(),
            span: None,
        })
    }
}

impl From<std::io::Error> for ErrorBuilder<Unspanned, FilesystemError> {
    fn from(err: std::io::Error) -> Self {
        FilesystemError::io_error(err.to_string())
    }
}

impl From<glob::PatternError> for ErrorBuilder<Unspanned, FilesystemError> {
    fn from(err: glob::PatternError) -> Self {
        FilesystemError::invalid_glob(err.to_string())
    }
}
