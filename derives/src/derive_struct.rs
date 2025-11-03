use crate::{resolve_defs, shared::*};
use convert_case::{Case, Casing};
use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, spanned::Spanned};

use crate::call_span;

#[derive(darling::FromField)]
#[darling(attributes(fields), forward_attrs(doc))]
pub struct Field {
    attrs: Vec<Attribute>,

    ident: Option<Ident>,
    ty: syn::Type,

    #[darling(default)]
    enm: bool,

    #[darling(default)]
    one_of: bool,

    #[darling(default)]
    describe: Option<DescOrPath>,
}

#[derive(darling::FromDeriveInput)]
#[darling(attributes(fields), forward_attrs(doc))]
pub struct Struct {
    attrs: Vec<Attribute>,

    ident: Ident,
    data: darling::ast::Data<(), Field>,

    version: usize,

    describe: Option<DescOrPath>,
}

resolve_defs! {
    Struct, Field
}

pub fn derive_struct(tokens: TokenStream) -> TokenStream {
    let input = call_span!(syn::parse(tokens.clone().into()));

    let mut s = match Struct::from_derive_input(&input) {
        Ok(s) => s,
        Err(e) => return e.write_errors(),
    };

    call_span!(s.resolve_defs());

    let desc = DescOrPath::resolve_defs(&s.ident, s.describe);

    let desc_value = desc.desc_value;
    let desc = desc.desc;

    let mut fields = call_span!(
        @opt s.data.take_struct();
        syn::Error::new(s.ident.span(), "#[derive(Struct)] can only be used on structs")
    )
    .fields;

    for field in fields.iter_mut() {
        call_span!(field.resolve_defs());
    }

    let fields_map = quote!(
        let mut m = std::collections::BTreeMap::<_, _>::new();
    );

    let parent_iden = s.ident.clone();

    if let Some(bad) = fields.iter().find(|f| f.ident.is_none()) {
        let err: syn::Result<()> = Err(syn::Error::new(
            bad.ty.span(),
            "struct fields must be named (no tuple or unit fields)",
        ));
        call_span!(err);
    }

    let mut fields_def = quote!();
    for field in &fields {
        let Some(iden) = field.ident.clone() else {
            continue;
        };

        if field.enm && field.one_of {
            return syn::Error::new(
                field.ty.span(), "cannot have both enum and one_of types. if you are using a literal, use enum. if you are using type discriminants, use one_of."
            ).into_compile_error();
        }

        let ty = field.ty.clone();
        let iden_str = format!("{iden}");

        let desc = DescOrPath::resolve_defs(&iden, field.describe.clone());

        let desc_value = desc.desc_value;
        let desc = desc.desc;

        fields_def.extend(quote!(
            #desc

            m.insert(stringify!(#iden).into(), kintsu_sdk::Field{
                meta: kintsu_sdk::Meta {
                    name: Some(#iden_str.into()),
                    namespace: Some(#parent_iden::NAMESPACE.into()),
                    description: #desc_value,
                    version: None,
                },
                ty: <#ty>::ty(),
                optional: false,
            }.into());
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
            kintsu_sdk::Definitions::StructV1(kintsu_sdk::Struct{
                meta: kintsu_sdk::Meta {
                    name: #iden_lit.into(),
                    namespace: #iden::NAMESPACE.into(),
                    version: VERSION.into(),
                    description: #desc_value,
                },
                fields: kintsu_sdk::FieldsList::new(m),
            })
        });

        impl kintsu_sdk::Typed for #iden {
            fn ty() -> kintsu_sdk::Type {
                kintsu_sdk::Type::CompoundType(
                    kintsu_sdk::CompoundType::Struct{
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
