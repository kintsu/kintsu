//! Test scenario builders for complex multi-entity test setups
//!
//! ScenarioBuilder orchestrates high-level test scenarios involving
//! multiple related entities and their relationships.

use chrono::{Duration, Utc};
use kintsu_registry_db::{
    PackageStorage, Result,
    engine::{OneTimeApiKey, OwnerId, PrincipalIdentity},
    entities::*,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::fixtures::{self, org_role, schema_role, version};

pub struct ScenarioBuilder<'a> {
    db: &'a DatabaseConnection,
    storage: Option<Arc<PackageStorage>>,
}

impl<'a> ScenarioBuilder<'a> {
    /// Create a new scenario builder with database connection
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db, storage: None }
    }

    /// Attach storage for package operations
    pub fn with_storage(
        mut self,
        storage: Arc<PackageStorage>,
    ) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Create a user who is admin of a new organization
    pub async fn create_user_with_org(
        &self,
        org_name: &str,
    ) -> Result<(User, Org)> {
        let user = fixtures::user().insert(self.db).await?;
        let org = fixtures::org()
            .name(org_name)
            .insert(self.db)
            .await?;

        // Grant admin role
        org_role(org.id, user.id)
            .admin()
            .insert(self.db)
            .await?;

        Ok((user, org))
    }

    /// Create a package with an initial version
    pub async fn create_package_with_version(
        &self,
        owner: OwnerId,
        pkg_name: &str,
        version_str: &str,
    ) -> Result<(Package, Version)> {
        let pkg = fixtures::package()
            .name(pkg_name)
            .insert(self.db)
            .await?;

        // Create version with appropriate publisher
        let ver = match owner {
            OwnerId::User(user_id) => {
                version(pkg.id)
                    .version(version_str)
                    .publisher_user(user_id)
                    .insert(self.db)
                    .await?
            },
            OwnerId::Org(org_id) => {
                version(pkg.id)
                    .version(version_str)
                    .publisher_org(org_id)
                    .insert(self.db)
                    .await?
            },
        };

        // Grant admin role to owner
        match owner {
            OwnerId::User(user_id) => {
                schema_role(pkg.id)
                    .user(user_id)
                    .admin()
                    .insert(self.db)
                    .await?;
            },
            OwnerId::Org(org_id) => {
                schema_role(pkg.id)
                    .org(org_id)
                    .admin()
                    .insert(self.db)
                    .await?;
            },
        }

        Ok((pkg, ver))
    }

    /// Create a user with a personal API key
    pub async fn create_user_with_api_key(
        &self,
        perms: Vec<Permission>,
    ) -> Result<(User, OneTimeApiKey)> {
        let user = fixtures::user().insert(self.db).await?;

        // Create session principal for token creation
        let principal = PrincipalIdentity::UserSession { user: user.clone() };

        let key = fixtures::api_key()
            .user(user.id)
            .permissions(perms)
            .scopes(vec!["*"])
            .insert(self.db, &principal)
            .await?;

        Ok((user, key))
    }

    /// Create a user with an API key scoped to specific packages
    pub async fn create_user_with_scoped_api_key(
        &self,
        perms: Vec<Permission>,
        scopes: Vec<&str>,
    ) -> Result<(User, OneTimeApiKey)> {
        let user = fixtures::user().insert(self.db).await?;
        let principal = PrincipalIdentity::UserSession { user: user.clone() };

        let key = fixtures::api_key()
            .user(user.id)
            .permissions(perms)
            .scopes(scopes)
            .insert(self.db, &principal)
            .await?;

        Ok((user, key))
    }

    /// Grant organization role to a user
    pub async fn grant_org_role(
        &self,
        org_id: i64,
        user_id: i64,
        role: OrgRoleType,
    ) -> Result<OrgRole> {
        let fixture = org_role(org_id, user_id);
        let fixture = match role {
            OrgRoleType::Admin => fixture.admin(),
            OrgRoleType::Member => fixture.member(),
        };
        fixture.insert(self.db).await
    }

    /// Grant schema role to a user or org
    pub async fn grant_schema_role(
        &self,
        pkg_id: i64,
        owner: OwnerId,
        role: SchemaRoleType,
    ) -> Result<SchemaRole> {
        let fixture = schema_role(pkg_id);
        let fixture = match owner {
            OwnerId::User(id) => fixture.user(id),
            OwnerId::Org(id) => fixture.org(id),
        };
        let fixture = match role {
            SchemaRoleType::Admin => fixture.admin(),
            SchemaRoleType::Author => fixture.author(),
        };
        fixture.insert(self.db).await
    }

    /// Create an org with an API key
    pub async fn create_org_with_api_key(
        &self,
        org_name: &str,
        perms: Vec<Permission>,
    ) -> Result<(User, Org, OneTimeApiKey)> {
        let (user, org) = self.create_user_with_org(org_name).await?;
        let principal = PrincipalIdentity::UserSession { user: user.clone() };

        let key = fixtures::api_key()
            .org(org.id)
            .permissions(perms)
            .scopes(vec!["*"])
            .insert(self.db, &principal)
            .await?;

        Ok((user, org, key))
    }

    /// Create a complete publishing scenario:
    /// - User with API key having PublishPackage permission
    /// - User is admin of their published packages
    pub async fn create_publisher(&self) -> Result<(User, OneTimeApiKey)> {
        self.create_user_with_api_key(vec![Permission::PublishPackage, Permission::YankPackage])
            .await
    }

    /// Create user, org, and package where org owns the package
    pub async fn create_org_owned_package(
        &self,
        org_name: &str,
        pkg_name: &str,
    ) -> Result<(User, Org, Package, Version)> {
        let (user, org) = self.create_user_with_org(org_name).await?;
        let (pkg, ver) = self
            .create_package_with_version(OwnerId::Org(org.id), pkg_name, "1.0.0")
            .await?;

        Ok((user, org, pkg, ver))
    }

    /// Create user and package where user owns the package
    pub async fn create_user_owned_package(
        &self,
        pkg_name: &str,
    ) -> Result<(User, Package, Version)> {
        let user = fixtures::user().insert(self.db).await?;
        let (pkg, ver) = self
            .create_package_with_version(OwnerId::User(user.id), pkg_name, "1.0.0")
            .await?;

        Ok((user, pkg, ver))
    }
}

/// Helper to create a session principal from a user
pub fn session_principal(user: User) -> PrincipalIdentity {
    PrincipalIdentity::UserSession { user }
}

/// Helper to check if an error matches expected variant
#[macro_export]
macro_rules! assert_error_matches {
    ($result:expr, $pattern:pat) => {
        match $result {
            Err($pattern) => {},
            Err(e) => {
                panic!(
                    "Expected error matching {}, got: {:?}",
                    stringify!($pattern),
                    e
                )
            },
            Ok(v) => {
                panic!(
                    "Expected error matching {}, got Ok: {:?}",
                    stringify!($pattern),
                    v
                )
            },
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_principal_from_user() {
        // Basic compile test for helper
        let user = User {
            id: 1,
            email: "test@example.com".to_string(),
            gh_id: 123,
            gh_login: "test".to_string(),
            gh_avatar: None,
        };
        let _principal = session_principal(user);
    }
}
