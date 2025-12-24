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

    pub async fn invite_to_org(
        &self,
        invite: &OrgInvite,
    ) -> Result<OrgInvite> {
        let invite = ();

        todo!()
    }
}

pub struct OrgInvite {
    pub org_id: i64,
    pub invitee_gh_login: String,
    pub role: OrgRoleType,
}

pub async fn import_organization<C: sea_orm::ConnectionTrait + TransactionTrait>(
    db: &C,
    principal: &super::principal::PrincipalIdentity,
    gh_id: i32,
    org_name: String,
    gh_avatar: String,
) -> Result<Org> {
    if !principal.is_session() {
        return Err(Error::Validation(
            "Organization import requires user session (not API key)".into(),
        ));
    }

    let user = principal
        .user()
        .ok_or_else(|| Error::Internal("Session principal missing user data".into()))?;

    let org_result = db
        .transaction::<_, Org, Error>(|txn| {
            let admin_user_id = user.id;
            let org_name = org_name.clone();
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
        .await?;

    let event = kintsu_registry_auth::AuditEvent::builder()
        .timestamp(chrono::Utc::now())
        .principal_type(principal.principal_type())
        .principal_id(principal.principal_id())
        .event_type(serde_json::to_value(
            &super::events::EventType::ImportOrganization {
                org_id: org_result.id,
                gh_org_id: gh_id,
                gh_org_login: org_name,
            },
        )?)
        .allowed(true)
        .reason("Session-only operation".to_string())
        .policy_checks(vec![])
        .build();

    kintsu_registry_events::emit_event(event)?;

    Ok(org_result)
}

pub async fn grant_role<C: sea_orm::ConnectionTrait>(
    db: &C,
    principal: &super::principal::PrincipalIdentity,
    org_id: i64,
    user_id: i64,
    role: OrgRoleType,
) -> Result<OrgRole> {
    let auth_result = super::fluent::AuthCheck::new(db, principal)
        .org(org_id)
        .can_grant_role()
        .await?;

    let event = principal.audit_event(
        super::events::EventType::PermissionProtected {
            permission: Permission::GrantOrgRole,
            resource: super::authorization::ResourceIdentifier::Organization(
                super::authorization::OrgResource { id: org_id },
            ),
        },
        &auth_result,
    )?;
    kintsu_registry_events::emit_event(event)?;

    auth_result.require()?;

    let existing = OrgRoleEntity::find()
        .filter(OrgRoleColumn::OrgId.eq(org_id))
        .filter(OrgRoleColumn::UserId.eq(user_id))
        .filter(OrgRoleColumn::Role.eq(role.clone()))
        .filter(OrgRoleColumn::RevokedAt.is_null())
        .one(db)
        .await?;

    if existing.is_some() {
        return Err(Error::Validation("Role already granted".into()));
    }

    let active_model = OrgRoleActiveModel {
        org_id: Set(org_id),
        user_id: Set(user_id),
        role: Set(role),
        revoked_at: NotSet,
    };

    Ok(active_model.insert(db).await?)
}

pub async fn revoke_role<C: sea_orm::ConnectionTrait>(
    db: &C,
    principal: &super::principal::PrincipalIdentity,
    org_id: i64,
    user_id: i64,
) -> Result<()> {
    let auth_result = super::fluent::AuthCheck::new(db, principal)
        .org(org_id)
        .can_revoke_role()
        .await?;

    let event = principal.audit_event(
        super::events::EventType::PermissionProtected {
            permission: Permission::RevokeOrgRole,
            resource: super::authorization::ResourceIdentifier::OrgRole(
                super::authorization::OrgRoleResource { org_id, user_id },
            ),
        },
        &auth_result,
    )?;
    kintsu_registry_events::emit_event(event)?;

    auth_result.require()?;

    let role = OrgRoleEntity::find()
        .filter(OrgRoleColumn::OrgId.eq(org_id))
        .filter(OrgRoleColumn::UserId.eq(user_id))
        .filter(OrgRoleColumn::RevokedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(|| Error::NotFound("Org role not found".into()))?;

    let mut active_model: OrgRoleActiveModel = role.into();
    active_model.revoked_at = Set(Some(chrono::Utc::now()));
    active_model.update(db).await?;

    Ok(())
}
