use crate::{
    Token,
    tokens::{Brace, EqToken, IdentToken, ToTokens},
};

use crate::{
    SpannedToken,
    ast::comment::CommentStream,
    defs::Spanned,
    tokens::{ImplDiagnostic, Parse, Peek, Repeated, brace},
};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct EnumValue<Value: Parse> {
    pub eq: Spanned<EqToken>,
    pub value: Spanned<Value>,
}

impl<Value: Parse> EnumValue<Value> {
    /// Get the inner value
    pub fn inner(&self) -> &Value {
        &self.value.value
    }
}

impl<Value: Parse> Peek for EnumValue<Value> {
    fn is(token: &crate::tokens::toks::Token) -> bool {
        <Token![=]>::is(token)
    }
}

impl<Value: Parse> Parse for EnumValue<Value> {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        Ok(Self {
            eq: stream.parse()?,
            value: stream.parse()?,
        })
    }
}

impl<V: Parse> ToTokens for EnumValue<V>
where
    Spanned<V>: ToTokens,
{
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.eq);
        tt.write(&self.value);
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct EnumVariant<Value: Parse> {
    pub comments: Spanned<CommentStream>,
    pub name: Spanned<IdentToken>,
    pub value: Option<EnumValue<Value>>,
}

impl<Value: Parse> EnumVariant<Value> {
    /// Get the variant name as a string
    pub fn name(&self) -> &str {
        self.name.borrow_string()
    }

    /// Get the enum value if present
    pub fn enum_value(&self) -> Option<&EnumValue<Value>> {
        self.value.as_ref()
    }
}

impl<Value: Parse + Peek> Peek for EnumVariant<Value> {
    fn is(token: &crate::tokens::toks::Token) -> bool {
        <Token![ident]>::is(token)
    }
}

impl<Value: Parse + Peek> Parse for EnumVariant<Value> {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        Ok(Self {
            comments: stream.parse()?,
            name: stream.parse()?,
            value: Option::parse(stream)?,
        })
    }
}

impl<Value: Parse + Peek> ImplDiagnostic for EnumVariant<Value> {
    fn fmt() -> &'static str {
        "enum variant"
    }
}

impl<V: Parse + Peek> ToTokens for EnumVariant<V>
where
    Spanned<V>: ToTokens,
{
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.comments);
        tt.write(&self.name);
        if let Some(val) = &self.value {
            tt.space();
            tt.write(&val.eq);
            tt.space();
            tt.write(&val.value);
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TypedEnum<Value: Parse + Peek> {
    pub kw: SpannedToken![enum],
    pub name: SpannedToken![ident],
    pub brace: Brace,
    pub variants: Spanned<Repeated<EnumVariant<Value>, Token![,]>>,
}

impl<Value: Parse + Peek + ImplDiagnostic + Clone> Clone for TypedEnum<Value> {
    fn clone(&self) -> Self {
        Self {
            kw: self.kw.clone(),
            name: self.name.clone(),
            brace: self.brace.clone(),
            variants: self.variants.clone(),
        }
    }
}

impl<Value: Parse + Peek + ImplDiagnostic> Parse for TypedEnum<Value> {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        let mut brace;
        Ok(Self {
            kw: stream.parse()?,
            name: stream.parse()?,
            brace: brace!(brace in stream),
            variants: brace.parse()?,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Enum {
    Int(TypedEnum<Token![number]>),
    Str(TypedEnum<Token![string]>),
}

impl Clone for Enum {
    fn clone(&self) -> Self {
        match self {
            Self::Int(i) => Self::Int(i.clone()),
            Self::Str(s) => Self::Str(s.clone()),
        }
    }
}

impl Peek for Enum {
    fn is(token: &crate::tokens::toks::Token) -> bool {
        <Token![enum]>::is(token)
    }
}

impl Parse for Enum {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        let mut f1 = stream.fork();

        let _: SpannedToken![enum] = f1.parse()?;
        let _: SpannedToken![ident] = f1.parse()?;

        let mut brace;
        brace!(
            brace in f1
        );

        Ok(
            if EnumVariant::<Token![number]>::parse(&mut brace).is_ok() {
                Self::Int(TypedEnum::parse(stream)?)
            } else {
                Self::Str(TypedEnum::parse(stream)?)
            },
        )
    }
}

impl ToTokens for Enum {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            Self::Int(i) => {
                tt.write(&i.kw);
                tt.space();
                tt.write(&i.name);
                tt.space();
                tt.open_block();
                tt.write_comma_separated(
                    i.variants
                        .values
                        .iter()
                        .map(|item| &item.value),
                );
                tt.close_block();
            },
            Self::Str(s) => {
                tt.write(&s.kw);
                tt.space();
                tt.write(&s.name);
                tt.space();
                tt.open_block();
                tt.write_comma_separated(
                    s.variants
                        .values
                        .iter()
                        .map(|item| &item.value),
                );
                tt.close_block();
            },
        }
    }
}

impl Enum {
    pub fn name(&self) -> &SpannedToken![ident] {
        match self {
            Self::Int(i) => &i.name,
            Self::Str(s) => &s.name,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tokens::tokenize;

    use super::*;

    #[test_case::test_case(
    "enum Foo {\n\tBar,\n\tBaz = 2\n}", |it| {
        assert!(matches!(it, Enum::Int(..)))
    };
    "parses int enum with mixed eq and default values"
)]
    #[test_case::test_case(
    "enum Foo {\n\tBar = \"a\",\n\tBaz = \"b\"\n}", |it| {
        assert!(matches!(it, Enum::Str(..)))
    };
    "parses str enum"
)]
    #[test_case::test_case(
    "enum Abc {\n\tBar = 1\n}", |it| {
        assert!(matches!(it, Enum::Int(..)))
    };
    "parses enum variant with int value"
)]
    #[test_case::test_case(
    "enum Abc {\n\tBar = \"a\"\n}", |it| {
        assert!(matches!(it, Enum::Str(..)))
    };
    "parses enum variant with str value"
)]
    fn test_enum_variant_parse_str(
        input: &str,
        matches: impl Fn(Enum),
    ) {
        let mut stream = tokenize(input).unwrap();
        let result = Enum::parse(&mut stream).unwrap();

        matches(result);

        crate::tst::round_trip::<Enum>(input).unwrap();
    }
}
