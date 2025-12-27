use crate::{
    SpannedToken, Token,
    ast::{anonymous::AnonymousStruct, comment::CommentStream, ty::Type},
    defs::Spanned,
    tokens::{ImplDiagnostic, Paren, Parse, Peek, ToTokens, paren, toks},
};

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum Variant {
    /// Unit variant with no payload - e.g., `Unknown`
    Unit {
        comments: CommentStream,
        name: SpannedToken![ident],
    },
    /// Tuple variant with a type reference - e.g., `NotFound(ResourceId)`
    Tuple {
        comments: CommentStream,
        name: SpannedToken![ident],
        paren: Paren,
        inner: Type,
    },
    /// Local struct variant with inline fields - e.g., `NotFound { message: str }`
    LocalStruct {
        comments: CommentStream,
        name: SpannedToken![ident],
        inner: Spanned<AnonymousStruct>,
    },
}

impl ImplDiagnostic for Variant {
    fn fmt() -> &'static str {
        "a(i32) | b { desc: str } | c"
    }
}

impl Parse for Variant {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        let comments = CommentStream::parse(stream)?;
        let name = stream.parse()?;

        Ok(if stream.peek::<toks::LBraceToken>() {
            // { fields } - LocalStruct
            Self::LocalStruct {
                comments,
                name,
                inner: stream.parse()?,
            }
        } else if stream.peek::<toks::LParenToken>() {
            // (Type) - Tuple
            let mut inner;
            let paren = paren!(inner in stream);
            let inner = Type::parse(&mut inner)?;
            Self::Tuple {
                comments,
                name,
                paren,
                inner,
            }
        } else {
            // Nothing follows - Unit variant
            Self::Unit { comments, name }
        })
    }
}

impl Peek for Variant {
    fn is(token: &toks::Token) -> bool {
        <Token![ident]>::is(token)
    }
}

impl ToTokens for Variant {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            Self::Unit { comments, name } => {
                tt.write(comments);
                tt.write(name);
            },
            Self::LocalStruct {
                comments,
                name,
                inner,
            } => {
                tt.write(comments);
                tt.write(name);
                tt.space();
                tt.write(inner);
            },
            Self::Tuple {
                comments,
                name,
                inner,
                paren,
            } => {
                tt.write(comments);
                tt.write(name);
                paren.write_with(tt, |tt| tt.write(inner))
            },
        }
    }
}

macro_rules! variadic {
    ($name: ident: [$kw: ty]) => {
        #[derive(serde::Deserialize, serde::Serialize, Clone)]
        pub struct $name {
            pub kw: $kw,
            pub name: crate::SpannedToken![ident],
            pub brace: crate::tokens::Brace,
            pub variants: crate::tokens::Repeated<crate::ast::variadic::Variant, crate::Token![,]>,
        }


        impl crate::tokens::Parse for $name {
            fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
                let mut inner;
                Ok(Self {
                    kw: stream.parse()?,
                    name: stream.parse()?,
                    brace: crate::tokens::brace!(inner in stream),
                    variants: crate::tokens::Repeated::parse(&mut inner)?,
                })
            }
        }

        impl crate::tokens::Peek for $name {
            fn is(token: &crate::tokens::toks::Token) -> bool {
                <$kw>::is(token)
            }
        }

        impl crate::tokens::ToTokens for $name {
            fn write(&self, tt: &mut crate::fmt::Printer) {
                tt.write(&self.kw);
                tt.space();
                tt.write(&self.name);
                tt.space();
                tt.open_block();

                tt.write_comma_separated(self.variants.values.iter().map(|item| &item.value));

                tt.close_block();
            }
        }
    };
}

pub(crate) use variadic;

#[cfg(test)]
mod test {
    #[test_case::test_case("a(i32)"; "tuple variant")]
    #[test_case::test_case("Unknown"; "unit variant")]
    #[test_case::test_case("b {\n\tdesc: str\n}"; "local struct variant")]
    #[test_case::test_case("/* some comment */\nb {\n\tdesc: str\n}"; "local struct with comment before")]
    #[test_case::test_case("b {\n\t// some comment\n\tdesc: str\n}"; "local struct with sl comment in fields")]
    #[test_case::test_case("b {\n\t/*\n\t\tsome\n\t\tcomment\n\t*/\n\tdesc: str\n}"; "local struct with ml comment in fields")]
    #[test_case::test_case("/* unit with comment */\nUnknown"; "unit variant with comment")]
    fn round_trip(src: &str) {
        crate::tst::round_trip::<super::Variant>(src).unwrap();
    }
}
