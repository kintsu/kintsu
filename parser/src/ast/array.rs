use crate::{
    SpannedToken, Token,
    ast::ty::Type,
    defs::Spanned,
    tokens::{Bracket, ImplDiagnostic, Parse, Peek, ToTokens, Token, bracket, toks},
};

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Array {
    Unsized {
        ty: Box<Spanned<Type>>,
        bracket: Bracket,
    },
    Sized {
        ty: Box<Spanned<Type>>,
        bracket: Bracket,
        size: SpannedToken![number],
    },
}

impl Array {
    pub fn type_name(&self) -> String {
        match self {
            Self::Unsized { ty, .. } => {
                format!("{}[]", ty.type_name())
            },
            Self::Sized { ty, size, .. } => {
                format!("{}[{}]", ty.type_name(), size.borrow_i32())
            },
        }
    }
}

impl ImplDiagnostic for Array {
    fn fmt() -> &'static str {
        "i32[] | i32[4]"
    }
}

impl Parse for Array {
    fn parse(stream: &mut crate::tokens::TokenStream) -> Result<Self, crate::tokens::LexingError> {
        tracing::trace!(cursor=%stream.cursor(), "parsing array");
        let ty = Box::new(stream.parse()?);
        let mut inner;
        let bracket = bracket!(inner in stream);
        Ok(if inner.peek::<Token![number]>() {
            tracing::trace!("parsing sized array");
            let size = inner.parse()?;
            Self::Sized { ty, bracket, size }
        } else {
            tracing::trace!("parsing unsized array");
            Self::Unsized { ty, bracket }
        })
    }
}

impl Peek for Array {
    fn peek(stream: &crate::tokens::TokenStream) -> bool {
        let mut fork = stream.fork();
        if fork.parse::<Spanned<Type>>().is_err() {
            return false;
        };
        fork.peek::<toks::LBraceToken>()
    }
}

impl ToTokens for Array {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            Self::Unsized { ty, bracket: _ } => {
                tt.write(ty);
                tt.token(&Token::LBracket);
                tt.token(&Token::RBracket);
            },
            Self::Sized {
                ty,
                size,
                bracket: _,
            } => {
                tt.write(ty);
                tt.token(&Token::LBracket);
                tt.write(size);
                tt.token(&Token::RBracket);
            },
        }
    }
}
