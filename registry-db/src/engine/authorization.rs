use crate::{Result, engine::OwnerId, entities::*};
use kintsu_registry_auth::{AuthorizationResult, Policy, PolicyCheck};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, QueryTrait,
};

// Re-export local wrapper types from events module for use throughout engine
pub use super::events::{
    OrgResource, OrgRoleResource, PackageResource, ResourceIdentifier, SchemaRoleResource,
    TokenResource,
};

pub trait Authorize {
    async fn authorize<C: ConnectionTrait>(
        &self,
        db: &C,
        principal: &super::principal::PrincipalIdentity,
        permission: Permission,
    ) -> Result<AuthorizationResult>;
}

impl Authorize for PackageResource {
    async fn authorize<C: ConnectionTrait>(
        &self,
        db: &C,
        principal: &super::principal::PrincipalIdentity,
        permission: Permission,
    ) -> Result<AuthorizationResult> {
        let mut checks = Vec::new();

        match permission {
            Permission::PublishPackage | Permission::YankPackage => {
                let is_api_key = principal.is_api_key();
                checks.push(PolicyCheck {
                    policy: Policy::ApiKeyRequired,
                    passed: is_api_key,
                    details: format!("{:?} requires API key", permission),
                });

                if !is_api_key {
                    return Ok(AuthorizationResult::deny(
                        format!("{:?} requires API key (not session)", permission),
                        checks,
                    ));
                }

                let api_key = principal.api_key().unwrap();

                let has_permission = api_key.permissions.contains(&permission);
                checks.push(PolicyCheck {
                    policy: Policy::ExplicitPermission,
                    passed: has_permission,
                    details: format!("API key has {:?} permission", permission),
                });

                if !has_permission {
                    return Ok(AuthorizationResult::deny(
                        format!("API key missing {:?} permission", permission),
                        checks,
                    ));
                }

                let scope_match = api_key
                    .scopes
                    .iter()
                    .any(|scope| Scope::is_match(scope, &self.name));
                checks.push(PolicyCheck {
                    policy: Policy::ScopeMatch,
                    passed: scope_match,
                    details: format!("API key scopes match package {}", self.name),
                });

                if !scope_match {
                    return Ok(AuthorizationResult::deny(
                        format!("Package {} not in API key scope", self.name),
                        checks,
                    ));
                }

                if self.id.is_some() {
                    let is_admin = self
                        .check_schema_admin(db, principal)
                        .await?;

                    checks.push(PolicyCheck {
                        policy: Policy::SchemaAdmin,
                        passed: is_admin,
                        details: format!("Principal is admin of package {}", self.name),
                    });

                    if !is_admin {
                        return Ok(AuthorizationResult::deny(
                            format!("Not admin of package {}", self.name),
                            checks,
                        ));
                    }
                } else {
                    checks.push(PolicyCheck {
                        policy: Policy::FirstPublish,
                        passed: true,
                        details: format!(
                            "First publish of package {}, will become admin",
                            self.name
                        ),
                    });
                }

                Ok(AuthorizationResult::allow("All checks passed", checks))
            },

            Permission::GrantSchemaRole | Permission::RevokeSchemaRole => {
                if let Some(api_key) = principal.api_key() {
                    let has_permission = api_key.permissions.contains(&permission);
                    checks.push(PolicyCheck {
                        policy: Policy::ExplicitPermission,
                        passed: has_permission,
                        details: format!("API key has {:?} permission", permission),
                    });

                    if !has_permission {
                        return Ok(AuthorizationResult::deny(
                            format!("API key missing {:?} permission", permission),
                            checks,
                        ));
                    }
                }

                let is_admin = self
                    .check_schema_admin(db, principal)
                    .await?;

                checks.push(PolicyCheck {
                    policy: Policy::SchemaAdmin,
                    passed: is_admin,
                    details: format!("Principal is admin of package {}", self.name),
                });

                if !is_admin {
                    return Ok(AuthorizationResult::deny(
                        format!("Not admin of package {}", self.name),
                        checks,
                    ));
                }

                Ok(AuthorizationResult::allow("All checks passed", checks))
            },

            _ => {
                Ok(AuthorizationResult::not_applicable(
                    &format!("{:?}", permission),
                    "PackageResource",
                ))
            },
        }
    }
}

impl PackageResource {
    async fn check_schema_admin<C: ConnectionTrait>(
        &self,
        db: &C,
        principal: &super::principal::PrincipalIdentity,
    ) -> Result<bool> {
        let pkg_id = match self.id {
            Some(id) => id,
            None => return Ok(false),
        };

        if let Some(user) = principal.user() {
            use sea_orm::QuerySelect;

            let is_admin = SchemaRoleEntity::find()
                .filter(SchemaRoleColumn::Package.eq(pkg_id))
                .filter(SchemaRoleColumn::Role.eq(SchemaRoleType::Admin))
                .filter(SchemaRoleColumn::RevokedAt.is_null())
                .filter(
                    SchemaRoleColumn::UserId
                        .eq(user.id)
                        .or(SchemaRoleColumn::OrgId.in_subquery(
                            OrgRoleEntity::find()
                                .filter(OrgRoleColumn::UserId.eq(user.id))
                                .filter(OrgRoleColumn::Role.eq(OrgRoleType::Admin))
                                .filter(OrgRoleColumn::RevokedAt.is_null())
                                .select_only()
                                .column(OrgRoleColumn::OrgId)
                                .into_query(),
                        )),
                )
                .count(db)
                .await?
                > 0;

            return Ok(is_admin);
        }

        if let Some(org) = principal.org() {
            let org_is_admin = SchemaRoleEntity::find()
                .filter(SchemaRoleColumn::Package.eq(pkg_id))
                .filter(SchemaRoleColumn::OrgId.eq(org.id))
                .filter(SchemaRoleColumn::Role.eq(SchemaRoleType::Admin))
                .filter(SchemaRoleColumn::RevokedAt.is_null())
                .count(db)
                .await?
                > 0;

            return Ok(org_is_admin);
        }

        Ok(false)
    }
}

impl Authorize for OrgResource {
    async fn authorize<C: ConnectionTrait>(
        &self,
        db: &C,
        principal: &super::principal::PrincipalIdentity,
        permission: Permission,
    ) -> Result<AuthorizationResult> {
        let mut checks = Vec::new();

        match permission {
            Permission::GrantOrgRole
            | Permission::RevokeOrgRole
            | Permission::CreateOrgToken
            | Permission::RevokeOrgToken
            | Permission::ListOrgToken => {
                if let Some(api_key) = principal.api_key() {
                    let has_permission = api_key.permissions.contains(&permission);
                    checks.push(PolicyCheck {
                        policy: Policy::ExplicitPermission,
                        passed: has_permission,
                        details: format!("API key has {:?} permission", permission),
                    });

                    if !has_permission {
                        return Ok(AuthorizationResult::deny(
                            format!("API key missing {:?} permission", permission),
                            checks,
                        ));
                    }
                }

                let is_admin = self.check_org_admin(db, principal).await?;
                checks.push(PolicyCheck {
                    policy: Policy::OrgAdmin,
                    passed: is_admin,
                    details: format!("Principal is admin of org {}", self.id),
                });

                if !is_admin {
                    return Ok(AuthorizationResult::deny(
                        format!("Not admin of organization {}", self.id),
                        checks,
                    ));
                }

                Ok(AuthorizationResult::allow("All checks passed", checks))
            },

            _ => {
                Ok(AuthorizationResult::not_applicable(
                    &format!("{:?}", permission),
                    "OrgResource",
                ))
            },
        }
    }
}

impl OrgResource {
    async fn check_org_admin<C: ConnectionTrait>(
        &self,
        db: &C,
        principal: &super::principal::PrincipalIdentity,
    ) -> Result<bool> {
        if let Some(org) = principal.org() {
            return Ok(org.id == self.id);
        }

        if let Some(user) = principal.user() {
            let is_admin = OrgRoleEntity::find()
                .filter(OrgRoleColumn::OrgId.eq(self.id))
                .filter(OrgRoleColumn::UserId.eq(user.id))
                .filter(OrgRoleColumn::Role.eq(OrgRoleType::Admin))
                .filter(OrgRoleColumn::RevokedAt.is_null())
                .count(db)
                .await?
                > 0;

            return Ok(is_admin);
        }

        Ok(false)
    }
}

impl Authorize for TokenResource {
    async fn authorize<C: ConnectionTrait>(
        &self,
        db: &C,
        principal: &super::principal::PrincipalIdentity,
        permission: Permission,
    ) -> Result<AuthorizationResult> {
        let mut checks = Vec::new();

        match permission {
            Permission::CreatePersonalToken | Permission::RevokePersonalToken => {
                if let Some(api_key) = principal.api_key() {
                    let has_permission = api_key.permissions.contains(&permission);
                    checks.push(PolicyCheck {
                        policy: Policy::ExplicitPermission,
                        passed: has_permission,
                        details: format!("API key has {:?} permission", permission),
                    });

                    if !has_permission {
                        return Ok(AuthorizationResult::deny(
                            format!("API key missing {:?} permission", permission),
                            checks,
                        ));
                    }
                }

                let matches_owner = match (&self.owner, principal.owner_id()) {
                    (OwnerId::User(token_uid), OwnerId::User(principal_uid)) => {
                        token_uid == &principal_uid
                    },
                    _ => false,
                };

                checks.push(PolicyCheck {
                    policy: Policy::TokenOwnership,
                    passed: matches_owner,
                    details: format!("Principal owns token {} (owner: {:?})", self.id, self.owner),
                });

                if !matches_owner {
                    return Ok(AuthorizationResult::deny(
                        "Principal does not own token",
                        checks,
                    ));
                }

                Ok(AuthorizationResult::allow("All checks passed", checks))
            },

            _ => {
                Ok(AuthorizationResult::not_applicable(
                    &format!("{:?}", permission),
                    "TokenResource",
                ))
            },
        }
    }
}
