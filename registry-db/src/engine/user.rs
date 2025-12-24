use crate::{Result, engine::OrgWithAdmin, entities::*};
use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, EntityTrait, NotSet, PaginatorTrait, QueryFilter, QueryOrder, Set,
    sea_query::OnConflict,
};

pub struct NewUser {
    pub email: String,
    pub gh_id: i32,
    pub gh_login: String,
    pub gh_avatar: Option<String>,
}

impl NewUser {
    pub async fn qualify(
        self,
        db: &sea_orm::DatabaseConnection,
    ) -> Result<User> {
        let active_model = UserActiveModel {
            id: NotSet,
            email: Set(self.email.clone()),
            gh_id: Set(self.gh_id),
            gh_login: Set(self.gh_login.clone()),
            gh_avatar: Set(self.gh_avatar.clone()),
        };

        Ok(UserEntity::insert(active_model)
            .on_conflict(
                OnConflict::column(UserColumn::GhId)
                    .update_columns([UserColumn::Email, UserColumn::GhLogin, UserColumn::GhAvatar])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await?)
    }
}

impl User {
    pub async fn by_id(
        db: &sea_orm::DatabaseConnection,
        user_id: i64,
    ) -> Result<Option<Self>> {
        UserEntity::find()
            .filter(UserColumn::Id.eq(user_id))
            .one(db)
            .await
            .map_err(Into::into)
    }

    pub async fn by_gh_id(
        db: &sea_orm::DatabaseConnection,
        gh_id: i32,
    ) -> Result<Option<Self>> {
        UserEntity::find()
            .filter(UserColumn::GhId.eq(gh_id))
            .one(db)
            .await
            .map_err(Into::into)
    }

    pub async fn by_email(
        db: &sea_orm::DatabaseConnection,
        email: &str,
    ) -> Result<Option<Self>> {
        UserEntity::find()
            .filter(UserColumn::Email.eq(email))
            .one(db)
            .await
            .map_err(Into::into)
    }

    pub async fn exists(
        db: &sea_orm::DatabaseConnection,
        user_id: i64,
    ) -> Result<bool> {
        let count = UserEntity::find()
            .filter(UserColumn::Id.eq(user_id))
            .count(db)
            .await?;

        Ok(count > 0)
    }

    pub async fn orgs(
        &self,
        db: &sea_orm::DatabaseConnection,
    ) -> Result<Vec<OrgWithAdmin>> {
        let orgs = OrgEntity::find()
            .inner_join(OrgRoleEntity)
            .filter(OrgRoleColumn::UserId.eq(self.id))
            .filter(OrgRoleColumn::RevokedAt.is_null())
            .all(db)
            .await?;

        Ok(orgs
            .into_iter()
            .map(|org| {
                OrgWithAdmin {
                    org,
                    user_is_admin: true,
                }
            })
            .collect())
    }

    pub async fn tokens(
        db: &sea_orm::DatabaseConnection,
        user_id: i64,
    ) -> Result<Vec<ApiKey>> {
        ApiKeyPrivateEntity::find()
            .filter(ApiKeyColumn::UserId.eq(user_id))
            .order_by_desc(ApiKeyColumn::Id)
            .into_partial_model()
            .all(db)
            .await
            .map_err(Into::into)
    }

    pub async fn request_personal_token(
        &self,
        db: &sea_orm::DatabaseConnection,
        principal: &super::principal::PrincipalIdentity,
        description: Option<String>,
        scopes: Vec<Scope>,
        permissions: Vec<Permission>,
        expires: DateTime<Utc>,
    ) -> Result<crate::engine::OneTimeApiKey> {
        crate::engine::NewApiKey::new_for_user(description, scopes, permissions, expires, self.id)
            .qualify(db, principal)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn request_org_token(
        &self,
        db: &sea_orm::DatabaseConnection,
        principal: &super::principal::PrincipalIdentity,
        description: Option<String>,
        scopes: Vec<Scope>,
        permissions: Vec<Permission>,
        expires: DateTime<Utc>,
        org_id: i64,
    ) -> Result<crate::engine::OneTimeApiKey> {
        crate::engine::NewApiKey::new_for_org(description, scopes, permissions, expires, org_id)
            .qualify(db, principal)
            .await
    }
}

pub async fn create_or_update_user_from_oauth(
    db: &sea_orm::DatabaseConnection,
    gh_id: i32,
    gh_login: &str,
    gh_avatar: Option<&str>,
    email: &str,
) -> Result<User> {
    let new_user = NewUser {
        gh_id,
        gh_login: gh_login.to_string(),
        gh_avatar: gh_avatar.map(|s| s.to_string()),
        email: email.to_string(),
    };

    new_user.qualify(db).await
}
