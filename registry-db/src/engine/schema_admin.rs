use serde::Serialize;

use crate::entities::SchemaRole;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SchemaRoleWithEntity {
    #[serde(flatten)]
    pub privileges: SchemaRole,
    #[serde(flatten)]
    pub entity: super::Entity,
    pub is_admin: bool,
}
