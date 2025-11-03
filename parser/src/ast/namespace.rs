use crate::{
    SpannedToken, Token,
    ast::AstStream,
    bail_unchecked,
    defs::Spanned,
    tokens::{self, Brace, LBraceToken, Parse, Peek, ToTokens, brace},
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Namespace {
    pub kw: SpannedToken![namespace],
    pub name: SpannedToken![ident],
}

impl Parse for Namespace {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        Ok(Self {
            kw: stream.parse()?,
            name: stream.parse()?,
        })
    }
}

impl Peek for Namespace {
    fn peek(stream: &tokens::TokenStream) -> bool {
        stream.peek::<Token![namespace]>() && !stream.peek::<SpannedNamespace>()
    }
}

impl ToTokens for Namespace {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.kw);
        tt.space();
        tt.write(&self.name);
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SpannedNamespace {
    pub kw: SpannedToken![namespace],
    pub name: SpannedToken![ident],
    pub brace: Brace,
    pub ast: Spanned<AstStream>,
}

impl Peek for SpannedNamespace {
    fn peek(stream: &tokens::TokenStream) -> bool {
        let mut fork = stream.fork();

        let _: SpannedToken![namespace] = bail_unchecked!(fork.parse(); false);
        let _: SpannedToken![ident] = bail_unchecked!(fork.parse(); false);

        fork.peek::<LBraceToken>()
    }
}

impl Parse for SpannedNamespace {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        let mut braced;
        Ok(Self {
            kw: stream.parse()?,
            name: stream.parse()?,
            brace: brace!(braced in stream),
            ast: braced.parse()?,
        })
    }
}

impl ToTokens for SpannedNamespace {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        tt.write(&self.kw);
        tt.space();
        tt.write(&self.name);
        tt.space();
        tt.open_block();
        tt.write(&self.ast);
        tt.close_block();
    }
}
