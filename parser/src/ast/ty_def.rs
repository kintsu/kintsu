use crate::{
    SpannedToken, Token,
    ast::ty::Type,
    defs::Spanned,
    tokens::{Parse, Peek},
};

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct NamedType {
    pub kw: SpannedToken![type],
    pub name: SpannedToken![ident],
    pub eq: SpannedToken![=],
    pub ty: Spanned<Type>,
}

impl Parse for NamedType {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        Ok(Self {
            kw: stream.parse()?,
            name: stream.parse()?,
            eq: stream.parse()?,
            ty: stream.parse()?,
        })
    }
}

impl Peek for NamedType {
    fn is(token: &crate::tokens::Token) -> bool {
        <Token![type]>::is(token)
    }
}

impl crate::tokens::ToTokens for NamedType {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.kw);
        tt.space();
        tt.write(&self.name);
        tt.space();
        tt.write(&self.eq);
        tt.space();
        tt.write(&self.ty);
    }
}
