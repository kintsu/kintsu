use super::scopes::Scope;
use crate::{
    Error,
    models::{Entity, OwnerId, org::Org, scopes::Permission, user::User},
    schema::{api_key, org, users},
};
use chrono::{DateTime, Utc};
use diesel::{pg::Pg, prelude::*};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

#[derive(Insertable)]
#[diesel(
    table_name = api_key,
    check_for_backend(diesel::pg::Pg)
)]
pub struct NewApiKey {
    #[diesel(skip_insertion)]
    one_time: crate::tokens::RawToken,

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
        let one_time = crate::tokens::RawToken::generate();
        Self {
            key: one_time.hashed(),
            one_time,
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

    pub async fn qualify(
        self,
        conn: &mut diesel_async::AsyncPgConnection,
        requesting_user_id: i64,
    ) -> crate::Result<OneTimeApiKey> {
        if let Some(..) = self.user_id {
        } else if let Some(org_id) = self.org_id
            && let Some(org) = super::org::Org::by_id(conn, org_id).await?
        {
            org.must_be_admin(conn, requesting_user_id)
                .await?;
        } else {
            return Err(Error::Validation(
                "API key must belong to either a user or a valid organization".into(),
            ));
        }
        Ok(OneTimeApiKey {
            key: self.one_time.expose_secret().to_string(),
            api_key: diesel::insert_into(api_key::table)
                .values(&self)
                .returning(ApiKey::as_returning())
                .get_result(conn)
                .await?,
        })
    }
}

/// WARNING: This struct contains the raw API key value. It should only be used
/// immediately after creation by the user, and never stored or logged.
#[derive(Serialize, utoipa::ToSchema)]
pub struct OneTimeApiKey {
    pub key: String,
    #[serde(flatten)]
    pub api_key: ApiKey,
}

#[derive(
    HasQuery, Insertable, Associations, Identifiable, Debug, Clone, Serialize, utoipa::ToSchema,
)]
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

impl ApiKey {
    pub fn revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    pub async fn by_id(
        conn: &mut AsyncPgConnection,
        id: i64,
    ) -> crate::Result<Self> {
        Ok(api_key::table
            .filter(api_key::id.eq(id))
            .select(ApiKey::as_select())
            .first(conn)
            .await?)
    }

    pub async fn by_raw_token(
        conn: &mut AsyncPgConnection,
        raw_token: &SecretString,
    ) -> crate::Result<Self> {
        let Some(token_hash) = crate::tokens::TokenHash::from_token(raw_token.expose_secret())
        else {
            return Err(Error::InvalidToken);
        };

        <ApiKey as HasQuery<Pg>>::query()
            .filter(api_key::key.eq(token_hash))
            .filter(api_key::expires.gt(Utc::now()))
            .filter(api_key::revoked_at.is_null())
            .select(ApiKey::as_select())
            .first(conn)
            .await
            .map_err(|db_err| {
                match db_err {
                    diesel::result::Error::NotFound => Error::InvalidToken,
                    _ => db_err.into(),
                }
            })
    }

    pub async fn get_token_owner(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> crate::Result<crate::models::Entity> {
        if let Some(org_id) = self.org_id {
            let org = org::table
                .filter(org::id.eq(org_id))
                .first::<Org>(conn)
                .await?;

            Ok(Entity::Org(org))
        } else if let Some(user_id) = self.user_id {
            let user = users::table
                .filter(users::id.eq(user_id))
                .first::<User>(conn)
                .await?;

            Ok(Entity::User(user))
        } else {
            unreachable!("postgres constraint should prevent api keys without an owner");
        }
    }

    pub async fn revoke_token(
        self,
        conn: &mut AsyncPgConnection,
        user: &User,
    ) -> crate::Result<()> {
        const PERM: &str = "You do not have sufficient priviledges to revoke this token";

        let owner = self.get_token_owner(conn).await?;

        match owner {
            Entity::Org(org) => {
                if !org.is_user_admin(conn, user.id).await? {
                    return Err(Error::Unauthorized(PERM.into()));
                }
            },
            Entity::User(owner) => {
                if owner.id != user.id {
                    return Err(Error::Unauthorized(PERM.into()));
                }
            },
        }

        let updated = diesel::update(api_key::table)
            .filter(api_key::id.eq(self.id))
            .set(api_key::revoked_at.eq(Some(Utc::now())))
            .execute(conn)
            .await?;

        if updated == 0 {
            return Err(Error::NotFound(
                "Token not found or not owned by user".into(),
            ));
        }

        drop(self);

        Ok(())
    }

    pub async fn revoke_token_by_id(
        conn: &mut AsyncPgConnection,
        token_id: i64,
        user: &User,
    ) -> crate::Result<()> {
        Ok(Self::by_id(conn, token_id)
            .await?
            .revoke_token(conn, user)
            .await?)
    }

    fn check_scope_match(
        &self,
        package_name: &str,
    ) -> bool {
        self.scopes.iter().any(|scope| {
            if scope.pattern.ends_with('*') {
                let prefix = scope.pattern.trim_end_matches('*');
                package_name.starts_with(prefix)
            } else {
                scope.pattern == package_name
            }
        })
    }

    pub fn check_permissions_for_package(
        &self,
        package_name: &str,
        permission: &Permission,
    ) -> AuthCheck {
        AuthCheck {
            scope_matches: self.check_scope_match(package_name),
            has_permission: self.permissions.contains(permission),
        }
    }

    pub fn must_have_permission_for_package(
        &self,
        package_name: &str,
        permission: &Permission,
    ) -> crate::Result<()> {
        let auth_check = self.check_permissions_for_package(package_name, permission);
        if !auth_check.ok() {
            return Err(Error::Unauthorized(format!(
                "Token does not have permission for '{}'. {}.",
                package_name,
                {
                    if !auth_check.scope_matches {
                        "Scope does not match".to_string()
                    } else {
                        format!("Token does not have '{}' permission", permission)
                    }
                }
            )));
        }
        Ok(())
    }

    pub fn owner_id(&self) -> OwnerId {
        if let Some(org_id) = self.org_id {
            OwnerId::Org(org_id)
        } else if let Some(user_id) = self.user_id {
            OwnerId::User(user_id)
        } else {
            unreachable!("postgres constraint should prevent api keys without an owner");
        }
    }
}

pub struct AuthCheck {
    pub scope_matches: bool,
    pub has_permission: bool,
}

impl AuthCheck {
    pub fn ok(&self) -> bool {
        self.scope_matches && self.has_permission
    }
}
