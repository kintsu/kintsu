use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "api_key")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub key: crate::tokens::TokenHash,
    pub description: Option<String>,
    pub expires: crate::DateTime,
    #[sea_orm(
        column_type = "Array(std::sync::Arc::new(sea_orm::ColumnType::String(StringLen::N(32))))"
    )]
    pub scopes: Vec<String>,
    pub permissions: Vec<super::types::Permission>,
    pub user_id: Option<i64>,
    pub org_id: Option<i64>,
    pub last_used_at: Option<crate::DateTime>,
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

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
