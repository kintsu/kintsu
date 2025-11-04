use super::scopes::Scope;
use crate::{models::scopes::Permission, schema::api_key};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

#[derive(Insertable, Debug)]
#[diesel(
    table_name = api_key,
    check_for_backend(diesel::pg::Pg)
)]
pub struct NewApiKey {
    pub description: Option<String>,

    pub key: crate::tokens::TokenHash,

    pub expires: DateTime<Utc>,

    pub scopes: Vec<Scope>,
    pub permissions: Vec<Permission>,

    pub user_id: Option<i64>,
    pub org_id: Option<i64>,
}

impl NewApiKey {
    fn new(
        description: Option<String>,
        scopes: Vec<Scope>,
        permissions: Vec<Permission>,
        expires: DateTime<Utc>,
        user_id: Option<i64>,
        org_id: Option<i64>,
    ) -> Self {
        Self {
            key: crate::tokens::RawToken::generate().hashed(),
            description,
            scopes,
            permissions,
            expires,
            user_id,
            org_id,
        }
    }

    pub fn new_for_user(
        description: Option<String>,
        scopes: Vec<Scope>,
        permissions: Vec<Permission>,
        expires: DateTime<Utc>,
        user_id: i64,
    ) -> Self {
        Self::new(
            description,
            scopes,
            permissions,
            expires,
            Some(user_id),
            None,
        )
    }

    pub fn new_for_org(
        description: Option<String>,
        scopes: Vec<Scope>,
        permissions: Vec<Permission>,
        expires: DateTime<Utc>,
        org_id: i64,
    ) -> Self {
        Self::new(
            description,
            scopes,
            permissions,
            expires,
            None,
            Some(org_id),
        )
    }
}

#[derive(HasQuery, Insertable, Associations, Identifiable, Debug, Clone, Serialize)]
#[diesel(
    table_name = api_key,
    check_for_backend(diesel::pg::Pg),
    primary_key(id),
    belongs_to(crate::models::user::User, foreign_key = user_id),
    belongs_to(crate::models::org::Org, foreign_key = org_id)
)]
pub struct ApiKey {
    pub id: i64,
    pub scopes: Vec<Scope>,
    pub permissions: Vec<Permission>,
    pub description: Option<String>,
    pub expires: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<i64>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

// impl ApiKey {
//     pub fn new(
//         id: i64,
//         key: SecretSlice<u8>,
//         owner_user: Option<i64>,
//         owner_org: Option<i64>,
//         scopes: Vec<String>,
//         exp: DateTime<Utc>,
//     ) -> Self {
//         Self {
//             id,
//             key: key.expose_secret().to_vec(),
//             owner_user,
//             owner_org,
//             scopes,
//             exp,
//             revoked_at: None,
//         }
//     }
// }

#[cfg(test)]
mod test {
    #[test]
    fn smoke() {}
}
