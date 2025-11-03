use crate::defs::Spanned;
pub use crate::tokens::{ast::*, error::*, stream::*, toks::*};

pub trait ImplDiagnostic {
    fn fmt() -> &'static str;
}

pub mod error;

pub mod toks;

pub type SpannedToken = Spanned<Token>;

pub mod stream;

pub mod ast;

pub type AstResult<T, E = LexingError> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use crate::defs::span::Span;
    use std::marker::PhantomData;

    use serde::Serialize;

    use super::*;

    const SAMPLE_FILES: &[&str] = &[
        include_str!("../samples/enum.ks"),
        include_str!("../samples/ns.ks"),
        include_str!("../samples/array.ks"),
        include_str!("../samples/complex_union.ks"),
        include_str!("../samples/some_import.ks"),
        include_str!("../samples/test_message.ks"),
    ];

    #[test]
    fn samples_lex_without_error_tokens() {
        for src in SAMPLE_FILES.iter() {
            let ts = tokenize(src).unwrap_or_else(|e| panic!("lex errors in sample {src}: {e:#?}"));
            assert!(!ts.all().is_empty());
        }
    }

    #[test_case::test_case(
        "// some inner comment value
        // next line comment
        namespace abc;
        ", |kinds| {
            assert!(matches!(kinds[0], Token::CommentSingleLine(cmt) if cmt == "some inner comment value"));
            assert!(matches!(kinds[2], Token::CommentSingleLine(cmt) if cmt == "next line comment"));
            assert!(matches!(kinds[4], Token::KwNamespace));
            assert!(matches!(kinds[5], Token::Ident(s) if s == "abc"));
        }; "parses single line comment"
    )]
    #[test_case::test_case(
        "/*
        some
        multiline
        comment
        */
        namespace abc;
        ", |kinds| {
            assert!(matches!(kinds[0], Token::CommentMultiLine(cmt) if cmt == "some\nmultiline\ncomment"));
            assert!(matches!(kinds[2], Token::KwNamespace));
            assert!(matches!(kinds[3], Token::Ident(s) if s == "abc"));
        }; "parses multi line comment"
    )]
    #[test_case::test_case(
        "namespace test;", |kinds| {
            assert!(matches!(kinds[0], Token::KwNamespace));
            assert!(matches!(kinds[1], Token::Ident(s) if s == "test"));
            assert!(matches!(kinds[2], Token::Semi));
        }; "parses namespace"
    )]
    #[test_case::test_case(
        "type cunion_1 = oneof str | i32;", |kinds| {
            assert!(matches!(kinds[0], Token::KwType));
            assert!(matches!(kinds[1], Token::Ident(s) if s == "cunion_1"));
            assert!(matches!(kinds[2], Token::Eq));
            assert!(matches!(kinds[3], Token::KwOneof));
            assert!(matches!(kinds[4], Token::KwStr));
            assert!(matches!(kinds[5], Token::Pipe));
            assert!(matches!(kinds[6], Token::KwI32));
            assert!(matches!(kinds[7], Token::Semi));
        }; "parses oneof type alias"
    )]
    #[test_case::test_case(
        "type aliased = i64;", |kinds| {
            assert!(matches!(kinds[0], Token::KwType));
            assert!(matches!(kinds[1], Token::Ident(s) if s == "aliased"));
            assert!(matches!(kinds[2], Token::Eq));
            assert!(matches!(kinds[3], Token::KwI64));
            assert!(matches!(kinds[4], Token::Semi));
        }; "parses simple type alias"
    )]
    #[test_case::test_case(
        "#[version(1)]", |kinds| {
            assert!(matches!(kinds[0], Token::Hash));
            assert!(matches!(kinds[1], Token::LBracket));
            assert!(matches!(kinds[2], Token::Ident(ver) if ver == "version"));
            assert!(matches!(kinds[3], Token::LParen));
            assert!(matches!(kinds[4], Token::Number(n) if *n == 1));
            assert!(matches!(kinds[5], Token::RParen));
            assert!(matches!(kinds[6], Token::RBracket));
        }; "parses outer meta"
    )]
    #[test_case::test_case(
        "#![version(1)]", |kinds| {
            assert!(matches!(kinds[0], Token::Hash));
            assert!(matches!(kinds[1], Token::Bang));
            assert!(matches!(kinds[2], Token::LBracket));
            assert!(matches!(kinds[3], Token::Ident(ver) if ver == "version"));
            assert!(matches!(kinds[4], Token::LParen));
            assert!(matches!(kinds[5], Token::Number(n) if *n == 1));
            assert!(matches!(kinds[6], Token::RParen));
            assert!(matches!(kinds[7], Token::RBracket));
        }; "parses inner meta"
    )]
    #[test_case::test_case(
        "struct msg_with_union_default {
            a: cunion_1,
            b: i32,
        };", |kinds| {
            assert!(matches!(kinds[0], Token::KwStruct));
            assert!(matches!(kinds[1], Token::Ident(msg) if msg == "msg_with_union_default"));
            assert!(matches!(kinds[2], Token::LBrace));
            assert!(matches!(kinds[4], Token::Ident(a) if a == "a"));
            assert!(matches!(kinds[5], Token::Colon));
            assert!(matches!(kinds[6], Token::Ident(t) if t == "cunion_1"));
            assert!(matches!(kinds[7], Token::Comma));
            assert!(matches!(kinds[9], Token::Ident(b) if b == "b"));
            assert!(matches!(kinds[10], Token::Colon));
            assert!(matches!(kinds[11], Token::KwI32));
            assert!(matches!(kinds[12], Token::Comma));
            assert!(matches!(kinds[14], Token::RBrace));
        }; "parses struct"
    )]
    fn keyword_then_ident<F: Fn(Vec<&Token>)>(
        src: &str,
        expect: F,
    ) {
        let ts = tokenize(src).unwrap();
        let kinds: Vec<&Token> = ts
            .all()
            .iter()
            .map(|t| &t.value)
            // .filter(|it| !matches!(it, Token::Newline))
            .collect();
        println!("{kinds:#?}");
        (expect)(kinds);
    }

    #[test]
    fn test_increment_parse() -> Result<(), LexingError> {
        let src = "namespace abc;";

        let mut ts = tokenize(src).unwrap();

        let _: Spanned<KwNamespaceToken> = ts.parse()?;
        let _: Spanned<IdentToken> = ts.parse()?;

        Ok(())
    }

    #[test_case::test_case(
        "namespace i32",
        |mut ts: TokenStream| {
            let _: Spanned<KwNamespaceToken> = ts.parse().unwrap();
            ts.parse::<IdentToken>().unwrap_err()
        }, "expected identifier, found i32", Span::new(10, 13);
        "expectation error: namespace followed by keyword"
    )]
    fn test_incremental_and_error_cases(
        src: &str,
        expect: impl FnOnce(TokenStream) -> LexingError,
        err_message: &str,
        expect_span: Span,
    ) {
        let ts = tokenize(src).unwrap();
        let err = expect(ts);
        let expect_span = expect_span.span();

        if let LexingError::Spanned { span, source } = err {
            let span = span.span();
            assert_eq!(format!("{source}"), err_message);

            assert_eq!(
                span.start, expect_span.start,
                "start of error span should align"
            );
            assert_eq!(span.end, expect_span.end, "end of error span should align");
        } else {
            panic!("expected spanned error");
        }
    }

    #[test_case::test_case(
        "// multiple
        // single
        // line
        // comments
        ",
        serde_json::json!({"span":{"end":65,"start":0,"type":"known"},"value":[{"span":{"end":11,"start":0,"type":"known"},"value":["multiple"]},{"span":{"end":29,"start":11,"type":"known"},"value":["single"]},{"span":{"end":45,"start":29,"type":"known"},"value":["line"]},{"span":{"end":65,"start":45,"type":"known"},"value":["comments"]}]}),
        PhantomData::<toks::CommentSingleLineToken>;
        "parses single line comments over new lines"
    )]
    #[test_case::test_case(
        "/*
            the inner comment value
        */ /*
            next comment value
        */",
        serde_json::json!({"span":{"end":94,"start":0,"type":"known"},"value":[{"span":{"end":49,"start":0,"type":"known"},"value":["the inner comment value"]},{"span":{"end":94,"start":50,"type":"known"},"value":["next comment value"]}]}),
        PhantomData::<toks::CommentMultiLineToken>;
        "parses comments over new lines"
    )]
    fn test_to_vec<T>(
        src: &str,
        expect: serde_json::Value,
        _: PhantomData<T>,
    ) -> Result<(), LexingError>
    where
        T: Peek + Parse + Serialize + ImplDiagnostic + Clone, {
        let mut ts = tokenize(src).unwrap();
        let found: Spanned<Vec<Spanned<T>>> = ts.parse()?;

        let as_j = serde_json::to_value(&found).unwrap();
        let debug = serde_json::to_string(&as_j).unwrap();

        println!("found: {debug}");

        assert_eq!(as_j, expect, "spans and span values must be exactly equal");
        Ok(())
    }

    #[test_case::test_case(
        "
            a,
            b,
            c
        ",
        serde_json::json!({"span":{"end":44,"start":0,"type":"known"},"value":{"values":[{"sep":{"span":{"end":15,"start":14,"type":"known"},"value":null},"value":{"span":{"end":14,"start":0,"type":"known"},"value":["a"]}},{"sep":{"span":{"end":30,"start":29,"type":"known"},"value":null},"value":{"span":{"end":29,"start":15,"type":"known"},"value":["b"]}},{"sep":null,"value":{"span":{"end":44,"start":30,"type":"known"},"value":["c"]}}]}}),
        PhantomData::<(toks::IdentToken, toks::CommaToken)>;
        "repeated idents with new lines"
    )]
    fn test_repeated<T, Sep>(
        src: &str,
        expect: serde_json::Value,
        _: PhantomData<(T, Sep)>,
    ) -> Result<(), LexingError>
    where
        T: Peek + Parse + Serialize + ImplDiagnostic + Clone,
        Sep: Peek + Parse + Serialize + ImplDiagnostic + Clone, {
        let mut ts = tokenize(src).unwrap();
        let found: Spanned<Repeated<T, Sep>> = ts.parse()?;
        let as_j = serde_json::to_value(&found).unwrap();
        let debug = serde_json::to_string(&as_j).unwrap();

        println!("found: {debug}");

        assert_eq!(as_j, expect, "spans and span values must be exactly equal");
        Ok(())
    }
}
