use crate::defs::Spanned;
use miette::{Diagnostic, NamedSource, SourceSpan};
use std::path::Path;
use thiserror::Error;

#[allow(unused_assignments)]
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
pub struct SpanDiagnostic {
    #[source_code]
    src: NamedSource<String>,

    #[label]
    span: Option<SourceSpan>,

    message: String,
    #[allow(dead_code)]
    label: String,
    #[diagnostic(help)]
    help: Option<String>,

    #[diagnostic(level)]
    level: miette::Severity,
}

impl SpanDiagnostic {
    pub fn new<T>(
        sp: &Spanned<T>,
        path: &Path,
        source: &str,
        message: impl Into<String>,
        label: impl Into<String>,
        help: Option<String>,
    ) -> Self {
        let span = sp.span();
        Self {
            src: NamedSource::new(path.to_string_lossy(), source.to_string()),
            span: Some(SourceSpan::new(
                span.start.into(),
                span.end.saturating_sub(span.start).max(1),
            )),
            message: message.into(),
            label: label.into(),
            help,
            level: miette::Severity::Error,
        }
    }

    pub fn no_span(
        path: &Path,
        source: &str,
        message: impl Into<String>,
        help: Option<String>,
    ) -> Self {
        Self {
            src: NamedSource::new(path.to_string_lossy(), source.to_string()),
            span: None,
            message: message.into(),
            label: String::new(),
            help,
            level: miette::Severity::Error,
        }
    }

    pub fn with_level(
        &mut self,
        level: miette::Severity,
    ) {
        self.level = level;
    }
}

pub trait SpannedExt<T> {
    fn error(
        &self,
        path: &Path,
        source: &str,
        message: impl Into<String>,
    ) -> SpanDiagnostic;
}

impl<T> SpannedExt<T> for Spanned<T> {
    fn error(
        &self,
        path: &Path,
        source: &str,
        message: impl Into<String>,
    ) -> SpanDiagnostic {
        SpanDiagnostic::new(self, path, source, message, "here", None)
    }
}
