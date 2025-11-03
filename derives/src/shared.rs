use convert_case::{Case, Casing};
use darling::FromMeta;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Expr, Ident, Lit, LitStr, Token,
    parse::{Parse, ParseStream},
    token::Token,
};

pub fn ident<D: AsRef<str>>(s: D) -> Ident {
    match Ident::from_string(s.as_ref()) {
        Ok(id) => id,
        Err(_) => Ident::new("_invalid_ident", proc_macro2::Span::call_site()),
    }
}

#[derive(darling::FromMeta, Clone, Debug)]
pub enum DescOrPath {
    Text(syn::LitStr),
    File(syn::LitStr),
}

pub mod kw {
    syn::custom_keyword!(text);
    syn::custom_keyword!(file);
    syn::custom_keyword!(describe);
}

pub fn kw_eq<Kw: Token + Parse, Ty: Parse>(input: ParseStream) -> syn::Result<Ty> {
    let _: Kw = input.parse()?;
    let _: Token![=] = input.parse()?;
    input.parse()
}

impl Parse for DescOrPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(if input.peek(kw::file) {
            // file = "path"
            Self::File(kw_eq::<kw::file, _>(input)?)
        } else if input.peek(kw::text) {
            // text = "literal description"
            Self::Text(kw_eq::<kw::text, _>(input)?)
        } else {
            return Err(syn::Error::new(
                input.span(),
                "expected 'file' or 'text' in describe(...) attribute",
            ));
        })
    }
}

impl DescOrPath {
    pub fn resolve_defs(
        parent: &Ident,
        this: Option<Self>,
    ) -> ResolvedDefs {
        let mut desc_iden: Option<Ident> = None;

        let desc = if let Some(desc) = this {
            let iden = ident(format!("{}_DESCRIPTION", parent).to_case(Case::UpperSnake));
            desc_iden = Some(iden.clone());
            quote! {
                const #iden: &'static str = #desc;
            }
        } else {
            quote!()
        };

        let desc_value = match desc_iden {
            Some(iden) => {
                quote!(Some(#iden.into()))
            },
            None => {
                quote!(None)
            },
        };

        ResolvedDefs { desc, desc_value }
    }

    pub fn maybe_rust_doc(opt: &Option<Self>) -> TokenStream {
        if let Some(desc) = opt {
            quote::quote!(#[doc = #desc])
        } else {
            quote::quote! {}
        }
    }
}

impl ToTokens for DescOrPath {
    fn to_tokens(
        &self,
        tokens: &mut TokenStream,
    ) {
        tokens.extend(match self {
            Self::File(file) => {
                let value = file.value();
                quote!(include_str!(#value))
            },
            Self::Text(lit) => {
                let value = lit.value();
                quote!(#value)
            },
        });
    }
}

#[derive(Debug)]
pub struct ResolvedDefs {
    pub desc: TokenStream,
    pub desc_value: TokenStream,
}

pub fn peek_parse<T: syn::parse::Parse, Tok: syn::parse::Peek>(
    input: ParseStream,
    tok: Tok,
    not: bool,
) -> syn::Result<Option<T>> {
    let peek = input.peek(tok);
    let peek = if not {
        !peek
    } else {
        peek
    };
    if peek {
        Ok(Some(input.parse()?))
    } else {
        Ok(None)
    }
}

pub fn maybe_toks<T: quote::ToTokens, Get: Fn(&T) -> proc_macro2::TokenStream>(
    tokens: &mut proc_macro2::TokenStream,
    v: &Option<T>,
    get: Get,
) {
    if let Some(v) = v {
        tokens.extend(get(v))
    }
}

pub fn maybe_docs(atts: &Vec<syn::Attribute>) -> Option<DescOrPath> {
    let mut docs = vec![];
    for att in atts {
        match att.path().segments.first() {
            Some(seg) => {
                if seg.ident == "doc" {
                    let value = att.meta.require_name_value().unwrap();
                    match &value.value {
                        Expr::Lit(lit) => {
                            match &lit.lit {
                                Lit::Str(str) => {
                                    let clean = String::from(str.value().trim_start());
                                    docs.push(clean)
                                },
                                lit => panic!("{lit:#?} is not supported"),
                            }
                        },
                        expr => panic!("{expr:#?} is not supported"),
                    }
                }
            },
            None => continue,
        };
    }
    if docs.is_empty() {
        None
    } else {
        let docs = docs.join("\n");

        Some(DescOrPath::Text(LitStr::new(&docs, Span::call_site())))
    }
}

pub fn assign_maybe_docs(
    attrs: &Vec<syn::Attribute>,
    value: &mut Option<DescOrPath>,
) -> syn::Result<()> {
    let maybe_defs = super::shared::maybe_docs(attrs);
    if let Some(rsdoc) = maybe_defs {
        if value.is_some() {
            return Err(syn::Error::new(
                Span::call_site(),
                "rust docs cannot be used with describe(...)",
            ));
        }
        *value = Some(rsdoc);
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use syn::Attribute;

    use crate::shared::*;

    struct WithAttr {
        atts: Vec<Attribute>,
    }

    impl syn::parse::Parse for WithAttr {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            Ok(Self {
                atts: Attribute::parse_outer(input)?,
            })
        }
    }

    #[test_case::test_case("/// abc doc", "abc doc")]
    #[test_case::test_case(
        r#"
    /// test multi
    /// line
    /// docs
    "#,
        "test multi\nline\ndocs"
    )]
    fn test_attribute_docs(
        eg: &str,
        asserts: &str,
    ) {
        let atts: WithAttr = syn::parse_str(eg).unwrap();
        let docs = maybe_docs(&atts.atts);
        let doc = match docs.unwrap() {
            DescOrPath::Text(value) => value.value(),
            _ => panic!("expected text"),
        };
        assert_eq!(doc, asserts)
    }
}
