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
#[sea_orm(table_name = "user_favourite")]
#[schema(as = UserFavourite)]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(
        unique_key = "user_package_favourite_idx",
        unique_key = "user_org_favourite_idx"
    )]
    pub user_id: i64,
    #[sea_orm(unique_key = "user_package_favourite_idx")]
    pub package_id: Option<i64>,
    #[sea_orm(unique_key = "user_org_favourite_idx")]
    pub org_id: Option<i64>,
    pub created_at: crate::DateTime,
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
        from = "Column::PackageId",
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
