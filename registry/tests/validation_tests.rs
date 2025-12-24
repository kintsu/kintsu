//! Input Validation Tests
//!
//! Tests for request body validation, missing fields, malformed requests,
//! and boundary conditions across registry routes.

mod common;

use common::TestRegistryCtx;
use kintsu_registry_db::{entities::Permission, fixtures};
use serde_json::json;

// Token Creation - CreateTokenRequest Validation

/// Test creating token with empty description (should fail - min 1 char)
#[actix_web::test]
async fn create_token_empty_description() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .json(&json!({
            "description": "",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test creating token with description exceeding max length (32 chars)
#[actix_web::test]
async fn create_token_description_too_long() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .json(&json!({
            "description": "a".repeat(33),
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test creating token with too many scopes (max 10)
#[actix_web::test]
async fn create_token_too_many_scopes() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    let scopes: Vec<String> = (0..11)
        .map(|i| format!("scope-{}", i))
        .collect();

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .json(&json!({
            "description": "Test",
            "scopes": scopes,
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test creating token with too many permissions (max 4)
#[actix_web::test]
async fn create_token_too_many_permissions() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    // All possible permissions exceeding the limit
    let permissions = vec![
        "publish-package",
        "yank-package",
        "grant-schema-role",
        "revoke-schema-role",
        "create-personal-token",
    ];

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .json(&json!({
            "description": "Test",
            "scopes": ["*"],
            "permissions": permissions,
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test creating token with negative expiration days
#[actix_web::test]
async fn create_token_negative_expiration() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .json(&json!({
            "description": "Test",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": -1
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test creating token with invalid permission name
#[actix_web::test]
async fn create_token_invalid_permission() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .json(&json!({
            "description": "Test",
            "scopes": ["*"],
            "permissions": ["invalid-permission"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test creating token with malformed JSON
#[actix_web::test]
async fn create_token_malformed_json() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .content_type("application/json")
        .body("{invalid json}")
        .send()
        .await
        .assert_bad_request();
}

/// Test creating token with wrong content type
#[actix_web::test]
async fn create_token_wrong_content_type() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .content_type("text/plain")
        .body("not json")
        .send()
        .await
        .assert_bad_request();
}

// Schema Role - GrantSchemaRoleRequest Validation

/// Test granting role with empty package name
#[actix_web::test]
async fn grant_schema_role_empty_package_name() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.post("/roles/package")
        .bearer(&token)
        .json(&json!({
            "package_name": "",
            "role": "Author",
            "user_id": 1
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test granting role without user_id or org_id
#[actix_web::test]
async fn grant_schema_role_no_target() {
    let ctx = TestRegistryCtx::new().await;
    let user = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let principal =
        kintsu_registry_db::engine::PrincipalIdentity::UserSession { user: user.clone() };
    let token = fixtures::api_key()
        .user(user.id)
        .permissions(vec![Permission::GrantSchemaRole])
        .insert(&ctx.db.conn, &principal)
        .await
        .unwrap();

    ctx.post("/roles/package")
        .bearer(&token.key)
        .json(&json!({
            "package_name": "test-pkg",
            "role": "Author"
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test granting role with invalid role type
#[actix_web::test]
async fn grant_schema_role_invalid_role() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.post("/roles/package")
        .bearer(&token)
        .json(&json!({
            "package_name": "test-pkg",
            "role": "InvalidRole",
            "user_id": 1
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test granting role with both user_id and org_id fails validation
#[actix_web::test]
async fn grant_schema_role_both_targets() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx
        .create_user_with_token_and_permission(Permission::GrantSchemaRole)
        .await;

    // Validation now rejects having both user_id and org_id
    ctx.post("/roles/package")
        .bearer(&token)
        .json(&json!({
            "package_name": "test-pkg",
            "role": "Author",
            "user_id": 1,
            "org_id": 1
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test revoking role with invalid role_id (0)
#[actix_web::test]
async fn revoke_schema_role_invalid_id() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx
        .create_user_with_token_and_permission(Permission::RevokeSchemaRole)
        .await;

    ctx.delete("/roles/package")
        .bearer(&token)
        .json(&json!({
            "role_id": 0
        }))
        .send()
        .await
        .assert_not_found();
}

/// Test revoking role with negative role_id
#[actix_web::test]
async fn revoke_schema_role_negative_id() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx
        .create_user_with_token_and_permission(Permission::RevokeSchemaRole)
        .await;

    ctx.delete("/roles/package")
        .bearer(&token)
        .json(&json!({
            "role_id": -1
        }))
        .send()
        .await
        .assert_not_found();
}

// Org Role - GrantOrgRoleRequest Validation

/// Test granting org role with invalid role type
#[actix_web::test]
async fn grant_org_role_invalid_role() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.post("/roles/org")
        .bearer(&token)
        .json(&json!({
            "org_id": 1,
            "user_id": 1,
            "role": "SuperAdmin"
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test granting org role with missing user_id
#[actix_web::test]
async fn grant_org_role_missing_user_id() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.post("/roles/org")
        .bearer(&token)
        .json(&json!({
            "org_id": 1,
            "role": "Member"
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test granting org role with missing org_id
#[actix_web::test]
async fn grant_org_role_missing_org_id() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.post("/roles/org")
        .bearer(&token)
        .json(&json!({
            "user_id": 1,
            "role": "Member"
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test revoking org role with missing user_id
#[actix_web::test]
async fn revoke_org_role_missing_user_id() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.delete("/roles/org")
        .bearer(&token)
        .json(&json!({
            "org_id": 1
        }))
        .send()
        .await
        .assert_bad_request();
}

/// Test revoking org role with missing org_id
#[actix_web::test]
async fn revoke_org_role_missing_org_id() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.delete("/roles/org")
        .bearer(&token)
        .json(&json!({
            "user_id": 1
        }))
        .send()
        .await
        .assert_bad_request();
}

// Import Org - ImportOrgRequest Validation

// Note: Import org route requires SessionData (browser session), not API key.
// We cannot fully test validation without a session, so we verify auth is enforced.

/// Test importing org requires session authentication
#[actix_web::test]
async fn import_org_requires_session() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    // Import org requires session auth, API key is rejected
    ctx.post("/orgs/import")
        .bearer(&token)
        .json(&json!({
            "org_name": "test-org"
        }))
        .send()
        .await
        .assert_unauthorized();
}

// Favourites - CreateFavouriteRequest Validation

// Note: Favourites routes require SessionData (browser session), not API key auth.
// These tests verify that API key auth is correctly rejected.

/// Test creating favourite with API key auth is rejected (session required)
#[actix_web::test]
async fn create_favourite_requires_session() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    // Favourites route requires session auth, not API key
    ctx.post("/favourites")
        .bearer(&token)
        .json(&json!({
            "type": "package",
            "id": 1
        }))
        .send()
        .await
        .assert_unauthorized();
}

/// Test deleting favourite with API key auth is rejected (session required)
#[actix_web::test]
async fn delete_favourite_requires_session() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.delete("/favourites")
        .bearer(&token)
        .json(&json!({
            "type": "package",
            "id": 1
        }))
        .send()
        .await
        .assert_unauthorized();
}

// Path Parameter Validation

/// Test package download with invalid version format
#[actix_web::test]
async fn package_download_invalid_version() {
    let ctx = TestRegistryCtx::new().await;

    // Package lookup happens first - returns 404 if package doesn't exist
    // Version validation only happens if the package is found
    ctx.get("/package/test-pkg/not-a-version")
        .send()
        .await
        .assert_not_found();
}

/// Test org token creation with non-numeric org ID
#[actix_web::test]
async fn org_token_invalid_org_id() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    // Non-numeric org ID should fail
    ctx.post("/org/abc/tokens")
        .bearer(&token)
        .json(&json!({
            "description": "Test",
            "scopes": ["*"],
            "permissions": ["publish-package"]
        }))
        .send()
        .await
        .assert_not_found();
}

/// Test org token creation with negative org ID
#[actix_web::test]
async fn org_token_negative_org_id() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    // Negative org ID triggers "Organization not found" error (404)
    ctx.post("/org/-1/tokens")
        .bearer(&token)
        .json(&json!({
            "description": "Test",
            "scopes": ["*"],
            "permissions": ["publish-package"]
        }))
        .send()
        .await
        .assert_not_found();
}

/// Test token revocation with non-numeric ID
#[actix_web::test]
async fn revoke_token_invalid_id() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.delete("/auth/tokens/abc")
        .bearer(&token)
        .send()
        .await
        .assert_not_found();
}

/// Test token revocation with nonexistent ID
#[actix_web::test]
async fn revoke_token_nonexistent() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx
        .create_user_with_token_and_permission(Permission::RevokePersonalToken)
        .await;

    ctx.delete("/auth/tokens/999999999")
        .bearer(&token)
        .send()
        .await
        .assert_not_found();
}

// Query Parameter Validation

/// Test favourites list with negative page number
#[actix_web::test]
async fn favourites_negative_page() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.get("/favourites")
        .query("page", "-1")
        .bearer(&token)
        .send()
        .await
        .assert_bad_request();
}

/// Test favourites list with zero page size
#[actix_web::test]
async fn favourites_zero_page_size() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.get("/favourites")
        .query("size", "0")
        .bearer(&token)
        .send()
        .await
        .assert_bad_request();
}

/// Test favourites list with excessive page size
#[actix_web::test]
async fn favourites_excessive_page_size() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.get("/favourites")
        .query("size", "10001")
        .bearer(&token)
        .send()
        .await
        .assert_bad_request();
}

// Org Name Validation

/// Test org exists check with empty name
#[actix_web::test]
async fn org_exists_empty_name() {
    let ctx = TestRegistryCtx::new().await;

    // Empty path segment should 404
    ctx.get("/orgs/exists/")
        .send()
        .await
        .assert_not_found();
}

/// Test org exists check with name exceeding limit
#[actix_web::test]
async fn org_exists_name_too_long() {
    let ctx = TestRegistryCtx::new().await;
    let long_name = "a".repeat(40);

    ctx.get(&format!("/orgs/exists/{}", long_name))
        .send()
        .await
        .assert_bad_request();
}

// Edge Cases and Boundary Conditions

/// Test creating token at maximum description length boundary (32 chars)
#[actix_web::test]
async fn create_token_description_at_limit() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .json(&json!({
            "description": "a".repeat(32),
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_ok();
}

/// Test creating token with exactly 10 scopes (at limit)
#[actix_web::test]
async fn create_token_scopes_at_limit() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    let scopes: Vec<String> = (0..10)
        .map(|i| format!("scope-{}", i))
        .collect();

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .json(&json!({
            "description": "Test",
            "scopes": scopes,
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_ok();
}

/// Test creating token with exactly 4 permissions (at limit)
#[actix_web::test]
async fn create_token_permissions_at_limit() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _user, token) = ctx.create_org_with_admin().await;

    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token)
        .json(&json!({
            "description": "Test",
            "scopes": ["*"],
            "permissions": [
                "publish-package",
                "yank-package",
                "grant-schema-role",
                "revoke-schema-role"
            ],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_ok();
}

/// Test org name at exactly 39 characters (GitHub limit)
#[actix_web::test]
async fn org_exists_name_at_limit() {
    let ctx = TestRegistryCtx::new().await;
    let max_name = "a".repeat(39);

    // Should pass validation (will return exists: false)
    ctx.get(&format!("/orgs/exists/{}", max_name))
        .send()
        .await
        .assert_ok();
}

/// Test empty request bodies
#[actix_web::test]
async fn grant_schema_role_empty_body() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.post("/roles/package")
        .bearer(&token)
        .json(&json!({}))
        .send()
        .await
        .assert_bad_request();
}

/// Test grant org role empty body
#[actix_web::test]
async fn grant_org_role_empty_body() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.post("/roles/org")
        .bearer(&token)
        .json(&json!({}))
        .send()
        .await
        .assert_bad_request();
}

/// Test revoke schema role empty body
#[actix_web::test]
async fn revoke_schema_role_empty_body() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.delete("/roles/package")
        .bearer(&token)
        .json(&json!({}))
        .send()
        .await
        .assert_bad_request();
}

/// Test revoke org role empty body
#[actix_web::test]
async fn revoke_org_role_empty_body() {
    let ctx = TestRegistryCtx::new().await;
    let (_user, token) = ctx.create_user_with_token().await;

    ctx.delete("/roles/org")
        .bearer(&token)
        .json(&json!({}))
        .send()
        .await
        .assert_bad_request();
}
