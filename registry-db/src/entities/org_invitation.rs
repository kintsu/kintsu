use super::types::OrgRoleType;
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
#[schema(as = OrgInvitation)]
#[sea_orm(table_name = "org_invitation")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub org_id: i64,
    pub inviting_user_id: i64,
    pub invited_user_gh_login: String,
    pub role: OrgRoleType,
    pub created_at: crate::DateTime,
    pub accepted_at: Option<crate::DateTime>,
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
        from = "Column::InvitingUserId",
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
