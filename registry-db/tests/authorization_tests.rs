//! Authorization Engine Tests
//!
//! Tests for registry-db/src/engine/authorization.rs
//! Covers package, org, and token authorization checks.

mod common;

use common::fixtures;
use kintsu_registry_auth::Policy;
use kintsu_registry_db::{
    engine::{
        OwnerId, PrincipalIdentity,
        authorization::{Authorize, PackageResource, TokenResource},
        fluent::AuthCheck,
    },
    entities::*,
    tst::TestDbCtx,
};

async fn create_api_key_principal(
    ctx: &TestDbCtx,
    user: &User,
    scopes: Vec<&str>,
    perms: Vec<Permission>,
) -> PrincipalIdentity {
    // Create session principal for API key creation
    let session = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(scopes)
        .permissions(perms)
        .insert(&ctx.conn, &session)
        .await
        .expect("Failed to create API key");

    // Build the ApiKey from the OneTimeApiKey
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
async fn package_publish_first_time() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::PublishPackage]).await;

    // Package doesn't exist - first publish
    let resource = PackageResource {
        name: "new-package".to_string(),
        id: None,
    };

    let result = resource
        .authorize(&ctx.conn, &principal, Permission::PublishPackage)
        .await
        .expect("Authorization failed");

    assert!(result.allowed);
    assert!(
        result
            .checks
            .iter()
            .any(|c| c.policy == Policy::FirstPublish && c.passed)
    );
}

#[tokio::test]
async fn package_publish_as_admin() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("admin-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // User is admin of package
    fixtures::schema_role(pkg.id)
        .user(user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::PublishPackage]).await;

    let resource = PackageResource {
        name: "admin-pkg".to_string(),
        id: Some(pkg.id),
    };

    let result = resource
        .authorize(&ctx.conn, &principal, Permission::PublishPackage)
        .await
        .expect("Authorization failed");

    assert!(result.allowed);
    assert!(
        result
            .checks
            .iter()
            .any(|c| c.policy == Policy::SchemaAdmin && c.passed)
    );
}

#[tokio::test]
async fn package_publish_not_admin() {
    let ctx = TestDbCtx::new().await;

    let user1 = fixtures::user()
        .gh_login("owner")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user1");

    let user2 = fixtures::user()
        .gh_login("attacker")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user2");

    let pkg = fixtures::package()
        .name("owned-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // user1 is admin
    fixtures::schema_role(pkg.id)
        .user(user1.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    // user2 tries to publish
    let principal =
        create_api_key_principal(&ctx, &user2, vec!["*"], vec![Permission::PublishPackage]).await;

    let resource = PackageResource {
        name: "owned-pkg".to_string(),
        id: Some(pkg.id),
    };

    let result = resource
        .authorize(&ctx.conn, &principal, Permission::PublishPackage)
        .await
        .expect("Authorization failed");

    assert!(!result.allowed);
    assert!(
        result
            .checks
            .iter()
            .any(|c| c.policy == Policy::SchemaAdmin && !c.passed)
    );
}

#[tokio::test]
async fn package_publish_no_permission() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    // API key without PublishPackage permission
    let principal = create_api_key_principal(
        &ctx,
        &user,
        vec!["*"],
        vec![Permission::ListOrgToken], // Wrong permission
    )
    .await;

    let resource = PackageResource {
        name: "some-pkg".to_string(),
        id: None,
    };

    let result = resource
        .authorize(&ctx.conn, &principal, Permission::PublishPackage)
        .await
        .expect("Authorization failed");

    assert!(!result.allowed);
    assert!(
        result
            .checks
            .iter()
            .any(|c| c.policy == Policy::ExplicitPermission && !c.passed)
    );
}

#[tokio::test]
async fn package_publish_scope_mismatch() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    // API key with limited scope
    let principal = create_api_key_principal(
        &ctx,
        &user,
        vec!["allowed-*"],
        vec![Permission::PublishPackage],
    )
    .await;

    let resource = PackageResource {
        name: "denied-pkg".to_string(),
        id: None,
    };

    let result = resource
        .authorize(&ctx.conn, &principal, Permission::PublishPackage)
        .await
        .expect("Authorization failed");

    assert!(!result.allowed);
    assert!(
        result
            .checks
            .iter()
            .any(|c| c.policy == Policy::ScopeMatch && !c.passed)
    );
}

#[tokio::test]
async fn package_publish_session_not_api_key() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    // Session principal (no API key)
    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let resource = PackageResource {
        name: "some-pkg".to_string(),
        id: None,
    };

    let result = resource
        .authorize(&ctx.conn, &principal, Permission::PublishPackage)
        .await
        .expect("Authorization failed");

    assert!(!result.allowed);
    assert!(
        result
            .checks
            .iter()
            .any(|c| c.policy == Policy::ApiKeyRequired && !c.passed)
    );
}

#[tokio::test]
async fn package_yank_as_admin() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("yank-pkg")
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
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::YankPackage]).await;

    let resource = PackageResource {
        name: "yank-pkg".to_string(),
        id: Some(pkg.id),
    };

    let result = resource
        .authorize(&ctx.conn, &principal, Permission::YankPackage)
        .await
        .expect("Authorization failed");

    assert!(result.allowed);
}

#[tokio::test]
async fn package_yank_not_admin() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("not-admin-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // No admin role granted
    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::YankPackage]).await;

    let resource = PackageResource {
        name: "not-admin-pkg".to_string(),
        id: Some(pkg.id),
    };

    let result = resource
        .authorize(&ctx.conn, &principal, Permission::YankPackage)
        .await
        .expect("Authorization failed");

    assert!(!result.allowed);
}

#[tokio::test]
async fn package_grant_role_as_admin() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("grant-role-pkg")
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

    let result = AuthCheck::new(&ctx.conn, &principal)
        .package("grant-role-pkg", Some(pkg.id))
        .can_grant_role()
        .await
        .expect("Authorization failed");

    assert!(result.allowed);
}

#[tokio::test]
async fn package_grant_role_not_admin() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("no-grant-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::GrantSchemaRole]).await;

    let result = AuthCheck::new(&ctx.conn, &principal)
        .package("no-grant-pkg", Some(pkg.id))
        .can_grant_role()
        .await
        .expect("Authorization failed");

    assert!(!result.allowed);
}

#[tokio::test]
async fn org_grant_role_as_admin() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::GrantOrgRole]).await;

    let result = AuthCheck::new(&ctx.conn, &principal)
        .org(org.id)
        .can_grant_role()
        .await
        .expect("Authorization failed");

    assert!(result.allowed);
}

#[tokio::test]
async fn org_grant_role_not_admin() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Only member, not admin
    fixtures::org_role(org.id, user.id)
        .member()
        .insert(&ctx.conn)
        .await
        .expect("Failed to add member");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::GrantOrgRole]).await;

    let result = AuthCheck::new(&ctx.conn, &principal)
        .org(org.id)
        .can_grant_role()
        .await
        .expect("Authorization failed");

    assert!(!result.allowed);
    assert!(
        result
            .checks
            .iter()
            .any(|c| c.policy == Policy::OrgAdmin && !c.passed)
    );
}

#[tokio::test]
async fn org_create_token_as_admin() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::CreateOrgToken]).await;

    let result = AuthCheck::new(&ctx.conn, &principal)
        .org(org.id)
        .can_create_token()
        .await
        .expect("Authorization failed");

    assert!(result.allowed);
}

#[tokio::test]
async fn create_personal_token_as_owner() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = create_api_key_principal(
        &ctx,
        &user,
        vec!["*"],
        vec![Permission::CreatePersonalToken],
    )
    .await;

    let resource = TokenResource {
        id: 0, // Doesn't matter for create
        owner: OwnerId::User(user.id),
    };

    let result = resource
        .authorize(&ctx.conn, &principal, Permission::CreatePersonalToken)
        .await
        .expect("Authorization failed");

    assert!(result.allowed);
    assert!(
        result
            .checks
            .iter()
            .any(|c| c.policy == Policy::TokenOwnership && c.passed)
    );
}

#[tokio::test]
async fn create_personal_token_wrong_user() {
    let ctx = TestDbCtx::new().await;

    let user1 = fixtures::user()
        .gh_login("user1")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user1");

    let user2 = fixtures::user()
        .gh_login("user2")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user2");

    let principal = create_api_key_principal(
        &ctx,
        &user1,
        vec!["*"],
        vec![Permission::CreatePersonalToken],
    )
    .await;

    // user1 tries to create token for user2
    let resource = TokenResource {
        id: 0,
        owner: OwnerId::User(user2.id),
    };

    let result = resource
        .authorize(&ctx.conn, &principal, Permission::CreatePersonalToken)
        .await
        .expect("Authorization failed");

    assert!(!result.allowed);
    assert!(
        result
            .checks
            .iter()
            .any(|c| c.policy == Policy::TokenOwnership && !c.passed)
    );
}

#[tokio::test]
async fn revoke_personal_token_as_owner() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = create_api_key_principal(
        &ctx,
        &user,
        vec!["*"],
        vec![Permission::RevokePersonalToken],
    )
    .await;

    let result = AuthCheck::new(&ctx.conn, &principal)
        .token(123, OwnerId::User(user.id))
        .can_revoke_personal()
        .await
        .expect("Authorization failed");

    assert!(result.allowed);
}

#[tokio::test]
async fn revoke_personal_token_not_owner() {
    let ctx = TestDbCtx::new().await;

    let user1 = fixtures::user()
        .gh_login("owner")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user1");

    let user2 = fixtures::user()
        .gh_login("attacker")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user2");

    let principal = create_api_key_principal(
        &ctx,
        &user2,
        vec!["*"],
        vec![Permission::RevokePersonalToken],
    )
    .await;

    // user2 tries to revoke user1's token
    let result = AuthCheck::new(&ctx.conn, &principal)
        .token(123, OwnerId::User(user1.id))
        .can_revoke_personal()
        .await
        .expect("Authorization failed");

    assert!(!result.allowed);
}

#[tokio::test]
async fn schema_admin_user_via_org() {
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
        .name("org-owned-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // User is admin of org
    fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant org admin");

    // Org is admin of package
    fixtures::schema_role(pkg.id)
        .org(org.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant schema admin to org");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::PublishPackage]).await;

    // User should be able to publish via org admin chain
    let result = AuthCheck::new(&ctx.conn, &principal)
        .package("org-owned-pkg", Some(pkg.id))
        .can_publish()
        .await
        .expect("Authorization failed");

    assert!(result.allowed);
    assert!(
        result
            .checks
            .iter()
            .any(|c| c.policy == Policy::SchemaAdmin && c.passed)
    );
}

#[tokio::test]
async fn schema_admin_revoked_role() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("revoked-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // Grant then revoke
    fixtures::schema_role(pkg.id)
        .user(user.id)
        .admin()
        .revoked()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create revoked role");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::PublishPackage]).await;

    let result = AuthCheck::new(&ctx.conn, &principal)
        .package("revoked-pkg", Some(pkg.id))
        .can_publish()
        .await
        .expect("Authorization failed");

    assert!(!result.allowed);
    assert!(
        result
            .checks
            .iter()
            .any(|c| c.policy == Policy::SchemaAdmin && !c.passed)
    );
}

#[tokio::test]
async fn fluent_package_can_publish() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["my-*"], vec![Permission::PublishPackage]).await;

    let result = AuthCheck::new(&ctx.conn, &principal)
        .package("my-new-package", None)
        .can_publish()
        .await
        .expect("Authorization failed");

    assert!(result.allowed);
}

#[tokio::test]
async fn fluent_org_can_grant_role() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::GrantOrgRole]).await;

    let result = AuthCheck::new(&ctx.conn, &principal)
        .org(org.id)
        .can_grant_role()
        .await
        .expect("Authorization failed");

    assert!(result.allowed);
}

#[tokio::test]
async fn auth_result_require_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal =
        create_api_key_principal(&ctx, &user, vec!["*"], vec![Permission::PublishPackage]).await;

    let result = AuthCheck::new(&ctx.conn, &principal)
        .package("any-pkg", None)
        .can_publish()
        .await
        .expect("Authorization failed");

    // Should succeed
    assert!(result.require().is_ok());
}

#[tokio::test]
async fn auth_result_require_failure() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    // No publish permission
    let principal = create_api_key_principal(&ctx, &user, vec!["*"], vec![]).await;

    let result = AuthCheck::new(&ctx.conn, &principal)
        .package("any-pkg", None)
        .can_publish()
        .await
        .expect("Authorization failed");

    // Should fail
    assert!(result.require().is_err());
}
