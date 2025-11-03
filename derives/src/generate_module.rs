use std::path::PathBuf;

use kintsu_core::generate::{Generation, GenerationConfig, files::MemCollector};
use kintsu_manifests::NewForConfig;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, LitStr, Visibility, braced, parse::Parse};

use crate::shared::ident;

use crate::call_span;

struct Attributes {
    src: PathBuf,
}

impl Parse for Attributes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path: LitStr = input
            .parse()
            .unwrap_or_else(|_| LitStr::new("./", Span::call_site()));
        Ok(Self {
            src: path.value().into(),
        })
    }
}

struct Module {
    vis: Visibility,
    name: Ident,
    contents: TokenStream,
}

impl Parse for Module {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let vis = match input.parse() {
            Ok(vis) => vis,
            Err(_) => Visibility::Inherited,
        };
        let _: syn::token::Mod = input.parse()?;

        let name = input.parse()?;
        let tokens;
        braced!(tokens in input);
        let contents = tokens.parse()?;
        Ok(Self {
            vis,
            name,
            contents,
        })
    }
}

pub fn generate_module(
    attr: TokenStream,
    tokens: TokenStream,
) -> TokenStream {
    let atts: Attributes = call_span!(syn::parse2(attr));
    let module: Module = call_span!(syn::parse2(tokens));
    let src = call_span!(
        std::env::current_dir(); |e| syn::Error::new(
            Span::call_site(),
            format!("cwd error: {e}"),
        )
    )
    .join(atts.src);

    let mut conf = match GenerationConfig::new(Some(src.display().to_string().as_str())) {
        Ok(c) => c,
        Err(e) => {
            let err: syn::Result<()> = Err(syn::Error::new(
                Span::call_site(),
                format!("generation config error: {e}"),
            ));
            call_span!(err);
            unreachable!();
        },
    };

    conf.set_mem(true);

    let generation = match Generation::new(conf) {
        Ok(g) => g,
        Err(e) => {
            let err: syn::Result<()> = Err(syn::Error::new(
                Span::call_site(),
                format!("context error: {e}"),
            ));
            call_span!(err);
            unreachable!();
        },
    };

    let collector = MemCollector::new();

    if let Err(e) = generation.generate_all_sync(Some(collector.mem_flush())) {
        let err: syn::Result<()> = Err(syn::Error::new(
            Span::call_site(),
            format!("generate error: {e}"),
        ));
        call_span!(err);
    }

    let generated = collector.files();

    let mut outputs = quote!();

    for (entry, data) in generated.iter() {
        let data = String::from_utf8_lossy(data);
        // we can use the file name as the mod name as we already converted to form when generating
        let Some(file_name) = entry.file_name() else {
            let err: syn::Result<()> = Err(syn::Error::new(
                Span::call_site(),
                "generated file path missing file_name",
            ));
            call_span!(err);
            unreachable!();
        };
        let Some(file_name) = file_name.to_str() else {
            let err: syn::Result<()> = Err(syn::Error::new(
                Span::call_site(),
                "generated file name not valid UTF-8",
            ));
            call_span!(err);
            unreachable!();
        };
        let mod_name = ident(file_name.replace(".rs", ""));
        let mod_data: TokenStream = call_span!(syn::parse_str(&data));
        outputs.extend(quote! {
            pub mod #mod_name {
                #mod_data
            }
        })
    }

    let vis = module.vis;
    let mod_name = module.name;
    let contents = module.contents;
    quote! {
        #vis mod #mod_name {
            #outputs
            #contents
        }
    }
}
