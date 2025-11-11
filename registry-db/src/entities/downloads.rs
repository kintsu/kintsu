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
#[sea_orm(table_name = "downloads")]
#[schema(as = Downloads)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub version: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub day: Date,
    pub count: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::version::Entity",
        from = "Column::Version",
        to = "super::version::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Version,
}

impl Related<super::version::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Version.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
