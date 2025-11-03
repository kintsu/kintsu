use convert_case::{Case, Casing};
use darling::{FromDeriveInput, FromField, ast::Fields};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, Type};

use crate::{resolve_defs, shared::*};

use crate::call_span;

#[derive(darling::FromVariant)]
#[darling(attributes(fields), forward_attrs(doc))]
pub struct OneOfField {
    pub attrs: Vec<Attribute>,
    ident: Ident,
    fields: Fields<OneOfInnerField>,

    #[darling(default)]
    describe: Option<DescOrPath>,
}

#[derive(FromField, Clone)]
#[darling(attributes(fields), forward_attrs(doc))]
pub struct OneOfInnerField {
    pub attrs: Vec<Attribute>,
    pub ty: Type,

    #[darling(default)]
    describe: Option<DescOrPath>,
}

#[derive(darling::FromDeriveInput)]
#[darling(attributes(fields), supports(enum_any), forward_attrs(doc))]
pub struct OneOfDesc {
    pub attrs: Vec<Attribute>,

    ident: Ident,
    data: darling::ast::Data<OneOfField, ()>,

    version: usize,

    #[darling(default)]
    describe: Option<DescOrPath>,
}

resolve_defs! {
    OneOfField, OneOfInnerField, OneOfDesc
}

pub fn derive_one_of(tokens: TokenStream) -> TokenStream {
    let input = call_span!(syn::parse(tokens.clone().into()));
    let mut s = match OneOfDesc::from_derive_input(&input) {
        Ok(s) => s,
        Err(e) => return e.write_errors(),
    };

    call_span!(s.resolve_defs());

    let desc = DescOrPath::resolve_defs(&s.ident, s.describe);
    let desc_value = desc.desc_value;
    let desc = desc.desc;

    let mut fields = call_span!(
        @opt s.data.take_enum();
        syn::Error::new(s.ident.span(), "#[derive(OneOf)] can only be used on enums")
    );

    for field in fields.iter_mut() {
        call_span!(field.resolve_defs());
        for inner in field.fields.fields.iter_mut() {
            call_span!(inner.resolve_defs());
        }
    }

    let fields_map = quote!(
        let mut m = std::collections::BTreeMap::<_, kintsu_sdk::OneOfVariant>::new();
    );

    let mut fields_def = quote!();
    let mut saw_nullish = false;
    for field in fields {
        let iden = field.ident.clone();
        let iden_str = format!("{iden}");
        let ty = match field
            .fields
            .fields
            .iter()
            .map(|it| &it.ty)
            .next()
        {
            Some(Type::Path(pat)) => quote::quote!(#pat),
            Some(..) => {
                return syn::Error::new(
                    field.ident.span(),
                    "a type path must be given for one_of variants",
                )
                .into_compile_error();
            },
            None => {
                if saw_nullish {
                    return syn::Error::new(
                        field.ident.span(),
                        "only one field variant may be assigned a zero value.",
                    )
                    .into_compile_error();
                }
                saw_nullish = true;
                quote::quote!(Option<()>)
            },
        };

        let desc = DescOrPath::resolve_defs(&field.ident, field.describe.clone());
        let desc_value = desc.desc_value;
        let desc = desc.desc;

        fields_def.extend(quote!(
            #desc

            m.insert(#iden_str.into(), kintsu_sdk::OneOfVariant{
                name: #iden_str.into(),
                ty: <#ty>::ty(),
                description: #desc_value,
            });
        ));
    }

    let iden = s.ident.clone();
    let iden_lit = iden.to_string();
    let version = s.version;
    let iden_def = ident(format!("{iden}_DEF").to_case(Case::UpperSnake));

    let def = quote!(
        static #iden_def: std::sync::LazyLock<kintsu_sdk::Definitions> = std::sync::LazyLock::new(|| {
            use kintsu_sdk::{OfNamespace, Typed};

            #fields_map
            #fields_def

            const VERSION: kintsu_sdk::Version = kintsu_sdk::Version::new(#version);
            kintsu_sdk::Definitions::OneOfV1(kintsu_sdk::OneOf{
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
                    kintsu_sdk::CompoundType::OneOf{
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
