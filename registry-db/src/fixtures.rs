//! Test fixtures for registry-db integration tests
//!
//! Provides fluent builder-style fixtures for creating test data.
//! Each fixture generates unique defaults using atomic counters.
//!
//! This module is only available when the `test` feature is enabled.

use crate::{
    Result,
    engine::{NewApiKey, OneTimeApiKey, PrincipalIdentity},
    entities::*,
};
use chrono::{DateTime, Duration, Utc};
use kintsu_manifests::version::{VersionSerde, parse_version};
use sea_orm::{ActiveModelTrait, DatabaseConnection, NotSet, Set};
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};

// Atomic counters for unique test data generation
static USER_COUNTER: AtomicI64 = AtomicI64::new(1);
static ORG_COUNTER: AtomicI64 = AtomicI64::new(1);
static PACKAGE_COUNTER: AtomicI64 = AtomicI64::new(1);
static GH_ID_COUNTER: AtomicI32 = AtomicI32::new(1000);

fn next_user_n() -> i64 {
    USER_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn next_org_n() -> i64 {
    ORG_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn next_package_n() -> i64 {
    PACKAGE_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn next_gh_id() -> i32 {
    GH_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

pub struct UserFixture {
    email: Option<String>,
    gh_id: Option<i32>,
    gh_login: Option<String>,
    gh_avatar: Option<String>,
}

pub fn user() -> UserFixture {
    UserFixture {
        email: None,
        gh_id: None,
        gh_login: None,
        gh_avatar: Some("https://github.com/avatar".to_string()),
    }
}

impl UserFixture {
    pub fn email(
        mut self,
        email: &str,
    ) -> Self {
        self.email = Some(email.to_string());
        self
    }

    pub fn gh_id(
        mut self,
        id: i32,
    ) -> Self {
        self.gh_id = Some(id);
        self
    }

    pub fn gh_login(
        mut self,
        login: &str,
    ) -> Self {
        self.gh_login = Some(login.to_string());
        self
    }

    pub fn gh_avatar(
        mut self,
        avatar: Option<&str>,
    ) -> Self {
        self.gh_avatar = avatar.map(|s| s.to_string());
        self
    }

    pub async fn insert(
        self,
        db: &DatabaseConnection,
    ) -> Result<User> {
        let n = next_user_n();
        let gh_id = self.gh_id.unwrap_or_else(next_gh_id);

        let active_model = UserActiveModel {
            id: NotSet,
            email: Set(self
                .email
                .unwrap_or_else(|| format!("test-{}@example.com", n))),
            gh_id: Set(gh_id),
            gh_login: Set(self
                .gh_login
                .unwrap_or_else(|| format!("testuser{}", n))),
            gh_avatar: Set(self.gh_avatar),
        };

        active_model
            .insert(db)
            .await
            .map_err(Into::into)
    }
}

pub struct OrgFixture {
    name: Option<String>,
    gh_id: Option<i32>,
    gh_avatar: Option<String>,
}

pub fn org() -> OrgFixture {
    OrgFixture {
        name: None,
        gh_id: None,
        gh_avatar: Some("https://github.com/org-avatar".to_string()),
    }
}

impl OrgFixture {
    pub fn name(
        mut self,
        name: &str,
    ) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn gh_id(
        mut self,
        id: i32,
    ) -> Self {
        self.gh_id = Some(id);
        self
    }

    pub fn gh_avatar(
        mut self,
        avatar: &str,
    ) -> Self {
        self.gh_avatar = Some(avatar.to_string());
        self
    }

    pub async fn insert(
        self,
        db: &DatabaseConnection,
    ) -> Result<Org> {
        let n = next_org_n();
        let gh_id = self.gh_id.unwrap_or_else(next_gh_id);

        let active_model = OrgActiveModel {
            id: NotSet,
            name: Set(self
                .name
                .unwrap_or_else(|| format!("testorg{}", n))),
            gh_id: Set(gh_id),
            gh_avatar: Set(self
                .gh_avatar
                .unwrap_or_else(|| "https://github.com/org-avatar".to_string())),
        };

        active_model
            .insert(db)
            .await
            .map_err(Into::into)
    }
}

pub struct PackageFixture {
    name: Option<String>,
}

pub fn package() -> PackageFixture {
    PackageFixture { name: None }
}

impl PackageFixture {
    pub fn name(
        mut self,
        name: &str,
    ) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub async fn insert(
        self,
        db: &DatabaseConnection,
    ) -> Result<Package> {
        let n = next_package_n();

        let active_model = PackageActiveModel {
            id: NotSet,
            name: Set(self
                .name
                .unwrap_or_else(|| format!("test-package{}", n))),
        };

        active_model
            .insert(db)
            .await
            .map_err(Into::into)
    }
}

pub struct VersionFixture {
    package_id: i64,
    qualified_version: String,
    source_checksum: String,
    declarations_checksum: String,
    description: Option<String>,
    homepage: Option<String>,
    license: String,
    license_text: String,
    readme: String,
    repository: String,
    dependencies: Vec<i64>,
    keywords: Vec<String>,
    publishing_user_id: Option<i64>,
    publishing_org_id: Option<i64>,
}

pub fn version(package_id: i64) -> VersionFixture {
    VersionFixture {
        package_id,
        qualified_version: "1.0.0".to_string(),
        source_checksum: "fake-src-checksum".to_string(),
        declarations_checksum: "fake-decl-checksum".to_string(),
        description: Some("Test package".to_string()),
        homepage: None,
        license: "MIT".to_string(),
        license_text: "".to_string(),
        readme: "# Test".to_string(),
        repository: "https://github.com/test/test".to_string(),
        dependencies: vec![],
        keywords: vec![],
        publishing_user_id: None,
        publishing_org_id: None,
    }
}

impl VersionFixture {
    pub fn version(
        mut self,
        v: &str,
    ) -> Self {
        self.qualified_version = v.to_string();
        self
    }

    pub fn source_checksum(
        mut self,
        checksum: &str,
    ) -> Self {
        self.source_checksum = checksum.to_string();
        self
    }

    pub fn declarations_checksum(
        mut self,
        checksum: &str,
    ) -> Self {
        self.declarations_checksum = checksum.to_string();
        self
    }

    pub fn description(
        mut self,
        desc: Option<&str>,
    ) -> Self {
        self.description = desc.map(|s| s.to_string());
        self
    }

    pub fn homepage(
        mut self,
        hp: Option<&str>,
    ) -> Self {
        self.homepage = hp.map(|s| s.to_string());
        self
    }

    pub fn license(
        mut self,
        lic: &str,
    ) -> Self {
        self.license = lic.to_string();
        self
    }

    pub fn readme(
        mut self,
        rm: &str,
    ) -> Self {
        self.readme = rm.to_string();
        self
    }

    pub fn repository(
        mut self,
        repo: &str,
    ) -> Self {
        self.repository = repo.to_string();
        self
    }

    pub fn dependencies(
        mut self,
        deps: Vec<i64>,
    ) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn keywords(
        mut self,
        kw: Vec<&str>,
    ) -> Self {
        self.keywords = kw
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        self
    }

    pub fn publisher_user(
        mut self,
        user_id: i64,
    ) -> Self {
        self.publishing_user_id = Some(user_id);
        self.publishing_org_id = None;
        self
    }

    pub fn publisher_org(
        mut self,
        org_id: i64,
    ) -> Self {
        self.publishing_org_id = Some(org_id);
        self.publishing_user_id = None;
        self
    }

    pub async fn insert(
        self,
        db: &DatabaseConnection,
    ) -> Result<Version> {
        let active_model = VersionActiveModel {
            id: NotSet,
            package: Set(self.package_id),
            qualified_version: Set(VersionSerde(
                parse_version(&self.qualified_version).unwrap(),
            )),
            source_checksum: Set(self.source_checksum),
            declarations_checksum: Set(self.declarations_checksum),
            description: Set(self.description),
            homepage: Set(self.homepage),
            license: Set(self.license),
            license_text: Set(self.license_text),
            readme: Set(self.readme),
            repository: Set(self.repository),
            dependencies: Set(self.dependencies),
            keywords: Set(self.keywords),
            created_at: Set(Utc::now()),
            yanked_at: Set(None),
            publishing_org_id: Set(self.publishing_org_id),
            publishing_user_id: Set(self.publishing_user_id),
        };

        active_model
            .insert(db)
            .await
            .map_err(Into::into)
    }
}

pub struct ApiKeyFixture {
    description: Option<String>,
    expires: DateTime<Utc>,
    scopes: Vec<String>,
    permissions: Vec<Permission>,
    user_id: Option<i64>,
    org_id: Option<i64>,
}

pub fn api_key() -> ApiKeyFixture {
    ApiKeyFixture {
        description: Some("Test key".to_string()),
        expires: Utc::now() + Duration::days(30),
        scopes: vec!["*".to_string()],
        permissions: vec![Permission::PublishPackage],
        user_id: None,
        org_id: None,
    }
}

impl ApiKeyFixture {
    pub fn description(
        mut self,
        desc: Option<&str>,
    ) -> Self {
        self.description = desc.map(|s| s.to_string());
        self
    }

    pub fn expires(
        mut self,
        exp: DateTime<Utc>,
    ) -> Self {
        self.expires = exp;
        self
    }

    pub fn scopes(
        mut self,
        scopes: Vec<&str>,
    ) -> Self {
        self.scopes = scopes
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        self
    }

    pub fn permissions(
        mut self,
        perms: Vec<Permission>,
    ) -> Self {
        self.permissions = perms;
        self
    }

    pub fn user(
        mut self,
        user_id: i64,
    ) -> Self {
        self.user_id = Some(user_id);
        self.org_id = None;
        self
    }

    pub fn org(
        mut self,
        org_id: i64,
    ) -> Self {
        self.org_id = Some(org_id);
        self.user_id = None;
        self
    }

    pub async fn insert(
        self,
        db: &DatabaseConnection,
        principal: &PrincipalIdentity,
    ) -> Result<OneTimeApiKey> {
        let scopes: Vec<Scope> = self
            .scopes
            .into_iter()
            .map(Scope::new)
            .collect();

        if let Some(user_id) = self.user_id {
            NewApiKey::new_for_user(
                self.description,
                scopes,
                self.permissions,
                self.expires,
                user_id,
            )
            .qualify(db, principal)
            .await
        } else if let Some(org_id) = self.org_id {
            NewApiKey::new_for_org(
                self.description,
                scopes,
                self.permissions,
                self.expires,
                org_id,
            )
            .qualify(db, principal)
            .await
        } else {
            Err(crate::Error::Validation(
                "ApiKeyFixture requires either user or org".to_string(),
            ))
        }
    }
}

pub struct OrgRoleFixture {
    org_id: i64,
    user_id: i64,
    role: OrgRoleType,
}

pub fn org_role(
    org_id: i64,
    user_id: i64,
) -> OrgRoleFixture {
    OrgRoleFixture {
        org_id,
        user_id,
        role: OrgRoleType::Member,
    }
}

impl OrgRoleFixture {
    pub fn admin(mut self) -> Self {
        self.role = OrgRoleType::Admin;
        self
    }

    pub fn member(mut self) -> Self {
        self.role = OrgRoleType::Member;
        self
    }

    pub async fn insert(
        self,
        db: &DatabaseConnection,
    ) -> Result<OrgRole> {
        let active_model = OrgRoleActiveModel {
            org_id: Set(self.org_id),
            user_id: Set(self.user_id),
            role: Set(self.role),
            revoked_at: Set(None),
        };

        active_model
            .insert(db)
            .await
            .map_err(Into::into)
    }
}

pub struct SchemaRoleFixture {
    package_id: i64,
    user_id: Option<i64>,
    org_id: Option<i64>,
    role: SchemaRoleType,
}

pub fn schema_role(package_id: i64) -> SchemaRoleFixture {
    SchemaRoleFixture {
        package_id,
        user_id: None,
        org_id: None,
        role: SchemaRoleType::Admin,
    }
}

impl SchemaRoleFixture {
    pub fn user(
        mut self,
        user_id: i64,
    ) -> Self {
        self.user_id = Some(user_id);
        self.org_id = None;
        self
    }

    pub fn org(
        mut self,
        org_id: i64,
    ) -> Self {
        self.org_id = Some(org_id);
        self.user_id = None;
        self
    }

    pub fn admin(mut self) -> Self {
        self.role = SchemaRoleType::Admin;
        self
    }

    pub fn author(mut self) -> Self {
        self.role = SchemaRoleType::Author;
        self
    }

    pub fn revoked(self) -> SchemaRoleFixtureRevoked {
        SchemaRoleFixtureRevoked {
            base: self,
            revoked_at: Utc::now(),
        }
    }

    pub async fn insert(
        self,
        db: &DatabaseConnection,
    ) -> Result<SchemaRole> {
        let active_model = SchemaRoleActiveModel {
            id: NotSet,
            package: Set(self.package_id),
            user_id: Set(self.user_id),
            org_id: Set(self.org_id),
            role: Set(self.role),
            revoked_at: Set(None),
        };

        active_model
            .insert(db)
            .await
            .map_err(Into::into)
    }
}

pub struct SchemaRoleFixtureRevoked {
    base: SchemaRoleFixture,
    revoked_at: DateTime<Utc>,
}

impl SchemaRoleFixtureRevoked {
    pub async fn insert(
        self,
        db: &DatabaseConnection,
    ) -> Result<SchemaRole> {
        let active_model = SchemaRoleActiveModel {
            id: NotSet,
            package: Set(self.base.package_id),
            user_id: Set(self.base.user_id),
            org_id: Set(self.base.org_id),
            role: Set(self.base.role),
            revoked_at: Set(Some(self.revoked_at)),
        };

        active_model
            .insert(db)
            .await
            .map_err(Into::into)
    }
}

pub struct DownloadsFixture {
    version_id: i64,
    count: i32,
    day: chrono::NaiveDate,
}

pub fn downloads(
    version_id: i64,
    count: i32,
) -> DownloadsFixture {
    DownloadsFixture {
        version_id,
        count,
        day: Utc::now().date_naive(),
    }
}

impl DownloadsFixture {
    pub fn day(
        mut self,
        day: chrono::NaiveDate,
    ) -> Self {
        self.day = day;
        self
    }

    pub fn count(
        mut self,
        count: i32,
    ) -> Self {
        self.count = count;
        self
    }

    pub async fn insert(
        self,
        db: &DatabaseConnection,
    ) -> Result<Downloads> {
        let active_model = DownloadsActiveModel {
            version: Set(self.version_id),
            day: Set(self.day),
            count: Set(self.count),
        };

        active_model
            .insert(db)
            .await
            .map_err(Into::into)
    }
}

pub struct OrgInvitationFixture {
    org_id: i64,
    inviting_user_id: i64,
    invited_user_gh_login: String,
    role: OrgRoleType,
}

pub fn org_invitation(
    org_id: i64,
    inviting_user_id: i64,
    invited_gh_login: &str,
) -> OrgInvitationFixture {
    OrgInvitationFixture {
        org_id,
        inviting_user_id,
        invited_user_gh_login: invited_gh_login.to_string(),
        role: OrgRoleType::Member,
    }
}

impl OrgInvitationFixture {
    pub fn admin(mut self) -> Self {
        self.role = OrgRoleType::Admin;
        self
    }

    pub fn member(mut self) -> Self {
        self.role = OrgRoleType::Member;
        self
    }

    pub async fn insert(
        self,
        db: &DatabaseConnection,
    ) -> Result<OrgInvitation> {
        let active_model = OrgInvitationActiveModel {
            id: NotSet,
            org_id: Set(self.org_id),
            inviting_user_id: Set(self.inviting_user_id),
            invited_user_gh_login: Set(self.invited_user_gh_login),
            role: Set(self.role),
            created_at: NotSet,
            accepted_at: NotSet,
            revoked_at: NotSet,
        };

        active_model
            .insert(db)
            .await
            .map_err(Into::into)
    }
}
