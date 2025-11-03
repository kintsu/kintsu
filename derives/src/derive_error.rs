use convert_case::{Case, Casing};
use darling::{
    FromDeriveInput, FromField, FromVariant,
    ast::{Fields, Style},
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, Visibility};

use crate::{call_span, resolve_defs, shared::*};

#[derive(FromField, Clone)]
#[darling(attributes(fields), forward_attrs(doc))]
pub struct ErrorNamedField {
    pub attrs: Vec<Attribute>,

    pub ident: Option<Ident>,
    pub ty: syn::Type,

    #[darling(default)]
    pub describe: Option<DescOrPath>,
}

#[derive(FromVariant)]
#[darling(attributes(fields), forward_attrs(doc))]
pub struct ErrorVariantDesc {
    pub attrs: Vec<Attribute>,

    pub ident: Ident,
    #[darling(default)]
    pub fields: Fields<ErrorNamedField>,

    #[darling(default)]
    pub describe: Option<DescOrPath>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(fields), supports(enum_any), forward_attrs(doc))]
pub struct ErrorDesc {
    pub attrs: Vec<Attribute>,

    pub vis: Visibility,

    pub ident: Ident,
    pub data: darling::ast::Data<ErrorVariantDesc, ()>,

    pub version: usize,

    #[darling(default)]
    pub describe: Option<DescOrPath>,
}

resolve_defs! { ErrorNamedField, ErrorVariantDesc, ErrorDesc }

pub fn derive_error(tokens: TokenStream) -> TokenStream {
    let input = call_span!(syn::parse(tokens.clone().into()));
    let mut s = match ErrorDesc::from_derive_input(&input) {
        Ok(s) => s,
        Err(e) => return e.write_errors(),
    };

    call_span!(s.resolve_defs());

    let desc = DescOrPath::resolve_defs(&s.ident, s.describe.clone());
    let desc_value = desc.desc_value;
    let desc_tokens = desc.desc;
    let version = s.version;
    let vis = s.vis;

    let mut variants = call_span!(
        @opt s.data.take_enum();
        syn::Error::new(s.ident.span(), "#[derive(Error)] can only be used on enums")
    );

    for v in variants.iter_mut() {
        call_span!(v.resolve_defs());
        for v in v.fields.fields.iter_mut() {
            call_span!(v.resolve_defs());
        }
    }

    let mut structs = quote!();
    let mut gen_variant = quote!();

    for var in variants.iter() {
        let var_ident = &var.ident;
        let var_ident_str = var_ident.to_string();
        let desc = DescOrPath::resolve_defs(&var.ident, var.describe.clone());
        let desc_value = desc.desc_value;
        let desc = desc.desc;

        match var.fields.style {
            Style::Tuple => {
                let unnamed = &var.fields.fields;

                if unnamed.len() != 1 {
                    return syn::Error::new(
                        var_ident.span(),
                        "tuple error variants must have exactly one field",
                    )
                    .into_compile_error();
                }
                let inner = &unnamed[0];

                let desc = DescOrPath::resolve_defs(
                    &inner
                        .ident
                        .clone()
                        .unwrap_or_else(|| var_ident.clone()),
                    inner.describe.clone(),
                );
                let desc_value = desc.desc_value;
                let desc = desc.desc;

                let ty = &inner.ty;
                gen_variant.extend(quote! {
                    #desc

                    m.insert(#var_ident_str.into(), kintsu_sdk::OneOfVariant {
                        name: #var_ident_str.into(),
                        ty: #ty::ty(),
                        description: #desc_value,
                    });
                });
            },
            Style::Struct => {
                let gen_name = ident(format!("{}{}", s.ident, var_ident));
                let mut struct_fields = quote!();
                let named = &var.fields.fields;

                // forward desc to struct
                let rust_desc = DescOrPath::maybe_rust_doc(&var.describe);

                for f in named.iter() {
                    let field_ident = match &f.ident {
                        Some(i) => i.clone(),
                        None => {
                            return syn::Error::new(
                                var_ident.span(),
                                "named variant fields must have identifiers",
                            )
                            .into_compile_error();
                        },
                    };
                    let ty = &f.ty;
                    let desc = DescOrPath::maybe_rust_doc(&f.describe);
                    struct_fields.extend(quote!( #desc pub #field_ident: #ty, ));
                }

                structs.extend(quote! {
                    #[allow(unused)]
                    #[derive(serde::Serialize, serde::Deserialize, kintsu_sdk::Struct)]
                    #[fields(version = #version)]
                    #rust_desc
                    #vis struct #gen_name {
                        #struct_fields
                    }
                });

                gen_variant.extend(quote! {
                    #desc
                    m.insert(#var_ident_str.into(), kintsu_sdk::OneOfVariant {
                        name: #var_ident_str.into(),
                        ty: #gen_name::ty(),
                        description: #desc_value,
                    });
                });
            },
            Style::Unit => {
                return syn::Error::new(
                    var_ident.span(),
                    "unit error variants are not supported (must carry data)",
                )
                .into_compile_error();
            },
        };
    }

    let enum_ident = s.ident.clone();
    let enum_ident_str = enum_ident.to_string();
    let version_lit = s.version;
    let def_static_ident = ident(format!("{}_DEF", enum_ident).to_case(Case::UpperSnake));

    let def_tokens = quote! {
        static #def_static_ident: std::sync::LazyLock<kintsu_sdk::Definitions> = std::sync::LazyLock::new(|| {
            use kintsu_sdk::{OfNamespace, Defined, Typed};

            let mut m = std::collections::BTreeMap::<_, kintsu_sdk::OneOfVariant>::new();
            #gen_variant

            const VERSION: kintsu_sdk::Version = kintsu_sdk::Version::new(#version_lit);
            kintsu_sdk::Definitions::ErrorV1(kintsu_sdk::Error {
                meta: kintsu_sdk::Meta {
                    name: #enum_ident_str.into(),
                    namespace: #enum_ident::NAMESPACE.into(),
                    version: VERSION.into(),
                    description: #desc_value,
                },
                variants: kintsu_sdk::Named::new(m),
            })
        });

        impl kintsu_sdk::Defined for #enum_ident {
            fn definition() -> &'static kintsu_sdk::Definitions {
                use std::ops::Deref;
                #def_static_ident.deref()
            }
        }
    };

    quote! {
        #desc_tokens
        #structs
        #def_tokens
    }
}
