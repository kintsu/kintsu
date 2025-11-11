use kintsu_manifests::config::NewForNamed;
use kintsu_registry_db::entities::{Permission, Scope};
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
    pub manifest: kintsu_manifests::package::PackageManifest,
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

                    if name == kintsu_manifests::package::PackageManifest::NAME {
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
