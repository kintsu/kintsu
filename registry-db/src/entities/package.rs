use sea_orm::entity::prelude::*;

#[derive(
    Clone,
    Debug,
    PartialEq,
    DeriveEntityModel,
    Eq,
    utoipa::ToSchema,
    serde::Serialize,
    serde::Deserialize,
)]
#[schema(as = Package)]
#[sea_orm(table_name = "package")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::schema_role::Entity")]
    SchemaRole,
    #[sea_orm(has_many = "super::user_favourite::Entity")]
    UserFavourite,
    #[sea_orm(has_many = "super::version::Entity")]
    Version,
}

impl Related<super::schema_role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SchemaRole.def()
    }
}

impl Related<super::user_favourite::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserFavourite.def()
    }
}

impl Related<super::version::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Version.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
