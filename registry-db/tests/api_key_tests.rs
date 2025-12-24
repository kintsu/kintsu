//! API Key Engine Tests
//!
//! Tests for registry-db/src/engine/api_key.rs
//! Covers key creation, lookup, revocation, scope matching, and permissions.

mod common;

use chrono::{Duration, Utc};
use common::fixtures;
use kintsu_registry_db::{
    engine::{Entity as EngineEntity, OwnerId, PrincipalIdentity, api_key::NewApiKey},
    entities::*,
    tst::TestDbCtx,
};
use secrecy::SecretString;

#[tokio::test]
async fn create_personal_key_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = NewApiKey::new_for_user(
        Some("Test personal key".to_string()),
        vec![Scope::new("*")],
        vec![Permission::PublishPackage],
        Utc::now() + Duration::days(30),
        user.id,
    )
    .qualify(&ctx.conn, &principal)
    .await
    .expect("Failed to create key");

    // Verify plain token format
    assert!(one_time.key.starts_with("kintsu_"));
    assert!(!one_time.key.is_empty());

    // Verify metadata
    assert_eq!(
        one_time.api_key.description,
        Some("Test personal key".to_string())
    );
    assert_eq!(one_time.api_key.user_id, Some(user.id));
    assert!(one_time.api_key.org_id.is_none());
    assert!(one_time.api_key.revoked_at.is_none());
}

#[tokio::test]
async fn create_personal_key_wrong_user() {
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

    // user1 is the principal
    let principal = PrincipalIdentity::UserSession {
        user: user1.clone(),
    };

    // Attempt to create key for user2
    let result = NewApiKey::new_for_user(
        Some("Wrong user key".to_string()),
        vec![Scope::new("*")],
        vec![Permission::PublishPackage],
        Utc::now() + Duration::days(30),
        user2.id, // user2's id
    )
    .qualify(&ctx.conn, &principal)
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn create_org_key_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .name("key-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Make user admin of org
    fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = NewApiKey::new_for_org(
        Some("Org key".to_string()),
        vec![Scope::new("*")],
        vec![Permission::PublishPackage],
        Utc::now() + Duration::days(30),
        org.id,
    )
    .qualify(&ctx.conn, &principal)
    .await
    .expect("Failed to create org key");

    assert_eq!(one_time.api_key.org_id, Some(org.id));
    assert!(one_time.api_key.user_id.is_none());
}

#[tokio::test]
async fn create_org_key_not_admin() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // User is NOT an admin of org
    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let result = NewApiKey::new_for_org(
        Some("Unauthorized org key".to_string()),
        vec![Scope::new("*")],
        vec![Permission::PublishPackage],
        Utc::now() + Duration::days(30),
        org.id,
    )
    .qualify(&ctx.conn, &principal)
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn create_org_key_org_not_found() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    // Non-existent org id
    let result = NewApiKey::new_for_org(
        Some("Key for missing org".to_string()),
        vec![Scope::new("*")],
        vec![Permission::PublishPackage],
        Utc::now() + Duration::days(30),
        999999,
    )
    .qualify(&ctx.conn, &principal)
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn lookup_by_id_found() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .description(Some("Lookup key"))
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    let found = ApiKey::by_id(&ctx.conn, one_time.api_key.id)
        .await
        .expect("Lookup failed");

    assert_eq!(found.id, one_time.api_key.id);
    assert_eq!(found.description, Some("Lookup key".to_string()));
}

#[tokio::test]
async fn lookup_by_id_not_found() {
    let ctx = TestDbCtx::new().await;

    let result = ApiKey::by_id(&ctx.conn, 999999).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn lookup_by_raw_token_valid() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .description(Some("Token lookup key"))
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    let secret = SecretString::from(one_time.key);
    let found = ApiKey::by_raw_token(&ctx.conn, &secret)
        .await
        .expect("Lookup failed");

    assert_eq!(found.id, one_time.api_key.id);
    assert_eq!(found.description, Some("Token lookup key".to_string()));
}

#[tokio::test]
async fn lookup_by_raw_token_invalid_format() {
    let ctx = TestDbCtx::new().await;

    let invalid = SecretString::from("not-a-valid-token");
    let result = ApiKey::by_raw_token(&ctx.conn, &invalid).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn lookup_by_raw_token_expired() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    // Create key that expires in the past
    let one_time = fixtures::api_key()
        .user(user.id)
        .expires(Utc::now() - Duration::hours(1))
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create expired key");

    let secret = SecretString::from(one_time.key);
    let result = ApiKey::by_raw_token(&ctx.conn, &secret).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn lookup_by_raw_token_revoked() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    // Revoke the key
    ApiKey::revoke_token_by_id(&ctx.conn, one_time.api_key.id, &principal)
        .await
        .expect("Failed to revoke");

    // Try to lookup
    let secret = SecretString::from(one_time.key);
    let result = ApiKey::by_raw_token(&ctx.conn, &secret).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn get_owner_user() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .gh_login("owner-user")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    let owner = one_time
        .api_key
        .get_token_owner(&ctx.conn)
        .await
        .expect("Failed to get owner");

    match owner {
        EngineEntity::User(u) => {
            assert_eq!(u.id, user.id);
            assert_eq!(u.gh_login, "owner-user");
        },
        _ => panic!("Expected User owner"),
    }
}

#[tokio::test]
async fn get_owner_org() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .name("owner-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Make user admin
    fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .org(org.id)
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    let owner = one_time
        .api_key
        .get_token_owner(&ctx.conn)
        .await
        .expect("Failed to get owner");

    match owner {
        EngineEntity::Org(o) => {
            assert_eq!(o.id, org.id);
            assert_eq!(o.name, "owner-org");
        },
        _ => panic!("Expected Org owner"),
    }
}

#[tokio::test]
async fn revoke_personal_token_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    // Revoke the token
    ApiKey::revoke_token_by_id(&ctx.conn, one_time.api_key.id, &principal)
        .await
        .expect("Failed to revoke");

    // Verify revoked
    let key = ApiKey::by_id(&ctx.conn, one_time.api_key.id)
        .await
        .expect("Lookup failed");
    assert!(key.revoked());
}

#[tokio::test]
async fn revoke_personal_token_unauthorized() {
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

    let principal1 = PrincipalIdentity::UserSession {
        user: user1.clone(),
    };

    // user1 creates key
    let one_time = fixtures::api_key()
        .user(user1.id)
        .insert(&ctx.conn, &principal1)
        .await
        .expect("Failed to create key");

    // user2 tries to revoke
    let principal2 = PrincipalIdentity::UserSession {
        user: user2.clone(),
    };
    let result = ApiKey::revoke_token_by_id(&ctx.conn, one_time.api_key.id, &principal2).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn revoke_org_token_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Make user admin
    fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .org(org.id)
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    // Revoke as admin
    ApiKey::revoke_token_by_id(&ctx.conn, one_time.api_key.id, &principal)
        .await
        .expect("Failed to revoke");

    let key = ApiKey::by_id(&ctx.conn, one_time.api_key.id)
        .await
        .expect("Lookup failed");
    assert!(key.revoked());
}

#[tokio::test]
async fn revoke_org_token_not_admin() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin");

    let non_admin_user = fixtures::user()
        .gh_login("nonadmin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create non-admin");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Make only admin_user an admin
    fixtures::org_role(org.id, admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let admin_principal = PrincipalIdentity::UserSession {
        user: admin_user.clone(),
    };

    // Create org key as admin
    let one_time = fixtures::api_key()
        .org(org.id)
        .insert(&ctx.conn, &admin_principal)
        .await
        .expect("Failed to create key");

    // non-admin tries to revoke
    let non_admin_principal = PrincipalIdentity::UserSession {
        user: non_admin_user.clone(),
    };
    let result =
        ApiKey::revoke_token_by_id(&ctx.conn, one_time.api_key.id, &non_admin_principal).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn scope_match_wildcard() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(vec!["*"])
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    assert!(
        one_time
            .api_key
            .check_scope_match("any-package")
    );
    assert!(
        one_time
            .api_key
            .check_scope_match("another-pkg")
    );
    assert!(one_time.api_key.check_scope_match("foo/bar"));
}

#[tokio::test]
async fn scope_match_prefix() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(vec!["my-pkg-*"])
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    assert!(
        one_time
            .api_key
            .check_scope_match("my-pkg-foo")
    );
    assert!(
        one_time
            .api_key
            .check_scope_match("my-pkg-bar")
    );
    assert!(
        !one_time
            .api_key
            .check_scope_match("other-pkg")
    );
    assert!(!one_time.api_key.check_scope_match("my-pkg")); // No trailing match without -
}

#[tokio::test]
async fn scope_match_exact() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(vec!["exact-name"])
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    assert!(
        one_time
            .api_key
            .check_scope_match("exact-name")
    );
    assert!(
        !one_time
            .api_key
            .check_scope_match("exact-name-other")
    );
    assert!(
        !one_time
            .api_key
            .check_scope_match("other-name")
    );
}

#[tokio::test]
async fn check_permissions_both_valid() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(vec!["pkg-*"])
        .permissions(vec![Permission::PublishPackage])
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    let check = one_time
        .api_key
        .check_permissions_for_package("pkg-foo", &Permission::PublishPackage);

    assert!(check.scope_matches);
    assert!(check.has_permission);
    assert!(check.ok());
}

#[tokio::test]
async fn check_permissions_scope_fail() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(vec!["other-*"])
        .permissions(vec![Permission::PublishPackage])
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    let check = one_time
        .api_key
        .check_permissions_for_package("pkg-foo", &Permission::PublishPackage);

    assert!(!check.scope_matches);
    assert!(check.has_permission);
    assert!(!check.ok());
}

#[tokio::test]
async fn check_permissions_perm_fail() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(vec!["*"])
        .permissions(vec![Permission::YankPackage])
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    let check = one_time
        .api_key
        .check_permissions_for_package("pkg-foo", &Permission::PublishPackage);

    assert!(check.scope_matches);
    assert!(!check.has_permission);
    assert!(!check.ok());
}

#[tokio::test]
async fn must_have_permission_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(vec!["*"])
        .permissions(vec![Permission::PublishPackage])
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    let result = one_time
        .api_key
        .must_have_permission_for_package("any-pkg", &Permission::PublishPackage);
    assert!(result.is_ok());
}

#[tokio::test]
async fn must_have_permission_failure() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let one_time = fixtures::api_key()
        .user(user.id)
        .scopes(vec!["limited-*"])
        .permissions(vec![Permission::YankPackage])
        .insert(&ctx.conn, &principal)
        .await
        .expect("Failed to create key");

    // Wrong scope
    let result1 = one_time
        .api_key
        .must_have_permission_for_package("other-pkg", &Permission::YankPackage);
    assert!(result1.is_err());

    // Wrong permission
    let result2 = one_time
        .api_key
        .must_have_permission_for_package("limited-pkg", &Permission::PublishPackage);
    assert!(result2.is_err());
}
