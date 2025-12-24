use crate::entities::Permission;
use kintsu_registry_auth::AuditPermission;
use std::ops::Deref;

/// Convert from sea-orm Permission to audit-safe AuditPermission
impl From<Permission> for AuditPermission {
    fn from(p: Permission) -> Self {
        match p {
            Permission::PublishPackage => AuditPermission::PublishPackage,
            Permission::YankPackage => AuditPermission::YankPackage,
            Permission::GrantSchemaRole => AuditPermission::GrantSchemaRole,
            Permission::RevokeSchemaRole => AuditPermission::RevokeSchemaRole,
            Permission::GrantOrgRole => AuditPermission::GrantOrgRole,
            Permission::RevokeOrgRole => AuditPermission::RevokeOrgRole,
            Permission::CreateOrgToken => AuditPermission::CreateOrgToken,
            Permission::RevokeOrgToken => AuditPermission::RevokeOrgToken,
            Permission::ListOrgToken => AuditPermission::ListOrgToken,
            Permission::CreatePersonalToken => AuditPermission::CreatePersonalToken,
            Permission::RevokePersonalToken => AuditPermission::RevokePersonalToken,
        }
    }
}

// ============================================================================
// Local wrapper types for registry-auth resource types
// These provide backwards compatibility while delegating to auth types
// ============================================================================

/// Wrapper for organization resource identifier
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OrgResource {
    pub id: i64,
}

impl Deref for OrgResource {
    type Target = kintsu_registry_auth::OrgResource;

    fn deref(&self) -> &Self::Target {
        // Safety: OrgResource has identical layout to auth::OrgResource
        unsafe { &*(self as *const Self as *const Self::Target) }
    }
}

impl AsRef<kintsu_registry_auth::OrgResource> for OrgResource {
    fn as_ref(&self) -> &kintsu_registry_auth::OrgResource {
        self.deref()
    }
}

impl From<OrgResource> for kintsu_registry_auth::OrgResource {
    fn from(r: OrgResource) -> Self {
        kintsu_registry_auth::OrgResource { id: r.id }
    }
}

/// Wrapper for organization role resource identifier
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OrgRoleResource {
    pub org_id: i64,
    pub user_id: i64,
}

impl Deref for OrgRoleResource {
    type Target = kintsu_registry_auth::OrgRoleResource;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const Self as *const Self::Target) }
    }
}

impl AsRef<kintsu_registry_auth::OrgRoleResource> for OrgRoleResource {
    fn as_ref(&self) -> &kintsu_registry_auth::OrgRoleResource {
        self.deref()
    }
}

impl From<OrgRoleResource> for kintsu_registry_auth::OrgRoleResource {
    fn from(r: OrgRoleResource) -> Self {
        kintsu_registry_auth::OrgRoleResource {
            org_id: r.org_id,
            user_id: r.user_id,
        }
    }
}

/// Wrapper for package resource identifier
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageResource {
    pub name: String,
    pub id: Option<i64>,
}

impl Deref for PackageResource {
    type Target = kintsu_registry_auth::PackageResource;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const Self as *const Self::Target) }
    }
}

impl AsRef<kintsu_registry_auth::PackageResource> for PackageResource {
    fn as_ref(&self) -> &kintsu_registry_auth::PackageResource {
        self.deref()
    }
}

impl From<PackageResource> for kintsu_registry_auth::PackageResource {
    fn from(r: PackageResource) -> Self {
        kintsu_registry_auth::PackageResource {
            name: r.name,
            id: r.id,
        }
    }
}

/// Wrapper for schema role resource identifier
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SchemaRoleResource {
    pub package_id: i64,
    pub role_id: i64,
}

impl Deref for SchemaRoleResource {
    type Target = kintsu_registry_auth::SchemaRoleResource;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const Self as *const Self::Target) }
    }
}

impl AsRef<kintsu_registry_auth::SchemaRoleResource> for SchemaRoleResource {
    fn as_ref(&self) -> &kintsu_registry_auth::SchemaRoleResource {
        self.deref()
    }
}

impl From<SchemaRoleResource> for kintsu_registry_auth::SchemaRoleResource {
    fn from(r: SchemaRoleResource) -> Self {
        kintsu_registry_auth::SchemaRoleResource {
            package_id: r.package_id,
            role_id: r.role_id,
        }
    }
}

/// Wrapper for token resource identifier
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TokenResource {
    pub id: i64,
    pub owner: super::OwnerId,
}

impl Deref for TokenResource {
    type Target = kintsu_registry_auth::TokenResource;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const Self as *const Self::Target) }
    }
}

impl AsRef<kintsu_registry_auth::TokenResource> for TokenResource {
    fn as_ref(&self) -> &kintsu_registry_auth::TokenResource {
        self.deref()
    }
}

impl From<TokenResource> for kintsu_registry_auth::TokenResource {
    fn from(r: TokenResource) -> Self {
        kintsu_registry_auth::TokenResource {
            id: r.id,
            owner: r.owner.into(),
        }
    }
}

/// Convert from engine OwnerId to auth OwnerId
impl From<super::OwnerId> for kintsu_registry_auth::OwnerId {
    fn from(o: super::OwnerId) -> Self {
        match o {
            super::OwnerId::User(id) => kintsu_registry_auth::OwnerId::User(id),
            super::OwnerId::Org(id) => kintsu_registry_auth::OwnerId::Org(id),
        }
    }
}

/// Wrapper for resource identifier enum
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResourceIdentifier {
    Package(PackageResource),
    Organization(OrgResource),
    Token(TokenResource),
    SchemaRole(SchemaRoleResource),
    OrgRole(OrgRoleResource),
}

impl From<ResourceIdentifier> for kintsu_registry_auth::ResourceIdentifier {
    fn from(r: ResourceIdentifier) -> Self {
        match r {
            ResourceIdentifier::Package(p) => {
                kintsu_registry_auth::ResourceIdentifier::Package(p.into())
            },
            ResourceIdentifier::Organization(o) => {
                kintsu_registry_auth::ResourceIdentifier::Organization(o.into())
            },
            ResourceIdentifier::Token(t) => {
                kintsu_registry_auth::ResourceIdentifier::Token(t.into())
            },
            ResourceIdentifier::SchemaRole(s) => {
                kintsu_registry_auth::ResourceIdentifier::SchemaRole(s.into())
            },
            ResourceIdentifier::OrgRole(o) => {
                kintsu_registry_auth::ResourceIdentifier::OrgRole(o.into())
            },
        }
    }
}
