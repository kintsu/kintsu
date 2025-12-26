//! Diagnostic integration with miette.
//! Converts errors to miette-compatible diagnostics for rich error reporting.

use crate::{ErrorCode, Severity, Span};
use miette::{Diagnostic, NamedSource, SourceSpan};
use std::path::Path;

/// A diagnostic with source span information for miette rendering.
#[derive(Debug)]
pub struct SpanDiagnostic {
    code: String,
    message: String,
    severity: Severity,
    src: Option<NamedSource<String>>,
    span: Option<SourceSpan>,
    label: Option<String>,
    help: Option<String>,
}

impl SpanDiagnostic {
    pub fn new(
        code: ErrorCode,
        message: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            severity,
            src: None,
            span: None,
            label: None,
            help: None,
        }
    }

    pub fn with_source(
        mut self,
        path: impl AsRef<Path>,
        source: impl Into<String>,
    ) -> Self {
        self.src = Some(NamedSource::new(
            path.as_ref().display().to_string(),
            source.into(),
        ));
        self
    }

    pub fn with_span(
        mut self,
        span: Span,
    ) -> Self {
        self.span = Some((span.start, span.len()).into());
        self
    }

    pub fn with_span_opt(
        mut self,
        span: Option<Span>,
    ) -> Self {
        if let Some(s) = span {
            self.span = Some((s.start, s.len()).into());
        }
        self
    }

    pub fn with_label(
        mut self,
        label: impl Into<String>,
    ) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_help(
        mut self,
        help: impl Into<String>,
    ) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_help_opt(
        mut self,
        help: Option<impl Into<String>>,
    ) -> Self {
        self.help = help.map(Into::into);
        self
    }
}

impl std::fmt::Display for SpanDiagnostic {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SpanDiagnostic {}

impl Diagnostic for SpanDiagnostic {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(&self.code))
    }

    fn severity(&self) -> Option<miette::Severity> {
        Some(match self.severity {
            Severity::Error => miette::Severity::Error,
            Severity::Warning => miette::Severity::Warning,
            Severity::Info | Severity::Hint => miette::Severity::Advice,
        })
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.help
            .as_ref()
            .map(|h| Box::new(h) as Box<dyn std::fmt::Display>)
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.src
            .as_ref()
            .map(|s| s as &dyn miette::SourceCode)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let span = self.span?;
        let label = self
            .label
            .clone()
            .unwrap_or_else(|| self.message.clone());
        Some(Box::new(std::iter::once(
            miette::LabeledSpan::new_with_span(Some(label), span),
        )))
    }
}

/// Builder for creating diagnostics from domain errors.
pub struct DiagnosticBuilder {
    code: ErrorCode,
    message: String,
    severity: Severity,
    help: Option<String>,
    span: Option<Span>,
    path: Option<std::path::PathBuf>,
    source: Option<String>,
}

impl DiagnosticBuilder {
    pub fn new(
        code: ErrorCode,
        message: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            severity,
            help: None,
            span: None,
            path: None,
            source: None,
        }
    }

    pub fn help(
        mut self,
        help: impl Into<String>,
    ) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn help_opt(
        mut self,
        help: Option<&'static str>,
    ) -> Self {
        self.help = help.map(String::from);
        self
    }

    pub fn span(
        mut self,
        span: Span,
    ) -> Self {
        self.span = Some(span);
        self
    }

    pub fn span_opt(
        mut self,
        span: Option<Span>,
    ) -> Self {
        self.span = span;
        self
    }

    pub fn source(
        mut self,
        path: impl Into<std::path::PathBuf>,
        source: impl Into<String>,
    ) -> Self {
        self.path = Some(path.into());
        self.source = Some(source.into());
        self
    }

    pub fn build(self) -> SpanDiagnostic {
        let mut diag = SpanDiagnostic::new(self.code, self.message, self.severity);

        if let (Some(path), Some(source)) = (self.path, self.source) {
            diag = diag.with_source(path, source);
        }

        if let Some(span) = self.span {
            diag = diag.with_span(span);
        }

        if let Some(help) = self.help {
            diag = diag.with_help(help);
        }

        diag
    }

    pub fn into_report(self) -> miette::Report {
        miette::Report::new(self.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Category, Domain};

    #[test]
    fn diagnostic_basic() {
        let code = ErrorCode::new(Domain::TR, Category::Resolution, 1);
        let diag = SpanDiagnostic::new(code, "type not found", Severity::Error);
        assert_eq!(diag.to_string(), "type not found");
    }

    #[test]
    fn diagnostic_with_source() {
        let code = ErrorCode::new(Domain::TR, Category::Resolution, 1);
        let diag = SpanDiagnostic::new(code, "undefined type 'Foo'", Severity::Error)
            .with_source("test.ks", "field: Foo,")
            .with_span(Span::new(7, 10))
            .with_help("check the type name");

        assert!(diag.source_code().is_some());
        assert!(diag.labels().is_some());
        assert!(diag.help().is_some());
    }
}
