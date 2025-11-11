use super::api_key::Entity as ApiKeyFull;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DerivePartialModel, Eq, utoipa::ToSchema, serde::Serialize)]
#[sea_orm(entity = "ApiKeyFull")]
#[schema(as = ApiKey)]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub description: Option<String>,
    pub expires: crate::DateTime,
    #[schema(value_type = Vec<super::types::Scope>)]
    pub scopes: Vec<String>,
    pub permissions: Vec<super::types::Permission>,
    pub user_id: Option<i64>,
    pub org_id: Option<i64>,
    pub last_used_at: Option<crate::DateTime>,
    pub revoked_at: Option<crate::DateTime>,
}
