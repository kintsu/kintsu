mod derive_enum;
mod derive_error;
mod derive_one_of;
mod derive_struct;
mod generate_module;
mod generate_ops;
pub(crate) mod shared;

use proc_macro::TokenStream;

#[proc_macro_derive(Enum, attributes(fields))]
pub fn derive_enum(tokens: TokenStream) -> TokenStream {
    derive_enum::derive_enum(tokens.into()).into()
}

#[proc_macro_derive(Struct, attributes(fields))]
pub fn derive_struct(tokens: TokenStream) -> TokenStream {
    derive_struct::derive_struct(tokens.into()).into()
}

#[proc_macro_derive(OneOf, attributes(fields))]
pub fn derive_one_of(tokens: TokenStream) -> TokenStream {
    derive_one_of::derive_one_of(tokens.into()).into()
}

#[proc_macro_derive(Error, attributes(fields))]
pub fn derive_error(tokens: TokenStream) -> TokenStream {
    derive_error::derive_error(tokens.into()).into()
}

#[proc_macro_attribute]
pub fn module(
    attr: TokenStream,
    tokens: TokenStream,
) -> TokenStream {
    generate_module::generate_module(attr.into(), tokens.into()).into()
}

#[allow(unused)]
#[proc_macro_attribute]
pub fn operation(
    attr: TokenStream,
    tokens: TokenStream,
) -> TokenStream {
    let atts: crate::generate_ops::Atts = syn::parse2(attr.into()).unwrap();
    let ops: crate::generate_ops::Op = syn::parse2(tokens.into()).unwrap();

    quote::quote! {#ops}.into()
}

mod internal {
    macro_rules! call_span {
        ($op: expr) => {
            match $op {
                Ok(v) => v,
                Err(e) => return e.to_compile_error(),
            }
        };
        ($op: expr; $err: expr) => {
            match $op {
                Ok(v) => v,
                Err(e) => return ($err)(e).to_compile_error(),
            }
        };
        (@opt $op: expr; $err: expr) => {
            match $op {
                Some(v) => v,
                None => return $err.to_compile_error(),
            }
        };
    }

    macro_rules! resolve_defs {
        ($t: ty) => {
            impl $t {
                pub fn resolve_defs(&mut self) -> syn::Result<()> {
                    $crate::shared::assign_maybe_docs(&self.attrs, &mut self.describe)?;
                    Ok(())
                }
            }
        };
        ($($t: ty), + $(,)?)=>{
            $(
                $crate::resolve_defs!($t);
            )*
        }
    }

    pub(crate) use call_span;
    pub(crate) use resolve_defs;
}

pub(crate) use internal::{call_span, resolve_defs};
