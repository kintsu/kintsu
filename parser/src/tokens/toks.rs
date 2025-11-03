use crate::{
    defs::{Spanned, span::Span},
    fmt::{FormatConfig, printer::Printer},
    tokens::{AstResult, ImplDiagnostic, error::LexingError},
};
use logos::Logos;
use std::fmt;

pub type SpannedToken = Spanned<Token>;

pub trait ToTokens {
    fn write(
        &self,
        tt: &mut Printer,
    );

    fn display(&self) -> String {
        let mut printer = Printer::new(&FormatConfig::default());
        self.write(&mut printer);
        printer.buf
    }
}

macro_rules! tokens {
    (
        $(
            $(#[token($met:literal$(, $($ex:ident = $v:literal), + $(,)?)?)])*
            $(#[regex($reg:literal $(,$e:expr)?)])*
            $(#[regfmt($fmt:expr)])?

            $(#[derive($($d:path), + $(,)?)])?
            $tok:ident $(($inner:ty))?
        ),+ $(,)?
    ) => {

        #[allow(non_snake_case, unused)]
        #[derive(Logos, Clone, PartialEq)]
        #[logos(skip r"[ \t\r\f]+")]
        #[logos(error = LexingError)]
        pub enum Token {
            $(
                $(#[token($met $(, $($ex = $v,)*)?)])*
                $(#[regex($reg $(,$e)?)])*
                $tok $(($inner))?,
            )*
        }

        paste::paste!{
            $(
                #[allow(non_snake_case, unused)]
                #[doc = concat!(
                    "Represents `", $($met)? $($fmt)?, "` token(s)"
                )]
                #[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize, $($($d,)*)*)]
                pub struct [<$tok Token>](
                    $(
                        $inner,
                    )*
                    #[serde(skip)]
                    ()
                );

                impl super::ToTokens for Spanned<[<$tok Token>]> {
                    fn write(&self, tt: &mut crate::fmt::Printer) {
                        tt.token(&self.token());
                    }
                }

                #[allow(non_snake_case, unused)]
                impl [<$tok Token>]{
                    #[allow(clippy::new_without_default)]
                    pub fn new($([<$inner:snake>]: $inner ,)*)-> Self {
                        Self($([<$inner:snake>],)*())
                    }

                    pub fn token(&self) -> Token {
                        #[allow(unused_parens)]
                        Token::$tok$(( { let _pd: ::core::marker::PhantomData<$inner> = ::core::marker::PhantomData; self.0.clone() } ))?
                    }

                    $(
                        pub fn [<borrow_ $inner:snake>](&self) -> &$inner {
                            &self.0
                        }
                    )*
                }

                impl ImplDiagnostic for [<$tok Token>] {
                    fn fmt() -> &'static str {
                        $($met)?
                        $($fmt)?
                    }
                }


                #[allow(non_snake_case, unused, dead_code)]
                impl crate::tokens::ast::Peek for [<$tok Token>] {
                    fn is(token: &Token) -> bool {
                        matches!(&token, Token::$tok $(
                            ($inner)
                        )?)
                    }
                }

                #[allow(non_snake_case)]
                impl crate::tokens::ast::Parse for [<$tok Token>] {
                    fn parse(tokens: &mut crate::tokens::stream::TokenStream) -> Result<Self, LexingError> {
                        #[allow(unused_parens)]
                        let ($($inner,)* close) = match tokens.next() {
                            Some(tok) => (
                                match tok.value {
                                    Token::$tok $(($inner))? => {
                                        ($($inner, )? ())
                                    },
                                    rest => return Err(LexingError::expected::<[<$tok Token>]>(
                                        rest
                                    ).with_span(
                                        tok.span
                                    ))
                                }
                            ),
                            None => return Err(LexingError::empty::<[<$tok Token>]>().with_span(
                                Span::new(tokens.cursor.saturating_sub(1), tokens.cursor)
                            ))
                        };
                        Ok(Self($($inner,)* close))
                    }
                }
            )*
        }
    };
}

pub(crate) use tokens as declare_tokens;

pub type PathInner = crate::ast::path::Path;

declare_tokens! {
    #[token(" ", priority = 0)]
    Space,
    #[token("\t", priority = 0)]
    Tab,

    #[token("->")]
    Return,
    #[token("&")]
    Amp,
    #[token("::")]
    DoubleColon,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(";")]
    Semi,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token("?")]
    QMark,
    #[token("=")]
    Eq,
    #[token("|")]
    Pipe,
    #[token("#")]
    Hash,
    #[token("!")]
    Bang,

    #[token("namespace")]
    KwNamespace,
    #[token("use")]
    KwUse,
    #[token("struct")]
    KwStruct,
    #[token("enum")]
    KwEnum,
    #[token("type")]
    KwType,
    #[token("oneof")]
    KwOneof,
    #[token("error")]
    KwError,
    #[token("operation")]
    KwOperation,


    #[token("bool")]
    KwBool,
    #[token("null")]
    KwNull,
    #[token("str")]
    KwStr,
    #[token("i8")]
    KwI8,
    #[token("i16")]
    KwI16,
    #[token("i32")]
    KwI32,
    #[token("i64")]
    KwI64,
    #[token("u8")]
    KwU8,
    #[token("u16")]
    KwU16,
    #[token("u32")]
    KwU32,
    #[token("u64")]
    KwU64,
    #[token("usize")]
    KwUsize,
    #[token("f16")]
    KwF16,
    #[token("f32")]
    KwF32,
    #[token("f64")]
    KwF64,
    #[token("complex")]
    KwComplex,
    #[token("datetime")]
    KwDateTime,
    #[token("binary")]
    KwBinary,
    #[token("base64")]
    KwBase64,

    #[token("never")]
    KwNever,
    #[token("schema")]
    KwSchema,

    #[regex(r"\r?\n")]
    #[regfmt("\\n")]
    Newline,

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex: &mut logos::Lexer<'_, Token>| -> String { lex.slice().to_string() })]
    #[regfmt("identifier")]
    #[derive(PartialOrd, Ord, Hash, Eq)]
    Ident(String),

    #[regex(r"(schema|[A-Za-z_][A-Za-z0-9_]*)::[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*", parse_path)]
    #[regfmt("path")]
    Path(PathInner),

    #[regex(r"[0-9]+", parse_number)]
    #[regfmt("number")]
    Number(i32),

    #[regex(r#""([^"\\]|\\.)*""#, parse_string)]
    #[regex(r#"'([^'\\]|\\.)*'"#, parse_string)]
    #[regfmt("string")]
    String(String),

    #[regex(r"//[^\n]*", |lex: &mut logos::Lexer<'_, Token>| -> String { unescape_comment(&lex.slice()[2..]) })]
    #[regfmt("comment")]
    CommentSingleLine(String),

    #[regex(r"/\*([^/*]*)\*/", |lex: &mut logos::Lexer<'_, Token>| -> String { unescape_comment(&lex.slice()[2..lex.slice().len()-2]) })]
    #[regfmt("comment")]

    CommentMultiLine(String),
}

fn parse_number(lex: &mut logos::Lexer<'_, Token>) -> Result<i32, LexingError> {
    Ok(lex.slice().to_string().parse()?)
}

fn parse_path(lex: &mut logos::Lexer<'_, Token>) -> AstResult<PathInner> {
    crate::ast::path::Path::parse(lex.slice()).map_err(|err| {
        let span = lex.span();
        err.with_span(Span::new(span.start, span.end))
    })
}

impl fmt::Display for Token {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        use Token::*;
        match self {
            Space => write!(f, " "),
            Tab => write!(f, "\t"),
            Amp => write!(f, "&"),
            DoubleColon => write!(f, "::"),
            LBrace => write!(f, "{{"),
            RBrace => write!(f, "}}"),
            LParen => write!(f, "("),
            RParen => write!(f, ")"),
            LBracket => write!(f, "["),
            RBracket => write!(f, "]"),
            Semi => write!(f, ";"),
            Colon => write!(f, ":"),
            Comma => write!(f, ","),
            QMark => write!(f, "?"),
            Eq => write!(f, "="),
            Pipe => write!(f, "|"),
            Hash => write!(f, "#"),
            Bang => write!(f, "!"),
            KwNamespace => write!(f, "namespace"),
            KwUse => write!(f, "use"),
            KwStruct => write!(f, "struct"),
            KwEnum => write!(f, "enum"),
            KwType => write!(f, "type"),
            KwOneof => write!(f, "oneof"),
            KwError => write!(f, "error"),
            KwOperation => write!(f, "operation"),
            KwBool => write!(f, "bool"),
            KwNull => write!(f, "null"),
            KwStr => write!(f, "str"),
            KwI8 => write!(f, "i8"),
            KwI16 => write!(f, "i16"),
            KwI32 => write!(f, "i32"),
            KwI64 => write!(f, "i64"),
            KwU8 => write!(f, "u8"),
            KwU16 => write!(f, "u16"),
            KwU32 => write!(f, "u32"),
            KwU64 => write!(f, "u64"),
            KwUsize => write!(f, "usize"),
            KwF16 => write!(f, "f16"),
            KwF32 => write!(f, "f32"),
            KwF64 => write!(f, "f64"),
            KwComplex => write!(f, "complex"),
            KwBinary => write!(f, "binary"),
            KwBase64 => write!(f, "base64"),
            KwDateTime => write!(f, "datetime"),
            KwNever => write!(f, "never"),
            KwSchema => write!(f, "schema"),
            Newline => writeln!(f),
            Return => write!(f, "->"),
            Ident(s) => write!(f, "{}", s),
            Path(p) => write!(f, "{}", p),
            Number(n) => write!(f, "{}", n),
            String(s) => write!(f, "\"{}\"", s),
            CommentSingleLine(s) => write!(f, "// {}", s),
            CommentMultiLine(s) => {
                if s.contains('\n') {
                    write!(f, "/*\n{}\n*/", s)
                } else {
                    write!(f, "/* {} */", s)
                }
            },
        }
    }
}

impl fmt::Debug for Token {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::Ident(s) => write!(f, "Ident({s})"),
            Self::Number(s) => write!(f, "Number({s})"),
            Self::String(s) => write!(f, "String({s})"),
            Self::Newline => write!(f, "\\n"),
            rest => write!(f, "{rest}"),
        }
    }
}

fn parse_string(lex: &mut logos::Lexer<'_, Token>) -> Option<String> {
    let slice = lex.slice();
    let inner = &slice[1..slice.len() - 1];
    Some(unescape(inner))
}

fn unescape(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                match n {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    '\\' => out.push('\\'),
                    '"' => out.push('"'),
                    '\'' => out.push('\''),
                    other => {
                        out.push(other);
                    },
                }
            }
        } else {
            out.push(c);
        }
    }
    out.trim().to_string()
}

fn unescape_comment(src: &str) -> String {
    src.lines()
        .map(|ln| ln.trim_start())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_start()
        .trim_end()
        .to_string()
}

#[macro_export]
macro_rules! Token {
    [->] => { $crate::tokens::toks::ReturnToken };
    [&] => { $crate::tokens::toks::AmpToken };
    [::] => { $crate::tokens::toks::DoubleColonToken };
    [;] => { $crate::tokens::toks::SemiToken };
    [:] => { $crate::tokens::toks::ColonToken };
    [,] => { $crate::tokens::toks::CommaToken };
    [?] => { $crate::tokens::toks::QMarkToken };
    [=] => { $crate::tokens::toks::EqToken };
    [|] => { $crate::tokens::toks::PipeToken };
    [#] => { $crate::tokens::toks::HashToken };
    [!] => { $crate::tokens::toks::BangToken };
    [namespace] => { $crate::tokens::toks::KwNamespaceToken };
    [use] => { $crate::tokens::toks::KwUseToken };
    [struct] => { $crate::tokens::toks::KwStructToken };
    [enum] => { $crate::tokens::toks::KwEnumToken };
    [type] => { $crate::tokens::toks::KwTypeToken };
    [oneof] => { $crate::tokens::toks::KwOneofToken };
    [error] => { $crate::tokens::toks::KwErrorToken };
    [operation] => { $crate::tokens::toks::KwOperationToken };
    [schema] => { $crate::tokens::toks::KwSchemaToken };
    [bool] => { $crate::tokens::toks::KwBoolToken };
    [null] => { $crate::tokens::toks::KwNullToken };
    [str] => { $crate::tokens::toks::KwStrToken };
    [i8] => { $crate::tokens::toks::KwI8Token };
    [i16] => { $crate::tokens::toks::KwI16Token };
    [i32] => { $crate::tokens::toks::KwI32Token };
    [i64] => { $crate::tokens::toks::KwI64Token };
    [u8] => { $crate::tokens::toks::KwU8Token };
    [u16] => { $crate::tokens::toks::KwU16Token };
    [u32] => { $crate::tokens::toks::KwU32Token };
    [u64] => { $crate::tokens::toks::KwU64Token };
    [f16] => { $crate::tokens::toks::KwF16Token };
    [f32] => { $crate::tokens::toks::KwF32Token };
    [f64] => { $crate::tokens::toks::KwF64Token };
    [binary] => { $crate::tokens::toks::KwBinaryToken };
    [base64] => { $crate::tokens::toks::KwBase64Token };
    [datetime] => { $crate::tokens::toks::KwDateTimeToken };
    [complex] => { $crate::tokens::toks::KwComplexToken };
    [never] => { $crate::tokens::toks::KwNeverToken };
    [newline] => { $crate::tokens::toks::NewlineToken };
    [ident] => { $crate::tokens::toks::IdentToken };
    [path] => { $crate::tokens::toks::PathToken };
    [number] => { $crate::tokens::toks::NumberToken };
    [string] => { $crate::tokens::toks::StringToken };
    [comment_single_line] => { $crate::tokens::toks::CommentSingleLineToken };
    [comment_multi_line] => { $crate::tokens::toks::CommentMultiLineToken };
}

#[macro_export]
macro_rules! SpannedToken {
    ($met: tt) => {
        $crate::defs::Spanned::<$crate::Token![$met]>
    };
}

macro_rules! straight_through {
    ($ty: ident $(<$g:ident>)? {$($field: ident), + $(,)?}) => {
        impl$(<$g: crate::Parse + crate::tokens::ToTokens>)? crate::tokens::ToTokens for $ty $(<$g>)? {
            fn write(&self, tt: &mut crate::fmt::Printer) {
                $(
                    tt.write(&self.$field);
                )*
            }
        }
    };
}

pub(crate) use straight_through;

macro_rules! syntax_desc {
    (
        {
            keywords: [$($kwd:ident = $kdesc: literal), + $(,)?],
            builtin: [$($t:ident = $bdesc: literal), + $(,)?],
            tokens: [$($tok:tt = $tdesc: literal), + $(,)?]
        }
    ) => {
        #[cfg(feature = "emit")]
        pub(crate) fn descs() -> serde_json::Value {
            paste::paste!(
                serde_json::json!({
                    "keywords": [
                        $(
                            {
                                "token": format!("{}", Token::[<Kw $kwd:camel>]),
                                "description": $kdesc
                            },
                        )*
                    ],
                    "builtin": [
                        $(
                            {
                                "token": format!("{}", Token::[<Kw $t:camel>]),
                                "description": $bdesc
                            },
                        )*
                    ],
                    "tokens": [
                        $(
                            {
                                "token": stringify!($tok),
                                "description": $tdesc
                            },
                        )*
                    ],
                })
            )
        }
    };
}

syntax_desc! {{
    keywords: [
        schema = "keyword `schema`. used to reference types within the same package.",
        namespace = "keyword `namespace`. should precede an identifier.",
        use = "keyword `use`. should precede a namespace to be used.",
        struct = "keyword `struct`. used to declare a struct.",
        enum = "keyword `enum`. used to declare an enumeration.",
        type = "keyword `type`. used to declare a type alias.",
        oneof = "keyword `oneof`. used to declare a sequence of type variants or named enumeration of types.",
        error = "keyword `error`. used to declare an error type.",
        operation = "keyword `operation`. used to declare an operation."
    ],
    builtin: [
        bool = "a boolean type (true | false)",
        str = "a string type",
        i8 = "signed 8-bit integer",
        i16 = "signed 16-bit integer",
        i32 = "signed 32-bit integer",
        i64 = "signed 64-bit integer",
        u8 = "unsigned 8-bit integer",
        u16 = "unsigned 16-bit integer",
        u32 = "unsigned 32-bit integer",
        u64 = "unsigned 64-bit integer",
        f16 = "a signed 16-bit floating point number",
        f32 = "a signed 32-bit floating point number",
        f64 = "a signed 64-bit floating point number",
        complex = "a complex number with real and imaginary parts.",
        DateTime = "a [iso 8601](https://en.wikipedia.org/wiki/ISO_8601) compliant datetime providing timezone.",
        never = "a unit type (0 size)",
        Binary = "a binary stream. this is distinct from u8[], where we may have language specific types to utilize if you intend to manipulate octal streams."
    ],
    tokens: [
        [] = "brackets are paired between spans. brackets are permitted in array types, meta fields, and spanned namespace declarations.",
        {} = "braces are paired between spans. braces are permitted in: named structs, anonymous structs, enums, oneofs, and errors",
        () = "parentheses are paired between spans. parentheses are permitted in: meta fields, types, operations, and errors",
        & = "amp tokens are supported in union types to separate type variants.",
        :: = "scope resolution operators are used to access named declarations of external namespaces, with no whitespace, and no trailing operator.",
        ; = "semicolons are used to terminate a top-level declaration (item).",
        : = "colons are used to separate a field from its type in arguments. there should be no proceeding whitespace between the proceeding `ident`, with a following space before the subsequent type.",
        , = "commas are used to separate fields, enum and error variants, and arguments. trailing commas are permitted.",
        ? = "used to indicate an optional type.",
        = = "equals is used to declare a named type, or provide a static value to an enum member.",
        # = "pound tokens are used in meta. e.g. `#[...]`",
        ! = "bang tokens are used to set meta as inner meta, or declare a return type may raise an error. e.g. `-> i32!`."
    ]
}}

#[cfg(feature = "emit")]
pub fn emit_syntax(p: impl AsRef<std::path::Path>) {
    let mut desc = descs();
    let m = desc.as_object_mut().unwrap();
    let tt = m
        .get_mut("tokens")
        .unwrap()
        .as_array_mut()
        .unwrap();

    tt.push(serde_json::json!({
        "token": "\\|",
        "description": "pipe tokens are supported in oneof types to separate type variants."
    }));
    tt.push(serde_json::json!({
        "token": "//",
        "description": "used to start a single-line comment, terminated by a new line."
    }));
    tt.push(serde_json::json!({
        "token": "/*",
        "description": "used to start a multi-line comment, terminated by `*/`"
    }));
    tt.push(serde_json::json!({
        "token": "*/",
        "description": "used to end a multi-line comment"
    }));

    let out = serde_json::to_string(&desc).unwrap();

    std::fs::write(p, out).unwrap()
}
