//! Rust code generation from parser declaration types.

use std::{io::Write, path::Path};

use convert_case::Casing;
use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    declare::{DeclEnum, DeclEnumDef, DeclError, DeclOneOf, DeclOperation, DeclStruct},
    generate::{
        RustConfig,
        decl_ext::{DeclCommentExt, DeclFieldExt, DeclMetaExt, DeclTypeExt},
        decl_gen::{DeclNsContext, GenerateDecl},
        files::WithFlush,
        rust::{RustGenState, RustGenerator, ident, lit},
    },
};

impl GenerateDecl<RustGenState, RustConfig> for RustGenerator {
    fn on_create_decl(
        _state: &DeclNsContext<'_, RustGenState, RustConfig, Self>,
        _fname: &Path,
        _f: &mut Box<dyn WithFlush>,
    ) -> std::io::Result<()> {
        Ok(())
    }

    fn new_state_decl(
        &self,
        _opts: &crate::generate::GenOpts<RustConfig>,
    ) -> RustGenState {
        RustGenState {}
    }

    fn gen_decl_struct(
        &self,
        state: &DeclNsContext<'_, RustGenState, RustConfig, Self>,
        def: &DeclStruct,
    ) -> crate::generate::Result<()> {
        let ns_file = state.ns_file();
        let mut tt = quote!();

        let desc_comment = def.comments.doc_comment();
        let iden = ident(def.name.to_case(convert_case::Case::Pascal));
        let version = def.meta.version_lit();

        let mut fields = quote!();
        for field in &def.fields {
            fields.extend(field.to_rust_field(&state.opts.opts));
        }

        tt.extend(quote! {
            #[derive(serde::Serialize, serde::Deserialize, kintsu_sdk::Struct)]
            #[fields(version = #version)]
            #desc_comment
            pub struct #iden {
                #fields
            }
        });

        tracing::info!("writing {} to '{}'", def.name, ns_file.display());

        state.with_file_handle(ns_file, |w| {
            write!(w, "{tt}")?;
            Ok(())
        })?;
        Ok(())
    }

    fn gen_decl_operation(
        &self,
        state: &DeclNsContext<'_, RustGenState, RustConfig, Self>,
        _def: &DeclOperation,
    ) -> crate::generate::Result<()> {
        let ns_file = state.ns_file();
        let tt = quote!();

        state.with_file_handle(ns_file, |w| {
            write!(w, "{tt}")?;
            Ok(())
        })?;
        Ok(())
    }

    fn gen_decl_enum(
        &self,
        state: &DeclNsContext<'_, RustGenState, RustConfig, Self>,
        def: &DeclEnumDef,
    ) -> crate::generate::Result<()> {
        let ns_file = state.ns_file();
        let mut tt = quote!();

        let name = ident(def.name.to_case(convert_case::Case::Pascal));
        let doc_comment = def.comments.doc_comment();
        let version = def.meta.version_lit();

        let (fields, atts): (TokenStream, TokenStream) = match &def.enum_def {
            DeclEnum::Int(variants) => {
                let fields: TokenStream = variants
                    .iter()
                    .map(|var| {
                        let iden = ident(var.name.to_case(convert_case::Case::Pascal));
                        let value = lit(var.value.to_string());
                        let vdoc = var.comments.doc_comment();
                        quote! {
                            #vdoc
                            #iden = #value,
                        }
                    })
                    .collect();

                let atts = quote!(
                    #[derive(kintsu_sdk::IntDeserialize, kintsu_sdk::IntSerialize)]
                    #[repr(u64)]
                );

                (fields, atts)
            },
            DeclEnum::String(variants) => {
                let fields: TokenStream = variants
                    .iter()
                    .map(|var| {
                        let iden = ident(var.name.to_case(convert_case::Case::Pascal));
                        let value = &var.value;
                        let vdoc = var.comments.doc_comment();
                        quote! {
                            #vdoc
                            #[fields(str_value = #value)]
                            #[serde(rename = #value)]
                            #iden,
                        }
                    })
                    .collect();

                let atts = quote!(
                    #[derive(serde::Serialize, serde::Deserialize)]
                );

                (fields, atts)
            },
        };

        tt.extend(quote! {
            #[derive(kintsu_sdk::Enum)]
            #[fields(version = #version)]
            #atts
            #doc_comment
            pub enum #name {
                #fields
            }
        });

        state.with_file_handle(ns_file, |w| {
            write!(w, "{tt}")?;
            Ok(())
        })?;
        Ok(())
    }

    fn gen_decl_one_of(
        &self,
        state: &DeclNsContext<'_, RustGenState, RustConfig, Self>,
        def: &DeclOneOf,
    ) -> crate::generate::Result<()> {
        let ns_file = state.ns_file();
        let mut tt = quote!();

        let name = ident(def.name.to_case(convert_case::Case::Pascal));
        let doc_comment = def.comments.doc_comment();
        let version = def.meta.version_lit();

        let fields: TokenStream = def
            .variants
            .iter()
            .map(|var| {
                let iden = ident(var.name.to_case(convert_case::Case::Pascal));
                let ty = var.ty.to_rust_tokens(&state.opts.opts);
                let vdoc = var.comments.doc_comment();

                if matches!(
                    var.ty,
                    crate::declare::DeclType::Builtin {
                        ty: crate::declare::Builtin::Never
                    }
                ) {
                    quote!(#vdoc #iden,)
                } else {
                    quote!(#vdoc #iden(#ty),)
                }
            })
            .collect();

        tt.extend(quote! {
            #[derive(serde::Serialize, serde::Deserialize, kintsu_sdk::OneOf)]
            #[serde(untagged)]
            #[fields(version = #version)]
            #doc_comment
            pub enum #name {
                #fields
            }
        });

        state.with_file_handle(ns_file, |w| {
            write!(w, "{tt}")?;
            Ok(())
        })?;
        Ok(())
    }

    fn gen_decl_error(
        &self,
        state: &DeclNsContext<'_, RustGenState, RustConfig, Self>,
        def: &DeclError,
    ) -> crate::generate::Result<()> {
        let ns_file = state.ns_file();
        let mut tt = quote!();

        let name = ident(def.name.to_case(convert_case::Case::Pascal));
        let doc_comment = def.comments.doc_comment();
        let version = def.meta.version_lit();

        let variants: TokenStream = def
            .variants
            .iter()
            .map(|var| {
                let iden = ident(var.name.to_case(convert_case::Case::Pascal));
                let ty = var.ty.to_rust_tokens(&state.opts.opts);
                let vdoc = var.comments.doc_comment();
                quote! { #vdoc #iden(#ty), }
            })
            .collect();

        tt.extend(quote! {
            #[derive(serde::Serialize, serde::Deserialize, kintsu_sdk::Error)]
            #[fields(version = #version)]
            #doc_comment
            #[serde(tag = "type", rename_all = "snake_case")]
            pub enum #name {
                #variants
            }
        });

        state.with_file_handle(ns_file, |w| {
            write!(w, "{tt}")?;
            Ok(())
        })?;
        Ok(())
    }
}
