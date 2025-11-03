#![allow(unused)]

use kintsu_core::namespace;
use kintsu_derives::Struct;
use kintsu_sdk::Defined;

include!("./shared.rs");

#[derive(Struct)]
#[fields(version = 1)]
#[fields(describe(text = "Some struct"))]
pub struct BasicStruct {
    #[fields(describe(text = "field a"))]
    a: i32,
    b: Option<i32>,
    c: Vec<f32>,
    d: [[f32; 4]; 4],
}

#[derive(Struct)]
#[fields(version = 1)]
#[fields(describe(file = "../../samples/some-readme.md"))]
pub struct BasicStructWithReadme {
    #[fields(describe(text = "field a"))]
    a: i32,
}

namespace! {
    "abc.corp.test" {
        BasicStruct, BasicStructWithReadme
    }
}

#[test]
fn smoke_basic_with_readme() {
    smoke_basic::<BasicStructWithReadme, _>("../samples/test-struct-readme.toml", |ser| {
        kintsu_testing::assert_yaml_snapshot!(ser)
    })
}

#[test]
fn smoke_basic_with_text() {
    smoke_basic::<BasicStruct, _>("../samples/test-struct-text.toml", |ser| {
        kintsu_testing::assert_yaml_snapshot!(ser)
    })
}
