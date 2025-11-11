//! Type definition declarations

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{
    comments::DeclComment,
    context::DeclNamedItemContext,
    enums::DeclEnum,
    fields::{DeclArg, DeclField},
    meta::Meta,
    types::DeclType,
};

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclStruct {
    pub name: String,
    pub fields: Vec<DeclField>,
    pub meta: Meta,
    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclEnumDef {
    pub name: String,
    pub enum_def: DeclEnum,
    pub meta: Meta,
    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclOneOfVariant {
    pub name: String,
    pub ty: DeclType,
    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclOneOf {
    pub name: String,
    pub variants: Vec<DeclOneOfVariant>,
    pub meta: Meta,
    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclTypeAlias {
    pub name: String,
    pub target: DeclType,
    pub meta: Meta,
    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclError {
    pub name: String,
    pub variants: Vec<DeclOneOfVariant>,
    pub meta: Meta,
    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclOperation {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<DeclArg>,
    pub return_type: DeclType,
    pub meta: Meta,
    #[serde(default, skip_serializing_if = "DeclComment::is_empty")]
    pub comments: DeclComment,
}

#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "definition_type", rename_all = "snake_case")]
pub enum TypeDefinition {
    Struct(DeclStruct),
    Enum(DeclEnumDef),
    OneOf(DeclOneOf),
    TypeAlias(DeclTypeAlias),
    Error(DeclError),
    Operation(DeclOperation),
}

impl TypeDefinition {
    pub fn name(&self) -> &str {
        match self {
            Self::Struct(s) => &s.name,
            Self::Enum(e) => &e.name,
            Self::OneOf(o) => &o.name,
            Self::TypeAlias(t) => &t.name,
            Self::Error(e) => &e.name,
            Self::Operation(o) => &o.name,
        }
    }

    pub fn meta(&self) -> &Meta {
        match self {
            Self::Struct(s) => &s.meta,
            Self::Enum(e) => &e.meta,
            Self::OneOf(o) => &o.meta,
            Self::TypeAlias(t) => &t.meta,
            Self::Error(e) => &e.meta,
            Self::Operation(o) => &o.meta,
        }
    }

    pub fn collect_external_refs(
        &self,
        root_package: &str,
        refs: &mut HashSet<DeclNamedItemContext>,
    ) {
        match self {
            Self::Struct(s) => {
                for field in &s.fields {
                    field
                        .ty
                        .collect_external_refs(root_package, refs);
                }
            },
            Self::OneOf(o) => {
                for variant in &o.variants {
                    variant
                        .ty
                        .collect_external_refs(root_package, refs);
                }
            },
            Self::TypeAlias(t) => {
                t.target
                    .collect_external_refs(root_package, refs);
            },
            Self::Error(e) => {
                for variant in &e.variants {
                    variant
                        .ty
                        .collect_external_refs(root_package, refs);
                }
            },
            Self::Operation(o) => {
                for arg in &o.args {
                    arg.ty
                        .collect_external_refs(root_package, refs);
                }
                o.return_type
                    .collect_external_refs(root_package, refs);
            },
            Self::Enum(_) => {
                // Enums have no type references
            },
        }
    }
}
