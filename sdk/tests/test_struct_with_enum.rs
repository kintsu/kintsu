#![allow(unused)]

use kintsu_core::namespace;

include!("./shared.rs");

#[derive(kintsu_sdk::Enum)]
#[fields(version = 1)]
/// some doc
enum SomeEnum {
    A,
}

#[derive(kintsu_sdk::Struct)]
#[fields(version = 1)]
struct SomeStructWithEnum {
    #[fields(enm)]
    /// field doc
    enum_value: SomeEnum,
}

namespace! {
    "abc.corp.test" {
        SomeEnum, SomeStructWithEnum,
    }
}

#[test]
fn test_some_struct_with_enum() {
    smoke_basic::<SomeStructWithEnum, _>("../samples/test-struct-with-enum.toml", |ser| {
        kintsu_testing::assert_yaml_snapshot!(ser)
    })
}

#[test]
fn test_some_enum_in_struct() {
    smoke_basic::<SomeEnum, _>("../samples/test-enum-in-struct.toml", |ser| {
        kintsu_testing::assert_yaml_snapshot!(ser)
    })
}
