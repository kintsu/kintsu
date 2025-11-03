use crate::Typed;

#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum ProtoResult<T, E> {
    Ok(T),
    Err(E),
}

impl<T: Typed, E: Typed> From<Result<T, E>> for ProtoResult<T, E> {
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(ok) => Self::Ok(ok),
            Err(err) => Self::Err(err),
        }
    }
}

impl<T: Typed, E: Typed> ProtoResult<T, E> {}

// impl<T: Typed, E: Typed, This: Into<E>> From<This> for ProtoResult<T, This> {
//     fn from(value: This) -> Self {

//     }
// }

// pub trait DefinedResult: serde::Serialize + crate::Defined {
//     type Error: serde::Serialize + crate::Defined;
// }

// impl<T: serde::Serialize + crate::Defined, E: Into<Err>, Err: Defined + serde::Serialize>
//     DefinedResult for Result<T, E>
// {
//     type Error = Err;
// }

// mod test {
//     use crate::protocol::DefinedResult;

//     #[derive(serde::Serialize)]
//     struct A {}

//     impl crate::Defined for A {
//         fn definition() -> &'static crate::Definitions {
//             todo!()
//         }
//     }

//     fn test_impls<T: DefinedResult>() {}

//     #[derive(Debug)]
//     enum E {}

//     enum Err {
//         D { desc: String },
//     }

//     impl Into<Err> for E {
//         fn into(self) -> Err {
//             Err::D { desc: "".into() }
//         }
//     }

//     type Result<T> = std::result::Result<T, E>;

//     #[test]
//     fn smoke() {
//         test_impls::<A>();
//         test_impls::<Result<A>>();
//     }
// }
