//! API Key creation and management tests
//!
//! Tests for API key generation, permission validation, and revocation
//!
//! Note: Many token management routes require SessionData (browser auth)
//! rather than API key auth. This test file focuses on routes that accept
//! API key authentication.

mod common;

use common::TestRegistryCtx;
use kintsu_registry_db::{entities::Permission, fixtures};
use serde_json::json;

// POST /org/{id}/tokens - Org Token Creation (Principal-based)

/// Test creating org token as admin succeeds
#[actix_web::test]
async fn create_org_token_as_admin() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .json(&json!({
            "description": "Org token",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_ok();
}

/// Test creating org token as member (non-admin) fails
#[actix_web::test]
async fn create_org_token_as_member() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _admin_user, _admin_token) = ctx.create_org_with_admin().await;
    let (_member, member_token) = ctx.create_org_member(org.id).await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&member_token)
        .json(&json!({
            "description": "Org token",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_forbidden();
}

/// Test creating org token as non-member fails
#[actix_web::test]
async fn create_org_token_as_non_member() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _admin_user, _admin_token) = ctx.create_org_with_admin().await;
    let (_other_user, other_token) = ctx.create_user_with_token().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&other_token)
        .json(&json!({
            "description": "Org token",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_forbidden();
}

/// Test creating org token without auth fails
#[actix_web::test]
async fn create_org_token_no_auth() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _admin_user, _admin_token) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .json(&json!({
            "description": "Org token",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_unauthorized();
}

/// Test creating org token with invalid body returns 400
#[actix_web::test]
async fn create_org_token_invalid_body() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .json(&json!({}))
        .send()
        .await
        .assert_bad_request();
}

// GET /org/{id}/tokens - List Org Tokens (Principal-based)

/// Test listing org tokens as admin succeeds
#[actix_web::test]
async fn list_org_tokens_as_admin() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    ctx.get(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .send()
        .await
        .assert_ok();
}

/// Test listing org tokens as member fails
#[actix_web::test]
async fn list_org_tokens_as_member() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _admin_user, _admin_token) = ctx.create_org_with_admin().await;
    let (_member, member_token) = ctx.create_org_member(org.id).await;

    ctx.get(&format!("/org/{}/tokens", org.id))
        .bearer(&member_token)
        .send()
        .await
        .assert_forbidden();
}

/// Test listing org tokens as non-member fails
#[actix_web::test]
async fn list_org_tokens_as_non_member() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _admin_user, _admin_token) = ctx.create_org_with_admin().await;
    let (_other_user, other_token) = ctx.create_user_with_token().await;

    ctx.get(&format!("/org/{}/tokens", org.id))
        .bearer(&other_token)
        .send()
        .await
        .assert_forbidden();
}

// DELETE /auth/tokens/{id} - Token Revocation (Principal-based)

/// Test revoking own token succeeds
#[actix_web::test]
async fn revoke_own_token() {
    let ctx = TestRegistryCtx::new().await;
    let user = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let principal =
        kintsu_registry_db::engine::PrincipalIdentity::UserSession { user: user.clone() };

    // Create the token to revoke
    let token_to_revoke = fixtures::api_key()
        .user(user.id)
        .permissions(vec![Permission::PublishPackage])
        .description(Some("Token to revoke"))
        .insert(&ctx.db.conn, &principal)
        .await
        .unwrap();

    // Create another token for auth
    let auth_token = fixtures::api_key()
        .user(user.id)
        .permissions(vec![Permission::RevokePersonalToken])
        .description(Some("Auth token"))
        .insert(&ctx.db.conn, &principal)
        .await
        .unwrap();

    ctx.delete(&format!("/auth/tokens/{}", token_to_revoke.api_key.id))
        .bearer(&auth_token.key)
        .send()
        .await
        .assert_ok();
}

/// Test revoking another user's token fails
#[actix_web::test]
async fn revoke_other_user_token() {
    let ctx = TestRegistryCtx::new().await;

    // User A creates a token
    let user_a = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let principal_a = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: user_a.clone(),
    };
    let token_a = fixtures::api_key()
        .user(user_a.id)
        .permissions(vec![Permission::PublishPackage])
        .insert(&ctx.db.conn, &principal_a)
        .await
        .unwrap();

    // User B tries to revoke it
    let user_b = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let principal_b = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: user_b.clone(),
    };
    let token_b = fixtures::api_key()
        .user(user_b.id)
        .permissions(vec![Permission::RevokePersonalToken])
        .insert(&ctx.db.conn, &principal_b)
        .await
        .unwrap();

    ctx.delete(&format!("/auth/tokens/{}", token_a.api_key.id))
        .bearer(&token_b.key)
        .send()
        .await
        .assert_forbidden();
}

/// Test revoking token without auth fails
#[actix_web::test]
async fn revoke_token_no_auth() {
    let ctx = TestRegistryCtx::new().await;
    let user = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let principal =
        kintsu_registry_db::engine::PrincipalIdentity::UserSession { user: user.clone() };
    let token = fixtures::api_key()
        .user(user.id)
        .permissions(vec![Permission::PublishPackage])
        .insert(&ctx.db.conn, &principal)
        .await
        .unwrap();

    ctx.delete(&format!("/auth/tokens/{}", token.api_key.id))
        .send()
        .await
        .assert_unauthorized();
}

// Adversarial Scenarios

/// Test that using a revoked token returns invalid token error
#[actix_web::test]
async fn token_reuse_after_revoke() {
    let ctx = TestRegistryCtx::new().await;
    let user = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let principal =
        kintsu_registry_db::engine::PrincipalIdentity::UserSession { user: user.clone() };

    // Create a token with revoke permission
    let token = fixtures::api_key()
        .user(user.id)
        .permissions(vec![
            Permission::PublishPackage,
            Permission::RevokePersonalToken,
        ])
        .insert(&ctx.db.conn, &principal)
        .await
        .unwrap();

    // First verify the token works - use packages/search which is public but tests auth
    let response = ctx
        .get("/packages/search")
        .query("q", "test")
        .bearer(&token.key)
        .send()
        .await;
    response.assert_ok();

    // Revoke it
    ctx.delete(&format!("/auth/tokens/{}", token.api_key.id))
        .bearer(&token.key)
        .send()
        .await
        .assert_ok();

    // Try to use revoked token on a protected route
    ctx.delete(&format!("/auth/tokens/{}", token.api_key.id))
        .bearer(&token.key)
        .send()
        .await
        .assert_invalid_token_error();
}

/// Test cross-user token access returns appropriate error
#[actix_web::test]
async fn cross_user_token_access() {
    let ctx = TestRegistryCtx::new().await;

    // User A creates a token
    let user_a = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let principal_a = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: user_a.clone(),
    };
    let token_a = fixtures::api_key()
        .user(user_a.id)
        .permissions(vec![Permission::PublishPackage])
        .insert(&ctx.db.conn, &principal_a)
        .await
        .unwrap();

    // User B tries to access token A's details
    let (_user_b, token_b) = ctx.create_user_with_token().await;

    // Should return 403 (forbidden) or 404 (not found)
    let response = ctx
        .delete(&format!("/auth/tokens/{}", token_a.api_key.id))
        .bearer(&token_b)
        .send()
        .await;

    let status = response.status();
    assert!(
        status == actix_web::http::StatusCode::NOT_FOUND
            || status == actix_web::http::StatusCode::FORBIDDEN,
        "Expected 404 or 403, got {}",
        status
    );
}

/// Test org token created by admin can be used
#[actix_web::test]
async fn org_token_works_after_creation() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, admin_token) = ctx.create_org_with_admin().await;

    // Create an org token
    let response = ctx
        .post(&format!("/org/{}/tokens", org.id))
        .bearer(&admin_token)
        .json(&json!({
            "description": "New org token",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_ok();

    // Parse the response to get the token key
    let one_time: serde_json::Value = response.json();
    let new_token = one_time["key"]
        .as_str()
        .expect("key should be string");

    // Use the new token to access a protected route
    ctx.get("/packages/search")
        .query("q", "test")
        .bearer(new_token)
        .send()
        .await
        .assert_ok();
}

// POST /auth/token - Personal Token Creation (Now accepts API keys)

/// Test creating personal token via API key with CreatePersonalToken permission
#[actix_web::test]
async fn create_personal_token_with_api_key() {
    let ctx = TestRegistryCtx::new().await;
    let user = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let principal =
        kintsu_registry_db::engine::PrincipalIdentity::UserSession { user: user.clone() };

    // Create a token with CreatePersonalToken permission
    let token = fixtures::api_key()
        .user(user.id)
        .permissions(vec![Permission::CreatePersonalToken])
        .insert(&ctx.db.conn, &principal)
        .await
        .unwrap();

    // Use the token to create a new personal token
    ctx.post("/auth/token")
        .bearer(&token.key)
        .json(&json!({
            "description": "New token via API key",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_ok();
}

/// Test creating personal token without CreatePersonalToken permission fails
#[actix_web::test]
async fn create_personal_token_without_permission_fails() {
    let ctx = TestRegistryCtx::new().await;
    let user = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let principal =
        kintsu_registry_db::engine::PrincipalIdentity::UserSession { user: user.clone() };

    // Create a token WITHOUT CreatePersonalToken permission
    let token = fixtures::api_key()
        .user(user.id)
        .permissions(vec![Permission::PublishPackage])
        .insert(&ctx.db.conn, &principal)
        .await
        .unwrap();

    // Should fail - missing CreatePersonalToken permission
    ctx.post("/auth/token")
        .bearer(&token.key)
        .json(&json!({
            "description": "New token",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_forbidden();
}

/// Test org API key cannot create personal tokens (no user principal)
#[actix_web::test]
async fn create_personal_token_with_org_key_fails() {
    let ctx = TestRegistryCtx::new().await;
    let (org, user, _admin_token) = ctx.create_org_with_admin().await;
    let principal =
        kintsu_registry_db::engine::PrincipalIdentity::UserSession { user: user.clone() };

    // Create an org API key (even with CreatePersonalToken permission)
    let org_token = fixtures::api_key()
        .org(org.id)
        .permissions(vec![
            Permission::CreatePersonalToken,
            Permission::PublishPackage,
        ])
        .insert(&ctx.db.conn, &principal)
        .await
        .unwrap();

    // Should fail - org keys cannot create personal tokens (no user in principal)
    // Returns 401 because org keys have no user identity at all
    ctx.post("/auth/token")
        .bearer(&org_token.key)
        .json(&json!({
            "description": "Token from org key",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_unauthorized();
}

/// Test creating personal token without auth fails
#[actix_web::test]
async fn create_personal_token_no_auth() {
    let ctx = TestRegistryCtx::new().await;

    ctx.post("/auth/token")
        .json(&json!({
            "description": "Token",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_unauthorized();
}

/// Test creating personal token with invalid body fails
#[actix_web::test]
async fn create_personal_token_invalid_body() {
    let ctx = TestRegistryCtx::new().await;
    let user = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let principal =
        kintsu_registry_db::engine::PrincipalIdentity::UserSession { user: user.clone() };
    let token = fixtures::api_key()
        .user(user.id)
        .permissions(vec![Permission::CreatePersonalToken])
        .insert(&ctx.db.conn, &principal)
        .await
        .unwrap();

    // Empty body should fail validation but pass auth
    ctx.post("/auth/token")
        .bearer(&token.key)
        .json(&json!({}))
        .send()
        .await
        .assert_bad_request();
}
