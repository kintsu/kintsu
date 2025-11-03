use syn::parse::Parse;

struct Opts {
    ignore: Option<syn::LitStr>,
    id: syn::Ident,
    name: syn::LitStr,
    purpose: syn::LitStr,
    expect_pass: bool,
    tags: syn::Expr,
    root: syn::LitStr,
    memory: syn::Expr,
    assertions: syn::ExprClosure,
}

impl Parse for Opts {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut ignore = None;
        let mut id = None;
        let mut name = None;
        let mut purpose = None;
        let mut expect_pass = None;
        let mut tags = None;
        let mut root = None;
        let mut memory = None;
        let mut assertions = None;

        loop {
            let field = input.parse::<syn::Ident>()?;
            input.parse::<syn::Token![:]>()?;
            match field.to_string().as_str() {
                "id" => {
                    id = Some(input.parse::<syn::Ident>()?);
                },
                "ignore" => {
                    ignore = Some(input.parse::<syn::LitStr>()?);
                },
                "name" => {
                    name = Some(input.parse::<syn::LitStr>()?);
                },
                "purpose" => {
                    purpose = Some(input.parse::<syn::LitStr>()?);
                },
                "expect_pass" => {
                    let lit: syn::LitBool = input.parse()?;
                    expect_pass = Some(lit.value);
                },
                "tags" => {
                    let expr: syn::Expr = input.parse()?;
                    tags = Some(expr);
                },
                "root" => {
                    root = Some(input.parse::<syn::LitStr>()?);
                },
                "memory" => {
                    let expr: syn::Expr = input.parse()?;
                    memory = Some(expr);
                },
                "assertions" => {
                    let closure: syn::ExprClosure = input.parse()?;
                    assertions = Some(closure);
                },
                _ => {
                    return Err(syn::Error::new(
                        field.span(),
                        format!("Unknown field: {}", field),
                    ));
                },
            }
            if !input.peek(syn::Token![,]) {
                break;
            }
            input.parse::<syn::Token![,]>()?;
            if !input.peek(syn::Ident) {
                break;
            }
        }

        Ok(Self {
            ignore,
            id: id.ok_or_else(|| syn::Error::new(input.span(), "Missing 'id' parameter"))?,
            name: name.ok_or_else(|| syn::Error::new(input.span(), "Missing 'name' parameter"))?,
            purpose: purpose
                .ok_or_else(|| syn::Error::new(input.span(), "Missing 'purpose' parameter"))?,
            expect_pass: expect_pass
                .ok_or_else(|| syn::Error::new(input.span(), "Missing 'expect_pass' parameter"))?,
            tags: tags.ok_or_else(|| syn::Error::new(input.span(), "Missing 'tags' parameter"))?,
            root: root.ok_or_else(|| syn::Error::new(input.span(), "Missing 'root' parameter"))?,
            memory: memory
                .ok_or_else(|| syn::Error::new(input.span(), "Missing 'memory' parameter"))?,
            assertions: assertions
                .ok_or_else(|| syn::Error::new(input.span(), "Missing 'assertions' parameter"))?,
        })
    }
}

pub fn compiler_test(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let opts = syn::parse_macro_input!(stream as Opts);
    let id = opts.id.to_string();
    let id_as_ident = &opts.id;
    let name = &opts.name;
    let purpose = &opts.purpose;
    let expect_pass = opts.expect_pass;
    let tags = &opts.tags;
    let root = &opts.root;
    let memory = &opts.memory;
    let assertions = &opts.assertions;

    let ignore = if let Some(ignore_msg) = opts.ignore {
        quote::quote! {
            #[ignore = #ignore_msg]
        }
    } else {
        quote::quote! {}
    };

    let handle = if expect_pass {
        quote::quote! { compile_pass }
    } else {
        quote::quote! { compile_fail }
    };

    quote::quote!(
        #ignore
        #[tokio::test]
        async fn #id_as_ident() {
            use kintsu_test_suite::TestHarness;
            use kintsu_test_suite::Tag;

            let fs = (#memory)();

            let mut harness = TestHarness::with_metadata(
                fs,
                #id,
                #name,
                #purpose,
                #expect_pass,
                #tags,
            ).with_root(#root);

            let ctx = harness.#handle().await;

            let mut assertions: Box<dyn Fn(TestHarness, _)> = Box::new(#assertions);
            (assertions)(harness, ctx);
        }
    )
    .into()
}
