impl MutTokenStream {
    pub fn tokens_ref(&self) -> &Vec<crate::tokens::Spanned<crate::tokens::toks::Token>> {
        &self.tokens
    }
}
use logos::Logos;
use std::{ops::Range, sync::Arc};

use crate::{
    defs::{Spanned, span::Span},
    tokens::{
        AstResult, ImplDiagnostic, NewlineToken, SpannedToken,
        ast::{Parse, Peek},
        error::LexingError,
        toks::Token,
    },
};

#[derive(Debug, Clone)]
pub struct TokenStream {
    pub(crate) source: Arc<str>,
    pub(crate) tokens: Arc<Vec<SpannedToken>>,
    pub(crate) cursor: usize,
    range_start: usize,
    range_end: usize,
    is_fork: bool,
}

macro_rules! shared_peek {
    ($var: ident: $t: ty | $toks: expr) => {
        pub fn has_tokens_before_newline<T: Peek>(
            $var: &$t,
            pos: usize,
        ) -> bool {
            let toks = ($toks)($var);
            if let Some(ref current) = toks.get(pos) {
                if T::is(current) {
                    return true;
                }
                let maybe_last = pos.saturating_sub(pos);
                for tok in toks[..maybe_last].iter().rev() {
                    if NewlineToken::is(tok) {
                        return false;
                    } else if T::is(tok) {
                        return true;
                    }
                }
                false
            } else {
                false
            }
        }
    };
}

impl TokenStream {
    pub fn lex(source: &str) -> Result<Self, LexingError> {
        let source: Arc<str> = Arc::from(source);
        let mut lex = Token::lexer(&source);

        let approx = (source.len() / 8).max(256);
        let mut toks = Vec::with_capacity(approx);
        while let Some(token) = lex.next() {
            let span = lex.span();
            let token = token.map_err(|e| e.with_span(Span::new(span.start, span.end)))?;
            toks.push(Spanned::new(span.start, span.end, token));
        }

        let range_end = toks.len();
        Ok(Self {
            source,
            tokens: Arc::new(toks),
            cursor: 0,
            range_start: 0,
            range_end,
            is_fork: false,
        })
    }
    pub fn fork(&self) -> Self {
        Self {
            source: self.source.clone(),
            tokens: self.tokens.clone(),
            cursor: self.cursor,
            range_start: self.range_start,
            range_end: self.range_end,
            is_fork: true,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.cursor >= self.range_end
    }

    pub fn peek_unchecked(&self) -> Option<&SpannedToken> {
        let mut cursor = self.cursor;
        let mut next = self.peek_unchecked_with_whitespace();
        while let Some(v) = next {
            if !matches!(v.value, Token::Newline | Token::Space | Token::Tab) {
                return Some(v);
            } else {
                cursor += 1;
                next = self.tokens.get(cursor);
            }
        }
        None
    }

    pub fn peek_unchecked_with_whitespace(&self) -> Option<&SpannedToken> {
        if self.cursor < self.range_end {
            self.tokens.get(self.cursor)
        } else {
            None
        }
    }

    pub fn nth(
        &self,
        n: usize,
    ) -> Option<&SpannedToken> {
        let idx = self.cursor + n;
        if idx < self.range_end {
            self.tokens.get(idx)
        } else {
            None
        }
    }

    pub fn next_with_whitespace(&mut self) -> Option<SpannedToken> {
        if self.cursor >= self.range_end {
            return None;
        }
        let t = self.tokens.get(self.cursor).cloned();
        if t.is_some() {
            self.cursor += 1;
        }
        t
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn rewind(
        &mut self,
        to: usize,
    ) {
        let clamped = to.clamp(self.range_start, self.range_end);
        self.cursor = clamped;
    }
    pub fn len(&self) -> usize {
        self.range_end.saturating_sub(self.cursor)
    }
    pub fn all(&self) -> &[SpannedToken] {
        &self.tokens[self.range_start..self.range_end]
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn span_slice(
        &self,
        span: &Span,
    ) -> &str {
        let span = span.span();
        &self.source[span.start..span.end]
    }

    pub fn parse<T: Parse>(&mut self) -> Result<Spanned<T>, LexingError> {
        T::parse_spanned(self)
    }

    pub fn peek<T: Peek>(&self) -> bool {
        T::peek(self)
    }

    pub fn span_of(
        &self,
        cursor: usize,
    ) -> Option<&Span> {
        if cursor < self.range_end {
            self.tokens.get(cursor).map(|t| &t.span)
        } else {
            None
        }
    }

    pub fn current_span(&self) -> &Span {
        self.span_of(self.cursor)
            .unwrap_or(&Span::CallSite)
    }

    pub fn last_span(&self) -> Option<&Span> {
        if self.cursor != 0 {
            self.span_of(self.cursor - 1)
        } else {
            None
        }
    }

    pub fn is_fork(&self) -> bool {
        self.is_fork
    }

    pub fn ensure_consumed(&self) -> Result<(), LexingError> {
        if self.is_fork {
            return Ok(());
        }
        if self.cursor < self.range_end {
            let mut idx = self.cursor;
            while idx < self.range_end {
                match &self.tokens[idx].value {
                    crate::tokens::toks::Token::Newline
                    | crate::tokens::toks::Token::Space
                    | crate::tokens::toks::Token::Tab => {
                        idx += 1;
                        continue;
                    },
                    _ => break,
                }
            }

            if idx >= self.range_end {
                return Ok(());
            }

            if let Some(next) = self.tokens.get(idx) {
                return Err(LexingError::expected::<crate::tokens::toks::SemiToken>(
                    next.value.clone(),
                )
                .with_span(next.span.clone()));
            }
            return Err(LexingError::empty::<crate::tokens::toks::SemiToken>());
        }
        Ok(())
    }

    pub fn extract_inner_tokens<
        Open: Parse + Peek + ImplDiagnostic,
        Close: Parse + Peek + ImplDiagnostic,
    >(
        &mut self
    ) -> AstResult<(TokenStream, Spanned<()>)> {
        let (mut depth, first_span) = if let Some(first) = self.next() {
            if !Open::is(&first) {
                return Err(LexingError::expected::<Open>(first.value).with_span(first.span));
            }
            (1_usize, first.span)
        } else {
            return Err(LexingError::empty::<Open>());
        };

        let open_index = self.cursor - 1;

        let mut end_pos = None;

        while let Some(tok) = self.next() {
            if Open::is(&tok) {
                depth += 1;
            } else if Close::is(&tok) && depth > 0 {
                depth -= 1;
                if depth == 0 {
                    end_pos = Some(self.cursor);
                    break;
                }
            }
        }

        if let Some(end) = end_pos {
            let close_index = end - 1;
            let inner_start = open_index + 1;
            let inner_end = close_index;
            Ok((
                TokenStream {
                    source: self.source.clone(),
                    tokens: self.tokens.clone(),
                    cursor: inner_start,
                    range_start: inner_start,
                    range_end: inner_end,
                    is_fork: true,
                },
                Spanned::new(open_index, end, ()),
            ))
        } else {
            let mut err = LexingError::empty::<Close>();
            if let Some(last) = self.tokens.get(self.cursor - 1) {
                err = err.with_span(last.span.clone());
            } else {
                err = err.with_span(first_span)
            }
            Err(err)
        }
    }
}

#[test]
fn parse_vs_token_contract() {
    crate::tst::logging();

    let src = "namespace test;\n\n// comment\n";
    let tt = tokenize(src).expect("tokenize");

    let fork = tt.fork();
    let full = fork.all();

    use crate::tokens::toks::Token;
    assert!(
        full.iter()
            .any(|t| matches!(t.value, Token::Space | Token::Newline | Token::Tab)),
        "expected whitespace tokens in token list"
    );

    let mut iter_count = 0usize;
    for _ in tt {
        iter_count += 1;
    }

    assert!(
        full.len() > iter_count,
        "expected full token list length ({}) > parse-iterator count ({})",
        full.len(),
        iter_count
    );
}

impl Iterator for TokenStream {
    type Item = SpannedToken;

    fn next(&mut self) -> Option<Self::Item> {
        let mut next = self.next_with_whitespace();
        while let Some(v) = next {
            if !matches!(v.value, Token::Newline | Token::Space | Token::Tab) {
                return Some(v);
            } else {
                next = self.next_with_whitespace();
            }
        }
        None
    }
}

pub fn tokenize(src: &str) -> Result<TokenStream, LexingError> {
    TokenStream::lex(src)
}

pub fn tokenize_with(
    path: impl AsRef<std::path::Path>,
    src: &str,
) -> miette::Result<TokenStream> {
    crate::bail_miette!(tokenize(src); (path.as_ref()) for src)
}

macro_rules! paired {
    ($tok: ident) => {
        paste::paste! {
            #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
            pub struct $tok(Spanned<()>);

            impl $tok {
                pub fn new(span: Spanned<()>) -> Self {
                    Self(span)
                }

                pub fn call_site() -> Self {
                    Self(Spanned::call_site(()))
                }

                pub fn span(&self) -> &Span {
                    &self.0.span
                }

                pub fn write_with<F: Fn(&mut $crate::fmt::Printer)>(
                    &self,
                    tt: &mut crate::fmt::Printer,
                    inner: F,
                ) {
                    tt.token(&$crate::tokens::toks::Token::[<L $tok:camel>]);
                    inner(tt);
                    tt.token(&$crate::tokens::toks::Token::[<R $tok:camel>]);
                }
            }

            macro_rules! [<$tok:snake>] {
                (
                    $tokens: ident in $input: ident
                ) => {
                    match $input.extract_inner_tokens::<
                            $crate::tokens::toks::[<L $tok:camel Token>],
                            $crate::tokens::toks::[<R $tok:camel Token>],
                        >() {
                        Ok((token, span)) => {
                            $tokens = token;
                            $crate::tokens::$tok::new(span)
                        },
                        Err(e) => return Err(e)
                    }
                };
                (
                    $tokens: ident in $input: ident; $err: expr
                ) => {
                    match $input.extract_inner_tokens::<
                            $crate::tokens::toks::[<L $tok:camel Token>],
                            $crate::tokens::toks::[<R $tok:camel Token>],
                        >() {
                        Ok((token, span)) => {
                            $tokens = token;
                            $crate::tokens::$tok::new(span)
                        },
                        Err(..) => return $err
                    }
                };
            }
            pub(crate) use [<$tok:snake>];
        }
    };
}

paired! {
    Brace
}
paired! {
    Bracket
}
paired! {
    Paren
}

#[derive(Default, Debug, Clone)]
pub struct MutTokenStream {
    pub(crate) tokens: Vec<Spanned<Token>>,
}

impl MutTokenStream {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(sz: usize) -> Self {
        Self {
            tokens: Vec::with_capacity(sz),
        }
    }

    pub fn push(
        &mut self,
        token: Spanned<Token>,
    ) {
        self.tokens.push(token)
    }

    pub fn extend<I: IntoIterator<Item = Spanned<Token>>>(
        &mut self,
        i: I,
    ) {
        self.tokens.extend(i);
    }

    fn last_span(&self) -> Span {
        self.tokens
            .last()
            .map(|it| it.span.clone())
            .unwrap_or(Span::new(0, 0))
    }

    pub fn write_with_last(
        &mut self,
        token: Token,
    ) {
        let last = self.last_span();
        let start = last.span().end.saturating_sub(1);
        self.push(Spanned::new(start, last.span().end, token));
    }

    pub fn write_with_span(
        &mut self,
        token: Token,
        start: usize,
        end: usize,
    ) {
        self.push(Spanned::new(start, end, token));
    }

    pub fn span_of_range(
        &self,
        range: &Range<usize>,
    ) -> Span {
        if self.tokens.is_empty() {
            return Span::new(0, 0);
        }
        let len = self.tokens.len();
        if range.start < range.end && range.start < len {
            let start_span = self.tokens[range.start].span.span().start;
            let end_span = if range.end <= len {
                self.tokens[range.end - 1].span.span().end
            } else {
                self.tokens[len - 1].span.span().end
            };
            return Span::new(start_span, end_span);
        }

        if range.start < len {
            let anchor = self.tokens[range.start].span.span().start;
            return Span::new(anchor, anchor);
        }

        let last = &self.tokens[len - 1];
        Span::new(last.span.span().end, last.span.span().end)
    }

    pub fn splice_with(
        &mut self,
        range: Range<usize>,
        with: Vec<Token>,
    ) {
        let spans = self.span_of_range(&range);
        let rng = spans.span();
        self.tokens.splice(
            range,
            with.into_iter()
                .map(|it| Spanned::new(rng.start, rng.end, it)),
        );
    }

    pub fn write_with_spanned() {}

    pub fn all_tokens(&self) -> &[Spanned<Token>] {
        &self.tokens
    }
}

shared_peek!(tokens: [SpannedToken] | |toks| toks);

impl IntoIterator for MutTokenStream {
    type Item = Spanned<Token>;
    type IntoIter = <Vec<Spanned<Token>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        defs::Spanned,
        tokens::{self, AstResult, TokenStream, tokenize},
    };

    #[test_case::test_case(
        "{a}", |tt| {
            let inner;

            brace!(inner in tt);

            Ok(inner)
        }, 3; "parses braced with exact inner"
    )]
    #[test_case::test_case(
        "
            {
                a
            }
        ", |tt| {
            let inner;

            brace!(inner in tt);

            Ok(inner)
        }, 6; "parses braced"
    )]
    #[test_case::test_case(
        "
            [
                a
            ]
        ", |tt| {
            let inner;

            bracket!(inner in tt);

            Ok(inner)
        }, 6; "parses bracketed"
    )]
    #[test_case::test_case(
        "
            (
                a
            )
        ", |tt| {
            let inner;

            paren!(inner in tt);

            Ok(inner)
        }, 6; "parses parenthesized"
    )]
    fn test_paired(
        src: &str,
        get: impl Fn(&mut TokenStream) -> AstResult<TokenStream>,
        cursor: usize,
    ) -> crate::tokens::AstResult<()> {
        let mut tt = tokenize(src).unwrap();
        let mut inner = get(&mut tt)?;

        let a: Spanned<tokens::IdentToken> = inner.parse()?;
        assert_eq!(a.borrow_string(), "a");
        assert_eq!(tt.cursor, cursor);

        Ok(())
    }
}
