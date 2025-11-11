use serde::{Deserialize, Serialize};

use super::{comments::DeclComment, types::DeclType};

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclField {
    pub name: String,
    pub ty: DeclType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclArg {
    pub name: String,
    pub ty: DeclType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}
