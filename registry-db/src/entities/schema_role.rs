use super::types::SchemaRoleType;
use sea_orm::entity::prelude::*;

#[derive(
    Clone,
    Debug,
    PartialEq,
    DeriveEntityModel,
    Eq,
    utoipa :: ToSchema,
    serde :: Serialize,
    serde :: Deserialize,
)]
#[sea_orm(table_name = "schema_role")]
#[schema(as = SchemaRole)]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(
        unique_key = "schema_user_roles_idx",
        unique_key = "schema_org_roles_idx"
    )]
    pub package: i64,
    #[sea_orm(unique_key = "schema_user_roles_idx")]
    pub user_id: Option<i64>,
    #[sea_orm(unique_key = "schema_org_roles_idx")]
    pub org_id: Option<i64>,
    pub role: SchemaRoleType,
    pub revoked_at: Option<crate::DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::org::Entity",
        from = "Column::OrgId",
        to = "super::org::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Org,
    #[sea_orm(
        belongs_to = "super::package::Entity",
        from = "Column::Package",
        to = "super::package::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Package,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Users,
}

impl Related<super::org::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Org.def()
    }
}

impl Related<super::package::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Package.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
