//! Permission enforcement tests
//!
//! Tests for role-based and scope-based permission enforcement
//! across packages, organizations, and schema management

mod common;

use common::TestRegistryCtx;
use kintsu_registry_db::{entities::Permission, fixtures};
use serde_json::json;

// Schema Role Management - POST /roles/package, DELETE /roles/package

/// Test schema admin can grant schema role to another user
#[actix_web::test]
async fn grant_schema_role_as_admin() {
    let ctx = TestRegistryCtx::new().await;

    // Create package owner (becomes admin on first publish)
    let owner = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let owner_principal = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: owner.clone(),
    };
    let owner_token = fixtures::api_key()
        .user(owner.id)
        .permissions(vec![
            Permission::GrantSchemaRole,
            Permission::RevokeSchemaRole,
        ])
        .insert(&ctx.db.conn, &owner_principal)
        .await
        .unwrap();

    // Create package and make owner an admin
    let pkg = fixtures::package()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    fixtures::schema_role(pkg.id)
        .user(owner.id)
        .admin()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Create target user to grant role to
    let target_user = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Grant schema role
    ctx.post("/roles/package")
        .bearer(&owner_token.key)
        .json(&json!({
            "package_name": pkg.name,
            "user_id": target_user.id,
            "role": "Author"
        }))
        .send()
        .await
        .assert_ok();
}

/// Test non-admin cannot grant schema role
#[actix_web::test]
async fn grant_schema_role_as_non_admin_fails() {
    let ctx = TestRegistryCtx::new().await;

    // Create package with owner
    let pkg = fixtures::package()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let owner = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    fixtures::schema_role(pkg.id)
        .user(owner.id)
        .admin()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Create non-admin user
    let non_admin = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let non_admin_principal = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: non_admin.clone(),
    };
    let non_admin_token = fixtures::api_key()
        .user(non_admin.id)
        .permissions(vec![Permission::GrantSchemaRole])
        .insert(&ctx.db.conn, &non_admin_principal)
        .await
        .unwrap();

    // Target user
    let target = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Should fail - not a schema admin
    ctx.post("/roles/package")
        .bearer(&non_admin_token.key)
        .json(&json!({
            "package_name": pkg.name,
            "user_id": target.id,
            "role": "Author"
        }))
        .send()
        .await
        .assert_forbidden();
}

/// Test schema author cannot grant roles
#[actix_web::test]
async fn grant_schema_role_as_author_fails() {
    let ctx = TestRegistryCtx::new().await;

    // Create package with owner
    let pkg = fixtures::package()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let owner = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    fixtures::schema_role(pkg.id)
        .user(owner.id)
        .admin()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Create author (not admin)
    let author = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    fixtures::schema_role(pkg.id)
        .user(author.id)
        .author()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    let author_principal = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: author.clone(),
    };
    let author_token = fixtures::api_key()
        .user(author.id)
        .permissions(vec![Permission::GrantSchemaRole])
        .insert(&ctx.db.conn, &author_principal)
        .await
        .unwrap();

    // Target user
    let target = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Should fail - authors cannot grant roles
    ctx.post("/roles/package")
        .bearer(&author_token.key)
        .json(&json!({
            "package_name": pkg.name,
            "user_id": target.id,
            "role": "Author"
        }))
        .send()
        .await
        .assert_forbidden();
}

/// Test schema admin can revoke schema role
#[actix_web::test]
async fn revoke_schema_role_as_admin() {
    let ctx = TestRegistryCtx::new().await;

    // Create package with owner
    let pkg = fixtures::package()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let owner = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    fixtures::schema_role(pkg.id)
        .user(owner.id)
        .admin()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    let owner_principal = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: owner.clone(),
    };
    let owner_token = fixtures::api_key()
        .user(owner.id)
        .permissions(vec![Permission::RevokeSchemaRole])
        .insert(&ctx.db.conn, &owner_principal)
        .await
        .unwrap();

    // Create author to revoke
    let author = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let author_role = fixtures::schema_role(pkg.id)
        .user(author.id)
        .author()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Revoke author role
    ctx.delete("/roles/package")
        .bearer(&owner_token.key)
        .json(&json!({
            "role_id": author_role.id
        }))
        .send()
        .await
        .assert_no_content();
}

/// Test non-admin cannot revoke schema role
#[actix_web::test]
async fn revoke_schema_role_as_non_admin_fails() {
    let ctx = TestRegistryCtx::new().await;

    // Create package with owner
    let pkg = fixtures::package()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let owner = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let owner_role = fixtures::schema_role(pkg.id)
        .user(owner.id)
        .admin()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Create non-admin
    let non_admin = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let non_admin_principal = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: non_admin.clone(),
    };
    let non_admin_token = fixtures::api_key()
        .user(non_admin.id)
        .permissions(vec![Permission::RevokeSchemaRole])
        .insert(&ctx.db.conn, &non_admin_principal)
        .await
        .unwrap();

    // Should fail
    ctx.delete("/roles/package")
        .bearer(&non_admin_token.key)
        .json(&json!({
            "role_id": owner_role.id
        }))
        .send()
        .await
        .assert_forbidden();
}

// Org Role Management - GrantOrgRole / RevokeOrgRole

/// Test org admin can grant org role to another user
#[actix_web::test]
async fn grant_org_role_as_admin() {
    let ctx = TestRegistryCtx::new().await;
    let (org, admin, admin_token) = ctx.create_org_with_admin().await;

    // Create target user
    let target_user = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Grant org role
    ctx.post("/roles/org")
        .bearer(&admin_token)
        .json(&json!({
            "org_id": org.id,
            "user_id": target_user.id,
            "role": "Member"
        }))
        .send()
        .await
        .assert_ok();
}

/// Test org member cannot grant org role
#[actix_web::test]
async fn grant_org_role_as_member_fails() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _admin, _admin_token) = ctx.create_org_with_admin().await;
    let (member, member_token) = ctx.create_org_member(org.id).await;

    // Create target user
    let target_user = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Should fail - members cannot grant roles
    ctx.post("/roles/org")
        .bearer(&member_token)
        .json(&json!({
            "org_id": org.id,
            "user_id": target_user.id,
            "role": "Member"
        }))
        .send()
        .await
        .assert_forbidden();
}

/// Test non-member cannot grant org role
#[actix_web::test]
async fn grant_org_role_as_non_member_fails() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _admin, _admin_token) = ctx.create_org_with_admin().await;
    let (_non_member, non_member_token) = ctx.create_user_with_token().await;

    // Create target user
    let target_user = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Should fail - not an org member
    ctx.post("/roles/org")
        .bearer(&non_member_token)
        .json(&json!({
            "org_id": org.id,
            "user_id": target_user.id,
            "role": "Member"
        }))
        .send()
        .await
        .assert_forbidden();
}

/// Test org admin can revoke org role
#[actix_web::test]
async fn revoke_org_role_as_admin() {
    let ctx = TestRegistryCtx::new().await;
    let (org, _admin, admin_token) = ctx.create_org_with_admin().await;
    let (member, _member_token) = ctx.create_org_member(org.id).await;

    // Revoke member role
    ctx.delete("/roles/org")
        .bearer(&admin_token)
        .json(&json!({
            "org_id": org.id,
            "user_id": member.id
        }))
        .send()
        .await
        .assert_no_content();
}

/// Test org member cannot revoke org role
#[actix_web::test]
async fn revoke_org_role_as_member_fails() {
    let ctx = TestRegistryCtx::new().await;
    let (org, admin, _admin_token) = ctx.create_org_with_admin().await;
    let (_member, member_token) = ctx.create_org_member(org.id).await;

    // Should fail - members cannot revoke roles
    ctx.delete("/roles/org")
        .bearer(&member_token)
        .json(&json!({
            "org_id": org.id,
            "user_id": admin.id
        }))
        .send()
        .await
        .assert_forbidden();
}

// Note: API key scope enforcement (restricting publish/yank to specific packages)
// is tested via unit tests in registry-db. Integration tests for publish with
// scope restrictions require a full publish flow which is covered separately.

// Permission Enforcement - Missing Permissions

/// Test token without GrantSchemaRole permission cannot grant schema role
#[actix_web::test]
async fn api_key_missing_grant_schema_role_permission() {
    let ctx = TestRegistryCtx::new().await;

    let owner = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let owner_principal = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: owner.clone(),
    };

    // Token without GrantSchemaRole permission
    let token = fixtures::api_key()
        .user(owner.id)
        .permissions(vec![Permission::PublishPackage]) // Missing GrantSchemaRole
        .insert(&ctx.db.conn, &owner_principal)
        .await
        .unwrap();

    let pkg = fixtures::package()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    fixtures::schema_role(pkg.id)
        .user(owner.id)
        .admin()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    let target = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Should fail - missing permission
    ctx.post("/roles/package")
        .bearer(&token.key)
        .json(&json!({
            "package_name": pkg.name,
            "user_id": target.id,
            "role": "Author"
        }))
        .send()
        .await
        .assert_forbidden();
}

/// Test token without GrantOrgRole permission cannot grant org role
#[actix_web::test]
async fn api_key_missing_grant_org_role_permission() {
    let ctx = TestRegistryCtx::new().await;

    let org = fixtures::org()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let admin = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    fixtures::org_role(org.id, admin.id)
        .admin()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    let admin_principal = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: admin.clone(),
    };

    // Token without GrantOrgRole permission
    let token = fixtures::api_key()
        .user(admin.id)
        .permissions(vec![Permission::PublishPackage]) // Missing GrantOrgRole
        .insert(&ctx.db.conn, &admin_principal)
        .await
        .unwrap();

    let target = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Should fail - missing permission
    ctx.post("/roles/org")
        .bearer(&token.key)
        .json(&json!({
            "org_id": org.id,
            "user_id": target.id,
            "role": "Member"
        }))
        .send()
        .await
        .assert_forbidden();
}

/// Test token without CreateOrgToken permission cannot create org token
#[actix_web::test]
async fn api_key_missing_create_org_token_permission() {
    let ctx = TestRegistryCtx::new().await;

    let org = fixtures::org()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let admin = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    fixtures::org_role(org.id, admin.id)
        .admin()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    let admin_principal = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: admin.clone(),
    };

    // Token without CreateOrgToken permission
    let token = fixtures::api_key()
        .user(admin.id)
        .permissions(vec![Permission::PublishPackage]) // Missing CreateOrgToken
        .insert(&ctx.db.conn, &admin_principal)
        .await
        .unwrap();

    // Should fail - missing permission
    ctx.post(&format!("/org/{}/tokens", org.id))
        .bearer(&token.key)
        .json(&json!({
            "description": "Token",
            "scopes": ["*"],
            "permissions": ["publish-package"],
            "expires_in_days": 30
        }))
        .send()
        .await
        .assert_forbidden();
}

/// Test token without ListOrgToken permission cannot list org tokens
#[actix_web::test]
async fn api_key_missing_list_org_token_permission() {
    let ctx = TestRegistryCtx::new().await;

    let org = fixtures::org()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let admin = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    fixtures::org_role(org.id, admin.id)
        .admin()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    let admin_principal = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: admin.clone(),
    };

    // Token without ListOrgToken permission
    let token = fixtures::api_key()
        .user(admin.id)
        .permissions(vec![Permission::PublishPackage]) // Missing ListOrgToken
        .insert(&ctx.db.conn, &admin_principal)
        .await
        .unwrap();

    // Should fail - missing permission
    ctx.get(&format!("/org/{}/tokens", org.id))
        .bearer(&token.key)
        .send()
        .await
        .assert_forbidden();
}

// Org Admin via Org Membership (Schema Admin Inheritance)

/// Test org admin inherits schema admin for packages published by org
#[actix_web::test]
async fn org_admin_inherits_package_admin() {
    let ctx = TestRegistryCtx::new().await;

    // Create org and admin
    let org = fixtures::org()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    let admin = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    fixtures::org_role(org.id, admin.id)
        .admin()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    let admin_principal = kintsu_registry_db::engine::PrincipalIdentity::UserSession {
        user: admin.clone(),
    };
    let admin_token = fixtures::api_key()
        .user(admin.id)
        .permissions(vec![
            Permission::GrantSchemaRole,
            Permission::RevokeSchemaRole,
        ])
        .insert(&ctx.db.conn, &admin_principal)
        .await
        .unwrap();

    // Create package owned by org (org has schema admin role)
    let pkg = fixtures::package()
        .insert(&ctx.db.conn)
        .await
        .unwrap();
    fixtures::schema_role(pkg.id)
        .org(org.id)
        .admin()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Target user
    let target = fixtures::user()
        .insert(&ctx.db.conn)
        .await
        .unwrap();

    // Admin should be able to grant schema role via org membership
    ctx.post("/roles/package")
        .bearer(&admin_token.key)
        .json(&json!({
            "package_name": pkg.name,
            "user_id": target.id,
            "role": "Author"
        }))
        .send()
        .await
        .assert_ok();
}
