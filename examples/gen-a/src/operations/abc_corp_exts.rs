#[derive(serde :: Serialize, serde :: Deserialize, kintsu_sdk :: OneOf)]
#[serde(untagged)]
#[fields(version = 1)]
pub enum MaybeFlagType {
    BoolFlag,
    #[doc = "some doc"]
    Int(i32),
    Str(String),
}
#[derive(serde :: Serialize, serde :: Deserialize, kintsu_sdk :: Struct)]
#[fields(version = 1)]
pub struct CommentWithTestChrono {
    #[serde(rename = "comment")]
    pub comment: String,
    #[serde(rename = "created_at")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "deleted_at")]
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "id")]
    pub id: i64,
}
#[derive(serde :: Serialize, serde :: Deserialize, kintsu_sdk :: Struct)]
#[fields(version = 1)]
pub struct CommentWithTestTime {
    #[serde(rename = "comment")]
    pub comment: String,
    #[serde(rename = "created_at")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "deleted_at")]
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "id")]
    pub id: i64,
}
#[derive(serde :: Serialize, serde :: Deserialize, kintsu_sdk :: Struct)]
#[fields(version = 1)]
pub struct StructWithOneOf {
    #[serde(rename = "my_flag")]
    #[fields(one_of)]
    pub my_flag: MaybeFlagType,
}
kintsu_sdk::namespace! { "abc.corp.exts" { CommentWithTestChrono , CommentWithTestTime , StructWithOneOf , MaybeFlagType , } }
