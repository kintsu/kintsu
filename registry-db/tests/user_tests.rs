//! User Engine Tests
//!
//! Tests for registry-db/src/engine/user.rs
//! Covers user creation, lookup, organizations, and token management.

mod common;

use chrono::{Duration, Utc};
use common::fixtures;
use kintsu_registry_db::{
    Error,
    engine::{
        PrincipalIdentity,
        user::{NewUser, create_or_update_user_from_oauth},
    },
    entities::*,
    tst::TestDbCtx,
};

#[tokio::test]
async fn create_new_user_success() {
    let ctx = TestDbCtx::new().await;

    let user = create_or_update_user_from_oauth(
        &ctx.conn,
        12345,
        "alice",
        Some("https://github.com/alice.png"),
        "alice@example.com",
    )
    .await
    .expect("Failed to create user");

    assert_eq!(user.gh_id, 12345);
    assert_eq!(user.gh_login, "alice");
    assert_eq!(user.email, "alice@example.com");
    assert_eq!(
        user.gh_avatar,
        Some("https://github.com/alice.png".to_string())
    );
    assert!(user.id > 0);
}

#[tokio::test]
async fn update_existing_user_on_conflict() {
    let ctx = TestDbCtx::new().await;

    // Create initial user
    let user1 = create_or_update_user_from_oauth(
        &ctx.conn,
        123,
        "old_login",
        Some("https://old-avatar.png"),
        "old@example.com",
    )
    .await
    .expect("Failed to create user");

    let original_id = user1.id;

    // Update with same gh_id but different fields
    let user2 = create_or_update_user_from_oauth(
        &ctx.conn,
        123,
        "new_login",
        Some("https://new-avatar.png"),
        "new@example.com",
    )
    .await
    .expect("Failed to update user");

    // ID should remain the same (upsert)
    assert_eq!(user2.id, original_id);
    // Fields should be updated
    assert_eq!(user2.gh_login, "new_login");
    assert_eq!(user2.email, "new@example.com");
    assert_eq!(user2.gh_avatar, Some("https://new-avatar.png".to_string()));
}

#[tokio::test]
async fn duplicate_email_different_gh_id() {
    let ctx = TestDbCtx::new().await;

    // Create first user
    let _user1 =
        create_or_update_user_from_oauth(&ctx.conn, 100, "user1", None, "shared@example.com")
            .await
            .expect("Failed to create user1");

    // Attempt to create second user with same email but different gh_id
    let result =
        create_or_update_user_from_oauth(&ctx.conn, 200, "user2", None, "shared@example.com").await;

    // Should fail due to unique email constraint
    assert!(result.is_err());
}

#[tokio::test]
async fn lookup_by_id_found() {
    let ctx = TestDbCtx::new().await;

    let created = fixtures::user()
        .gh_login("findme")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let found = User::by_id(&ctx.conn, created.id)
        .await
        .expect("Query failed");

    assert!(found.is_some());
    let user = found.unwrap();
    assert_eq!(user.id, created.id);
    assert_eq!(user.gh_login, "findme");
}

#[tokio::test]
async fn lookup_by_id_not_found() {
    let ctx = TestDbCtx::new().await;

    let found = User::by_id(&ctx.conn, 999999)
        .await
        .expect("Query failed");

    assert!(found.is_none());
}

#[tokio::test]
async fn lookup_by_gh_id() {
    let ctx = TestDbCtx::new().await;

    let created = fixtures::user()
        .gh_id(999)
        .gh_login("gh_user")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let found = User::by_gh_id(&ctx.conn, 999)
        .await
        .expect("Query failed");

    assert!(found.is_some());
    let user = found.unwrap();
    assert_eq!(user.id, created.id);
    assert_eq!(user.gh_id, 999);
}

#[tokio::test]
async fn lookup_by_email() {
    let ctx = TestDbCtx::new().await;

    let created = fixtures::user()
        .email("unique@test.com")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let found = User::by_email(&ctx.conn, "unique@test.com")
        .await
        .expect("Query failed");

    assert!(found.is_some());
    let user = found.unwrap();
    assert_eq!(user.id, created.id);
    assert_eq!(user.email, "unique@test.com");
}

#[tokio::test]
async fn exists_returns_true() {
    let ctx = TestDbCtx::new().await;

    let created = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let exists = User::exists(&ctx.conn, created.id)
        .await
        .expect("Query failed");

    assert!(exists);
}

#[tokio::test]
async fn exists_returns_false() {
    let ctx = TestDbCtx::new().await;

    let exists = User::exists(&ctx.conn, 999999)
        .await
        .expect("Query failed");

    assert!(!exists);
}

#[tokio::test]
async fn user_orgs_empty() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let orgs = user
        .orgs(&ctx.conn)
        .await
        .expect("Query failed");

    assert!(orgs.is_empty());
}

#[tokio::test]
async fn user_orgs_multiple_roles() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org1 = fixtures::org()
        .name("org-admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org1");

    let org2 = fixtures::org()
        .name("org-member")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org2");

    // Add user as admin to org1
    fixtures::org_role(org1.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin role");

    // Add user as member to org2
    fixtures::org_role(org2.id, user.id)
        .member()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant member role");

    let orgs = user
        .orgs(&ctx.conn)
        .await
        .expect("Query failed");

    assert_eq!(orgs.len(), 2);

    let org_names: Vec<&str> = orgs
        .iter()
        .map(|o| o.org.name.as_str())
        .collect();
    assert!(org_names.contains(&"org-admin"));
    assert!(org_names.contains(&"org-member"));
}

#[tokio::test]
async fn user_orgs_exclude_revoked() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .name("revoked-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Grant and then revoke role
    let role = fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant role");

    // Manually revoke by updating revoked_at
    use sea_orm::{ActiveModelTrait, Set};
    let mut active: OrgRoleActiveModel = role.into();
    active.revoked_at = Set(Some(Utc::now()));
    active
        .update(&ctx.conn)
        .await
        .expect("Failed to revoke");

    let orgs = user
        .orgs(&ctx.conn)
        .await
        .expect("Query failed");

    // Revoked org should not appear
    assert!(orgs.is_empty());
}

#[tokio::test]
async fn user_tokens_empty() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let tokens = User::tokens(&ctx.conn, user.id)
        .await
        .expect("Query failed");

    assert!(tokens.is_empty());
}

#[tokio::test]
async fn user_tokens_multiple() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    // Create 3 API keys
    for i in 0..3 {
        fixtures::api_key()
            .user(user.id)
            .description(Some(&format!("Key {}", i)))
            .insert(&ctx.conn, &principal)
            .await
            .expect("Failed to create API key");
    }

    let tokens = User::tokens(&ctx.conn, user.id)
        .await
        .expect("Query failed");

    assert_eq!(tokens.len(), 3);

    // Verify ordered by id desc (most recent first)
    for i in 0..tokens.len() - 1 {
        assert!(tokens[i].id > tokens[i + 1].id);
    }
}

#[tokio::test]
async fn user_tokens_excludes_org_keys() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Make user admin of org
    fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin role");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    // Create user token
    fixtures::api_key()
        .user(user.id)
        .description(Some("User token"))
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create user token");

    // Create org token
    fixtures::api_key()
        .org(org.id)
        .description(Some("Org token"))
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create org token");

    let tokens = User::tokens(&ctx.conn, user.id)
        .await
        .expect("Query failed");

    // Should only return user token
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].description, Some("User token".to_string()));
}

#[tokio::test]
async fn request_personal_token_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time_key = user
        .request_personal_token(
            &ctx.conn,
            &principal,
            Some("Test token".to_string()),
            vec![Scope::new("*")],
            vec![Permission::PublishPackage],
            Utc::now() + Duration::days(30),
        )
        .await
        .expect("Failed to create personal token");

    // Verify plain token is returned
    assert!(!one_time_key.key.is_empty());
    assert!(one_time_key.key.starts_with("kintsu_"));

    // Verify API key metadata
    assert_eq!(
        one_time_key.api_key.description,
        Some("Test token".to_string())
    );
    assert_eq!(one_time_key.api_key.user_id, Some(user.id));
    assert!(one_time_key.api_key.org_id.is_none());
}

#[tokio::test]
async fn request_personal_token_unauthorized() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    // First create a valid token via session
    let session_principal = PrincipalIdentity::UserSession { user: user.clone() };
    let one_time = fixtures::api_key()
        .user(user.id)
        .insert(&ctx.conn, &session_principal)
        .await
        .expect("Failed to create API key");

    // Now try to create a token using an API key principal
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

    let api_key_principal = PrincipalIdentity::UserApiKey {
        user: user.clone(),
        key: api_key,
    };

    let result = user
        .request_personal_token(
            &ctx.conn,
            &api_key_principal,
            Some("Another token".to_string()),
            vec![Scope::new("*")],
            vec![Permission::PublishPackage],
            Utc::now() + Duration::days(30),
        )
        .await;

    // Should fail - API key principals cannot create tokens
    assert!(result.is_err());
}

#[tokio::test]
async fn request_org_token_as_admin() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .name("token-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Make user admin
    fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin role");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time_key = user
        .request_org_token(
            &ctx.conn,
            &principal,
            Some("Org token".to_string()),
            vec![Scope::new("*")],
            vec![Permission::PublishPackage],
            Utc::now() + Duration::days(30),
            org.id,
        )
        .await
        .expect("Failed to create org token");

    assert!(!one_time_key.key.is_empty());
    assert_eq!(one_time_key.api_key.org_id, Some(org.id));
    assert!(one_time_key.api_key.user_id.is_none());
}

#[tokio::test]
async fn request_org_token_not_admin() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .name("foreign-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // User is NOT a member of org

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let result = user
        .request_org_token(
            &ctx.conn,
            &principal,
            Some("Unauthorized org token".to_string()),
            vec![Scope::new("*")],
            vec![Permission::PublishPackage],
            Utc::now() + Duration::days(30),
            org.id,
        )
        .await;

    // Should fail - user is not admin of org
    assert!(result.is_err());
}
