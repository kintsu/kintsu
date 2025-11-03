use std::{collections::BTreeMap, io::Write, path::Path};

use convert_case::Casing;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Ident, Lit, LitInt};

use crate::{
    Contiguous, EnumValueType, ErrorTy, Operation, StrOrInt, Struct,
    generate::{
        GenOpts, Generate, LanguageTrait, RustConfig, context::WithNsContext, files::WithFlush,
    },
};

pub struct RustGenerator;

impl LanguageTrait for RustGenerator {
    fn file_case() -> convert_case::Case<'static> {
        convert_case::Case::Snake
    }

    fn file_ext() -> &'static str {
        "rs"
    }
}

pub(crate) struct RustGenState {}

impl Generate<RustGenState, RustConfig> for RustGenerator {
    #[allow(unused)]
    fn on_create(
        state: &WithNsContext<'_, RustGenState, RustConfig, Self>,
        fname: &Path,
        f: &mut Box<dyn WithFlush>,
    ) -> std::io::Result<()> {
        Ok(())
    }

    #[allow(unused)]
    fn with_all_namespaces(
        &self,
        ctx: &crate::context::Context,
        opts: &GenOpts<RustConfig>,
        ctx_ns: BTreeMap<crate::Ident, WithNsContext<'_, RustGenState, RustConfig, Self>>,
    ) -> super::Result<()> {
        for ns in ctx.namespaces.values() {
            let ctx = ctx_ns.get(&ns.name).unwrap();
            let mut tt = quote!();

            let mut keys = ctx.ns.defs.keys().collect::<Vec<_>>();

            keys.append(&mut ctx.ns.enums.keys().collect());
            keys.append(&mut ctx.ns.one_ofs.keys().collect());
            keys.append(&mut ctx.ns.errors.keys().collect());
            for it in keys {
                let created = def_ident(it.clone());
                tt.extend(quote!(
                    #created,
                ))
            }
            let ns = ns.name.to_string();
            tt = quote!(kintsu_sdk::namespace! { #ns { #tt }});
            ctx.with_file_handle(ctx.ns_file(), |w| write!(w, "{tt}"))?;
        }
        Ok(())
    }

    #[allow(unused)]
    fn new_state(
        &self,
        opts: &GenOpts<RustConfig>,
    ) -> RustGenState {
        RustGenState {}
    }

    #[allow(unused)]
    fn gen_operation(
        &self,
        state: &WithNsContext<'_, RustGenState, RustConfig, Self>,
        def: &Operation,
    ) -> super::Result<()> {
        let ns_file = state.ns_file();
        let tt = quote::quote!();

        state.with_file_handle(ns_file, |w| {
            write!(w, "{tt}")?;
            Ok(())
        })?;
        Ok(())
    }

    #[allow(unused)]
    fn gen_struct(
        &self,
        state: &WithNsContext<'_, RustGenState, RustConfig, Self>,
        def: &Struct,
    ) -> super::Result<()> {
        let ns_file = state.ns_file();
        let mut tt = quote::quote!();

        let vis = state
            .opts
            .opts
            .vis
            .as_rust(state, &def.meta.name);
        let desc_comment = def.meta.doc_comment();
        let iden = def.meta.ident_as_pascal();
        let version = def.meta.version();

        let mut fields = quote!();
        for (field_name, field) in def.fields.iter() {
            let f = ident(
                field_name
                    .to_string()
                    .to_case(convert_case::Case::Snake),
            );

            let name = field_name.clone().to_string();
            let field = field.unwrap_value();
            let atts = field.ty.rust_attrs();
            fields.extend({
                let comment = field.meta.doc_comment();
                let ty = field.ty.ty(&state.opts.opts);
                quote!(
                    #[serde(rename = #name)]

                    #comment
                    #atts
                    #vis #f: #ty,
                )
            })
        }

        tt.extend(quote::quote! {
            #[derive(serde::Serialize, serde::Deserialize, kintsu_sdk::Struct)]
            #[fields(version = #version)]
            #desc_comment

            #vis struct #iden {
                #fields
            }
        });

        tracing::info!("writing {} to '{}'", def.meta.name, ns_file.display());

        state.with_file_handle(ns_file, |w| {
            write!(w, "{tt}")?;
            Ok(())
        })?;
        Ok(())
    }

    fn gen_enum(
        &self,
        state: &WithNsContext<'_, RustGenState, RustConfig, Self>,
        def: &crate::Enum,
    ) -> super::Result<()> {
        let ns_file = state.ns_file();
        let mut tt = quote!();

        let name = def.meta.ident_as_pascal();
        let vis = state
            .opts
            .opts
            .vis
            .as_rust(state, &def.meta.name);

        let doc_comment = def.meta.doc_comment();
        let version = def.meta.version();

        let inner_ty = def.variants.is_contiguous(&def.meta.name)?;
        let fields: TokenStream = def
            .variants
            .iter()
            .map(|(_, var)| {
                let mut atts = quote!();
                let value = match &var.value {
                    StrOrInt::String(str) => {
                        atts.extend(quote!(
                            #[fields(str_value = #str)]
                            #[serde(rename = #str)]
                        ));
                        quote!()
                    },
                    StrOrInt::Int(value) => {
                        let lit = lit(value.to_string());

                        quote!( = #lit )
                    },
                };
                let doc_comment = var.meta.doc_comment();
                let iden = var.meta.ident_as_pascal();
                quote! {
                    #doc_comment
                    #atts
                    #iden #value,
                }
            })
            .collect();

        let atts = match &inner_ty {
            EnumValueType::Int => {
                quote!(
                    #[derive(
                        kintsu_sdk::IntDeserialize,
                        kintsu_sdk::IntSerialize
                    )]
                    #[repr(u64)]
                )
            },
            EnumValueType::String => {
                quote!(
                    #[derive(
                        serde::Serialize, serde::Deserialize
                    )]
                )
            },
        };

        tt.extend(quote! {
            #[derive(kintsu_sdk::Enum)]
            #[fields(version = #version)]
            #atts
            #doc_comment

            #vis enum #name {
                #fields
            }
        });

        state.with_file_handle(ns_file, |w| {
            write!(w, "{tt}")?;
            Ok(())
        })?;
        Ok(())
    }

    fn gen_one_of(
        &self,
        state: &WithNsContext<'_, RustGenState, RustConfig, Self>,
        def: &crate::OneOf,
    ) -> super::Result<()> {
        let ns_file = state.ns_file();
        let mut tt = quote!();

        let vis = state
            .opts
            .opts
            .vis
            .as_rust(state, &def.meta.name);
        let name = def.meta.ident_as_pascal();
        let doc_comment = def.meta.doc_comment();
        let version = def.meta.version();

        let fields: TokenStream = def
            .variants
            .iter()
            .map(|(iden, var)| {
                let iden_pascal = ident(
                    iden.to_string()
                        .to_case(convert_case::Case::Pascal),
                );
                let ty = var.ty.ty(&state.opts.opts);
                let vdoc = crate::generate::rust::comment(&var.description);
                if matches!(var.ty, crate::ty::Type::Never) {
                    quote!( #vdoc #iden_pascal, )
                } else {
                    quote!( #vdoc #iden_pascal(#ty), )
                }
            })
            .collect();

        tt.extend(quote! {
            #[derive(serde::Serialize, serde::Deserialize, kintsu_sdk::OneOf)]
            #[serde(untagged)]
            #[fields(version = #version)]
            #doc_comment

            #vis enum #name {
                #fields
            }
        });

        state.with_file_handle(ns_file, |w| {
            write!(w, "{tt}")?;
            Ok(())
        })?;
        Ok(())
    }

    fn gen_error(
        &self,
        state: &WithNsContext<'_, RustGenState, RustConfig, Self>,
        def: &ErrorTy,
    ) -> super::Result<()> {
        let ns_file = state.ns_file();
        let mut tt = quote!();

        let vis = state
            .opts
            .opts
            .vis
            .as_rust(state, &def.meta.name);
        let name = def.meta.ident_as_pascal();
        let doc_comment = def.meta.doc_comment();
        let version = def.meta.version();

        let variants: proc_macro2::TokenStream = def
            .variants
            .iter()
            .map(|(ident, variant)| {
                let name = crate::generate::rust::ident(
                    ident
                        .to_string()
                        .to_case(convert_case::Case::Pascal),
                );
                let ty = variant.ty.ty(&state.opts.opts);
                let variant = &variant;
                let vdoc = crate::generate::rust::comment(&variant.description);
                quote! { #vdoc #name(#ty), }
            })
            .collect();

        tt.extend(quote! {
            #[derive(serde::Serialize, serde::Deserialize, kintsu_sdk::Error)]
            #[fields(version = #version)]
            #doc_comment
            #[serde(tag = "type", rename_all = "snake_case")]
            #vis enum #name {
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

fn def_ident(def: crate::Ident) -> Ident {
    ident(
        def.to_string()
            .to_case(convert_case::Case::Pascal),
    )
}

pub(crate) fn ident<D: AsRef<str>>(s: D) -> Ident {
    Ident::new(s.as_ref(), proc_macro2::Span::call_site())
}

pub(crate) fn lit(value: String) -> TokenStream {
    let lit = Lit::Int(LitInt::new(&value, proc_macro2::Span::call_site()));
    quote! {#lit}
}

pub(crate) fn comment<T: ToTokens>(desc: &Option<T>) -> proc_macro2::TokenStream {
    match desc {
        Some(desc) => {
            quote!(
                #[doc = #desc]
            )
        },
        None => quote!(),
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_ident() {
        super::ident("SomeStruct");
    }
}
