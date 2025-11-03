mod compiler_test;

#[proc_macro]
pub fn compiler_test(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compiler_test::compiler_test(stream.into())
}
