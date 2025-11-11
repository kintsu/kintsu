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
#[sea_orm(table_name = "org")]
#[schema(as = Org)]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub name: String,
    #[sea_orm(unique)]
    pub gh_id: i32,
    pub gh_avatar: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::api_key::Entity")]
    ApiKey,
    #[sea_orm(has_many = "super::org_invitation::Entity")]
    OrgInvitation,
    #[sea_orm(has_many = "super::org_role::Entity")]
    OrgRole,
    #[sea_orm(has_many = "super::schema_role::Entity")]
    SchemaRole,
    #[sea_orm(has_many = "super::user_favourite::Entity")]
    UserFavourite,
    #[sea_orm(has_many = "super::version::Entity")]
    Version,
}

impl Related<super::api_key::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApiKey.def()
    }
}

impl Related<super::org_invitation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrgInvitation.def()
    }
}

impl Related<super::org_role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrgRole.def()
    }
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

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        super::org_role::Relation::Users.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::org_role::Relation::Org.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
