use divan::black_box;
use kintsu_parser::{
    ast::{self, ty::Type},
    tokens::{Parse, tokenize},
};

const BENCH_STRUCT: &str = include_str!("../../parser/samples/bench_struct.ks");
const BENCH_ENUM: &str = include_str!("../../parser/samples/bench_enum.ks");
const SAMPLE_MSG: &str =
    include_str!("../../parser/samples/abc-corp/schema/foo/message_with_enum.ks");

#[inline(always)]
fn ast_parse(s: &str) {
    black_box(ast::AstStream::from_string(black_box(s)).unwrap());
}

#[divan::bench(name = "tokenize test message")]
fn tokenize_test_message() {
    black_box(tokenize(black_box(SAMPLE_MSG)).unwrap());
}

#[divan::bench(name = "ast parse struct")]
fn ast_parse_struct() {
    ast_parse(black_box(BENCH_STRUCT))
}

#[divan::bench(name = "ast parse enum")]
fn ast_parse_enum() {
    ast_parse(black_box(BENCH_ENUM))
}

#[divan::bench(args = [
    "i32",
    "f32",
    "complex",
    "never",
    "oneof i32|i64",
    "i64[][]",
    "i64[]",
    "{ a: i32, b: i64 }",
])]
fn type_parse(t: &str) {
    let tt = tokenize(black_box(t)).unwrap();
    let mut tt_clone = tt.clone();
    let parsed: Type = Type::parse(&mut tt_clone).unwrap();
    divan::black_box(parsed);
}

fn main() {
    divan::main();
}
