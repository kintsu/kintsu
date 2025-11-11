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
#[sea_orm(table_name = "version")]
#[schema(as = Version)]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique_key = "package_version_idx")]
    pub package: i64,
    #[sea_orm(unique_key = "package_version_idx")]
    #[schema(value_type = String, example = "0.2.0rc1")]
    pub qualified_version: kintsu_manifests::version::Version,
    pub source_checksum: String,
    pub declarations_checksum: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: String,
    pub license_text: String,
    pub readme: String,
    pub repository: String,
    pub dependencies: Vec<i64>,
    pub keywords: Vec<String>,
    pub created_at: crate::DateTime,
    pub yanked_at: Option<crate::DateTime>,
    pub publishing_org_id: Option<i64>,
    pub publishing_user_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::downloads::Entity")]
    Downloads,
    #[sea_orm(
        belongs_to = "super::org::Entity",
        from = "Column::PublishingOrgId",
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
        from = "Column::PublishingUserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Users,
}

impl Related<super::downloads::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Downloads.def()
    }
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
