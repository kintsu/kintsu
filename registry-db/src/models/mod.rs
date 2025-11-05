pub mod api_key;
pub mod org;
pub mod org_admin;
pub mod package;
pub mod schema_admin;
pub mod scopes;
pub mod user;
pub mod version;

#[derive(Debug, serde::Serialize, Clone, utoipa::ToSchema)]
#[serde(tag = "type", content = "entity", rename_all = "snake_case")]
pub enum Entity {
    User(user::User),
    Org(org::Org),
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum OwnerId {
    User(i64),
    Org(i64),
}

impl OwnerId {
    pub fn user_id(&self) -> Option<i64> {
        match self {
            OwnerId::User(id) => Some(*id),
            OwnerId::Org(_) => None,
        }
    }

    pub fn org_id(&self) -> Option<i64> {
        match self {
            OwnerId::User(_) => None,
            OwnerId::Org(id) => Some(*id),
        }
    }
}

impl Entity {
    pub fn id(&self) -> i64 {
        match self {
            Entity::User(user) => user.id,
            Entity::Org(org) => org.id,
        }
    }

    pub fn owner_id(&self) -> OwnerId {
        match self {
            Entity::User(user) => OwnerId::User(user.id),
            Entity::Org(org) => OwnerId::Org(org.id),
        }
    }

    pub fn gh_id(&self) -> i32 {
        match self {
            Entity::User(user) => user.gh_id,
            Entity::Org(org) => org.gh_id,
        }
    }

    pub fn login(&self) -> &str {
        match self {
            Entity::User(user) => &user.gh_login,
            Entity::Org(org) => &org.name,
        }
    }
}

use diesel::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Page {
    pub number: i64,
    pub size: i64,
}

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub page: Page,
    pub next_page: Option<i64>,
    pub total_items: i64,
    pub total_pages: i64,
}
