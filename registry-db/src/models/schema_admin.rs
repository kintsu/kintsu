use crate::schema::schema_admin;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

#[derive(Debug, Insertable, Associations, Identifiable, Clone, Serialize, utoipa::ToSchema)]
#[diesel(
    table_name = schema_admin,
    check_for_backend(diesel::pg::Pg),
    primary_key(id),
    belongs_to(crate::models::org::Org, foreign_key = org_id),
    belongs_to(crate::models::user::User, foreign_key = user_id),

    belongs_to(crate::models::package::Package, foreign_key = package),
)]
pub struct SchemaAdmin {
    pub id: i64,
    pub package: i64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,

    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SchemaWithEntity {
    #[serde(flatten)]
    pub privileges: SchemaAdmin,
    pub admin: super::Entity,
}

#[derive(Debug, Insertable, Clone, Serialize)]
#[diesel(
    table_name = schema_admin,
    check_for_backend(diesel::pg::Pg),
)]
pub struct NewSchemaAdmin {
    pub package: i64,
    pub org_id: Option<i64>,
    pub user_id: Option<i64>,
}
