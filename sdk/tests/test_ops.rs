use kintsu_sdk::operation;

use crate::ns::ErrorDesc;

mod ns {
    use kintsu_sdk::{Enum, Error as DeriveError, Struct, namespace};

    #[derive(Enum, kintsu_sdk::IntDeserialize, kintsu_sdk::IntSerialize)]
    #[fields(version = 1)]
    #[repr(u64)]
    pub enum ErrorCode {
        Unknown,
        Internal,
    }

    #[derive(Struct, serde::Serialize, serde::Deserialize)]
    #[fields(version = 1)]
    pub struct ErrorDesc {
        pub code: ErrorCode,
        pub desc: String,
    }

    #[allow(unused)]
    #[derive(DeriveError, serde::Serialize, serde::Deserialize)]
    #[serde(untagged, rename_all = "snake_case")]
    #[fields(version = 1)]
    pub enum ServerError {
        Described(ErrorDesc),
    }

    namespace! {
        "abc.test.namespace" {
            ErrorCode, ErrorDesc, ServerError
        }
    }
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<Error> for ns::ServerError {
    fn from(value: Error) -> Self {
        match value {
            Error::Io(io) => {
                eprintln!("{io:#?}");
                Self::Described(ErrorDesc {
                    code: ns::ErrorCode::Internal,
                    desc: "internal server error".into(),
                })
            },
        }
    }
}

#[operation(version = 1, describe(text = "sums values in a list of integers"))]
fn sum(values: Vec<i32>) -> i32 {
    values.iter().sum()
}

#[operation(version = 1)]
/// this is a test description
fn with_lt(value: &str) -> &str {
    value
}

#[operation(version = 1)]
#[allow(unused)]
fn with_return() -> Result<(), Error> {
    Ok(())
}

#[test]
fn test() {
    sum(vec![1, 2, 3]);
    with_lt("abc");
}
