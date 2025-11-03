pub fn main() {
    #[cfg(feature = "emit")]
    {
        kintsu_parser::tokens::toks::emit_syntax("syntax.json");
    }
    #[cfg(not(feature = "emit"))]
    panic!("cannot emit without feature enabled")
}
