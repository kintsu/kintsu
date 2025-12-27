//! Kintsu Error System
//!
//! Unified error handling for the Kintsu compiler.
//!
//! # Overview
//!
//! This crate provides a centralized error system following [RFC-0023](https://docs.kintsu.dev/specs/rfc/rfc-0023) and [SPEC-0022](https://docs.kintsu.dev/specs/spec/spec-0022).
//! All compiler errors use the K[Domain][Category][Sequence] code format
//!
//! # Example
//!
//! ```
//! use kintsu_errors::{CompilerError, ResolutionError, Span};
//!
//! let err: CompilerError = ResolutionError::undefined_type("User")
//!     .at(Span::new(10, 14))
//!     .build();
//!
//! assert_eq!(err.error_code().to_string(), "KTR1002");
//! ```

#![allow(clippy::large_enum_variant)]

mod code;
mod diagnostic;
#[macro_use]
mod macros;
mod builder;
mod span;

pub mod domains;

pub use builder::{DomainError, ErrorBuilder, SourceContext, Spanned, Unlocated, Unspanned};
pub use code::{Category, Domain, ErrorCode, Severity};
pub use diagnostic::{DiagnosticBuilder, SpanDiagnostic};
pub use span::{HasSpan, SourceAttachment, Span};

pub use domains::{
    FilesystemError, InternalError, LexicalError, MetadataError, NamespaceError, PackageError,
    ParsingError, ResolutionError, TaggingError, TypeDefError, TypeExprError, UnionError,
};

use std::{path::PathBuf, sync::Arc};

/// Unified error type for all compiler operations.
///
/// This enum wraps all domain-specific errors and provides common operations
/// like error code generation, message formatting, and diagnostic conversion.
#[derive(Debug, Clone)]
pub enum CompilerError {
    Lexical(LexicalError),
    Parsing(ParsingError),
    Namespace(NamespaceError),
    TypeDef(TypeDefError),
    Resolution(ResolutionError),
    Union(UnionError),
    Metadata(MetadataError),
    Tagging(TaggingError),
    TypeExpr(TypeExprError),
    Package(PackageError),
    Filesystem(FilesystemError),
    Internal(InternalError),

    /// Error with attached source file context.
    WithSource {
        inner: Box<CompilerError>,
        path: PathBuf,
        source: Arc<String>,
    },

    /// Error with secondary labels for multi-location highlighting.
    WithSecondaryLabels {
        inner: Box<CompilerError>,
        labels: Vec<(Span, String)>,
    },

    /// Multiple errors collected together.
    Multiple(Vec<CompilerError>),
}

impl CompilerError {
    /// Returns the error code for this error.
    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::Lexical(e) => e.error_code(),
            Self::Parsing(e) => e.error_code(),
            Self::Namespace(e) => e.error_code(),
            Self::TypeDef(e) => e.error_code(),
            Self::Resolution(e) => e.error_code(),
            Self::Union(e) => e.error_code(),
            Self::Metadata(e) => e.error_code(),
            Self::Tagging(e) => e.error_code(),
            Self::TypeExpr(e) => e.error_code(),
            Self::Package(e) => e.error_code(),
            Self::Filesystem(e) => e.error_code(),
            Self::Internal(e) => e.error_code(),
            Self::WithSource { inner, .. } => inner.error_code(),
            Self::WithSecondaryLabels { inner, .. } => inner.error_code(),
            Self::Multiple(errs) => {
                errs.first()
                    .map(|e| e.error_code())
                    .unwrap_or_else(|| ErrorCode::new(Domain::IN, Category::Internal, 1))
            },
        }
    }

    /// Returns the human-readable error message.
    pub fn message(&self) -> String {
        match self {
            Self::Lexical(e) => e.message(),
            Self::Parsing(e) => e.message(),
            Self::Namespace(e) => e.message(),
            Self::TypeDef(e) => e.message(),
            Self::Resolution(e) => e.message(),
            Self::Union(e) => e.message(),
            Self::Metadata(e) => e.message(),
            Self::Tagging(e) => e.message(),
            Self::TypeExpr(e) => e.message(),
            Self::Package(e) => e.message(),
            Self::Filesystem(e) => e.message(),
            Self::Internal(e) => e.message(),
            Self::WithSource { inner, .. } => inner.message(),
            Self::WithSecondaryLabels { inner, .. } => inner.message(),
            Self::Multiple(errs) => {
                if errs.len() == 1 {
                    errs[0].message()
                } else {
                    format!("{} errors occurred", errs.len())
                }
            },
        }
    }

    /// Returns the severity level.
    pub fn severity(&self) -> Severity {
        match self {
            Self::Lexical(e) => e.severity(),
            Self::Parsing(e) => e.severity(),
            Self::Namespace(e) => e.severity(),
            Self::TypeDef(e) => e.severity(),
            Self::Resolution(e) => e.severity(),
            Self::Union(e) => e.severity(),
            Self::Metadata(e) => e.severity(),
            Self::Tagging(e) => e.severity(),
            Self::TypeExpr(e) => e.severity(),
            Self::Package(e) => e.severity(),
            Self::Filesystem(e) => e.severity(),
            Self::Internal(e) => e.severity(),
            Self::WithSource { inner, .. } => inner.severity(),
            Self::WithSecondaryLabels { inner, .. } => inner.severity(),
            Self::Multiple(errs) => {
                errs.iter()
                    .map(|e| e.severity())
                    .max_by_key(|s| {
                        match s {
                            Severity::Error => 3,
                            Severity::Warning => 2,
                            Severity::Info => 1,
                            Severity::Hint => 0,
                        }
                    })
                    .unwrap_or(Severity::Error)
            },
        }
    }

    /// Returns optional help text.
    pub fn help_text(&self) -> Option<&'static str> {
        match self {
            Self::Lexical(e) => e.help_text(),
            Self::Parsing(e) => e.help_text(),
            Self::Namespace(e) => e.help_text(),
            Self::TypeDef(e) => e.help_text(),
            Self::Resolution(e) => e.help_text(),
            Self::Union(e) => e.help_text(),
            Self::Metadata(e) => e.help_text(),
            Self::Tagging(e) => e.help_text(),
            Self::TypeExpr(e) => e.help_text(),
            Self::Package(e) => e.help_text(),
            Self::Filesystem(e) => e.help_text(),
            Self::Internal(e) => e.help_text(),
            Self::WithSource { inner, .. } => inner.help_text(),
            Self::WithSecondaryLabels { inner, .. } => inner.help_text(),
            Self::Multiple(_) => None,
        }
    }

    /// Returns the primary span if available.
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Lexical(e) => e.span(),
            Self::Parsing(e) => e.span(),
            Self::Namespace(e) => e.span(),
            Self::TypeDef(e) => e.span(),
            Self::Resolution(e) => e.span(),
            Self::Union(e) => e.span(),
            Self::Metadata(e) => e.span(),
            Self::Tagging(e) => e.span(),
            Self::TypeExpr(e) => e.span(),
            Self::Package(e) => e.span(),
            Self::Filesystem(e) => e.span(),
            Self::Internal(e) => e.span(),
            Self::WithSource { inner, .. } => inner.span(),
            Self::WithSecondaryLabels { inner, .. } => inner.span(),
            Self::Multiple(errs) => errs.first().and_then(|e| e.span()),
        }
    }

    /// Wraps this error with source file context.
    pub fn with_source(
        self,
        path: impl Into<PathBuf>,
        source: impl Into<String>,
    ) -> Self {
        Self::WithSource {
            inner: Box::new(self),
            path: path.into(),
            source: Arc::new(source.into()),
        }
    }

    /// Wraps this error with source file context (Arc version).
    pub fn with_source_arc(
        self,
        path: impl Into<PathBuf>,
        source: Arc<String>,
    ) -> Self {
        Self::WithSource {
            inner: Box::new(self),
            path: path.into(),
            source,
        }
    }

    /// Adds a secondary label for multi-location highlighting.
    pub fn with_secondary_label(
        self,
        span: Span,
        label: impl Into<String>,
    ) -> Self {
        match self {
            Self::WithSecondaryLabels { inner, mut labels } => {
                labels.push((span, label.into()));
                Self::WithSecondaryLabels { inner, labels }
            },
            other => {
                Self::WithSecondaryLabels {
                    inner: Box::new(other),
                    labels: vec![(span, label.into())],
                }
            },
        }
    }

    /// Extracts secondary labels from nested WithSecondaryLabels wrappers.
    pub fn extract_secondary_labels(&self) -> Vec<(Span, String)> {
        match self {
            Self::WithSecondaryLabels { inner, labels } => {
                let mut all_labels = inner.extract_secondary_labels();
                all_labels.extend(labels.clone());
                all_labels
            },
            Self::WithSource { inner, .. } => inner.extract_secondary_labels(),
            _ => Vec::new(),
        }
    }

    /// Extracts source information from nested WithSource wrappers.
    pub fn extract_source(&self) -> Option<(&std::path::Path, &str)> {
        match self {
            Self::WithSource {
                inner,
                path,
                source,
            } => {
                inner
                    .extract_source()
                    .or(Some((path.as_path(), source.as_str())))
            },
            Self::WithSecondaryLabels { inner, .. } => inner.extract_source(),
            _ => None,
        }
    }

    /// Recursively extracts the deepest span from nested errors.
    pub fn extract_deepest_span(&self) -> Option<Span> {
        match self {
            Self::WithSource { inner, .. } => {
                inner
                    .extract_deepest_span()
                    .or_else(|| inner.span())
            },
            Self::WithSecondaryLabels { inner, .. } => {
                inner
                    .extract_deepest_span()
                    .or_else(|| inner.span())
            },
            other => other.span(),
        }
    }

    /// Converts to a miette Report for display.
    pub fn to_report(&self) -> miette::Report {
        let (path, source) = self.extract_source().unzip();
        let span = self.extract_deepest_span();
        let secondary_labels = self.extract_secondary_labels();

        let mut builder =
            DiagnosticBuilder::new(self.error_code(), self.message(), self.severity())
                .help_opt(self.help_text())
                .span_opt(span)
                .secondary_labels(secondary_labels);

        if let (Some(p), Some(s)) = (path, source) {
            builder = builder.source(p, s);
        }

        builder.into_report()
    }

    /// Creates a closure for wrapping errors with source context.
    pub fn with_context(
        path: PathBuf,
        source: Arc<String>,
    ) -> impl FnOnce(Self) -> Self {
        move |err| err.with_source_arc(path, source)
    }

    /// Creates a closure for converting to miette Report.
    pub fn to_report_fn<'a>(
        path: &'a std::path::Path,
        source: &'a str,
    ) -> impl FnOnce(Self) -> miette::Report + 'a {
        move |err| err.with_source(path, source).to_report()
    }

    /// Returns true if this is a fatal error.
    pub fn is_fatal(&self) -> bool {
        self.severity().is_fatal()
    }
}

impl std::fmt::Display for CompilerError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for CompilerError {}

impl From<LexicalError> for CompilerError {
    fn from(e: LexicalError) -> Self {
        Self::Lexical(e)
    }
}

impl From<ParsingError> for CompilerError {
    fn from(e: ParsingError) -> Self {
        Self::Parsing(e)
    }
}

impl From<NamespaceError> for CompilerError {
    fn from(e: NamespaceError) -> Self {
        Self::Namespace(e)
    }
}

impl From<TypeDefError> for CompilerError {
    fn from(e: TypeDefError) -> Self {
        Self::TypeDef(e)
    }
}

impl From<ResolutionError> for CompilerError {
    fn from(e: ResolutionError) -> Self {
        Self::Resolution(e)
    }
}

impl From<UnionError> for CompilerError {
    fn from(e: UnionError) -> Self {
        Self::Union(e)
    }
}

impl From<MetadataError> for CompilerError {
    fn from(e: MetadataError) -> Self {
        Self::Metadata(e)
    }
}

impl From<TaggingError> for CompilerError {
    fn from(e: TaggingError) -> Self {
        Self::Tagging(e)
    }
}

impl From<TypeExprError> for CompilerError {
    fn from(e: TypeExprError) -> Self {
        Self::TypeExpr(e)
    }
}

impl From<PackageError> for CompilerError {
    fn from(e: PackageError) -> Self {
        Self::Package(e)
    }
}

impl From<FilesystemError> for CompilerError {
    fn from(e: FilesystemError) -> Self {
        Self::Filesystem(e)
    }
}

impl From<InternalError> for CompilerError {
    fn from(e: InternalError) -> Self {
        Self::Internal(e)
    }
}

impl From<std::io::Error> for CompilerError {
    fn from(e: std::io::Error) -> Self {
        FilesystemError::io_error(e.to_string())
            .unlocated()
            .build()
    }
}

impl From<glob::PatternError> for CompilerError {
    fn from(e: glob::PatternError) -> Self {
        FilesystemError::invalid_glob(e.to_string())
            .unlocated()
            .build()
    }
}

impl From<std::convert::Infallible> for CompilerError {
    fn from(_: std::convert::Infallible) -> Self {
        unreachable!()
    }
}

/// Type alias for Results using CompilerError.
pub type Result<T, E = CompilerError> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_generation() {
        let err: CompilerError = ResolutionError::undefined_type("Foo")
            .unlocated()
            .build();
        assert_eq!(err.error_code().to_string(), "KTR1002");
    }

    #[test]
    fn error_message() {
        let err: CompilerError = ResolutionError::undefined_type("User")
            .unlocated()
            .build();
        assert_eq!(err.message(), "undefined type: 'User'");
    }

    #[test]
    fn error_with_source() {
        let err: CompilerError = ResolutionError::undefined_type("Foo")
            .at(Span::new(10, 13))
            .build();
        let err = err.with_source("test.ks", "field: Foo,");

        let (path, source) = err.extract_source().unwrap();
        assert_eq!(path.to_str().unwrap(), "test.ks");
        assert_eq!(source, "field: Foo,");
    }

    #[test]
    fn multiple_errors() {
        let errs = vec![
            ResolutionError::undefined_type("A")
                .unlocated()
                .build(),
            ResolutionError::undefined_type("B")
                .unlocated()
                .build(),
        ];
        let err = CompilerError::Multiple(errs);
        assert_eq!(err.message(), "2 errors occurred");
    }
}
