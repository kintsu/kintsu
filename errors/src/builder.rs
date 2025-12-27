//! Type-enforced error builder for span attachment.
//!
//! Uses a sealed type pattern to ensure every error either has an explicit span
//! or is explicitly marked as unlocated. This eliminates accidental spanless errors.
//!
//! # Example
//!
//! ```ignore
//! use kintsu_errors::{ErrorBuilder, ResolutionError};
//!
//! // With a span:
//! let err = ErrorBuilder::new(ResolutionError::undefined_type("Foo"))
//!     .at(span)
//!     .build();
//!
//! // Without a span (explicit opt-out):
//! let err = ErrorBuilder::new(FilesystemError::not_found(path))
//!     .unlocated()
//!     .build();
//!
//! // This won't compile - span decision is mandatory:
//! // let err = ErrorBuilder::new(ResolutionError::undefined_type("Foo")).build();
//! ```

use crate::{CompilerError, HasSpan, Span};
use std::sync::Arc;

/// Marker for errors that don't yet have a span decision.
pub struct Unspanned;

/// Marker for errors that have an attached span.
pub struct Spanned(pub(crate) Span);

/// Marker for errors explicitly without location information.
pub struct Unlocated;

/// Source attachment for rich error context.
#[derive(Debug, Clone)]
pub struct SourceContext {
    /// Display name (usually file path)
    pub name: String,
    /// Actual source content
    pub content: Arc<str>,
}

impl SourceContext {
    /// Create a new source context.
    pub fn new(
        name: impl Into<String>,
        content: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
        }
    }
}

/// A trait for domain errors that can be wrapped into CompilerError.
///
/// This is automatically implemented for all domain error types generated
/// by the `define_domain_errors!` macro.
pub trait DomainError: Sized {
    /// Convert into a CompilerError (without modifying span).
    fn into_compiler_error(self) -> CompilerError;

    /// Attach a span to this error.
    fn with_span(
        self,
        span: Span,
    ) -> Self;
}

/// Builder for constructing errors with type-enforced span decisions.
///
/// The type parameter `S` tracks whether a span decision has been made:
/// - `Unspanned`: No decision yet - must call `at()`, `at_node()`, or `unlocated()`
/// - `Spanned`: Has a span - can call `build()` or `in_source()`
/// - `Unlocated`: Explicitly no location - can call `build()`
pub struct ErrorBuilder<S, E> {
    error: E,
    span_state: S,
    source: Option<SourceContext>,
}

impl<E: DomainError> ErrorBuilder<Unspanned, E> {
    /// Create a new error builder from a domain error.
    pub fn new(error: E) -> Self {
        Self {
            error,
            span_state: Unspanned,
            source: None,
        }
    }

    /// Attach a span to this error.
    pub fn at(
        self,
        span: impl Into<Span>,
    ) -> ErrorBuilder<Spanned, E> {
        ErrorBuilder {
            error: self.error,
            span_state: Spanned(span.into()),
            source: self.source,
        }
    }

    /// Attach an optional span. If None, marks the error as unlocated.
    /// Returns a CompilerError directly since the span decision is made.
    pub fn at_opt(
        self,
        span: Option<Span>,
    ) -> CompilerError {
        match span {
            Some(s) => self.at(s).build(),
            None => self.unlocated().build(),
        }
    }

    /// Attach a span from any HasSpan-implementing node.
    pub fn at_node<T: HasSpan>(
        self,
        node: &T,
    ) -> ErrorBuilder<Spanned, E> {
        self.at(node.span())
    }

    /// Explicitly mark this error as having no location information.
    ///
    /// Use this for errors that inherently don't have a source location,
    /// like filesystem errors or configuration errors.
    pub fn unlocated(self) -> ErrorBuilder<Unlocated, E> {
        ErrorBuilder {
            error: self.error,
            span_state: Unlocated,
            source: self.source,
        }
    }
}

impl<E: DomainError> ErrorBuilder<Spanned, E> {
    /// Attach source context for rich diagnostics.
    pub fn in_source(
        mut self,
        source: &SourceContext,
    ) -> Self {
        self.source = Some(source.clone());
        self
    }

    /// Build the final CompilerError.
    pub fn build(self) -> CompilerError {
        let span = self.span_state.0;
        let err = self.error.with_span(span);
        let mut result = err.into_compiler_error();

        if let Some(source) = self.source {
            result = CompilerError::WithSource {
                inner: Box::new(result),
                path: source.name.into(),
                source: Arc::new(source.content.to_string()),
            };
        }

        result
    }
}

impl<E: DomainError> ErrorBuilder<Unlocated, E> {
    /// Build the final CompilerError (without span).
    pub fn build(self) -> CompilerError {
        self.error.into_compiler_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Category, Domain, ResolutionError};

    // Test that the builder pattern works
    #[test]
    fn builder_with_span() {
        let err = ResolutionError::undefined_type("Foo")
            .at(Span::new(10, 20))
            .build();

        assert_eq!(err.error_code().domain, Domain::TR);
        assert_eq!(err.error_code().category, Category::Resolution);
    }

    #[test]
    fn builder_unlocated() {
        let err = ResolutionError::undefined_type("Bar")
            .unlocated()
            .build();

        assert_eq!(err.error_code().domain, Domain::TR);
    }

    #[test]
    fn builder_with_source() {
        let source = SourceContext::new("test.ks", "struct Foo {}");
        let err = ResolutionError::undefined_type("Foo")
            .at(Span::new(0, 10))
            .in_source(&source)
            .build();

        // Should be wrapped with source
        assert!(matches!(err, CompilerError::WithSource { .. }));
    }
}
