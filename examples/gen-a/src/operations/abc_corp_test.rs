#[derive(kintsu_sdk :: Enum)]
#[fields(version = 1)]
#[derive(kintsu_sdk :: IntDeserialize, kintsu_sdk :: IntSerialize)]
#[repr(u64)]
#[doc = "some int based enum"]
pub enum BasicIntEnum {
    A = 0,
    #[doc = "B is 99"]
    B = 99,
}
#[derive(kintsu_sdk :: Enum)]
#[fields(version = 1)]
#[derive(serde :: Serialize, serde :: Deserialize)]
pub enum BasicStrEnum {
    #[doc = "some doc"]
    #[fields(str_value = "a")]
    #[serde(rename = "a")]
    A,
    #[fields(str_value = "b")]
    #[serde(rename = "b")]
    B,
}
#[derive(kintsu_sdk :: Enum)]
#[fields(version = 1)]
#[derive(kintsu_sdk :: IntDeserialize, kintsu_sdk :: IntSerialize)]
#[repr(u64)]
pub enum ErrorCode {
    Baz = 2,
    Foo = 1,
}
#[derive(kintsu_sdk :: Enum)]
#[fields(version = 1)]
#[derive(kintsu_sdk :: IntDeserialize, kintsu_sdk :: IntSerialize)]
#[repr(u64)]
#[doc = "some doc"]
pub enum SomeEnum {
    A = 0,
}
#[derive(serde :: Serialize, serde :: Deserialize, kintsu_sdk :: Struct)]
#[fields(version = 1)]
#[doc = "Some struct"]
pub(crate) struct BasicStruct {
    #[serde(rename = "a")]
    #[doc = "field a"]
    pub(crate) a: i32,
    #[serde(rename = "b")]
    pub(crate) b: Option<i32>,
    #[serde(rename = "c")]
    pub(crate) c: Vec<f32>,
    #[serde(rename = "d")]
    pub(crate) d: [[f32; 4]; 4],
}
#[derive(serde :: Serialize, serde :: Deserialize, kintsu_sdk :: Struct)]
#[fields(version = 1)]
#[doc = "# test\nthis is a struct description\n"]
pub struct BasicStructWithReadme {
    #[serde(rename = "a")]
    #[doc = "field a"]
    pub a: i32,
}
#[derive(serde :: Serialize, serde :: Deserialize, kintsu_sdk :: Struct)]
#[fields(version = 1)]
pub struct KnownError {
    #[serde(rename = "code")]
    #[fields(enm)]
    pub code: ErrorCode,
    #[serde(rename = "desc")]
    pub desc: String,
}
#[derive(serde :: Serialize, serde :: Deserialize, kintsu_sdk :: Struct)]
#[fields(version = 1)]
pub struct OperationErrorUnknown {
    #[serde(rename = "desc")]
    #[doc = "some nested doc"]
    pub desc: String,
}
#[derive(serde :: Serialize, serde :: Deserialize, kintsu_sdk :: Struct)]
#[fields(version = 1)]
pub struct SomeStructWithEnum {
    #[serde(rename = "enum_value")]
    #[doc = "field doc"]
    #[fields(enm)]
    pub enum_value: SomeEnum,
}
#[derive(serde :: Serialize, serde :: Deserialize, kintsu_sdk :: Error)]
#[fields(version = 1)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OperationError {
    Known(KnownError),
    Unknown(OperationErrorUnknown),
}
kintsu_sdk::namespace! { "abc.corp.test" { BasicStruct , BasicStructWithReadme , KnownError , OperationErrorUnknown , SomeStructWithEnum , BasicIntEnum , BasicStrEnum , ErrorCode , SomeEnum , OperationError , } }
