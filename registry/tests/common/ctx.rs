//! Test registry context - combines database and storage containers with app

use actix_web::{App, cookie::Key, dev::ServiceResponse, test, web};
use kintsu_parser::declare::DeclarationVersion;
use kintsu_registry::{
    app::ApiDoc,
    bind_app,
    routes::{auth, favourites, org, packages},
};
use kintsu_registry_db::{
    engine::PrincipalIdentity,
    entities::{Permission, User},
    fixtures,
    tst::TestDbCtx,
};
use kintsu_registry_storage::{manager::StorageManager, tst::TestS3Ctx};
use secrecy::SecretString;
use utoipa::OpenApi;
use utoipa_actix_web::AppExt;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};

use super::RequestBuilder;

/// Test registry context providing database, storage, and app for integration tests
pub struct TestRegistryCtx {
    pub db: TestDbCtx,
    pub s3: TestS3Ctx,
    pub storage: web::Data<StorageManager<DeclarationVersion>>,
    pub cookie_key: web::Data<Key>,
    pub session_config: web::Data<kintsu_registry::config::SessionConfig>,
    pub client: web::Data<kintsu_registry::oauth::AuthClient>,
}

const TEST_SESSION_KEY: &str =
    "test-session-key-must-be-at-least-64-bytes-long-for-cookie-key-derivation-0123456789";

impl TestRegistryCtx {
    /// Create a new test context with database and storage containers
    pub async fn new() -> Self {
        let (db, s3) = tokio::join!(TestDbCtx::new(), TestS3Ctx::new());

        let storage = web::Data::new(s3.managed::<DeclarationVersion>().await);
        let cookie_key = web::Data::new(Key::derive_from(TEST_SESSION_KEY.as_bytes()));
        let session_config = web::Data::new(kintsu_registry::config::SessionConfig {
            domain: "localhost".to_string(),
            key: SecretString::from(TEST_SESSION_KEY),
        });

        // Create a mock OAuth config for testing
        let gh_config = kintsu_registry::oauth::GhOauthConfig {
            base_url: url::Url::parse("https://github.com").unwrap(),
            api_url: url::Url::parse("https://api.github.com").unwrap(),
            client: kintsu_registry::oauth::GhClientConfig {
                id: "test-client-id".to_string(),
                secret: SecretString::from("test-client-secret"),
            },
        };
        let client = web::Data::new(kintsu_registry::oauth::AuthClient::new(gh_config).unwrap());

        Self {
            db,
            s3,
            storage,
            cookie_key,
            session_config,
            client,
        }
    }

    /// Get database connection
    pub fn conn(&self) -> &sea_orm::DatabaseConnection {
        &self.db.conn
    }

    /// Build the actix-web test app with all routes configured using bind_app! macro
    pub async fn app(
        &self
    ) -> impl actix_web::dev::Service<
        actix_http::Request,
        Response = ServiceResponse,
        Error = actix_web::Error,
    > {
        let db = web::Data::new(self.db.conn.clone());
        let s3 = self.storage.clone();
        let session_config = self.session_config.clone();
        let cookie_key = self.cookie_key.clone();
        let client = self.client.clone();

        test::init_service(bind_app!(session_config, db, s3, client, cookie_key,)()).await
    }

    // Helper methods for common test setups

    /// Create a user and return with an API token
    pub async fn create_user_with_token(&self) -> (User, String) {
        let user = fixtures::user()
            .insert(&self.db.conn)
            .await
            .unwrap();
        let principal = PrincipalIdentity::UserSession { user: user.clone() };
        let one_time = fixtures::api_key()
            .user(user.id)
            .permissions(vec![
                Permission::PublishPackage,
                Permission::YankPackage,
                Permission::GrantSchemaRole,
                Permission::RevokeSchemaRole,
                Permission::GrantOrgRole,
                Permission::RevokeOrgRole,
            ])
            .insert(&self.db.conn, &principal)
            .await
            .unwrap();
        (user, one_time.key)
    }

    /// Create a user with only publish permissions
    pub async fn create_publisher(&self) -> (User, String) {
        let user = fixtures::user()
            .insert(&self.db.conn)
            .await
            .unwrap();
        let principal = PrincipalIdentity::UserSession { user: user.clone() };
        let one_time = fixtures::api_key()
            .user(user.id)
            .permissions(vec![Permission::PublishPackage])
            .insert(&self.db.conn, &principal)
            .await
            .unwrap();
        (user, one_time.key)
    }

    /// Create a user with read-only token (no write permissions)
    pub async fn create_reader(&self) -> (User, String) {
        let user = fixtures::user()
            .insert(&self.db.conn)
            .await
            .unwrap();
        let principal = PrincipalIdentity::UserSession { user: user.clone() };
        let one_time = fixtures::api_key()
            .user(user.id)
            .permissions(vec![]) // No permissions - read only
            .insert(&self.db.conn, &principal)
            .await
            .unwrap();
        (user, one_time.key)
    }

    /// Create an org and an admin user with token
    pub async fn create_org_with_admin(&self) -> (kintsu_registry_db::entities::Org, User, String) {
        let org = fixtures::org()
            .insert(&self.db.conn)
            .await
            .unwrap();
        let user = fixtures::user()
            .insert(&self.db.conn)
            .await
            .unwrap();
        fixtures::org_role(org.id, user.id)
            .admin()
            .insert(&self.db.conn)
            .await
            .unwrap();
        let principal = PrincipalIdentity::UserSession { user: user.clone() };
        let one_time = fixtures::api_key()
            .user(user.id)
            .permissions(vec![
                Permission::PublishPackage,
                Permission::YankPackage,
                Permission::GrantSchemaRole,
                Permission::RevokeSchemaRole,
                Permission::GrantOrgRole,
                Permission::RevokeOrgRole,
                Permission::CreateOrgToken,
                Permission::RevokeOrgToken,
                Permission::ListOrgToken,
            ])
            .insert(&self.db.conn, &principal)
            .await
            .unwrap();
        (org, user, one_time.key)
    }

    /// Create an org member (non-admin) with token
    pub async fn create_org_member(
        &self,
        org_id: i64,
    ) -> (User, String) {
        let user = fixtures::user()
            .insert(&self.db.conn)
            .await
            .unwrap();
        fixtures::org_role(org_id, user.id)
            .member()
            .insert(&self.db.conn)
            .await
            .unwrap();
        let principal = PrincipalIdentity::UserSession { user: user.clone() };
        let one_time = fixtures::api_key()
            .user(user.id)
            .permissions(vec![Permission::PublishPackage])
            .insert(&self.db.conn, &principal)
            .await
            .unwrap();
        (user, one_time.key)
    }

    /// Create an expired API key for testing expiration handling
    pub async fn create_user_with_expired_token(&self) -> (User, String) {
        use chrono::{Duration, Utc};
        let user = fixtures::user()
            .insert(&self.db.conn)
            .await
            .unwrap();
        let principal = PrincipalIdentity::UserSession { user: user.clone() };
        let one_time = fixtures::api_key()
            .user(user.id)
            .expires(Utc::now() - Duration::days(1)) // Expired yesterday
            .insert(&self.db.conn, &principal)
            .await
            .unwrap();
        (user, one_time.key)
    }

    /// Create a user with a single specific permission
    pub async fn create_user_with_token_and_permission(
        &self,
        permission: Permission,
    ) -> (User, String) {
        let user = fixtures::user()
            .insert(&self.db.conn)
            .await
            .unwrap();
        let principal = PrincipalIdentity::UserSession { user: user.clone() };
        let one_time = fixtures::api_key()
            .user(user.id)
            .permissions(vec![permission])
            .insert(&self.db.conn, &principal)
            .await
            .unwrap();
        (user, one_time.key)
    }
}

// Convenience methods for making requests
impl TestRegistryCtx {
    /// Start a GET request builder
    pub fn get<'a>(
        &'a self,
        path: &str,
    ) -> RequestBuilder<'a> {
        RequestBuilder::get(self, path)
    }

    /// Start a POST request builder
    pub fn post<'a>(
        &'a self,
        path: &str,
    ) -> RequestBuilder<'a> {
        RequestBuilder::post(self, path)
    }

    /// Start a DELETE request builder
    pub fn delete<'a>(
        &'a self,
        path: &str,
    ) -> RequestBuilder<'a> {
        RequestBuilder::delete(self, path)
    }

    /// Start a PUT request builder
    pub fn put<'a>(
        &'a self,
        path: &str,
    ) -> RequestBuilder<'a> {
        RequestBuilder::put(self, path)
    }
}
