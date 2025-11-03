use crate::{
    SpannedToken, Token,
    ast::{anonymous::AnonymousStruct, comment::CommentStream, ty::Type},
    defs::Spanned,
    tokens::{ImplDiagnostic, Paren, Parse, Peek, ToTokens, paren, toks},
};

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum Variant {
    Tuple {
        comments: CommentStream,
        name: SpannedToken![ident],
        paren: Paren,
        inner: Type,
    },
    LocalStruct {
        comments: CommentStream,
        name: SpannedToken![ident],
        inner: Spanned<AnonymousStruct>,
    },
}

impl ImplDiagnostic for Variant {
    fn fmt() -> &'static str {
        "a(i32) | b { desc: str }"
    }
}

impl Parse for Variant {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        let comments = CommentStream::parse(stream)?;
        let name = stream.parse()?;

        let mut inner;

        Ok(if stream.peek::<toks::LBraceToken>() {
            Self::LocalStruct {
                comments,
                name,
                inner: stream.parse()?,
            }
        } else {
            let paren = paren!(inner in stream);
            let inner = Type::parse(&mut inner)?;
            Self::Tuple {
                comments,
                name,
                paren,
                inner,
            }
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
    #[test_case::test_case("a(i32)"; "type variant")]
    #[test_case::test_case("b {\n\tdesc: str\n}"; "type anonymous struct")]
    #[test_case::test_case("/* some comment */\nb {\n\tdesc: str\n}"; "type anonymous struct with comment before")]
    #[test_case::test_case("b {\n\t// some comment\n\tdesc: str\n}"; "type anonymous struct with sl comment in fields")]
    #[test_case::test_case("b {\n\t/*\n\t\tsome\n\t\tcomment\n\t*/\n\tdesc: str\n}"; "type anonymous struct with ml comment in fields")]
    fn round_trip(src: &str) {
        crate::tst::round_trip::<super::Variant>(src).unwrap();
    }
}
