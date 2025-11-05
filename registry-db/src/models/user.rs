use crate::{
    models::scopes::{Permission, Scope},
    schema::{api_key, org, org_admin, users},
};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, upsert::excluded};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Identifiable, AsChangeset, HasQuery, Serialize, Deserialize, Clone, ToSchema)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
pub struct PublicUser {
    #[schema(example = 1)]
    pub id: i64,
    #[schema(example = 123)]
    pub gh_id: i32,
    #[schema(example = "foobar")]
    pub gh_login: String,
    #[schema(example = "https://avatars.githubusercontent.com/u/123?v=4")]
    pub gh_avatar: Option<String>,
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            gh_id: user.gh_id,
            gh_login: user.gh_login,
            gh_avatar: user.gh_avatar,
        }
    }
}

#[derive(Debug, Identifiable, AsChangeset, HasQuery, Serialize, Deserialize, Clone, ToSchema)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
pub struct User {
    #[schema(example = 1)]
    pub id: i64,
    #[schema(example = "foo@bar.com")]
    pub email: String,
    #[schema(example = 123)]
    pub gh_id: i32,
    #[schema(example = "foobar")]
    pub gh_login: String,
    #[schema(example = "https://avatars.githubusercontent.com/u/123?v=4")]
    pub gh_avatar: Option<String>,
}

impl User {
    pub async fn orgs(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> crate::Result<Vec<super::org::OrgWithAdmin>> {
        Ok(org::table
            .inner_join(org_admin::table.on(org::id.eq(org_admin::org_id)))
            .filter(org_admin::user_id.eq(self.id))
            .filter(org_admin::revoked_at.is_null())
            .select(super::org::Org::as_select())
            .load(conn)
            .await?
            .into_iter()
            .map(|org| {
                super::org::OrgWithAdmin {
                    org: org.clone(),
                    user_is_admin: true,
                }
            })
            .collect())
    }

    /// **WARNING**: This loads all API keys for the user, including expired and revoked keys.
    pub async fn tokens(
        conn: &mut AsyncPgConnection,
        user_id: i64,
    ) -> crate::Result<Vec<super::api_key::ApiKey>> {
        api_key::table
            .filter(api_key::user_id.eq(user_id))
            .order(api_key::id.desc())
            .select(super::api_key::ApiKey::as_select())
            .load(conn)
            .await
            .map_err(Into::into)
    }

    pub async fn request_personal_token(
        &self,
        conn: &mut AsyncPgConnection,
        description: Option<String>,
        scopes: Vec<Scope>,
        permissions: Vec<Permission>,
        expires: DateTime<Utc>,
    ) -> crate::Result<super::api_key::OneTimeApiKey> {
        Ok(super::api_key::NewApiKey::new_for_user(
            description,
            scopes,
            permissions,
            expires,
            self.id,
        )
        .qualify(conn, self.id)
        .await?)
    }

    pub async fn request_org_token(
        &self,
        conn: &mut AsyncPgConnection,
        description: Option<String>,
        scopes: Vec<Scope>,
        permissions: Vec<Permission>,
        expires: DateTime<Utc>,
        org_id: i64,
    ) -> crate::Result<super::api_key::OneTimeApiKey> {
        let api_key = super::api_key::NewApiKey::new_for_org(
            description,
            scopes,
            permissions,
            expires,
            org_id,
        );
        Ok(api_key.qualify(conn, self.id).await?)
    }
}

#[derive(Debug, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub email: &'a str,
    pub gh_id: i32,
    pub gh_login: &'a str,
    pub gh_avatar: Option<&'a str>,
}

impl<'a> NewUser<'a> {
    pub async fn qualify(
        self,
        conn: &mut diesel_async::AsyncPgConnection,
    ) -> crate::Result<User> {
        Ok(diesel::insert_into(users::table)
            .values(&self)
            .on_conflict(users::gh_id)
            .do_update()
            .set((
                users::email.eq(excluded(users::email)),
                users::gh_login.eq(excluded(users::gh_login)),
                users::gh_avatar.eq(excluded(users::gh_avatar)),
            ))
            .get_result(conn)
            .await?)
    }
}
