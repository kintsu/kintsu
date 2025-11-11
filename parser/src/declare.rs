pub mod comments;
pub mod context;
pub mod definitions;
pub mod enums;
pub mod fields;
pub mod meta;
pub mod namespace;
pub mod root;
pub mod types;

mod convert;

pub use comments::DeclComment;
pub use context::{DeclNamedItemContext, DeclRefContext};
pub use definitions::{
    DeclEnumDef, DeclError, DeclOneOf, DeclOneOfVariant, DeclOperation, DeclStruct, DeclTypeAlias,
    TypeDefinition,
};
pub use enums::{DeclEnum, DeclEnumValueType, DeclIntVariant, DeclStringVariant};
pub use fields::{DeclArg, DeclField};
pub use meta::Meta;
pub use namespace::DeclNamespace;
pub use root::TypeRegistryDeclaration;
pub use types::{Builtin, DeclType};

#[cfg_attr(feature = "db", derive(sea_orm::prelude::FromJsonQueryResult))]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Clone)]
#[serde(tag = "version", content = "declarations", rename_all = "lowercase")]
pub enum DeclarationVersion {
    V1(TypeRegistryDeclaration),
}

impl Eq for DeclarationVersion {}
