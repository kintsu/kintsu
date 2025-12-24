pub mod api_key;
pub mod authorization;
pub mod downloads;
pub mod events;
pub mod favourites;
pub mod fluent;
pub mod org;
pub mod org_invite;
pub mod package;
pub mod principal;
pub mod schema_admin;
pub mod schema_role;
pub mod user;
pub mod version;

pub use api_key::*;
pub use authorization::*;
pub use events::*;
pub use favourites::*;
pub use fluent::*;
pub use org::*;
pub use package::*;
pub use principal::*;
use serde::Deserialize;

use crate::entities::{Org, User};

#[derive(Debug, serde::Serialize, Clone, utoipa::ToSchema)]
#[serde(tag = "type", content = "entity", rename_all = "snake_case")]
pub enum Entity {
    User(User),
    Org(Org),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, validator::Validate, Debug)]
pub struct Page {
    #[validate(range(min = 1))]
    pub number: i64,
    #[validate(range(min = 1, max = 100))]
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

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PackageOrderingField {
    Name,
    DownloadCount,
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy)]
pub struct PackageOrdering {
    pub field: PackageOrderingField,
    pub direction: OrderDirection,
}
