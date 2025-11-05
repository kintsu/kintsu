use kintsu_registry_db::models::scopes::{Permission, Scope};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response type for package download statistics
#[derive(Serialize, ToSchema)]
pub struct DownloadStats {
    /// Package name
    pub package: String,
    /// Total number of downloads across all versions
    pub total_downloads: i64,
}

/// Request body for creating API tokens (personal or org)
#[derive(Deserialize, ToSchema, validator::Validate)]
pub struct CreateTokenRequest {
    #[validate(length(min = 1, max = 32))]
    /// Optional description for the token
    pub description: Option<String>,
    #[validate(length(min = 1, max = 10))]
    /// Package name patterns this token can access (supports wildcards)
    #[serde(default)]
    pub scopes: Vec<Scope>,
    #[validate(length(min = 1, max = 4))]
    /// Permissions granted to this token
    #[serde(default)]
    pub permissions: Vec<Permission>,
    /// Token expiration in days (default: 90)
    pub expires_in_days: Option<i64>,
}

/// Candidate GitHub organization that can be imported
#[derive(Serialize, ToSchema)]
pub struct CandidateOrg {
    /// GitHub organization ID
    #[schema(example = 123456)]
    pub gh_id: i32,

    /// Organization name/login on GitHub
    #[schema(example = "acme-corp")]
    pub name: String,

    /// Organization avatar URL
    #[schema(example = "https://avatars.githubusercontent.com/u/123456?v=4")]
    pub avatar_url: String,

    /// Whether this organization has already been imported to the registry
    #[schema(example = false)]
    pub is_imported: bool,
}

/// Request body for importing a GitHub organization
#[derive(Deserialize, ToSchema, validator::Validate)]
pub struct ImportOrgRequest {
    /// GitHub organization name/login (1-39 characters per GitHub limits)
    #[validate(length(min = 1, max = 39))]
    #[schema(example = "acme-corp")]
    pub org_name: String,
}
