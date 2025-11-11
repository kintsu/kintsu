//! Enum declarations

use serde::{Deserialize, Serialize};

use super::comments::DeclComment;

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclEnumValueType {
    Int,
    String,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclIntVariant {
    pub name: String,
    pub value: u32,
    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclStringVariant {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "enum_type", content = "variants", rename_all = "snake_case")]
pub enum DeclEnum {
    Int(Vec<DeclIntVariant>),
    String(Vec<DeclStringVariant>),
}
