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

use std::{collections::HashSet, convert::Infallible, path::PathBuf};

use crate::tokens::{LexingError, ToTokens};
use thiserror::Error;

pub use tokens::{ImplDiagnostic, Parse, Peek};

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Fs(#[from] kintsu_fs::Error),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("{0}")]
    Infallible(#[from] Infallible),

    #[error("{} has conflicts. {tag} {} is declared multiple times.", namespace.borrow_string(), ident.display())]
    IdentConflict {
        namespace: SpannedToken![ident],
        ident: ctx::NamedItemContext,
        tag: &'static str,
    },

    #[error("namespace is not declared")]
    NsNotDeclared,

    #[error("only one namespace may be declared in a payload declaration file.")]
    NsConflict,

    #[error(
        "namespace {namespace} is already declared for {parent}, {attempted} cannot be declared"
    )]
    NsDirConflict {
        namespace: String,
        parent: PathBuf,
        attempted: String,
    },

    #[error("resolution error. could not resolve '{}'", ident.borrow_path_inner())]
    ResolutionError { ident: SpannedToken![path] },

    #[error("{inner}")]
    WithSpan {
        #[source]
        inner: Box<Error>,
        start: usize,
        end: usize,
    },

    #[error("{inner}")]
    WithSource {
        #[source]
        inner: Box<Error>,
        path: PathBuf,
        source: std::sync::Arc<String>,
    },

    #[error("version attribute conflict: values={values:?}")]
    VersionConflict {
        values: Vec<usize>,
        spans: Vec<(usize, usize)>,
    },

    #[error("lex error: invalid character '{ch}'")]
    LexError { ch: char, start: usize, end: usize },

    #[error("{0}")]
    AstError(#[from] tokens::LexingError),

    #[error("{0}")]
    ManifestError(#[from] kintsu_manifests::Error),

    #[error("linting errors")]
    LintFailure,

    #[error("missing lib.ks: every schema must have a schema/lib.ks file")]
    MissingLibError,

    #[error("glob pattern error: {0}")]
    GlobError(#[from] glob::PatternError),

    #[error("duplicate type '{name}' already registered")]
    DuplicateType { name: String },

    #[error("lib.ks should only contain single-segment use statements (e.g., 'use foo')")]
    LibPldMultiSegmentImport,

    #[error("lib.ks should only contain namespace declaration and use statements")]
    LibPldInvalidItem,

    #[error("lib.ks must contain a namespace declaration")]
    LibPldMissingNamespace,

    #[error("namespace mismatch: expected {expected}, found {found}")]
    NamespaceMismatch { expected: String, found: String },

    #[error("namespace {name} is declared multiple times")]
    DuplicateNamespace { name: String },

    #[error("use statement '{name}' is not a local namespace and not in package dependencies")]
    UnresolvedDependency { name: String },

    #[error("use statement '{name}' does not correspond to a .ks file or directory")]
    UsePathNotFound { name: String },

    #[error("{attribute} attribute is declared multiple times in {}", path.display())]
    DuplicateMetaAttribute { attribute: String, path: PathBuf },

    #[error("no files provided to load_files")]
    EmptyFileList,

    #[error("failed to create namespace context")]
    FailedToCreateNamespaceCtx,

    #[error("circular dependency detected: {}", chain.join(" -> "))]
    CircularDependency { chain: Vec<String> },

    #[error("circular schema dependency detected: {}", schemas.join(" -> "))]
    SchemaCircularDependency { schemas: Vec<String> },

    #[error("circular type dependency detected: {}", types.join(" -> "))]
    TypeCircularDependency { types: Vec<String> },

    #[error("operation '{}' returns a fallible type but has no error type defined", operation.borrow_string())]
    MissingErrorType { operation: SpannedToken![ident] },

    #[error("undefined type: '{name}'")]
    UndefinedType { name: String },

    #[error("version incompatibility: {package} requires version {required}, but found {found}")]
    VersionIncompatibility {
        package: String,
        required: String,
        found: String,
    },

    #[error("{0}")]
    VersionError(#[from] kintsu_manifests::version::VersionError),

    #[error("circular alias detected: {}", chain.iter().map(AsRef::<str>::as_ref).collect::<Vec<_>>().join(" -> "))]
    CircularAlias { chain: HashSet<String> },

    #[error("unresolved type: '{name}'")]
    UnresolvedType { name: String },

    #[error("internal error: {message}")]
    InternalError { message: String },

    #[error("union operand must be struct type: found {found_type} '{operand_name}'")]
    UnionOperandMustBeStruct {
        found_type: String,
        operand_name: String,
    },
}

impl Error {
    pub fn ns_dir(
        namespace: &SpannedToken![ident],
        attempt: &SpannedToken![ident],
        source: PathBuf,
    ) -> Self {
        let span = attempt.span();
        let (start, end) = (span.start, span.end);

        Self::NsDirConflict {
            namespace: namespace.borrow_string().clone(),
            parent: source
                .parent()
                .expect("parent path of named file")
                .into(),
            attempted: attempt.borrow_string().clone(),
        }
        .with_span(start, end)
    }

    pub fn conflict(
        namespace: SpannedToken![ident],
        ident: ctx::NamedItemContext,
        tag: &'static str,
    ) -> Self {
        let span = ident.name.span();
        let (start, end) = (span.start, span.end);
        Self::IdentConflict {
            namespace,
            ident,
            tag,
        }
        .with_span(start, end)
    }

    pub fn resolution(ident: SpannedToken![path]) -> Self {
        let span = ident.span();
        let (start, end) = (span.start, span.end);
        Self::ResolutionError { ident }.with_span(start, end)
    }

    /// wrap error with span information
    pub fn with_span(
        self,
        start: usize,
        end: usize,
    ) -> Self {
        Error::WithSpan {
            inner: Box::new(self),
            start,
            end,
        }
    }

    /// wrap error with source file information
    pub fn with_source(
        self,
        path: PathBuf,
        source: std::sync::Arc<String>,
    ) -> Self {
        Error::WithSource {
            inner: Box::new(self),
            path,
            source,
        }
    }

    /// callback version of with_span
    pub fn then_with_span(
        start: usize,
        end: usize,
    ) -> impl FnOnce(Self) -> Self {
        move |this: Error| this.with_span(start, end)
    }

    /// recursively extract the deepest span from nested WithSpan errors
    fn extract_deepest_span(&self) -> Option<(usize, usize)> {
        match self {
            Error::WithSpan { inner, start, end } => {
                // check if inner has a more specific span
                inner
                    .extract_deepest_span()
                    .or(Some((*start, *end)))
            },
            Error::WithSource { inner, .. } => inner.extract_deepest_span(),
            Error::AstError(LexingError::Spanned { span, .. }) => {
                let span = span.span();
                Some((span.start, span.end))
            },
            Error::VersionConflict { spans, .. } if !spans.is_empty() => Some(spans[0]),
            _ => None,
        }
    }

    /// recursively extract source from nested WithSource errors
    fn extract_source(&self) -> Option<(&std::path::Path, &str)> {
        match self {
            Error::WithSource {
                inner,
                path,
                source,
            } => {
                // check if inner has source, otherwise use ours
                inner
                    .extract_source()
                    .or(Some((path.as_path(), source.as_str())))
            },
            Error::WithSpan { inner, .. } => inner.extract_source(),
            _ => None,
        }
    }

    /// convert to miette report, using embedded source if available
    pub fn to_report(
        &self,
        override_path: Option<&std::path::Path>,
        override_source: Option<&str>,
        override_span: Option<(usize, usize)>,
    ) -> miette::Report {
        use miette::NamedSource;

        // extract source from error if not overridden
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

        // use override span if provided, otherwise extract deepest span
        let effective_span = override_span.or_else(|| self.extract_deepest_span());

        if let Some((s, e)) = effective_span {
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

    /// convert to miette report with explicit path and source (legacy method)
    pub fn to_report_with(
        &self,
        path: &std::path::Path,
        source: &str,
        override_span: Option<(usize, usize)>,
    ) -> miette::Report {
        self.to_report(Some(path), Some(source), override_span)
    }

    /// create a closure that wraps errors with source information
    pub fn with_context(
        path: PathBuf,
        source: std::sync::Arc<String>,
    ) -> impl FnOnce(Self) -> Self {
        move |err: Error| err.with_source(path, source)
    }

    /// create a closure that converts to report with source
    pub fn to_report_fn<'a>(
        path: &'a std::path::Path,
        source: &'a str,
    ) -> impl FnOnce(Self) -> miette::Report + 'a {
        move |err: Error| err.to_report_with(path, source, None)
    }

    pub fn circular_alias(i: impl IntoIterator<Item = String>) -> Self {
        Self::CircularAlias {
            chain: i.into_iter().collect(),
        }
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
