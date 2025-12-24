use super::authorization::ResourceIdentifier;
use crate::entities::Permission;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum EventType {
    PermissionProtected {
        permission: Permission,
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
