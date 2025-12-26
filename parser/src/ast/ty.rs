use std::collections::BTreeSet;

use crate::{
    SpannedToken, Token,
    ast::{anonymous::AnonymousStruct, array::Array, one_of::AnonymousOneOf, union::Union},
    ctx::{NamedItemContext, RefOrItemContext},
    defs::Spanned,
    tokens::*,
};

macro_rules! builtin {
    ($($t: ident), + $(,)?) => {
        paste::paste!{
            #[derive(serde::Serialize, serde::Deserialize, Clone)]
            #[serde(tag = "type", rename_all = "snake_case")]
            pub enum Builtin {
                $(
                    $t(crate::defs::Spanned<crate::tokens::toks::[<Kw $t Token>]>),
                )*
            }

            impl Peek for Builtin {
                fn is(token: &toks::Token) -> bool {
                    false  $(
                       || crate::tokens::toks::[<Kw $t Token>]::is(token)
                    )*
                }
            }

            impl ImplDiagnostic for Builtin {
                fn fmt() -> &'static str {
                    "builtin (i16, i32, str, ...)"
                }
            }

            impl ToTokens for Builtin {
                fn write(&self, tt: &mut crate::fmt::Printer) {
                    match self {
                        $(
                            Self::$t(t) => tt.write(t),
                        )*
                    };
                }
            }


            impl Parse for Builtin {
                fn parse(stream: &mut TokenStream) -> AstResult<Self> {
                    $(
                        if stream.peek::<crate::tokens::toks::[<Kw $t Token>]>() {
                            return Ok(Self::$t(
                                stream.parse()?
                            ))
                        }
                    )*

                    let tys: Vec<_> = vec![
                        $(
                            crate::tokens::toks::[<Kw $t Token>]::fmt(),
                        )*
                    ];

                    let next = stream.next().ok_or_else(
                        || LexingError::empty_oneof(tys.clone())
                    )?;
                    Err(
                        LexingError::expected_oneof(
                            tys.clone(), next.value
                        )
                    )
                }


            }



        }


    };
}

builtin! {
    I8,
    I16,
    I32,
    I64,

    U8,
    U16,
    U32,
    U64,

    Usize,

    F16,
    F32,
    F64,

    Bool,
    Str,

    DateTime,
    Complex,
    Binary,
    Base64,

    Never
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(untagged)]
pub enum PathOrIdent {
    Path(SpannedToken![path]),
    Ident(SpannedToken![ident]),
}

impl std::fmt::Display for PathOrIdent {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Path(p) => write!(f, "{}", p.value.borrow_path_inner()),
            Self::Ident(i) => write!(f, "{}", i.value.borrow_string()),
        }
    }
}

impl TriesQualify for SpannedToken![ident] {
    fn qualify_one(
        &self,
        context: &RefOrItemContext,
        potential: &mut BTreeSet<NamedItemContext>,
    ) {
        match context {
            RefOrItemContext::Ref(ctx) => {
                potential.insert(ctx.item(self.clone()));
            },
            RefOrItemContext::Item(item) => {
                potential.insert(item.clone());
            },
        }
    }
}

pub trait TriesQualify {
    fn qualify_one(
        &self,
        context: &RefOrItemContext,
        potential: &mut BTreeSet<NamedItemContext>,
    );

    fn qualified_in_context(
        &self,
        context: &[RefOrItemContext],
    ) -> BTreeSet<NamedItemContext> {
        let mut qualified = BTreeSet::new();
        for ctx in context {
            match ctx {
                RefOrItemContext::Ref(..) => {
                    self.qualify_one(ctx, &mut qualified);
                },
                RefOrItemContext::Item(qual) => {
                    qualified.insert(qual.clone());
                },
            }
        }

        qualified
    }
}

impl TriesQualify for PathOrIdent {
    fn qualify_one(
        &self,
        context: &RefOrItemContext,
        potential: &mut BTreeSet<NamedItemContext>,
    ) {
        match self {
            Self::Ident(id) => {
                id.qualify_one(context, potential);
            },
            Self::Path(p) => {
                p.borrow_path_inner()
                    .qualify_one(context, potential);
            },
        }
    }
}

impl Peek for PathOrIdent {
    fn is(token: &Token) -> bool {
        <Token![path]>::is(token) || <Token![ident]>::is(token)
    }
}

impl Parse for PathOrIdent {
    fn parse(stream: &mut TokenStream) -> Result<Self, LexingError> {
        Ok(if stream.peek::<Token![path]>() {
            Self::Path(stream.parse()?)
        } else if stream.peek::<Token![ident]>() {
            Self::Ident(stream.parse()?)
        } else {
            let last = stream
                .last()
                .map(|it| it.span.clone())
                .unwrap_or(crate::defs::Span::new(0, 0));
            return Err(LexingError::one_of(
                stream,
                vec![<Token![path]>::fmt(), <Token![ident]>::fmt()],
                &last,
            ));
        })
    }
}

impl ToTokens for PathOrIdent {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            Self::Ident(t) => t.write(tt),
            Self::Path(t) => t.write(tt),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    Builtin {
        ty: Spanned<Builtin>,
    },
    Ident {
        to: PathOrIdent,
    },
    OneOf {
        ty: Spanned<AnonymousOneOf>,
    },
    Array {
        ty: Spanned<Array>,
    },
    Paren {
        paren: Paren,
        ty: Spanned<Box<Type>>,
    },
    Union {
        ty: Spanned<Union>,
    },
    UnionOr {
        lhs: Spanned<Box<Type>>,
        op: SpannedToken![union_or],
        rhs: Spanned<Box<Type>>,
    },
    Struct {
        ty: Spanned<AnonymousStruct>,
    },
    Result {
        ty: Spanned<Box<Type>>,
        ex: SpannedToken![!],
    },
    /// Type expression (Pick, Omit, Partial, Required, Exclude, Extract, ArrayItem)
    /// per RFC-0018. Resolves to concrete types during Phase 3.6.
    TypeExpr {
        expr: Spanned<super::type_expr::TypeExpr>,
    },
}

impl Type {
    pub fn type_name(&self) -> String {
        match self {
            Self::Builtin { ty } => ty.display(),
            Self::Ident { .. } => "reference".into(),
            Self::OneOf { .. } => "oneof".into(),
            Self::Paren { ty, .. } => format!("({})", ty.type_name()),
            Self::Result { ty, .. } => format!("{}!", ty.type_name()),
            Self::Struct { .. } => "anon struct".into(),
            Self::Union { .. } => "union".into(),
            Self::UnionOr { .. } => "union or".into(),
            Self::Array { ty } => ty.type_name(),
            Self::TypeExpr { .. } => "type expr".into(),
        }
    }
}

impl Parse for Type {
    fn parse(stream: &mut TokenStream) -> Result<Self, LexingError> {
        tracing::trace!(cursor=%stream.cursor(), "parsing type");
        let start = stream.current_span().span().start;
        let current: Type = if stream.peek::<AnonymousOneOf>() {
            tracing::trace!("parsing oneof in type");
            Type::OneOf {
                ty: stream.parse()?,
            }
        } else if stream.peek::<Union>() {
            tracing::trace!("parsing union in type");
            Type::Union {
                ty: stream.parse()?,
            }
        } else if stream.peek::<toks::LParenToken>() {
            tracing::trace!("parsing paren type");
            let mut inner;
            let paren = paren!(inner in stream);
            Type::Paren {
                paren,
                ty: inner.parse()?,
            }
        } else if stream.peek::<Builtin>() {
            tracing::trace!("parsing builtin in type");
            Type::Builtin {
                ty: stream.parse()?,
            }
        } else if super::type_expr::TypeExprOp::peek(stream) {
            // Type expression operators (Pick, Omit, etc.) per RFC-0018
            // Must check before PathOrIdent since ops look like idents
            tracing::trace!("parsing type expr in type");
            let expr_start = stream.cursor();
            let expr = super::type_expr::TypeExpr::parse(stream)?;
            let expr_end = stream.cursor();
            Type::TypeExpr {
                expr: Spanned::new(expr_start, expr_end, expr),
            }
        } else if stream.peek::<PathOrIdent>() {
            tracing::trace!("parsing ident in type");
            Type::Ident {
                to: PathOrIdent::parse(stream)?,
            }
        } else if stream.peek::<AnonymousStruct>() {
            tracing::trace!("parsing struct in type");
            Type::Struct {
                ty: stream.parse()?,
            }
        } else {
            let expect = vec![
                AnonymousOneOf::fmt(),
                Builtin::fmt(),
                <Token![ident]>::fmt(),
            ];
            return Err(if let Some(next) = stream.peek_unchecked() {
                LexingError::expected_oneof(expect, next.value.clone())
            } else {
                LexingError::empty_oneof(expect)
            });
        };

        let end = stream.current_span().span().end;
        let mut current = Spanned::new(start, end, current);

        while stream.peek::<toks::LBracketToken>() {
            let mut inner_tokens;
            let bracket = bracket!(inner_tokens in stream);
            let size: Option<SpannedToken![number]> = if inner_tokens.peek::<Token![number]>() {
                Some(inner_tokens.parse()?)
            } else {
                None
            };

            let start = current.span().start;
            let end = stream
                .tokens
                .get(stream.cursor - 1)
                .expect("cursor after consuming RBracket")
                .span()
                .end;

            let inner_spanned = current;
            let array_value = match size {
                Some(sz) => {
                    Array::Sized {
                        ty: Box::new(inner_spanned),
                        bracket,
                        size: sz,
                    }
                },
                None => {
                    Array::Unsized {
                        ty: Box::new(inner_spanned),
                        bracket,
                    }
                },
            };
            let array_spanned = Spanned::new(start, end, array_value);
            current = Spanned::new(start, end, Type::Array { ty: array_spanned });
        }

        // Parse &| (union or) - left-associative per RFC-0016
        while stream.peek::<toks::AmpPipeToken>() {
            let op: SpannedToken![union_or] = stream.parse()?;
            let rhs_start = stream.current_span().span().start;
            let rhs: Type = Type::parse(stream)?;
            let rhs_end = stream.current_span().span().end;
            let rhs_spanned = Spanned::new(rhs_start, rhs_end, Box::new(rhs));

            let lhs_start = current.span().start;
            let end = rhs_spanned.span().end;
            current = Spanned::new(
                lhs_start,
                end,
                Type::UnionOr {
                    lhs: current.map(Box::new),
                    op,
                    rhs: rhs_spanned,
                },
            );
        }

        if stream.peek::<Token![!]>() {
            let ex: Spanned<BangToken> = stream.parse()?;
            current = Spanned::new(
                start,
                ex.span().end,
                Type::Result {
                    ty: current.map(Box::new),
                    ex,
                },
            )
        }
        Ok(current.value)
    }
}

impl Peek for Type {
    fn peek(stream: &TokenStream) -> bool {
        stream.peek::<AnonymousOneOf>()
            || stream.peek::<AnonymousStruct>()
            || stream.peek::<Builtin>()
            || stream.peek::<Token![ident]>()  // Covers TypeExprOp (Pick, Omit, etc.)
            || stream.peek::<toks::LParenToken>()
    }
}

impl ToTokens for Type {
    fn write(
        &self,
        tt: &mut crate::fmt::Printer,
    ) {
        match self {
            Self::Builtin { ty } => ty.write(tt),
            Self::Ident { to } => to.write(tt),
            Self::OneOf { ty } => ty.write(tt),
            Self::Array { ty } => ty.write(tt),
            Self::Struct { ty } => ty.write(tt),
            Self::Union { ty } => ty.write(tt),
            Self::UnionOr { lhs, op, rhs } => {
                lhs.write(tt);
                tt.space();
                op.write(tt);
                tt.space();
                rhs.write(tt);
            },
            Self::Result { ty, ex } => {
                ty.write(tt);
                ex.write(tt);
            },
            Self::Paren { ty, paren: _ } => {
                tt.token(&Token::LParen);
                tt.write(ty);
                tt.token(&Token::RParen);
            },
            Self::TypeExpr { expr } => expr.value.write(tt),
        }
    }
}

impl Type {
    pub fn is_builtin(&self) -> bool {
        matches!(self, Type::Builtin { .. })
    }

    pub fn is_ident(&self) -> bool {
        matches!(self, Type::Ident { .. })
    }

    pub fn is_union(&self) -> bool {
        matches!(self, Type::Union { .. })
    }

    pub fn is_union_or(&self) -> bool {
        matches!(self, Type::UnionOr { .. })
    }

    pub fn is_anonymous_struct(&self) -> bool {
        matches!(self, Type::Struct { .. })
    }
}

#[cfg(test)]
mod test {
    use crate::{
        ast::ty::TriesQualify,
        ctx::RefOrItemContext,
        defs::Spanned,
        tokens::{IdentToken, ToTokens},
    };

    #[test_case::test_case("i8")]
    #[test_case::test_case("i16")]
    #[test_case::test_case("i32")]
    #[test_case::test_case("i64")]
    #[test_case::test_case("u8")]
    #[test_case::test_case("u16")]
    #[test_case::test_case("u32")]
    #[test_case::test_case("u64")]
    #[test_case::test_case("f16")]
    #[test_case::test_case("f32")]
    #[test_case::test_case("f64")]
    #[test_case::test_case("bool")]
    #[test_case::test_case("str")]
    #[test_case::test_case("base64")]
    #[test_case::test_case("str!"; "builtin result")]
    #[test_case::test_case("i32[]"; "round trip unsized array")]
    #[test_case::test_case("i64[][]"; "round trip double unsized array")]
    #[test_case::test_case("i32[10]"; "round trip sized array")]
    #[test_case::test_case("i64[][5]"; "round trip mixed sized/unsized array")]
    #[test_case::test_case("oneof i32 | i64"; "round trip oneof builtins")]
    #[test_case::test_case("oneof i32[] | i64[][]"; "round trip oneof arrays")]
    #[test_case::test_case("oneof i32 | i64 | str | bool | f32 | u8[]"; "round trip nested oneof")]
    #[test_case::test_case("str[][][]"; "round trip triple unsized array")]
    #[test_case::test_case("bool[42]"; "round trip sized bool array")]
    #[test_case::test_case("binary"; "round trip binary")]
    #[test_case::test_case("datetime"; "round trip datetime")]
    #[test_case::test_case("never"; "round trip never")]
    #[test_case::test_case("(oneof i32 | f32)[]"; "round trip nested oneof array with paren")]
    #[test_case::test_case("(oneof i32 | f32)[]!"; "round trip nested oneof array with paren result")]
    #[test_case::test_case("oneof my_struct | never"; "round trip ident and never")]
    #[test_case::test_case("my_struct & other_struct"; "basic union")]
    #[test_case::test_case("my_struct & ((other_struct & inner_struct) & next_struct)"; "nested union")]
    #[test_case::test_case("A &| B"; "basic union or")]
    #[test_case::test_case("A &| B &| C"; "chained union or left associative")]
    #[test_case::test_case("(A &| B) &| C"; "union or explicit left grouping")]
    #[test_case::test_case("A &| (B &| C)"; "union or explicit right grouping")]
    fn round_trip(src: &str) {
        crate::tst::round_trip::<super::Type>(src).unwrap();
    }

    #[test_case::test_case("MyStruct", vec!["foo::bar::MyStruct".into()])]
    fn test_qualify(
        src: &str,
        expect: Vec<String>,
    ) {
        let a = crate::tst::round_trip::<super::PathOrIdent>(src).unwrap();

        let pkg = crate::ctx::RefContext::new("foo".into(), vec!["bar".into()]);
        let qual = a.qualified_in_context(&[
            // e.g. `use foo::bar::MyStruct;`
            RefOrItemContext::Item(
                pkg.item(Spanned::call_site(IdentToken::new("MyStruct".into()))),
            ),
            // e.g. `use foo::bar;`
            RefOrItemContext::Ref(pkg),
        ]);

        assert_eq!(
            qual.iter()
                .map(ToTokens::display)
                .collect::<Vec<_>>(),
            expect
        );
    }
}
