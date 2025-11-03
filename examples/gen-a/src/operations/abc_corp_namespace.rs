#[derive(serde :: Serialize, serde :: Deserialize, kintsu_sdk :: Struct)]
#[fields(version = 2)]
pub struct SomeStruct {
    #[serde(rename = "a")]
    pub a: i32,
    #[serde(rename = "b")]
    pub b: f32,
    #[serde(rename = "c")]
    pub c: [[f32; 4]; 4],
    #[serde(rename = "d")]
    pub d: [[f32; 4]; 4],
}
kintsu_sdk::namespace! { "abc.corp.namespace" { SomeStruct , } }
