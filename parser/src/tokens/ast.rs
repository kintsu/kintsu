use crate::{
    defs::{Spanned, span::Span},
    tokens::{
        ImplDiagnostic, ToTokens, Token, error::LexingError, stream::TokenStream,
        toks::SpannedToken,
    },
};

#[allow(unused_variables)]
pub trait Peek: Sized {
    fn peek(stream: &TokenStream) -> bool {
        if let Some(token) = stream.peek_unchecked() {
            Self::is(&token.value)
        } else {
            false
        }
    }

    fn is(token: &Token) -> bool {
        false
    }
}

fn extract_span(checkpoint: Option<&SpannedToken>) -> Option<&Span> {
    if let Some(check) = checkpoint {
        Some(&check.span)
    } else {
        None
    }
}

pub trait Parse: Sized {
    fn parse(stream: &mut TokenStream) -> Result<Self, LexingError>;
    fn parse_spanned(stream: &mut TokenStream) -> Result<Spanned<Self>, LexingError> {
        let start = extract_span(stream.tokens.get(stream.cursor))
            .unwrap_or(&Span::CallSite)
            .span()
            .start;

        let p = Self::parse(stream)?;

        let end = stream
            .tokens
            .get(stream.cursor.saturating_sub(1))
            .map(|it| it.span().end)
            .unwrap_or(0);

        Ok(Spanned::new(start, end, p))
    }
}

impl<T: Parse> Parse for Spanned<T> {
    fn parse(stream: &mut TokenStream) -> Result<Self, LexingError> {
        stream.parse()
    }
}

impl<T: Peek + Parse> Parse for Option<T> {
    fn parse(stream: &mut TokenStream) -> Result<Self, LexingError> {
        if stream.peek::<T>() {
            Ok(Some(T::parse(stream)?))
        } else {
            Ok(None)
        }
    }
}

impl<T: Parse> Parse for Box<T> {
    fn parse(stream: &mut TokenStream) -> Result<Self, LexingError> {
        Ok(Box::new(T::parse(stream)?))
    }
}

impl<T: Peek> Peek for Box<T> {
    fn is(token: &Token) -> bool {
        T::is(token)
    }
}

#[derive(PartialEq, Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct RepeatedItem<T: Peek + Parse, Sep: Peek + Parse> {
    pub value: Spanned<T>,
    pub(crate) sep: Option<Spanned<Sep>>,
}

#[derive(PartialEq, Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Repeated<T: Peek + Parse, Sep: Peek + Parse> {
    pub values: Vec<RepeatedItem<T, Sep>>,
}

impl<T: Peek + Parse, Sep: Peek + Parse> Repeated<T, Sep> {
    pub fn empty() -> Self {
        Self {
            values: Vec::with_capacity(0),
        }
    }
}

impl<T: Peek + Parse, Sep: Peek + Parse> IntoIterator for Repeated<T, Sep> {
    type Item = RepeatedItem<T, Sep>;
    type IntoIter = <Vec<RepeatedItem<T, Sep>> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<T: Peek + Parse + ImplDiagnostic, Sep: Peek + Parse + Clone + ImplDiagnostic> Peek
    for Repeated<T, Sep>
{
    fn is(token: &Token) -> bool {
        T::is(token)
    }
    fn peek(stream: &TokenStream) -> bool {
        T::peek(stream)
    }
}

impl<T: Peek + Parse + ImplDiagnostic, Sep: Peek + Parse + Clone + ImplDiagnostic> Parse
    for Repeated<T, Sep>
{
    fn parse(stream: &mut TokenStream) -> Result<Self, LexingError> {
        let mut values = Vec::new();
        if !stream.peek::<T>() {
            return Err(LexingError::empty::<T>());
        }
        let first: Spanned<T> = stream.parse()?;
        let mut sep: Option<Spanned<Sep>> = None;
        if stream.peek::<Sep>() {
            let s: Spanned<Sep> = stream.parse()?;
            sep = Some(s);
        }
        values.push(RepeatedItem {
            value: first,
            sep: sep.clone(),
        });
        while sep.is_some() {
            if !stream.peek::<T>() {
                break;
            }
            let next: Spanned<T> = stream.parse()?;
            let mut next_sep: Option<Spanned<Sep>> = None;
            if stream.peek::<Sep>() {
                let s: Spanned<Sep> = stream.parse()?;
                next_sep = Some(s);
            }
            values.push(RepeatedItem {
                value: next,
                sep: next_sep.clone(),
            });
            sep = next_sep;
        }
        Ok(Self { values })
    }
}

impl<T: Parse + Peek> Parse for Vec<Spanned<T>> {
    fn parse(stream: &mut TokenStream) -> Result<Self, LexingError> {
        let mut out = vec![];
        loop {
            if !stream.peek::<T>() {
                break;
            }
            out.push(stream.parse()?);
        }
        Ok(out)
    }
}

impl<T: Peek + Parse, Sep: Peek + Parse> super::ToTokens for Repeated<T, Sep>
where
    Spanned<T>: ToTokens,
    Spanned<Sep>: ToTokens,
{
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        for v in &self.values {
            v.value.write(tt);
            v.sep.write(tt);
        }
    }
}

impl<T: ToTokens> ToTokens for Option<T> {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        if let Some(v) = self {
            tt.write(v);
        }
    }
}

impl<T: ToTokens> ToTokens for &Option<T> {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        if let Some(v) = self {
            tt.write(v);
        }
    }
}

impl<T: ToTokens> ToTokens for Box<T> {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(self.as_ref())
    }
}

impl<T: ToTokens> ToTokens for Vec<T> {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        for it in self {
            it.write(tt);
        }
    }
}
