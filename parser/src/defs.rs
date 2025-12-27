use std::hash::Hash;

use crate::{Peek, tokens::ToTokens};
use kintsu_errors::HasSpan;

pub use span::Span;

pub mod span {
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
    )]
    pub struct RawSpan {
        pub start: usize,
        pub end: usize,
    }

    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
    )]
    #[serde(rename_all = "snake_case", tag = "type")]
    pub enum Span {
        CallSite,
        Known(RawSpan),
    }

    impl Span {
        pub fn new(
            start: usize,
            end: usize,
        ) -> Self {
            Self::Known(RawSpan { start, end })
        }

        pub fn len(&self) -> usize {
            match self {
                Self::Known(span) => span.end - span.start,
                _ => 1,
            }
        }

        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        pub fn span(&self) -> &RawSpan {
            match self {
                Self::CallSite => &RawSpan { start: 0, end: 0 },
                Self::Known(known) => known,
            }
        }
    }
}

pub trait Spans: Sized {
    fn with_span(
        self,
        span: span::Span,
    ) -> Spanned<Self> {
        Spanned { span, value: self }
    }
}

impl<T: Sized> Spans for T {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Spanned<T> {
    pub span: span::Span,
    pub value: T,
}

impl<T: PartialEq<T>> PartialEq<Self> for Spanned<T> {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.value == other.value
    }
}

impl<T: Eq> Eq for Spanned<T> {}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl<T: Ord + PartialEq> PartialOrd for Spanned<T> {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T: Ord + Eq> Ord for Spanned<T> {
    fn cmp(
        &self,
        other: &Self,
    ) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: Hash> Hash for Spanned<T> {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.value.hash(state)
    }
}

impl<T> Spanned<T> {
    pub fn new(
        start: usize,
        end: usize,
        value: T,
    ) -> Self {
        Self {
            value,
            span: span::Span::new(start, end),
        }
    }

    pub fn call_site(value: T) -> Self {
        Self {
            value,
            span: span::Span::CallSite,
        }
    }

    pub fn map<U>(
        self,
        f: impl FnOnce(T) -> U,
    ) -> Spanned<U> {
        Spanned {
            value: f(self.value),
            span: self.span,
        }
    }

    pub fn len(&self) -> usize {
        self.span.len()
    }

    pub fn is_empty(&self) -> bool {
        self.span.is_empty()
    }

    pub fn span(&self) -> &span::RawSpan {
        self.span.span()
    }
}

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: ToTokens> ToTokens for Spanned<T> {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.value)
    }
}

impl<T: ToTokens> ToTokens for &Spanned<T> {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.value)
    }
}

impl<T: Peek> Peek for Spanned<T> {
    fn is(token: &crate::tokens::Token) -> bool {
        T::is(token)
    }

    fn peek(stream: &crate::tokens::TokenStream) -> bool {
        T::peek(stream)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Spanned<T> {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl<T> HasSpan for Spanned<T> {
    fn span(&self) -> kintsu_errors::Span {
        let raw = self.span.span();
        kintsu_errors::Span::new(raw.start, raw.end)
    }
}
