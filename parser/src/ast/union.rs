use crate::{
    SpannedToken,
    ast::{anonymous::AnonymousStruct, ty::PathOrIdent},
    bail_unchecked,
    defs::Spanned,
    tokens::{self, ImplDiagnostic, Paren, Parse, Peek, Repeated, ToTokens, paren},
};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum UnionDiscriminant {
    Ref(PathOrIdent),
    Anonymous(AnonymousStruct),
}

impl Peek for UnionDiscriminant {
    fn peek(stream: &tokens::TokenStream) -> bool {
        stream.peek::<PathOrIdent>() || stream.peek::<AnonymousStruct>()
    }
}

impl Parse for UnionDiscriminant {
    fn parse(stream: &mut tokens::TokenStream) -> Result<Self, tokens::LexingError> {
        Ok(if stream.peek::<AnonymousStruct>() {
            Self::Anonymous(AnonymousStruct::parse(stream)?)
        } else {
            Self::Ref(PathOrIdent::parse(stream)?)
        })
    }
}

impl ToTokens for UnionDiscriminant {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            Self::Anonymous(anon) => anon.write(tt),
            Self::Ref(ty) => ty.write(tt),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum IdentOrUnion {
    Ident(UnionDiscriminant),
    Union {
        paren: Paren,
        inner: Spanned<Box<Union>>,
    },
}

impl ToTokens for IdentOrUnion {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            Self::Ident(iden) => iden.write(tt),
            Self::Union { inner, paren } => paren.write_with(tt, |tt| tt.write(inner)),
        }
    }
}

impl ImplDiagnostic for IdentOrUnion {
    fn fmt() -> &'static str {
        "identifier or parenthesized union"
    }
}

impl Peek for IdentOrUnion {
    fn peek(stream: &crate::tokens::TokenStream) -> bool {
        stream.peek::<tokens::IdentToken>()
            || stream.peek::<tokens::LParenToken>()
            || stream.peek::<tokens::LBraceToken>()
    }
}

impl Parse for IdentOrUnion {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        use crate::tokens::error::LexingError;
        if stream.peek::<tokens::LParenToken>() {
            let mut inner;
            let paren = paren!(inner in stream);
            let inner_union: Union = Union::parse(&mut inner)?;
            let (start, end) = if let (Some(first), Some(last)) = (
                inner_union.types.values.first(),
                inner_union.types.values.last(),
            ) {
                (first.value.span().start, last.value.span().end)
            } else {
                (stream.cursor(), stream.cursor())
            };
            return Ok(IdentOrUnion::Union {
                paren,
                inner: Spanned::new(start, end, Box::new(inner_union)),
            });
        }
        if stream.peek::<UnionDiscriminant>() {
            return Ok(Self::Ident(UnionDiscriminant::parse(stream)?));
        }
        Err(if let Some(next) = stream.peek_unchecked() {
            LexingError::expected_oneof(
                vec![
                    <tokens::IdentToken as ImplDiagnostic>::fmt(),
                    tokens::LParenToken::fmt(),
                ],
                next.value.clone(),
            )
        } else {
            LexingError::empty_oneof(vec![<tokens::IdentToken as ImplDiagnostic>::fmt(), "("])
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Union {
    /// preserves order (left to right)
    pub types: Repeated<IdentOrUnion, tokens::AmpToken>,
}

impl ImplDiagnostic for Union {
    fn fmt() -> &'static str {
        "a & b"
    }
}

impl Parse for Union {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        Ok(Self {
            types: Repeated::parse(stream)?,
        })
    }
}

impl Peek for Union {
    fn peek(stream: &tokens::TokenStream) -> bool {
        let mut fork = stream.fork();
        let _: IdentOrUnion = bail_unchecked!(IdentOrUnion::parse(&mut fork); false);
        let _: SpannedToken![&] = bail_unchecked!(fork.parse(); false);
        true
    }
}

impl ToTokens for Union {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        let len = self.types.values.len();
        for (i, item) in self.types.values.iter().enumerate() {
            item.value.write(tt);
            if item.sep.is_some() && i < len - 1 {
                tt.space();
                item.sep.as_ref().unwrap().write(tt);
                tt.space();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Union;

    #[test_case::test_case("some_struct & (other_struct & last_struct)"; "inner parenthesized")]
    #[test_case::test_case("some_struct & other_struct & last_struct"; "no paren triplets")]
    #[test_case::test_case("some_struct & other_struct & {\n\ta: i32\n}"; "triplets with anonymous")]
    fn round_trip(src: &str) {
        crate::tst::round_trip::<Union>(src).unwrap();
    }
}
