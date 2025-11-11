use crate::{Error, Result, entities::*};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, QueryFilter, Set, prelude::Expr,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub struct OneTimeApiKey {
    pub key: String,
    #[serde(flatten)]
    pub api_key: ApiKey,
}

pub struct NewApiKey {
    one_time: crate::tokens::RawToken,
    pub description: Option<String>,
    pub expires: crate::DateTime,
    pub scopes: Vec<Scope>,
    pub permissions: Vec<Permission>,
    pub user_id: Option<i64>,
    pub org_id: Option<i64>,
}

impl NewApiKey {
    pub fn new_for_user(
        description: Option<String>,
        scopes: Vec<Scope>,
        permissions: Vec<Permission>,
        expires: crate::DateTime,
        user_id: i64,
    ) -> Self {
        let one_time = crate::tokens::RawToken::generate();
        Self {
            one_time,
            description,
            expires,
            scopes,
            permissions,
            user_id: Some(user_id),
            org_id: None,
        }
    }

    pub fn new_for_org(
        description: Option<String>,
        scopes: Vec<Scope>,
        permissions: Vec<Permission>,
        expires: crate::DateTime,
        org_id: i64,
    ) -> Self {
        let one_time = crate::tokens::RawToken::generate();
        Self {
            one_time,
            description,
            expires,
            scopes,
            permissions,
            user_id: None,
            org_id: Some(org_id),
        }
    }

    pub async fn qualify<C: sea_orm::ConnectionTrait>(
        self,
        db: &C,
        requesting_user_id: i64,
    ) -> Result<OneTimeApiKey> {
        if let Some(..) = self.user_id {
        } else if let Some(org_id) = self.org_id {
            if let Some(org) = Org::by_id(db, org_id).await? {
                org.must_be_admin(db, requesting_user_id)
                    .await?;
            } else {
                return Err(Error::Validation("Organization not found".into()));
            }
        } else {
            return Err(Error::Validation(
                "API key must belong to either a user or a valid organization".into(),
            ));
        }

        let scopes: Vec<String> = self
            .scopes
            .iter()
            .map(|ok| ok.into())
            .collect();

        let active_model = ApiKeyActiveModel {
            id: NotSet,
            key: Set(self.one_time.hashed()),
            description: Set(self.description.clone()),
            expires: Set(self.expires),
            scopes: Set(scopes.clone()),
            permissions: Set(self.permissions.clone()),
            user_id: Set(self.user_id),
            org_id: Set(self.org_id),
            last_used_at: NotSet,
            revoked_at: NotSet,
        };

        let result = active_model.insert(db).await?;

        Ok(OneTimeApiKey {
            key: self.one_time.expose_secret().to_string(),
            api_key: ApiKey {
                id: result.id,
                description: self.description,
                expires: result.expires,
                scopes,
                permissions: self.permissions,
                user_id: result.user_id,
                org_id: result.org_id,
                last_used_at: result.last_used_at,
                revoked_at: result.revoked_at,
            },
        })
    }
}

impl ApiKey {
    pub async fn by_id<C: sea_orm::ConnectionTrait>(
        db: &C,
        id: i64,
    ) -> Result<Self> {
        ApiKeyPrivateEntity::find()
            .filter(ApiKeyColumn::Id.eq(id))
            .into_partial_model()
            .one(db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("API key {} not found", id)))
    }

    pub async fn by_raw_token<C: sea_orm::ConnectionTrait>(
        db: &C,
        raw_token: &SecretString,
    ) -> Result<Self> {
        let Some(token_hash) = crate::tokens::TokenHash::from_token(raw_token.expose_secret())
        else {
            return Err(Error::InvalidToken);
        };

        let result = ApiKeyPrivateEntity::find()
            .filter(ApiKeyColumn::Key.eq(token_hash))
            .filter(ApiKeyColumn::Expires.gt(Utc::now()))
            .filter(ApiKeyColumn::RevokedAt.is_null())
            .into_partial_model()
            .one(db)
            .await
            .map_err(|_| Error::InvalidToken)?
            .ok_or(Error::InvalidToken)?;

        Ok(result)
    }

    pub async fn get_token_owner<C: sea_orm::ConnectionTrait>(
        &self,
        db: &C,
    ) -> Result<crate::engine::Entity> {
        if let Some(org_id) = self.org_id {
            let org = OrgEntity::find()
                .filter(OrgColumn::Id.eq(org_id))
                .one(db)
                .await?
                .ok_or_else(|| Error::NotFound("Organization not found".into()))?;

            Ok(crate::engine::Entity::Org(org))
        } else if let Some(user_id) = self.user_id {
            let user = UserEntity::find()
                .filter(UserColumn::Id.eq(user_id))
                .one(db)
                .await?
                .ok_or_else(|| Error::NotFound("User not found".into()))?;

            Ok(crate::engine::Entity::User(user))
        } else {
            unreachable!("postgres constraint prevents api keys without owner");
        }
    }

    pub async fn revoke_token<C: sea_orm::ConnectionTrait>(
        self,
        db: &C,
        user: &User,
    ) -> Result<()> {
        const PERM: &str = "You do not have sufficient priviledges to revoke this token";

        let owner = self.get_token_owner(db).await?;

        match owner {
            crate::engine::Entity::Org(org) => {
                if !org.is_user_admin(db, user.id).await? {
                    return Err(Error::Unauthorized(PERM.into()));
                }
            },
            crate::engine::Entity::User(owner) => {
                if owner.id != user.id {
                    return Err(Error::Unauthorized(PERM.into()));
                }
            },
        }

        let count = ApiKeyPrivateEntity::update_many()
            .col_expr(ApiKeyColumn::RevokedAt, Expr::value(Utc::now()))
            .filter(ApiKeyColumn::Id.eq(self.id))
            .exec(db)
            .await?;

        Ok(if count.rows_affected == 0 {
            return Err(Error::NotFound(
                "Token not found, already revoked, or not owned by user".into(),
            ));
        } else {
            ()
        })
    }

    pub async fn revoke_token_by_id<C: sea_orm::ConnectionTrait>(
        db: &C,
        token_id: i64,
        user: &User,
    ) -> Result<()> {
        Self::by_id(db, token_id)
            .await?
            .revoke_token(db, user)
            .await
    }

    pub fn revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    pub fn check_scope_match(
        &self,
        package_name: &str,
    ) -> bool {
        self.scopes
            .iter()
            .any(|scope| Scope::is_match(scope, package_name))
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
    ) -> Result<()> {
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

    pub fn owner_id(&self) -> crate::engine::OwnerId {
        if let Some(org_id) = self.org_id {
            crate::engine::OwnerId::Org(org_id)
        } else if let Some(user_id) = self.user_id {
            crate::engine::OwnerId::User(user_id)
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
