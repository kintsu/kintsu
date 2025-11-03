#![allow(unused)]

use kintsu_core::namespace;
use kintsu_sdk::*;

include!("./shared.rs");

#[derive(kintsu_sdk::Enum, serde::Serialize)]
#[fields(version = 1)]
pub enum BasicStrEnum {
    #[fields(str_value = "a")]
    /// some doc
    A,
    #[fields(str_value = "b")]
    B,
}

#[derive(kintsu_sdk::Enum, serde::Serialize)]
#[fields(version = 1, describe(text = "some int based enum"))]
pub enum BasicIntEnum {
    A,
    #[fields(describe(text = "B is 99"))]
    B = 99,
}

namespace! {
    "abc.corp.test" {
        BasicStrEnum, BasicIntEnum,
    }
}

#[test]
fn test_basic_str_enum() {
    smoke_basic::<BasicStrEnum, _>("../samples/test-str-enum.toml", |ser| {
        kintsu_testing::assert_yaml_snapshot!(ser)
    })
}

#[test]
fn test_basic_int_enum() {
    smoke_basic::<BasicIntEnum, _>("../samples/test-enum.toml", |ser| {
        kintsu_testing::assert_yaml_snapshot!(ser)
    })
}
