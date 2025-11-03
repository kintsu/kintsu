use convert_case::{Case, Casing};
use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Expr, Ident, Lit, LitStr};

use crate::{call_span, resolve_defs, shared::*};

#[derive(darling::FromVariant)]
#[darling(attributes(fields), forward_attrs(doc))]
pub struct Field {
    attrs: Vec<Attribute>,

    ident: Ident,
    discriminant: Option<syn::Expr>,

    #[darling(default)]
    describe: Option<DescOrPath>,

    #[darling(default)]
    str_value: Option<LitStr>,
}

#[derive(darling::FromDeriveInput)]
#[darling(attributes(fields), supports(enum_any), forward_attrs(doc))]
pub struct Enum {
    attrs: Vec<Attribute>,

    ident: Ident,
    data: darling::ast::Data<Field, ()>,

    version: usize,

    describe: Option<DescOrPath>,
}

resolve_defs!(Enum, Field);

pub fn derive_enum(tokens: TokenStream) -> TokenStream {
    let input = call_span!(syn::parse(tokens.clone().into()));
    let mut s = match Enum::from_derive_input(&input) {
        Ok(s) => s,
        Err(e) => return e.write_errors(),
    };

    call_span!(s.resolve_defs());

    let desc = DescOrPath::resolve_defs(&s.ident, s.describe);

    let desc_value = desc.desc_value;
    let desc = desc.desc;

    let mut fields = call_span!(
        @opt s.data.take_enum();
        syn::Error::new(s.ident.span(), "#[derive(Enum)] can only be used on enums")
    );

    for f in fields.iter_mut() {
        call_span!(f.resolve_defs())
    }

    let fields_map = quote!(
        let mut m = std::collections::BTreeMap::<_, _>::new();
    );

    let parent_iden = s.ident.clone();

    let fields_def: TokenStream = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let iden = field.ident.clone();
            let iden_str = format!("{iden}");

            let desc = DescOrPath::resolve_defs(&iden, field.describe.clone());

            let desc_value = desc.desc_value;
            let desc = desc.desc;
            let value = match &field.str_value {
                Some(value) => {
                    quote!(kintsu_sdk::StrOrInt::String(#value.into()))
                },
                None => {
                    match field.discriminant.clone() {
                        Some(expr) => {
                            match expr {
                                Expr::Lit(lit) => {
                                    match lit.lit {
                                        Lit::Int(int) => {
                                            quote!({
                                                let value: usize = #int;
                                                kintsu_sdk::StrOrInt::Int(value)
                                            })
                                        },
                                        lit => panic!("{lit:#?} type is unsupported"),
                                    }
                                },
                                _ => panic!("only lit exprs are permitted"),
                            }
                        },
                        None => quote!(kintsu_sdk::StrOrInt::Int(#i)),
                    }
                },
            };

            quote!(
                #desc

                m.insert(#iden_str.into(), kintsu_sdk::VariantKind{
                    meta: kintsu_sdk::Meta {
                        name: #iden_str.into(),
                        namespace: Some(#parent_iden::NAMESPACE.into()),
                        description: #desc_value,
                        version: None,
                    },
                    value: #value
                }.into());
            )
        })
        .collect();

    let iden = s.ident.clone();
    let iden_lit = iden.to_string();
    let version = s.version;
    let iden_def = ident(format!("{iden}_DEF").to_case(Case::UpperSnake));

    let def = quote!(
        static #iden_def: std::sync::LazyLock<kintsu_sdk::Definitions> = std::sync::LazyLock::new(|| {
            use kintsu_sdk::OfNamespace;

            #fields_map
            #fields_def

            const VERSION: kintsu_sdk::Version = kintsu_sdk::Version::new(#version);
            kintsu_sdk::Definitions::EnumV1(kintsu_sdk::Enum{
                meta: kintsu_sdk::Meta {
                    name: #iden_lit.into(),
                    namespace: #iden::NAMESPACE.into(),
                    version: VERSION.into(),
                    description: #desc_value,
                },
                variants: kintsu_sdk::Named::new(m),
            })
        });

        impl kintsu_sdk::Typed for #iden {
            fn ty() -> kintsu_sdk::Type {
                kintsu_sdk::Type::CompoundType(
                    kintsu_sdk::CompoundType::Enum{
                        to: #iden_lit.into()
                    }
                )
            }
        }

        impl kintsu_sdk::Defined for #iden {
            fn definition() -> &'static kintsu_sdk::Definitions {
                use std::ops::Deref;
                #iden_def.deref()
            }
        }
    );
    quote! {
        #desc

        #def
    }
}
