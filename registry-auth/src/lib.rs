use actix::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrincipalType {
    UserSession,
    UserApiKey,
    OrgApiKey,
}

/// Permission types for audit logging (mirrors registry-db Permission without sea-orm deps)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuditPermission {
    PublishPackage,
    YankPackage,
    GrantSchemaRole,
    RevokeSchemaRole,
    GrantOrgRole,
    RevokeOrgRole,
    CreateOrgToken,
    RevokeOrgToken,
    ListOrgToken,
    CreatePersonalToken,
    RevokePersonalToken,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageResource {
    pub name: String,
    pub id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrgResource {
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum OwnerId {
    User(i64),
    Org(i64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenResource {
    pub id: i64,
    pub owner: OwnerId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchemaRoleResource {
    pub package_id: i64,
    pub role_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrgRoleResource {
    pub org_id: i64,
    pub user_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "resource_type", rename_all = "snake_case")]
pub enum ResourceIdentifier {
    Package(PackageResource),
    Organization(OrgResource),
    Token(TokenResource),
    SchemaRole(SchemaRoleResource),
    OrgRole(OrgRoleResource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AuditEventType {
    PermissionProtected {
        permission: AuditPermission,
        resource: ResourceIdentifier,
    },
    ImportOrganization {
        org_id: i64,
        gh_org_id: i32,
        gh_org_login: String,
    },
    OrganizationInviteResponse {
        invitation_id: i64,
        org_id: i64,
        accepted: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Policy {
    ApiKeyRequired,
    ExplicitPermission,
    ScopeMatch,
    SchemaAdmin,
    FirstPublish,
    OrgAdmin,
    TokenOwnership,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationResult {
    pub allowed: bool,
    pub reason: String,
    pub checks: Vec<PolicyCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCheck {
    pub policy: Policy,
    pub passed: bool,
    pub details: String,
}

impl AuthorizationResult {
    pub fn allow(
        reason: impl Into<String>,
        checks: Vec<PolicyCheck>,
    ) -> Self {
        Self {
            allowed: true,
            reason: reason.into(),
            checks,
        }
    }

    pub fn deny(
        reason: impl Into<String>,
        checks: Vec<PolicyCheck>,
    ) -> Self {
        Self {
            allowed: false,
            reason: reason.into(),
            checks,
        }
    }

    pub fn not_applicable(
        permission: &str,
        resource: &str,
    ) -> Self {
        #[cfg(debug_assertions)]
        panic!(
            "Authorization check not applicable: permission '{}' on resource '{}'",
            permission, resource
        );

        Self {
            allowed: false,
            reason: format!(
                "Internal error: permission '{}' not applicable to resource '{}'",
                permission, resource
            ),
            checks: vec![PolicyCheck {
                policy: Policy::NotApplicable,
                passed: false,
                details: "Authorization check misconfiguration".to_string(),
            }],
        }
    }

    pub fn into_result(self) -> Result<(), AuthorizationError> {
        if self.allowed {
            Ok(())
        } else {
            if self
                .checks
                .iter()
                .any(|c| c.policy == Policy::NotApplicable)
            {
                Err(AuthorizationError::NotApplicable {
                    reason: self.reason,
                })
            } else {
                Err(AuthorizationError::Denied {
                    reason: self.reason,
                    checks: self.checks,
                })
            }
        }
    }

    pub fn require(self) -> Result<(), AuthorizationError> {
        self.into_result()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    #[error("Authorization denied: {reason}")]
    Denied {
        reason: String,
        checks: Vec<PolicyCheck>,
    },

    #[error("Internal authorization error: {reason}")]
    NotApplicable { reason: String },
}

#[derive(Debug, Clone, Serialize, bon::Builder, Message)]
#[rtype(result = "()")]
pub struct AuditEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub principal_type: PrincipalType,
    pub principal_id: i64,
    pub event_type: AuditEventType,
    pub allowed: bool,
    #[builder(into)]
    pub reason: String,
    pub policy_checks: Vec<PolicyCheck>,
    #[builder(into)]
    pub request_id: Option<String>,
    #[builder(into)]
    pub ip_address: Option<String>,
    #[builder(into)]
    pub user_agent: Option<String>,
}
