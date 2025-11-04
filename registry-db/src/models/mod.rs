pub mod api_key;
pub mod org;
pub mod org_admin;
pub mod package;
pub mod schema_admin;
pub mod scopes;
pub mod user;
pub mod version;

#[derive(Debug, serde::Serialize, Clone)]
#[serde(tag = "type", content = "entity")]
pub enum Entity {
    User(user::User),
    Org(org::Org),
}

impl Entity {
    pub fn id(&self) -> i64 {
        match self {
            Entity::User(user) => user.id,
            Entity::Org(org) => org.id,
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Page {
    pub number: i64,
    pub size: i64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub page: Page,
    pub next_page: Option<i64>,
    pub total_items: i64,
    pub total_pages: i64,
}
