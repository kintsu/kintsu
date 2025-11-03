use kintsu_sdk::{Enum, Error, Struct, namespace};

include!("./shared.rs");

#[derive(Enum, serde::Serialize, serde::Deserialize)]
#[fields(version = 1)]
pub enum ErrorCode {
    Foo = 1,
    Baz = 2,
}

#[derive(Struct, serde::Serialize, serde::Deserialize)]
#[fields(version = 1)]
pub struct KnownError {
    code: ErrorCode,
    desc: String,
}

#[derive(Error, serde::Serialize, serde::Deserialize)]
#[fields(version = 1)]
pub enum OperationError {
    Unknown {
        /// some nested doc
        desc: String,
    },
    Known(KnownError),
}

namespace! {
    "abc.corp.test" {
        KnownError, ErrorCode, OperationError, OperationErrorUnknown
    }
}

#[test]
fn smoke_error_code() {
    smoke_basic::<ErrorCode, _>("../samples/test-error-code.toml", |ser| {
        kintsu_testing::assert_yaml_snapshot!(ser)
    })
}

#[test]
fn smoke_error_unknown() {
    smoke_basic::<OperationErrorUnknown, _>(
        "../samples/test-struct-operation-error-unknown.toml",
        |ser| kintsu_testing::assert_yaml_snapshot!(ser),
    )
}

#[test]
fn smoke_known_error() {
    smoke_basic::<KnownError, _>("../samples/test-struct-known-error.toml", |ser| {
        kintsu_testing::assert_yaml_snapshot!(ser)
    })
}

#[test]
fn smoke_basic_error() {
    smoke_basic::<OperationError, _>("../samples/test-operation-error.toml", |ser| {
        kintsu_testing::assert_yaml_snapshot!(ser)
    })
}
