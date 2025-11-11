use crate::{Error, Result, entities::*};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

#[derive(Debug, serde::Serialize, Clone, utoipa::ToSchema)]
pub struct OrgWithAdmin {
    #[serde(flatten)]
    pub org: Org,
    pub user_is_admin: bool,
}

impl Org {
    pub async fn by_id<C: sea_orm::ConnectionTrait>(
        db: &C,
        org_id: i64,
    ) -> Result<Option<Self>> {
        OrgEntity::find()
            .filter(OrgColumn::Id.eq(org_id))
            .one(db)
            .await
            .map_err(Into::into)
    }

    pub async fn by_name<C: sea_orm::ConnectionTrait>(
        db: &C,
        org_name: &str,
    ) -> Result<Option<Self>> {
        OrgEntity::find()
            .filter(OrgColumn::Name.eq(org_name))
            .one(db)
            .await
            .map_err(Into::into)
    }

    pub async fn exists<C: sea_orm::ConnectionTrait>(
        db: &C,
        org_name: &str,
    ) -> Result<bool> {
        let count = OrgEntity::find()
            .filter(OrgColumn::Name.eq(org_name))
            .count(db)
            .await?;

        Ok(count > 0)
    }

    pub async fn exists_bulk<C: sea_orm::ConnectionTrait>(
        db: &C,
        org_names: &[&str],
    ) -> Result<Vec<String>> {
        let existing_orgs = OrgEntity::find()
            .filter(OrgColumn::Name.is_in(org_names.iter().copied()))
            .all(db)
            .await?;

        Ok(existing_orgs
            .into_iter()
            .map(|org| org.name)
            .collect())
    }

    pub async fn tokens<C: sea_orm::ConnectionTrait>(
        db: &C,
        org_id: i64,
    ) -> Result<Vec<ApiKey>> {
        Ok(ApiKeyPrivateEntity::find()
            .filter(ApiKeyColumn::OrgId.eq(org_id))
            .order_by_desc(ApiKeyColumn::Id)
            .into_partial_model()
            .all(db)
            .await?)
    }

    pub async fn is_user_admin<C: sea_orm::ConnectionTrait>(
        &self,
        db: &C,
        user_id: i64,
    ) -> Result<bool> {
        Ok(OrgRoleEntity::find()
            .filter(OrgRoleColumn::OrgId.eq(self.id))
            .filter(OrgRoleColumn::UserId.eq(user_id))
            .filter(OrgRoleColumn::Role.eq(OrgRoleType::Admin))
            .filter(OrgRoleColumn::RevokedAt.is_null())
            .limit(1)
            .count(db)
            .await?
            > 0)
    }

    pub async fn must_be_admin<C: sea_orm::ConnectionTrait>(
        &self,
        db: &C,
        user_id: i64,
    ) -> Result<()> {
        if !self.is_user_admin(db, user_id).await? {
            return Err(Error::Unauthorized(
                "User is not an admin of the organization".into(),
            ));
        }
        Ok(())
    }
}

pub async fn import_organization<C: sea_orm::ConnectionTrait + TransactionTrait>(
    db: &C,
    gh_id: i32,
    org_name: String,
    gh_avatar: String,
    admin_user_id: i64,
) -> Result<Org> {
    Ok(db
        .transaction::<_, Org, Error>(|txn| {
            Box::pin(async move {
                let existing = OrgEntity::find()
                    .filter(
                        sea_orm::Condition::any()
                            .add(OrgColumn::Name.eq(&org_name))
                            .add(OrgColumn::GhId.eq(gh_id)),
                    )
                    .one(txn)
                    .await?;

                if let Some(existing_org) = existing {
                    return Err(Error::Conflict(format!(
                        "Organization '{}' (gh_id: {}) already imported",
                        existing_org.name, existing_org.gh_id
                    )));
                }

                let org_active_model = OrgActiveModel {
                    id: NotSet,
                    name: Set(org_name),
                    gh_id: Set(gh_id),
                    gh_avatar: Set(gh_avatar),
                };

                let new_org = org_active_model.insert(txn).await?;

                let org_role_active_model = OrgRoleActiveModel {
                    org_id: Set(new_org.id),
                    user_id: Set(admin_user_id),
                    role: Set(OrgRoleType::Admin),
                    revoked_at: NotSet,
                };

                org_role_active_model.insert(txn).await?;

                Ok(new_org)
            })
        })
        .await?)
}
