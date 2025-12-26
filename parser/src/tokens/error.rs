use std::{
    num::{ParseFloatError, ParseIntError},
    str::ParseBoolError,
    string::ParseError,
};

use crate::{
    defs::span::Span,
    tokens::{ImplDiagnostic, Token, TokenStream},
};

#[derive(thiserror::Error, Debug, Clone, PartialEq, Default)]
pub enum LexingError {
    #[default]
    #[error("unknown lexing error")]
    Unknown,

    #[error("parse int error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("parse bool error: {0}")]
    ParseBool(#[from] ParseBoolError),
    #[error("parse float error: {0}")]
    ParseFloat(#[from] ParseFloatError),

    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("expected {expect}, found end of token stream")]
    EmptyTokens { expect: &'static str },

    #[error("expected {}, found end of token stream", expect.join(" |"))]
    EmptyOneOfTokens { expect: Vec<&'static str> },

    #[error("expected {expect}, found {found}")]
    ExpectationFailure { expect: &'static str, found: Token },

    #[error("expected {}, found '{found}'", expect.join(" |"))]
    ExpectationFailures {
        expect: Vec<&'static str>,
        found: Token,
    },

    #[error("unknown meta attribute, '{found}'. expected one of {}", expect.join(", "))]
    UnknownMeta {
        expect: Vec<&'static str>,
        found: String,
    },

    #[error("unknown type expression operator '{found}'. expected one of {}", expect.join(", "))]
    UnknownTypeExprOp {
        expect: Vec<&'static str>,
        found: String,
    },

    #[error("invalid path: {input}. {reason}")]
    InvalidPath { input: String, reason: String },

    #[error("{source}")]
    Spanned { source: Box<Self>, span: Span },
}

impl LexingError {
    pub fn one_of<I: IntoIterator<Item = &'static str>>(
        stream: &mut TokenStream,
        expect: I,
        empty_span: &Span,
    ) -> Self {
        if let Some(next) = stream.next() {
            Self::expected_oneof(expect, next.value).with_span(next.span.clone())
        } else {
            Self::empty_oneof(expect).with_span(empty_span.clone())
        }
    }

    pub fn empty_oneof<I: IntoIterator<Item = &'static str>>(expect: I) -> Self {
        Self::EmptyOneOfTokens {
            expect: expect.into_iter().collect(),
        }
    }

    pub fn empty<D: ImplDiagnostic>() -> Self {
        Self::EmptyTokens { expect: D::fmt() }
    }

    pub fn expected<D: ImplDiagnostic>(found: Token) -> Self {
        Self::ExpectationFailure {
            expect: D::fmt(),
            found,
        }
    }

    pub fn expected_oneof<I: IntoIterator<Item = &'static str>>(
        expect: I,
        found: Token,
    ) -> Self {
        Self::ExpectationFailures {
            expect: expect.into_iter().collect(),
            found,
        }
    }

    pub fn with_span(
        self,
        span: Span,
    ) -> Self {
        Self::Spanned {
            source: Box::new(self),
            span,
        }
    }

    pub fn then_with_span(span: Span) -> impl FnOnce(Self) -> Self {
        move |this| this.with_span(span)
    }

    pub fn unknown_meta<I: IntoIterator<Item = &'static str>>(
        expect: I,
        found: String,
        span: &Span,
    ) -> Self {
        Self::UnknownMeta {
            expect: expect.into_iter().collect(),
            found,
        }
        .with_span(span.clone())
    }

    pub fn unknown_type_expr_op<I: IntoIterator<Item = &'static str>>(
        expect: I,
        found: String,
        span: &Span,
    ) -> Self {
        Self::UnknownTypeExprOp {
            expect: expect.into_iter().collect(),
            found,
        }
        .with_span(span.clone())
    }
}

#[macro_export]
macro_rules! bail_unchecked {
    (
        $e:expr; $ret: expr
    ) => {
        match $e {
            Ok(v) => v,
            Err(..) => return $ret,
        }
    };
}
