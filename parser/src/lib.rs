#![allow(
    clippy::large_enum_variant,
    clippy::result_large_err,
    clippy::never_loop,
    clippy::ptr_arg
)]

pub mod ast;
pub mod ctx;
pub mod declare;
pub mod defs;
pub mod diagnostics;
pub mod fmt;
pub mod tokens;
pub(crate) mod utils;

#[cfg(test)]
pub(crate) mod tst;

use std::{convert::Infallible, path::PathBuf, sync::Arc};

use crate::tokens::{LexingError, ToTokens};
use thiserror::Error;

pub use kintsu_errors::{
    CompilerError, DomainError, ErrorBuilder, FilesystemError, HasSpan, InternalError,
    LexicalError, MetadataError, NamespaceError, PackageError, ParsingError, ResolutionError,
    SourceContext, Span, TaggingError, TypeDefError, TypeExprError, UnionError,
};
pub use tokens::{ImplDiagnostic, Parse, Peek};

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Fs(#[from] kintsu_fs::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Infallible(#[from] Infallible),

    #[error("{0}")]
    Manifest(#[from] kintsu_manifests::Error),

    #[error("{0}")]
    Version(#[from] kintsu_manifests::version::VersionError),

    #[error("{0}")]
    Glob(#[from] glob::PatternError),

    #[error("{0}")]
    Lexing(#[from] tokens::LexingError),

    #[error("{0}")]
    Compiler(#[from] CompilerError),

    #[error("{inner}")]
    WithSource {
        #[source]
        inner: Box<Error>,
        path: PathBuf,
        source: Arc<String>,
    },
}

impl Error {
    pub fn with_source(
        self,
        path: PathBuf,
        source: Arc<String>,
    ) -> Self {
        Error::WithSource {
            inner: Box::new(self),
            path,
            source,
        }
    }

    /// Conditionally attach source context if source is available
    pub fn with_source_arc_if(
        self,
        path: PathBuf,
        source: Option<Arc<String>>,
    ) -> Self {
        if let Some(src) = source {
            self.with_source(path, src)
        } else {
            self
        }
    }

    pub fn with_context(
        path: PathBuf,
        source: Arc<String>,
    ) -> impl FnOnce(Self) -> Self {
        move |err: Error| err.with_source(path, source)
    }

    fn extract_source(&self) -> Option<(&std::path::Path, &str)> {
        match self {
            Error::WithSource {
                inner,
                path,
                source,
            } => {
                inner
                    .extract_source()
                    .or(Some((path.as_path(), source.as_str())))
            },
            _ => None,
        }
    }

    pub fn to_report(
        &self,
        override_path: Option<&std::path::Path>,
        override_source: Option<&str>,
        override_span: Option<(usize, usize)>,
    ) -> miette::Report {
        use miette::NamedSource;

        let (path, source) = match (override_path, override_source) {
            (Some(p), Some(s)) => (p, s),
            (Some(p), None) => {
                if let Some((_, es)) = self.extract_source() {
                    (p, es)
                } else {
                    return miette::Report::msg(format!("{self}"));
                }
            },
            (None, Some(_)) => {
                return miette::Report::msg(format!("cannot provide source without path: {self}"));
            },
            (None, None) => {
                if let Some((ep, es)) = self.extract_source() {
                    (ep, es)
                } else {
                    return miette::Report::msg(format!("{self}"));
                }
            },
        };

        let named = NamedSource::new(path.to_string_lossy(), source.to_string());

        if let Some((s, e)) = override_span {
            let diag = crate::diagnostics::SpanDiagnostic::new(
                &crate::defs::Spanned::new(s, e, ()),
                path,
                source,
                format!("{self}"),
                format!("{self}"),
                None,
            );
            miette::Report::new(diag)
        } else {
            miette::Report::msg(format!("{self}")).with_source_code(named)
        }
    }

    pub fn to_report_with(
        &self,
        path: &std::path::Path,
        source: &str,
        override_span: Option<(usize, usize)>,
    ) -> miette::Report {
        self.to_report(Some(path), Some(source), override_span)
    }

    pub fn to_report_fn<'a>(
        path: &'a std::path::Path,
        source: &'a str,
    ) -> impl FnOnce(Self) -> miette::Report + 'a {
        move |err: Error| err.to_report_with(path, source, None)
    }

    pub fn to_compiler_error(&self) -> CompilerError {
        match self {
            Self::Compiler(e) => e.clone(),
            Self::Fs(e) => {
                FilesystemError::io_error(e.to_string())
                    .unlocated()
                    .build()
            },
            Self::Io(e) => {
                FilesystemError::io_error(e.to_string())
                    .unlocated()
                    .build()
            },
            Self::Manifest(e) => {
                PackageError::manifest_error(e.to_string())
                    .unlocated()
                    .build()
            },
            Self::Version(e) => {
                PackageError::version_error(e.to_string())
                    .unlocated()
                    .build()
            },
            Self::Glob(e) => {
                FilesystemError::invalid_glob(e.to_string())
                    .unlocated()
                    .build()
            },
            Self::Lexing(e) => {
                let span = match e {
                    LexingError::Spanned { span, .. } => {
                        let s = span.span();
                        Some(Span::new(s.start, s.end))
                    },
                    _ => None,
                };
                LexicalError::lexer_error(e.to_string()).at_opt(span)
            },
            Self::WithSource {
                inner,
                path,
                source,
            } => {
                inner
                    .to_compiler_error()
                    .with_source_arc(path.clone(), Arc::clone(source))
            },
            Self::Infallible(_) => unreachable!(),
        }
    }
}

impl From<Error> for CompilerError {
    fn from(err: Error) -> Self {
        err.to_compiler_error()
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

macro_rules! bail_miette {
    ($e: expr; ($path: expr) for $src: expr) => {
        $e.map_err(|lex| {
            let crate_err: crate::Error = lex.into();
            crate_err.to_report_with($path, &$src, None)
        })
    };
}

pub(crate) use bail_miette;
