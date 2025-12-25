use kintsu_manifests::config::NewForNamed;
use kintsu_registry_db::entities::{OrgRoleType, Permission, SchemaRoleType, Scope};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::PackagingError;

pub use kintsu_registry_db::entities::{ApiKey, Org, Package, User, Version};

/// Response type for package download statistics
#[derive(Serialize, ToSchema)]
pub struct DownloadStats {
    /// Package name
    pub package: String,
    /// Total number of downloads across all versions
    pub total_downloads: i64,
}

/// Request body for creating API tokens (personal or org)
#[derive(Serialize, Deserialize, ToSchema, validator::Validate)]
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
    #[validate(range(min = 1, max = 365))]
    /// Token expiration in days (default: 90, max: 365)
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
#[derive(Serialize, Deserialize, ToSchema, validator::Validate)]
pub struct ImportOrgRequest {
    /// GitHub organization name/login (1-39 characters per GitHub limits)
    #[validate(length(min = 1, max = 39))]
    #[schema(example = "acme-corp")]
    pub org_name: String,
}

#[derive(Serialize, Deserialize, ToSchema, validator::Validate)]
pub struct PublishPackageRequest {
    // raw manifest file content - we need to get the meta from this + memory fs
    #[validate(nested)]
    pub manifest: kintsu_manifests::package::PackageManifests,
    pub package_data: kintsu_fs::memory::MemoryFileSystem,
}

impl PublishPackageRequest {
    pub fn validate_publishing_package_data(&self) -> std::result::Result<(), Vec<PackagingError>> {
        if self.package_data.list_files().is_empty() {
            return Err(vec![PackagingError::EmptyPackageData]);
        }

        let mut has_manifest = false;
        let mut has_schema_lib = false;
        let mut invalid_files = vec![];

        self.package_data
            .list_files()
            .iter()
            .for_each(|file| {
                if let Some(name) = file.file_name()
                    && let Some(ext) = file.extension()
                {
                    let name = name.to_string_lossy();
                    let ext = ext.to_string_lossy();

                    if name == kintsu_manifests::package::PackageManifests::NAME {
                        has_manifest = true;
                    } else if name == "lib.ks" {
                        has_schema_lib = true;
                    } else if ext != "ks" && ext != "toml" && ext != "md" && ext != "txt" {
                        invalid_files.push(PackagingError::InvalidFile {
                            path: format!("{}", file.display()),
                            reason: format!("Invalid file extension: {}", ext),
                        });
                    }
                }
            });

        if !invalid_files.is_empty() {
            return Err(invalid_files);
        }

        Ok(())
    }
}
#[derive(Serialize, ToSchema)]
pub struct PublishPackageResponse {
    pub url: String,
    pub version: kintsu_registry_db::entities::Version,
}

#[derive(serde::Deserialize, Validate, ToSchema)]
#[validate(schema(function = "validate_grant_schema_role"))]
pub struct GrantSchemaRoleRequest {
    #[validate(length(min = 1))]
    pub package_name: String,
    pub role: SchemaRoleType,
    pub user_id: Option<i64>,
    pub org_id: Option<i64>,
}

fn validate_grant_schema_role(
    req: &GrantSchemaRoleRequest
) -> Result<(), validator::ValidationError> {
    match (&req.user_id, &req.org_id) {
        (Some(_), Some(_)) => {
            let mut err = validator::ValidationError::new("exclusive_target");
            err.message = Some("Cannot specify both user_id and org_id".into());
            Err(err)
        },
        (None, None) => {
            let mut err = validator::ValidationError::new("missing_target");
            err.message = Some("Must specify either user_id or org_id".into());
            Err(err)
        },
        _ => Ok(()),
    }
}

#[derive(serde::Deserialize, Validate, ToSchema)]
pub struct RevokeSchemaRoleRequest {
    pub role_id: i64,
}

#[derive(serde::Deserialize, Validate, ToSchema)]
pub struct GrantOrgRoleRequest {
    pub org_id: i64,
    pub user_id: i64,
    pub role: OrgRoleType,
}

#[derive(serde::Deserialize, Validate, ToSchema)]
pub struct RevokeOrgRoleRequest {
    pub org_id: i64,
    pub user_id: i64,
}

/// Target for a user favourite (either package or org)
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum FavouriteTargetRequest {
    /// Favourite a package by ID
    Package(i64),
    /// Favourite an organization by ID
    Org(i64),
}

/// Request body for creating a user favourite
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateFavouriteRequest {
    #[serde(flatten)]
    /// The target to favourite (package or org)
    pub target: FavouriteTargetRequest,
}

/// Request body for deleting a user favourite
pub type DeleteFavouriteRequest = CreateFavouriteRequest;

#[derive(Serialize, ToSchema)]
pub struct FavouritesCount {
    pub count: u64,
}
