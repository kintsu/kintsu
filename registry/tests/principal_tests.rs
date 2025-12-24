//! Principal extractor tests
//!
//! Tests for cookie/API key/public extraction with valid/invalid/expired scenarios
//!
//! 22 tests covering:
//! - No authentication (6 tests)
//! - Invalid authorization header (6 tests)
//! - Entity resolution from token (3 tests)
//! - Additional edge cases (7 tests)

mod common;

use common::TestRegistryCtx;
use kintsu_registry_db::entities::SchemaRoleType;
use serde_json::json;

// 1. No Authentication Provided Tests

/// Test that POST /roles/package without auth returns 401 Unauthorized
#[actix_web::test]
async fn no_auth_grant_role() {
    let ctx = TestRegistryCtx::new().await;

    ctx.post("/roles/package")
        .json(&json!({
            "package_name": "test-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await
        .assert_unauthorized();
}

/// Test that DELETE /roles/package without auth returns 401 Unauthorized
#[actix_web::test]
async fn no_auth_revoke_role() {
    let ctx = TestRegistryCtx::new().await;

    ctx.delete("/roles/package")
        .json(&json!({
            "role_id": 1
        }))
        .send()
        .await
        .assert_unauthorized();
}

/// Test that POST /packages/publish without auth returns 401 Unauthorized
#[actix_web::test]
async fn no_auth_publish() {
    let ctx = TestRegistryCtx::new().await;

    ctx.post("/packages/publish")
        .bytes(vec![]) // Empty publish data
        .send()
        .await
        .assert_unauthorized();
}

/// Test that POST /org/{id}/tokens without auth returns 401 Unauthorized
#[actix_web::test]
async fn no_auth_org_token() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _, _) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .json(&json!({
            "description": "test token",
            "scopes": ["*"],
            "permissions": ["publish-package"]
        }))
        .send()
        .await
        .assert_unauthorized();
}

/// Test that GET /orgs/mine without auth returns 401 Unauthorized
#[actix_web::test]
async fn no_auth_my_orgs() {
    let ctx = TestRegistryCtx::new().await;

    ctx.get("/orgs/mine")
        .send()
        .await
        .assert_unauthorized();
}

/// Test that POST /favourites without auth returns 401 Unauthorized
#[actix_web::test]
async fn no_auth_create_favourite() {
    let ctx = TestRegistryCtx::new().await;

    ctx.post("/favourites")
        .json(&json!({
            "package_id": 1
        }))
        .send()
        .await
        .assert_unauthorized();
}

// 2. Invalid Authorization Header Tests

/// Test that requests without Bearer prefix return 401
#[actix_web::test]
async fn malformed_auth_no_bearer() {
    let ctx = TestRegistryCtx::new().await;

    ctx.post("/roles/package")
        .header("Authorization", "token123")
        .json(&json!({
            "package_name": "test-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await
        .assert_unauthorized();
}

/// Test that "Bearer " with empty token returns 401
#[actix_web::test]
async fn malformed_auth_empty_bearer() {
    let ctx = TestRegistryCtx::new().await;

    ctx.post("/roles/package")
        .bearer("")
        .json(&json!({
            "package_name": "test-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await
        .assert_unauthorized();
}

/// Test that very long authorization headers return 401
#[actix_web::test]
async fn auth_header_too_long() {
    let ctx = TestRegistryCtx::new().await;
    let long_token = "a".repeat(10000);

    ctx.post("/roles/package")
        .bearer(&long_token)
        .json(&json!({
            "package_name": "test-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await
        .assert_unauthorized();
}

/// Test that invalid token format returns invalid token error
#[actix_web::test]
async fn invalid_token_format() {
    let ctx = TestRegistryCtx::new().await;

    ctx.post("/roles/package")
        .bearer("notavalidtoken")
        .json(&json!({
            "package_name": "test-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await
        .assert_invalid_token_error();
}

/// Test that expired tokens return invalid token error
/// (Current implementation doesn't distinguish between invalid and expired)
#[actix_web::test]
async fn expired_token() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_expired_token().await;

    ctx.post("/roles/package")
        .bearer(&token)
        .json(&json!({
            "package_name": "test-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await
        .assert_invalid_token_error();
}

/// Test that revoked tokens return invalid token error
#[actix_web::test]
async fn revoked_token() {
    let ctx = TestRegistryCtx::new().await;
    let (user, token) = ctx.create_user_with_token().await;

    // Revoke the token (using a different principal to have permission)
    use kintsu_registry_db::{engine::PrincipalIdentity, entities::ApiKey};
    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    // Get the api key and revoke it
    let api_key = ApiKey::by_raw_token(ctx.conn(), &secrecy::SecretString::from(token.clone()))
        .await
        .unwrap();

    api_key
        .revoke_token(ctx.conn(), &principal)
        .await
        .unwrap();

    ctx.post("/roles/package")
        .bearer(&token)
        .json(&json!({
            "package_name": "test-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await
        .assert_invalid_token_error();
}

// 3. Entity Resolution from Token Tests

/// Test that valid user API key authenticates and resolves principal
#[actix_web::test]
async fn user_api_key_resolves_user() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    // Should not get 401 - authentication succeeded
    let resp = ctx
        .post("/roles/package")
        .bearer(&token)
        .json(&json!({
            "package_name": "nonexistent-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await;

    // If we got past 401, authentication worked
    assert_ne!(
        resp.status(),
        actix_web::http::StatusCode::UNAUTHORIZED,
        "User API key should authenticate successfully"
    );
}

/// Test that org API key authenticates and resolves principal
#[actix_web::test]
async fn org_api_key_resolves_org() {
    let ctx = TestRegistryCtx::new().await;
    use kintsu_registry_db::{engine::PrincipalIdentity, entities::Permission, fixtures};

    let org = fixtures::org()
        .insert(ctx.conn())
        .await
        .unwrap();
    let user = fixtures::user()
        .insert(ctx.conn())
        .await
        .unwrap();

    // Make user an admin of the org
    fixtures::org_role(org.id, user.id)
        .admin()
        .insert(ctx.conn())
        .await
        .unwrap();

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    // Create an org token
    let one_time = fixtures::api_key()
        .org(org.id)
        .permissions(vec![Permission::PublishPackage])
        .insert(ctx.conn(), &principal)
        .await
        .unwrap();

    // Use the org token
    let resp = ctx
        .post("/roles/package")
        .bearer(&one_time.key)
        .json(&json!({
            "package_name": "nonexistent-package",
            "org_id": org.id,
            "role": "Admin"
        }))
        .send()
        .await;

    // If we got past 401, authentication worked
    assert_ne!(
        resp.status(),
        actix_web::http::StatusCode::UNAUTHORIZED,
        "Org API key should authenticate successfully"
    );
}

// 4. Additional Edge Case Tests

/// Test that token with wrong prefix returns invalid token
#[actix_web::test]
async fn wrong_token_prefix() {
    let ctx = TestRegistryCtx::new().await;

    // Token should start with "kintsu_" according to the token module
    ctx.post("/roles/package")
        .bearer("wrong_prefix_token_that_is_64_characters_long_aaaaaaaaaaa")
        .json(&json!({
            "package_name": "test-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await
        .assert_invalid_token_error();
}

/// Test that token with correct prefix but wrong length returns invalid token
#[actix_web::test]
async fn token_wrong_length() {
    let ctx = TestRegistryCtx::new().await;

    // Token should be exactly 64 characters
    ctx.post("/roles/package")
        .bearer("kintsu_short")
        .json(&json!({
            "package_name": "test-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await
        .assert_invalid_token_error();
}

/// Test that token with correct format but non-existent returns invalid token
#[actix_web::test]
async fn token_correct_format_but_nonexistent() {
    let ctx = TestRegistryCtx::new().await;

    // Valid format (kintsu_ prefix, 64 chars total) but doesn't exist in DB
    let fake_token = "kintsu_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    ctx.post("/roles/package")
        .bearer(fake_token)
        .json(&json!({
            "package_name": "test-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await
        .assert_invalid_token_error();
}

/// Test that user with specific scoped token can authenticate
#[actix_web::test]
async fn scoped_token_authenticates() {
    let ctx = TestRegistryCtx::new().await;
    use kintsu_registry_db::{engine::PrincipalIdentity, entities::Permission, fixtures};

    let user = fixtures::user()
        .insert(ctx.conn())
        .await
        .unwrap();
    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    // Create token with specific scope pattern
    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(vec!["mypackage/*"])
        .permissions(vec![Permission::PublishPackage])
        .insert(ctx.conn(), &principal)
        .await
        .unwrap();

    // Should authenticate (scope check happens later)
    let resp = ctx
        .post("/roles/package")
        .bearer(&one_time.key)
        .json(&json!({
            "package_name": "mypackage/subpackage",
            "user_id": user.id,
            "role": "Admin"
        }))
        .send()
        .await;

    assert_ne!(
        resp.status(),
        actix_web::http::StatusCode::UNAUTHORIZED,
        "Scoped token should authenticate successfully"
    );
}

/// Test that multiple invalid auth methods both fail
#[actix_web::test]
async fn multiple_invalid_auth_methods_fail() {
    let ctx = TestRegistryCtx::new().await;

    // Send both invalid bearer token and no session cookie
    ctx.post("/roles/package")
        .bearer("invalid-token")
        .json(&json!({
            "package_name": "test-package",
            "user_id": 1,
            "role": "Admin"
        }))
        .send()
        .await
        .assert_unauthorized();
}

/// Test that routes not requiring auth work without auth
#[actix_web::test]
async fn public_route_without_auth() {
    let ctx = TestRegistryCtx::new().await;

    // GET /package/{name}/{version} is public (doesn't require Principal)
    let resp = ctx
        .get("/package/some-package/1.0.0")
        .send()
        .await;

    // Should be 404 (package not found), not 401 (unauthorized)
    assert_ne!(
        resp.status(),
        actix_web::http::StatusCode::UNAUTHORIZED,
        "Public routes should not require authentication"
    );
}

/// Test that listing packages works without auth
#[actix_web::test]
async fn list_packages_without_auth() {
    let ctx = TestRegistryCtx::new().await;

    // GET /packages is public
    let resp = ctx.get("/packages").send().await;

    // Should be 200 with empty list or similar, not 401
    assert_ne!(
        resp.status(),
        actix_web::http::StatusCode::UNAUTHORIZED,
        "List packages should be public"
    );
}
