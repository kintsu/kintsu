//! Code generation helpers for parser declaration types.
//!
//! Extension traits and helper functions for converting parser types to Rust code.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    declare::{Builtin, DeclComment, DeclEnumDef, DeclField, DeclMeta, DeclType},
    generate::{DateTimeLibrary, RustConfig},
};

pub trait DeclTypeExt {
    fn to_rust_tokens(
        &self,
        opts: &RustConfig,
    ) -> TokenStream;
    fn rust_attrs(&self) -> TokenStream;
}

impl DeclTypeExt for DeclType {
    fn to_rust_tokens(
        &self,
        opts: &RustConfig,
    ) -> TokenStream {
        match self {
            DeclType::Builtin { ty } => ty.to_rust_tokens(opts),
            DeclType::Named { reference } => {
                let ident = super::rust::ident(&reference.name);
                quote!(#ident)
            },
            DeclType::Array { element_type } => {
                let inner = element_type.to_rust_tokens(opts);
                quote!(Vec<#inner>)
            },
            DeclType::SizedArray { element_type, size } => {
                let inner = element_type.to_rust_tokens(opts);
                let size = super::rust::lit(size.to_string());
                quote!([#inner; #size])
            },
            DeclType::Result { ok_type, error } => {
                let ok = ok_type.to_rust_tokens(opts);
                let err = super::rust::ident(&error.name);
                quote!(Result<#ok, #err>)
            },
            DeclType::Optional { inner_type } => {
                let inner = inner_type.to_rust_tokens(opts);
                quote!(Option<#inner>)
            },
            DeclType::Map {
                key_type,
                value_type,
            } => {
                let key = key_type.to_rust_tokens(opts);
                let val = value_type.to_rust_tokens(opts);
                quote!(std::collections::BTreeMap<#key, #val>)
            },
            DeclType::TypeExpr {
                op,
                target,
                selectors,
            } => {
                todo!(
                    "Type expressions are not yet supported in code generation: {:?} {:?} {:?}",
                    op,
                    target,
                    selectors
                )
            },
            DeclType::Paren { inner_type } => inner_type.to_rust_tokens(opts),
        }
    }

    fn rust_attrs(&self) -> TokenStream {
        match self {
            DeclType::Named { .. } => quote!(#[fields(enm)]),
            _ => quote!(),
        }
    }
}

pub trait BuiltinExt {
    fn to_rust_tokens(
        &self,
        opts: &RustConfig,
    ) -> TokenStream;
}

impl BuiltinExt for Builtin {
    fn to_rust_tokens(
        &self,
        opts: &RustConfig,
    ) -> TokenStream {
        match self {
            Builtin::I8 => quote!(i8),
            Builtin::I16 => quote!(i16),
            Builtin::I32 => quote!(i32),
            Builtin::I64 => quote!(i64),
            Builtin::U8 => quote!(u8),
            Builtin::U16 => quote!(u16),
            Builtin::U32 => quote!(u32),
            Builtin::U64 => quote!(u64),
            Builtin::Usize => quote!(usize),
            Builtin::F16 => quote!(f32),
            Builtin::F32 => quote!(f32),
            Builtin::F64 => quote!(f64),
            Builtin::Bool => quote!(bool),
            Builtin::Str => quote!(String),
            Builtin::DateTime => {
                match opts.time {
                    DateTimeLibrary::Chrono => {
                        cfg_if::cfg_if! {
                            if #[cfg(feature = "chrono")] {
                                quote!(chrono::DateTime::<chrono::Utc>)
                            } else {
                                panic!("feature 'chrono' must be enabled to emit chrono::DateTime")
                            }
                        }
                    },
                    DateTimeLibrary::Time => {
                        cfg_if::cfg_if! {
                            if #[cfg(feature = "time")] {
                                quote!(time::UtcDateTime)
                            } else {
                                panic!("feature 'time' must be enabled to emit time::UtcDateTime")
                            }
                        }
                    },
                }
            },
            Builtin::Complex => quote!(f64),
            Builtin::Binary => quote!(Vec<u8>),
            Builtin::Base64 => quote!(Vec<u8>),
            Builtin::Never => quote!(()),
        }
    }
}

pub trait DeclMetaExt {
    fn doc_comment(&self) -> TokenStream;
    fn version_lit(&self) -> TokenStream;
}

impl DeclMetaExt for DeclMeta {
    fn doc_comment(&self) -> TokenStream {
        quote!()
    }

    fn version_lit(&self) -> TokenStream {
        super::rust::lit(self.version.to_string())
    }
}

pub trait DeclCommentExt {
    fn doc_comment(&self) -> TokenStream;
}

impl DeclCommentExt for DeclComment {
    fn doc_comment(&self) -> TokenStream {
        if self.is_empty() {
            return quote!();
        }
        let comments: Vec<_> = self
            .comments
            .iter()
            .map(|c| quote!(#[doc = #c]))
            .collect();
        quote!(#(#comments)*)
    }
}

pub trait DeclFieldExt {
    fn to_rust_field(
        &self,
        opts: &RustConfig,
    ) -> TokenStream;
}

impl DeclFieldExt for DeclField {
    fn to_rust_field(
        &self,
        opts: &RustConfig,
    ) -> TokenStream {
        let name_str = &self.name;
        let ident = super::rust::ident(
            self.name
                .as_str()
                .to_lowercase()
                .replace('-', "_"),
        );
        let ty = if self.optional {
            let inner = self.ty.to_rust_tokens(opts);
            quote!(Option<#inner>)
        } else {
            self.ty.to_rust_tokens(opts)
        };
        let attrs = self.ty.rust_attrs();
        let comment = self.comments.doc_comment();

        quote! {
            #[serde(rename = #name_str)]
            #comment
            #attrs
            pub #ident: #ty,
        }
    }
}

pub trait DeclEnumDefExt {
    fn is_int_enum(&self) -> bool;
    fn is_string_enum(&self) -> bool;
}

impl DeclEnumDefExt for DeclEnumDef {
    fn is_int_enum(&self) -> bool {
        matches!(&self.enum_def, crate::declare::DeclEnum::Int(_))
    }

    fn is_string_enum(&self) -> bool {
        matches!(&self.enum_def, crate::declare::DeclEnum::String(_))
    }
}
