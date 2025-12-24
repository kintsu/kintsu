use super::{
    authorization::{Authorize, OrgResource, PackageResource, TokenResource},
    principal::PrincipalIdentity,
};
use crate::{Result, engine::OwnerId, entities::Permission};
use kintsu_registry_auth::AuthorizationResult;
use sea_orm::ConnectionTrait;

pub struct AuthCheck<'a, C: ConnectionTrait> {
    db: &'a C,
    principal: &'a PrincipalIdentity,
}

impl<'a, C: ConnectionTrait> AuthCheck<'a, C> {
    pub fn new(
        db: &'a C,
        principal: &'a PrincipalIdentity,
    ) -> Self {
        Self { db, principal }
    }

    pub fn package(
        self,
        name: &str,
        id: Option<i64>,
    ) -> PackageAuthCheck<'a, C> {
        PackageAuthCheck {
            db: self.db,
            principal: self.principal,
            resource: PackageResource {
                name: name.to_string(),
                id,
            },
        }
    }

    pub fn org(
        self,
        id: i64,
    ) -> OrgAuthCheck<'a, C> {
        OrgAuthCheck {
            db: self.db,
            principal: self.principal,
            resource: OrgResource { id },
        }
    }

    pub fn token(
        self,
        id: i64,
        owner: OwnerId,
    ) -> TokenAuthCheck<'a, C> {
        TokenAuthCheck {
            db: self.db,
            principal: self.principal,
            resource: TokenResource { id, owner },
        }
    }
}

pub struct PackageAuthCheck<'a, C: ConnectionTrait> {
    db: &'a C,
    principal: &'a PrincipalIdentity,
    resource: PackageResource,
}

impl<'a, C: ConnectionTrait> PackageAuthCheck<'a, C> {
    pub async fn can_publish(&self) -> Result<AuthorizationResult> {
        self.resource
            .authorize(self.db, self.principal, Permission::PublishPackage)
            .await
    }

    pub async fn can_yank(&self) -> Result<AuthorizationResult> {
        self.resource
            .authorize(self.db, self.principal, Permission::YankPackage)
            .await
    }

    pub async fn can_grant_role(&self) -> Result<AuthorizationResult> {
        self.resource
            .authorize(self.db, self.principal, Permission::GrantSchemaRole)
            .await
    }

    pub async fn can_revoke_role(&self) -> Result<AuthorizationResult> {
        self.resource
            .authorize(self.db, self.principal, Permission::RevokeSchemaRole)
            .await
    }
}

pub struct OrgAuthCheck<'a, C: ConnectionTrait> {
    db: &'a C,
    principal: &'a PrincipalIdentity,
    resource: OrgResource,
}

impl<'a, C: ConnectionTrait> OrgAuthCheck<'a, C> {
    pub async fn can_grant_role(&self) -> Result<AuthorizationResult> {
        self.resource
            .authorize(self.db, self.principal, Permission::GrantOrgRole)
            .await
    }

    pub async fn can_revoke_role(&self) -> Result<AuthorizationResult> {
        self.resource
            .authorize(self.db, self.principal, Permission::RevokeOrgRole)
            .await
    }

    pub async fn can_create_token(&self) -> Result<AuthorizationResult> {
        self.resource
            .authorize(self.db, self.principal, Permission::CreateOrgToken)
            .await
    }

    pub async fn can_revoke_token(&self) -> Result<AuthorizationResult> {
        self.resource
            .authorize(self.db, self.principal, Permission::RevokeOrgToken)
            .await
    }

    pub async fn can_list_tokens(&self) -> Result<AuthorizationResult> {
        self.resource
            .authorize(self.db, self.principal, Permission::ListOrgToken)
            .await
    }
}

pub struct TokenAuthCheck<'a, C: ConnectionTrait> {
    db: &'a C,
    principal: &'a PrincipalIdentity,
    resource: TokenResource,
}

impl<'a, C: ConnectionTrait> TokenAuthCheck<'a, C> {
    pub async fn can_create_personal(&self) -> Result<AuthorizationResult> {
        self.resource
            .authorize(self.db, self.principal, Permission::CreatePersonalToken)
            .await
    }

    pub async fn can_revoke_personal(&self) -> Result<AuthorizationResult> {
        self.resource
            .authorize(self.db, self.principal, Permission::RevokePersonalToken)
            .await
    }
}
