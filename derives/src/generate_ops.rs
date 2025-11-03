use quote::{ToTokens, quote};

use syn::{
    Attribute, Block, FnArg, Generics, Ident, LitInt, Token, Visibility, parenthesized,
    parse::Parse, punctuated::Punctuated,
};

use crate::{
    resolve_defs,
    shared::{DescOrPath, kw_eq, maybe_toks, peek_parse},
};

mod kws {
    syn::custom_keyword!(version);
    syn::custom_keyword!(namespace);
}

#[allow(unused)]
pub struct Atts {
    attrs: Vec<Attribute>,
    version: LitInt,
    describe: Option<DescOrPath>,
}

resolve_defs! {
    Atts
}

impl Parse for Atts {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = Attribute::parse_outer(input)?;
        let mut version: Option<LitInt> = None;
        let mut describe: Option<DescOrPath> = None;
        loop {
            if input.peek(kws::version) {
                version = Some(kw_eq::<kws::version, _>(input)?);
            } else if input.peek(crate::shared::kw::describe) {
                let _: crate::shared::kw::describe = input.parse()?;
                let paren;
                parenthesized!(paren in input);
                describe = Some(paren.parse()?);
            } else if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            } else if input.is_empty() {
                break;
            } else {
                return Err(syn::Error::new(input.span(), "unknown tokens"));
            }
        }

        let mut this = Self {
            attrs,
            version: match version {
                Some(ver) => ver,
                None => return Err(syn::Error::new(input.span(), "version is required")),
            },
            describe,
        };

        this.resolve_defs()?;

        Ok(this)
    }
}

#[derive(Debug)]
pub struct Op {
    attrs: Vec<Attribute>,
    vis: Option<Visibility>,
    asy: Option<Token![async]>,
    f: Token![fn],
    name: Ident,
    generics: Generics,
    #[allow(unused)]
    paren: syn::token::Paren,
    args: Punctuated<FnArg, syn::Token![,]>,
    #[allow(unused)]
    ret_sig: syn::Token![->],
    ret_ty: syn::Type,
    block: Block,
}

impl Parse for Op {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let paren;
        Ok(Self {
            attrs: Attribute::parse_outer(input)?,
            vis: peek_parse(input, Token![pub], false)?,
            asy: peek_parse(input, Token![async], false)?,
            f: input.parse()?,
            name: input.parse()?,
            generics: input.parse()?,
            paren: parenthesized!(paren in input),
            args: paren.parse_terminated(FnArg::parse, Token![,])?,
            ret_sig: input.parse()?,
            ret_ty: input.parse()?,
            block: input.parse()?,
        })
    }
}

// #[derive(Debug, Clone)]
// enum ReturnType {
//     Result { ok: syn::Type, err: syn::Type },
//     Infallible(syn::Type),
// }

// impl ReturnType {
//     fn from_type(ty: &syn::Type) -> Self {
//         use syn::{GenericArgument, PathArguments, Type};
//         match ty {
//             Type::Path(p) => {
//                 if let Some(seg) = p.path.segments.last() {
//                     if seg.ident == "Result" {
//                         if let PathArguments::AngleBracketed(ab) = &seg.arguments {
//                             let mut args: Vec<syn::Type> = ab
//                                 .args
//                                 .iter()
//                                 .filter_map(|ga| {
//                                     match ga {
//                                         GenericArgument::Type(t) => Some(t.clone()),
//                                         _ => None,
//                                     }
//                                 })
//                                 .collect();
//                             if args.len() == 2 {
//                                 let ok = args.remove(0);
//                                 let err = args.remove(0);
//                                 return Self::Result { ok, err };
//                             }
//                         }
//                     }
//                 }
//                 Self::Infallible(ty.clone())
//             },
//             other => Self::Infallible(other.clone()),
//         }
//     }
// }

// struct ParsedOp {
//     inner: Op,
//     result_type: ReturnType,
// }

// impl TryFrom<Op> for ParsedOp {
//     type Error = syn::Error;
//     fn try_from(op: Op) -> Result<Self, Self::Error> {
//         let rt = ReturnType::from_type(&op.ret_ty);
//         Ok(Self {
//             inner: op,
//             result_type: rt,
//         })
//     }
// }

impl ToTokens for Op {
    fn to_tokens(
        &self,
        tokens: &mut proc_macro2::TokenStream,
    ) {
        tokens.extend(
            self.attrs
                .iter()
                .map(|it| it.to_token_stream()),
        );

        maybe_toks(tokens, &self.vis, |v| quote! {#v});
        maybe_toks(tokens, &self.asy, |v| quote! {#v});

        tokens.extend(self.f.to_token_stream());
        tokens.extend(self.name.to_token_stream());

        tokens.extend(self.generics.to_token_stream());

        tokens.extend({
            let args = self.args.to_token_stream();
            let ty = &self.ret_ty;
            quote::quote! {(#args) -> #ty}
        });

        tokens.extend(self.block.to_token_stream());
    }
}

// #[cfg(test)]
// mod test {
//     use darling::ToTokens;

//     #[test_case::test_case(
//         r#"
//             fn sum(values: Vec<i32>) -> i32 {
//                 values.iter().sum()
//             }
//         "#; "basic fn"
//     )]
//     #[test_case::test_case(
//         r#"
//             pub(crate) async fn sum(values: Vec<i32>) -> i32 {
//                 3
//             }
//         "#; "async vis fn"
//     )]
//     #[test_case::test_case(
//         r#"
//             pub(crate) async fn join<'a, 'b, 'c>(v1: &'a str, v2: &'b str) -> &'c str
//             {
//                 ""
//             }
//         "#; "fn with lifetimes"
//     )]
//     fn test_parse(stream: &str) {
//         let _: super::Op = syn::parse_str(&stream).unwrap();
//     }

//     #[test_case::test_case(
//         r#"fn a() -> Result<u32, i32> { unimplemented!() }"#,
//         true;
//         "detect result"
//     )]
//     #[test_case::test_case(
//         r#"fn b() -> std::result::Result<String, MyErr> { unimplemented!() } struct MyErr;"#,
//         true;
//         "detect std::result::Result"
//     )]
//     #[test_case::test_case(
//         r#"fn c() -> core::result::Result<Vec<u8>, ()> { unimplemented!() }"#,
//         true;
//         "detect core::result::Result"
//     )]
//     #[test_case::test_case(
//         r#"fn d() -> i64 { 0 }"#,
//         false;
//         "non result"
//     )]

//     fn test_return_type_detect(
//         src: &str,
//         is_result: bool,
//     ) {
//         let file: syn::File = syn::parse_str(src).unwrap();
//         let item_fn = file
//             .items
//             .iter()
//             .find_map(|it| {
//                 match it {
//                     syn::Item::Fn(f) => Some(f),
//                     _ => None,
//                 }
//             })
//             .expect("fn");

//         let op_src = item_fn.to_token_stream().to_string();
//         let op: super::Op = syn::parse_str(&op_src).unwrap();
//         let parsed = super::ReturnType::from_type(&op.ret_ty);
//         match (is_result, parsed) {
//             (true, super::ReturnType::Result { .. }) => {},
//             (false, super::ReturnType::Infallible(_)) => {},
//             (true, other) => panic!("expected Result variant, got {other:?}"),
//             (false, other) => panic!("expected Infallible variant, got {other:?}"),
//         }
//     }
// }
