//! Schema Role Engine Tests
//!
//! Tests for registry-db/src/engine/schema_role.rs
//! Covers granting and revoking schema roles for packages.

mod common;

use common::fixtures;
use kintsu_registry_db::{
    Error,
    engine::{
        PrincipalIdentity,
        schema_role::{grant_role, revoke_role},
    },
    entities::*,
    tst::TestDbCtx,
};
use sea_orm::EntityTrait;

async fn create_api_key_principal(
    ctx: &TestDbCtx,
    user: &User,
    scopes: Vec<&str>,
    perms: Vec<Permission>,
) -> PrincipalIdentity {
    let session = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(scopes)
        .permissions(perms)
        .insert(&ctx.conn, &session)
        .await
        .expect("Failed to create API key");

    let api_key = ApiKey {
        id: one_time.api_key.id,
        description: one_time.api_key.description,
        expires: one_time.api_key.expires,
        scopes: one_time.api_key.scopes,
        permissions: one_time.api_key.permissions,
        user_id: one_time.api_key.user_id,
        org_id: one_time.api_key.org_id,
        last_used_at: None,
        revoked_at: None,
    };

    PrincipalIdentity::UserApiKey {
        user: user.clone(),
        key: api_key,
    }
}

#[tokio::test]
async fn grant_role_user_success() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin user");

    let grantee = fixtures::user()
        .gh_login("grantee")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create grantee");

    let pkg = fixtures::package()
        .name("grant-test-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // Make admin_user an admin of the package
    fixtures::schema_role(pkg.id)
        .user(admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal = create_api_key_principal(
        &ctx,
        &admin_user,
        vec!["*"],
        vec![Permission::GrantSchemaRole],
    )
    .await;

    let role = grant_role(
        &ctx.conn,
        &principal,
        "grant-test-pkg",
        Some(grantee.id),
        None,
        SchemaRoleType::Admin,
    )
    .await
    .expect("Failed to grant role");

    assert_eq!(role.package, pkg.id);
    assert_eq!(role.user_id, Some(grantee.id));
    assert_eq!(role.org_id, None);
    assert_eq!(role.role, SchemaRoleType::Admin);
    assert!(role.revoked_at.is_none());
}

#[tokio::test]
async fn grant_role_org_success() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin user");

    let org = fixtures::org()
        .name("grantee-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    let pkg = fixtures::package()
        .name("org-grant-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::schema_role(pkg.id)
        .user(admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal = create_api_key_principal(
        &ctx,
        &admin_user,
        vec!["*"],
        vec![Permission::GrantSchemaRole],
    )
    .await;

    let role = grant_role(
        &ctx.conn,
        &principal,
        "org-grant-pkg",
        None,
        Some(org.id),
        SchemaRoleType::Admin,
    )
    .await
    .expect("Failed to grant role");

    assert_eq!(role.org_id, Some(org.id));
    assert_eq!(role.user_id, None);
}

#[tokio::test]
async fn grant_role_requires_one_owner_both() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    let pkg = fixtures::package()
        .name("both-owner-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::schema_role(pkg.id)
        .user(user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::GrantSchemaRole]).await;

    // Both user_id and org_id provided - should fail
    let result = grant_role(
        &ctx.conn,
        &principal,
        "both-owner-pkg",
        Some(user.id),
        Some(org.id),
        SchemaRoleType::Admin,
    )
    .await;

    assert!(matches!(result, Err(Error::Validation(_))));
}

#[tokio::test]
async fn grant_role_requires_one_owner_neither() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("neither-owner-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::schema_role(pkg.id)
        .user(user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::GrantSchemaRole]).await;

    // Neither user_id nor org_id provided - should fail
    let result = grant_role(
        &ctx.conn,
        &principal,
        "neither-owner-pkg",
        None,
        None,
        SchemaRoleType::Admin,
    )
    .await;

    assert!(matches!(result, Err(Error::Validation(_))));
}

#[tokio::test]
async fn grant_role_package_not_found() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::GrantSchemaRole]).await;

    let result = grant_role(
        &ctx.conn,
        &principal,
        "nonexistent-package",
        Some(user.id),
        None,
        SchemaRoleType::Admin,
    )
    .await;

    assert!(matches!(result, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn grant_role_unauthorized() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin user");

    let attacker = fixtures::user()
        .gh_login("attacker")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create attacker");

    let pkg = fixtures::package()
        .name("unauth-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // admin_user owns the package
    fixtures::schema_role(pkg.id)
        .user(admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    // attacker tries to grant
    let principal = create_api_key_principal(
        &ctx,
        &attacker,
        vec!["*"],
        vec![Permission::GrantSchemaRole],
    )
    .await;

    let result = grant_role(
        &ctx.conn,
        &principal,
        "unauth-pkg",
        Some(attacker.id),
        None,
        SchemaRoleType::Admin,
    )
    .await;

    assert!(matches!(result, Err(Error::AuthorizationDenied(_))));
}

#[tokio::test]
async fn grant_role_duplicate() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin user");

    let grantee = fixtures::user()
        .gh_login("grantee")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create grantee");

    let pkg = fixtures::package()
        .name("dup-grant-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::schema_role(pkg.id)
        .user(admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal = create_api_key_principal(
        &ctx,
        &admin_user,
        vec!["*"],
        vec![Permission::GrantSchemaRole],
    )
    .await;

    // First grant - should succeed
    grant_role(
        &ctx.conn,
        &principal,
        "dup-grant-pkg",
        Some(grantee.id),
        None,
        SchemaRoleType::Admin,
    )
    .await
    .expect("First grant failed");

    // Second grant - same role - should fail
    let result = grant_role(
        &ctx.conn,
        &principal,
        "dup-grant-pkg",
        Some(grantee.id),
        None,
        SchemaRoleType::Admin,
    )
    .await;

    assert!(matches!(result, Err(Error::Validation(_))));
}

#[tokio::test]
async fn revoke_role_success() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin user");

    let grantee = fixtures::user()
        .gh_login("grantee")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create grantee");

    let pkg = fixtures::package()
        .name("revoke-test-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::schema_role(pkg.id)
        .user(admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    // Grant role to grantee
    let role = fixtures::schema_role(pkg.id)
        .user(grantee.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant role");

    let principal = create_api_key_principal(
        &ctx,
        &admin_user,
        vec!["*"],
        vec![Permission::RevokeSchemaRole],
    )
    .await;

    // Revoke the role
    revoke_role(&ctx.conn, &principal, role.id)
        .await
        .expect("Failed to revoke role");

    // Verify revoked
    let revoked = SchemaRoleEntity::find_by_id(role.id)
        .one(&ctx.conn)
        .await
        .expect("Failed to lookup role")
        .expect("Role not found");

    assert!(revoked.revoked_at.is_some());
}

#[tokio::test]
async fn revoke_role_not_found() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::RevokeSchemaRole]).await;

    let result = revoke_role(&ctx.conn, &principal, 99999).await;

    assert!(matches!(result, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn revoke_role_unauthorized() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin user");

    let attacker = fixtures::user()
        .gh_login("attacker")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create attacker");

    let grantee = fixtures::user()
        .gh_login("grantee")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create grantee");

    let pkg = fixtures::package()
        .name("revoke-unauth-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::schema_role(pkg.id)
        .user(admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let role = fixtures::schema_role(pkg.id)
        .user(grantee.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant role");

    // Attacker tries to revoke
    let principal = create_api_key_principal(
        &ctx,
        &attacker,
        vec!["*"],
        vec![Permission::RevokeSchemaRole],
    )
    .await;

    let result = revoke_role(&ctx.conn, &principal, role.id).await;

    assert!(matches!(result, Err(Error::AuthorizationDenied(_))));
}
