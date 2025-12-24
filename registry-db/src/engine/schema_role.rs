use crate::{Error, Result, entities::*};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, QueryFilter, Set};

pub async fn grant_role<C: sea_orm::ConnectionTrait>(
    db: &C,
    principal: &super::principal::PrincipalIdentity,
    package_name: &str,
    user_id: Option<i64>,
    org_id: Option<i64>,
    role: SchemaRoleType,
) -> Result<SchemaRole> {
    if (user_id.is_some() && org_id.is_some()) || (user_id.is_none() && org_id.is_none()) {
        return Err(Error::Validation(
            "Must specify exactly one of user_id or org_id".into(),
        ));
    }

    let pkg = PackageEntity::find()
        .filter(PackageColumn::Name.eq(package_name))
        .one(db)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Package '{}' not found", package_name)))?;

    let auth_result = super::fluent::AuthCheck::new(db, principal)
        .package(package_name, Some(pkg.id))
        .can_grant_role()
        .await?;

    let event = principal.audit_event(
        kintsu_registry_auth::AuditEventType::PermissionProtected {
            permission: Permission::GrantSchemaRole.into(),
            resource: super::authorization::ResourceIdentifier::Package(
                super::authorization::PackageResource {
                    name: package_name.to_string(),
                    id: Some(pkg.id),
                },
            )
            .into(),
        },
        &auth_result,
    );
    kintsu_registry_events::emit_event(event)?;

    auth_result.require()?;

    let mut query = SchemaRoleEntity::find()
        .filter(SchemaRoleColumn::Package.eq(pkg.id))
        .filter(SchemaRoleColumn::Role.eq(role.clone()))
        .filter(SchemaRoleColumn::RevokedAt.is_null());

    query = if let Some(uid) = user_id {
        query.filter(SchemaRoleColumn::UserId.eq(uid))
    } else {
        query.filter(SchemaRoleColumn::OrgId.eq(org_id.unwrap()))
    };

    if query.one(db).await?.is_some() {
        return Err(Error::Validation("Role already granted".into()));
    }

    let active_model = SchemaRoleActiveModel {
        id: NotSet,
        package: Set(pkg.id),
        user_id: Set(user_id),
        org_id: Set(org_id),
        role: Set(role),
        revoked_at: NotSet,
    };

    Ok(active_model.insert(db).await?)
}

pub async fn revoke_role<C: sea_orm::ConnectionTrait>(
    db: &C,
    principal: &super::principal::PrincipalIdentity,
    role_id: i64,
) -> Result<()> {
    let role = SchemaRoleEntity::find_by_id(role_id)
        .one(db)
        .await?
        .ok_or_else(|| Error::NotFound("Schema role not found".into()))?;

    let pkg = PackageEntity::find_by_id(role.package)
        .one(db)
        .await?
        .ok_or_else(|| Error::NotFound("Package not found".into()))?;

    let auth_result = super::fluent::AuthCheck::new(db, principal)
        .package(&pkg.name, Some(pkg.id))
        .can_revoke_role()
        .await?;

    let event = principal.audit_event(
        kintsu_registry_auth::AuditEventType::PermissionProtected {
            permission: Permission::RevokeSchemaRole.into(),
            resource: super::authorization::ResourceIdentifier::SchemaRole(
                super::authorization::SchemaRoleResource {
                    package_id: pkg.id,
                    role_id,
                },
            )
            .into(),
        },
        &auth_result,
    );
    kintsu_registry_events::emit_event(event)?;

    auth_result.require()?;

    let mut active_model: SchemaRoleActiveModel = role.into();
    active_model.revoked_at = Set(Some(Utc::now()));
    active_model.update(db).await?;

    Ok(())
}
