#![allow(unused)]

use kintsu_core::namespace;
use kintsu_sdk::{OneOf, Struct};

include!("shared.rs");

#[derive(OneOf)]
#[fields(version = 1)]
pub enum MaybeFlagType {
    /// some doc
    Int(i32),
    Str(String),
    BoolFlag,
}

#[derive(Struct)]
#[fields(version = 1)]
pub struct StructWithOneOf {
    #[fields(one_of)]
    my_flag: MaybeFlagType,
}

namespace! {
    "abc.corp.exts" {
        MaybeFlagType, StructWithOneOf
    }
}

#[test]
fn test_one_of() {
    smoke_basic::<MaybeFlagType, _>("../samples/test-one-of.toml", |ser| {
        kintsu_testing::assert_yaml_snapshot!(ser);
    });
}

#[test]
fn test_struct_with_one_of() {
    smoke_basic::<StructWithOneOf, _>("../samples/test-struct-with-one-of.toml", |ser| {
        kintsu_testing::assert_yaml_snapshot!(ser);
    });
}
