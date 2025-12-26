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

    #[error(
        "adjacent tagging: name field '{name}' and content field '{content}' must be different"
    )]
    AdjacentTagConflict { name: String, content: String },

    #[error(
        "internal tagging: tag field '{tag_field}' conflicts with existing field in variant '{variant}'"
    )]
    InternalTagFieldConflict { tag_field: String, variant: String },

    #[error("untagged union has duplicate type '{type_name}' at indices {indices}")]
    UntaggedDuplicateType { type_name: String, indices: String },

    #[error("untagged variants at indices {indices} have indistinguishable structure")]
    UntaggedIndistinguishable { indices: String },

    #[error("internal tagging: tuple variant '{variant}' must reference a struct type")]
    InternalTagTupleNotStruct { variant: String },

    // Type Expression Errors (KTE) - RFC-0018, SPEC-0017, TSY-0014
    #[error("{operator}: expected {expected} type, found {actual}")]
    TypeExprTargetKindMismatch {
        operator: String,
        expected: String,
        actual: String,
    },

    #[error("{operator}: field '{field}' not found in type {type_name}")]
    TypeExprFieldNotFound {
        operator: String,
        field: String,
        type_name: String,
    },

    #[error("{operator}: variant '{variant}' not found in type {type_name}")]
    TypeExprVariantNotFound {
        operator: String,
        variant: String,
        type_name: String,
    },

    #[error("{operator}: selector list cannot be empty")]
    TypeExprEmptySelectors { operator: String },

    #[error("{operator}: no fields remain after operation")]
    TypeExprNoFieldsRemain { operator: String },

    #[error("{operator}: no variants remain after operation")]
    TypeExprNoVariantsRemain { operator: String },

    #[error("type expression cycle detected: {}", chain.join(" -> "))]
    TypeExprCycle { chain: Vec<String> },

    #[error("unresolved type in type expression: '{name}'")]
    TypeExprUnresolvedType { name: String },
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

    /// Converts this error to a CompilerError for the new error system.
    pub fn to_compiler_error(&self) -> kintsu_errors::CompilerError {
        use kintsu_errors::{domains::TaggingError, *};

        let span = self
            .extract_deepest_span()
            .map(|(s, e)| Span::new(s, e));

        match self {
            Self::NsNotDeclared => {
                NamespaceError::not_declared()
                    .with_span_opt(span)
                    .into()
            },
            Self::NsConflict => {
                NamespaceError::conflict()
                    .with_span_opt(span)
                    .into()
            },
            Self::NsDirConflict {
                namespace,
                parent,
                attempted,
            } => {
                NamespaceError::dir_conflict(namespace, parent.display().to_string(), attempted)
                    .with_span_opt(span)
                    .into()
            },
            Self::NamespaceMismatch { expected, found } => {
                NamespaceError::mismatch(expected, found)
                    .with_span_opt(span)
                    .into()
            },
            Self::DuplicateNamespace { name } => {
                NamespaceError::duplicate(name)
                    .with_span_opt(span)
                    .into()
            },
            Self::UnresolvedDependency { name } => {
                NamespaceError::unresolved_dep(name)
                    .with_span_opt(span)
                    .into()
            },
            Self::UsePathNotFound { name } => {
                NamespaceError::use_not_found(name)
                    .with_span_opt(span)
                    .into()
            },
            Self::ResolutionError { ident } => {
                ResolutionError::undefined_type(ident.borrow_path_inner().to_string())
                    .with_span_opt(span)
                    .into()
            },
            Self::UndefinedType { name } => {
                ResolutionError::undefined_type(name)
                    .with_span_opt(span)
                    .into()
            },
            Self::UnresolvedType { name } => {
                ResolutionError::undefined_type(name)
                    .with_span_opt(span)
                    .into()
            },
            Self::CircularDependency { chain }
            | Self::SchemaCircularDependency { schemas: chain } => {
                ResolutionError::circular_dependency(chain.clone())
                    .with_span_opt(span)
                    .into()
            },
            Self::TypeCircularDependency { types } => {
                ResolutionError::type_cycle(types.clone())
                    .with_span_opt(span)
                    .into()
            },
            Self::CircularAlias { chain } => {
                ResolutionError::circular_alias_from_set(chain.clone())
                    .with_span_opt(span)
                    .into()
            },
            Self::DuplicateType { name } => {
                TypeDefError::duplicate_type(name)
                    .with_span_opt(span)
                    .into()
            },
            Self::VersionConflict { values, .. } => {
                MetadataError::version_conflict(values.iter().copied())
                    .with_span_opt(span)
                    .into()
            },
            Self::DuplicateMetaAttribute { attribute, path } => {
                MetadataError::duplicate_attribute(attribute, path.display().to_string())
                    .with_span_opt(span)
                    .into()
            },
            Self::VersionIncompatibility {
                package,
                required,
                found,
            } => {
                MetadataError::version_incompatibility(package, required, found)
                    .with_span_opt(span)
                    .into()
            },
            Self::LexError { ch, .. } => {
                LexicalError::unknown_char(*ch)
                    .with_span_opt(span)
                    .into()
            },
            Self::MissingErrorType { operation } => {
                TypeDefError::missing_error_type(operation.borrow_string())
                    .with_span_opt(span)
                    .into()
            },
            Self::UnionOperandMustBeStruct {
                found_type,
                operand_name,
            } => {
                UnionError::non_struct_operand(operand_name, found_type)
                    .with_span_opt(span)
                    .into()
            },
            Self::AdjacentTagConflict { name, content } => {
                UnionError::adjacent_tag_conflict(name, content)
                    .with_span_opt(span)
                    .into()
            },
            Self::InternalTagFieldConflict { tag_field, variant } => {
                UnionError::internal_tag_field_conflict(tag_field, variant)
                    .with_span_opt(span)
                    .into()
            },
            Self::UntaggedDuplicateType { type_name, indices } => {
                TaggingError::untagged_duplicate(
                    type_name,
                    indices
                        .split(", ")
                        .filter_map(|s| s.parse().ok()),
                )
                .with_span_opt(span)
                .into()
            },
            Self::UntaggedIndistinguishable { indices } => {
                TaggingError::untagged_indistinguishable(
                    indices
                        .split(", ")
                        .filter_map(|s| s.parse().ok()),
                )
                .with_span_opt(span)
                .into()
            },
            Self::InternalTagTupleNotStruct { .. } => {
                TaggingError::internal_requires_struct()
                    .with_span_opt(span)
                    .into()
            },
            Self::IdentConflict {
                namespace,
                ident,
                tag,
            } => {
                TypeDefError::ident_conflict(
                    namespace.borrow_string(),
                    *tag,
                    ident.name.borrow_string(),
                )
                .with_span_opt(span)
                .into()
            },
            Self::MissingLibError => FilesystemError::missing_lib_ks().into(),
            Self::EmptyFileList => FilesystemError::empty_file_list().into(),
            Self::GlobError(e) => FilesystemError::invalid_glob(e.to_string()).into(),
            Self::IoError(e) => FilesystemError::io_error(e.to_string()).into(),
            Self::Fs(e) => FilesystemError::io_error(e.to_string()).into(),
            Self::LibPldMultiSegmentImport => {
                ParsingError::lib_multi_segment_import()
                    .with_span_opt(span)
                    .into()
            },
            Self::LibPldInvalidItem => {
                ParsingError::lib_invalid_item()
                    .with_span_opt(span)
                    .into()
            },
            Self::LibPldMissingNamespace => {
                ParsingError::lib_missing_namespace()
                    .with_span_opt(span)
                    .into()
            },
            Self::LintFailure => InternalError::internal("linting errors").into(),
            Self::FailedToCreateNamespaceCtx => InternalError::failed_namespace_ctx().into(),
            Self::InternalError { message } => InternalError::internal(message).into(),
            Self::WithSpan { inner, .. } => inner.to_compiler_error(),
            Self::WithSource {
                inner,
                path,
                source,
            } => {
                inner
                    .to_compiler_error()
                    .with_source_arc(path.clone(), std::sync::Arc::clone(source))
            },
            // These need separate handling or conversion
            Self::AstError(e) => {
                LexicalError::lexer_error(e.to_string())
                    .with_span_opt(span)
                    .into()
            },
            Self::ManifestError(e) => {
                PackageError::manifest_error(e.to_string())
                    .with_span_opt(span)
                    .into()
            },
            Self::VersionError(e) => {
                PackageError::version_error(e.to_string())
                    .with_span_opt(span)
                    .into()
            },
            // Type Expression Errors (KTE) - RFC-0018, SPEC-0017, TSY-0014
            Self::TypeExprTargetKindMismatch {
                operator,
                expected,
                actual,
            } => {
                TypeDefError::type_expr_target_mismatch(operator, expected, actual)
                    .with_span_opt(span)
                    .into()
            },
            Self::TypeExprFieldNotFound {
                operator,
                field,
                type_name,
            } => {
                TypeDefError::type_expr_field_not_found(operator, field, type_name)
                    .with_span_opt(span)
                    .into()
            },
            Self::TypeExprVariantNotFound {
                operator,
                variant,
                type_name,
            } => {
                TypeDefError::type_expr_variant_not_found(operator, variant, type_name)
                    .with_span_opt(span)
                    .into()
            },
            Self::TypeExprEmptySelectors { operator } => {
                TypeDefError::type_expr_empty_selectors(operator)
                    .with_span_opt(span)
                    .into()
            },
            Self::TypeExprNoFieldsRemain { operator } => {
                TypeDefError::type_expr_no_fields_remain(operator)
                    .with_span_opt(span)
                    .into()
            },
            Self::TypeExprNoVariantsRemain { operator } => {
                TypeDefError::type_expr_no_variants_remain(operator)
                    .with_span_opt(span)
                    .into()
            },
            Self::TypeExprCycle { chain } => {
                TypeDefError::type_expr_cycle(chain.clone())
                    .with_span_opt(span)
                    .into()
            },
            Self::TypeExprUnresolvedType { name } => {
                TypeDefError::type_expr_unresolved(name)
                    .with_span_opt(span)
                    .into()
            },
            Self::Infallible(_) => unreachable!(),
        }
    }
}

impl From<Error> for kintsu_errors::CompilerError {
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
