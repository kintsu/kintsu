mod common;

use common::fixtures;
use kintsu_registry_db::{
    engine::{
        PrincipalIdentity,
        org::{grant_role, import_organization, revoke_role},
    },
    entities::*,
    tst::TestDbCtx,
};

#[tokio::test]
async fn lookup_by_id_found() {
    let ctx = TestDbCtx::new().await;

    let created = fixtures::org()
        .name("lookup-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    let found = Org::by_id(&ctx.conn, created.id)
        .await
        .expect("Query failed");

    assert!(found.is_some());
    let org = found.unwrap();
    assert_eq!(org.id, created.id);
    assert_eq!(org.name, "lookup-org");
}

#[tokio::test]
async fn lookup_by_id_not_found() {
    let ctx = TestDbCtx::new().await;

    let found = Org::by_id(&ctx.conn, 999999)
        .await
        .expect("Query failed");

    assert!(found.is_none());
}

#[tokio::test]
async fn lookup_by_name_found() {
    let ctx = TestDbCtx::new().await;

    let created = fixtures::org()
        .name("acme")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    let found = Org::by_name(&ctx.conn, "acme")
        .await
        .expect("Query failed");

    assert!(found.is_some());
    let org = found.unwrap();
    assert_eq!(org.id, created.id);
    assert_eq!(org.name, "acme");
}

#[tokio::test]
async fn lookup_by_name_not_found() {
    let ctx = TestDbCtx::new().await;

    let found = Org::by_name(&ctx.conn, "nonexistent")
        .await
        .expect("Query failed");

    assert!(found.is_none());
}

#[tokio::test]
async fn exists_true() {
    let ctx = TestDbCtx::new().await;

    fixtures::org()
        .name("existing-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    let exists = Org::exists(&ctx.conn, "existing-org")
        .await
        .expect("Query failed");

    assert!(exists);
}

#[tokio::test]
async fn exists_false() {
    let ctx = TestDbCtx::new().await;

    let exists = Org::exists(&ctx.conn, "nonexistent-org")
        .await
        .expect("Query failed");

    assert!(!exists);
}

#[tokio::test]
async fn exists_bulk() {
    let ctx = TestDbCtx::new().await;

    // Create orgs
    for name in ["acme", "beta", "gamma"] {
        fixtures::org()
            .name(name)
            .insert(&ctx.conn)
            .await
            .expect("Failed to create org");
    }

    // Query for mix of existing and non-existing
    let existing = Org::exists_bulk(&ctx.conn, &["acme", "delta", "gamma"])
        .await
        .expect("Query failed");

    assert_eq!(existing.len(), 2);
    assert!(existing.contains(&"acme".to_string()));
    assert!(existing.contains(&"gamma".to_string()));
    assert!(!existing.contains(&"delta".to_string()));
}

#[tokio::test]
async fn import_org_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .gh_login("importer")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    let org = import_organization(
        &ctx.conn,
        &principal,
        999,
        "imported-org".to_string(),
        "https://github.com/avatar.png".to_string(),
    )
    .await
    .expect("Failed to import org");

    assert_eq!(org.name, "imported-org");
    assert_eq!(org.gh_id, 999);
    assert_eq!(org.gh_avatar, "https://github.com/avatar.png");

    // Verify user is admin
    let is_admin = org
        .is_user_admin(&ctx.conn, user.id)
        .await
        .expect("Query failed");
    assert!(is_admin);
}

#[tokio::test]
async fn import_org_requires_session() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    // Create an API key for user
    let session_principal = PrincipalIdentity::UserSession { user: user.clone() };
    let one_time = fixtures::api_key()
        .user(user.id)
        .insert(&ctx.conn, &session_principal)
        .await
        .expect("Failed to create API key");

    // Try to import with API key principal
    let api_key_principal = PrincipalIdentity::UserApiKey {
        user: user.clone(),
        key: ApiKey {
            id: one_time.api_key.id,
            description: one_time.api_key.description,
            expires: one_time.api_key.expires,
            scopes: one_time.api_key.scopes,
            permissions: one_time.api_key.permissions,
            user_id: one_time.api_key.user_id,
            org_id: one_time.api_key.org_id,
            last_used_at: None,
            revoked_at: None,
        },
    };

    let result = import_organization(
        &ctx.conn,
        &api_key_principal,
        888,
        "api-key-org".to_string(),
        "https://avatar.png".to_string(),
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn import_org_duplicate_name() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    // First import succeeds
    import_organization(
        &ctx.conn,
        &principal,
        100,
        "duplicate-name-org".to_string(),
        "https://avatar.png".to_string(),
    )
    .await
    .expect("First import should succeed");

    // Second import with same name fails
    let result = import_organization(
        &ctx.conn,
        &principal,
        200,
        "duplicate-name-org".to_string(),
        "https://avatar2.png".to_string(),
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn import_org_duplicate_gh_id() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    // First import succeeds
    import_organization(
        &ctx.conn,
        &principal,
        123,
        "org-one".to_string(),
        "https://avatar.png".to_string(),
    )
    .await
    .expect("First import should succeed");

    // Second import with same gh_id fails
    let result = import_organization(
        &ctx.conn,
        &principal,
        123,
        "org-two".to_string(),
        "https://avatar2.png".to_string(),
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn grant_org_role_success() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .gh_login("admin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin");

    let target_user = fixtures::user()
        .gh_login("target")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create target user");

    let org = fixtures::org()
        .name("role-test-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Make admin_user an admin
    fixtures::org_role(org.id, admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant initial admin");

    let principal = PrincipalIdentity::UserSession {
        user: admin_user.clone(),
    };

    // Grant admin role to target user
    let role = grant_role(
        &ctx.conn,
        &principal,
        org.id,
        target_user.id,
        OrgRoleType::Admin,
    )
    .await
    .expect("Failed to grant role");

    assert_eq!(role.org_id, org.id);
    assert_eq!(role.user_id, target_user.id);
    assert_eq!(role.role, OrgRoleType::Admin);
    assert!(role.revoked_at.is_none());
}

#[tokio::test]
async fn grant_role_already_granted() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin");

    let target_user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create target user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Make admin_user an admin
    fixtures::org_role(org.id, admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant initial admin");

    let principal = PrincipalIdentity::UserSession {
        user: admin_user.clone(),
    };

    // Grant role first time
    grant_role(
        &ctx.conn,
        &principal,
        org.id,
        target_user.id,
        OrgRoleType::Member,
    )
    .await
    .expect("First grant should succeed");

    // Attempt to grant same role again
    let result = grant_role(
        &ctx.conn,
        &principal,
        org.id,
        target_user.id,
        OrgRoleType::Member,
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn grant_role_unauthorized() {
    let ctx = TestDbCtx::new().await;

    let non_admin_user = fixtures::user()
        .gh_login("nonadmin")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create non-admin");

    let target_user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create target user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // non_admin_user is NOT an admin of org
    let principal = PrincipalIdentity::UserSession {
        user: non_admin_user.clone(),
    };

    // Attempt to grant role
    let result = grant_role(
        &ctx.conn,
        &principal,
        org.id,
        target_user.id,
        OrgRoleType::Member,
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn revoke_role_success() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin");

    let target_user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create target user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Make admin_user an admin
    fixtures::org_role(org.id, admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant initial admin");

    // Grant role to target
    fixtures::org_role(org.id, target_user.id)
        .member()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant member role");

    let principal = PrincipalIdentity::UserSession {
        user: admin_user.clone(),
    };

    // Revoke role
    revoke_role(&ctx.conn, &principal, org.id, target_user.id)
        .await
        .expect("Failed to revoke role");

    // Verify user is no longer admin
    let is_admin = org
        .is_user_admin(&ctx.conn, target_user.id)
        .await
        .expect("Query failed");
    assert!(!is_admin);
}

#[tokio::test]
async fn revoke_role_not_found() {
    let ctx = TestDbCtx::new().await;

    let admin_user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin");

    let target_user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create target user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Make admin_user an admin
    fixtures::org_role(org.id, admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant initial admin");

    let principal = PrincipalIdentity::UserSession {
        user: admin_user.clone(),
    };

    // Attempt to revoke non-existent role
    let result = revoke_role(&ctx.conn, &principal, org.id, target_user.id).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn revoke_role_unauthorized() {
    let ctx = TestDbCtx::new().await;

    let non_admin_user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create non-admin");

    let target_user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create target user");

    let admin_user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create admin");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Make admin_user an admin
    fixtures::org_role(org.id, admin_user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant initial admin");

    // Grant role to target
    fixtures::org_role(org.id, target_user.id)
        .member()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant member role");

    // non_admin_user is NOT an admin
    let principal = PrincipalIdentity::UserSession {
        user: non_admin_user.clone(),
    };

    // Attempt to revoke role
    let result = revoke_role(&ctx.conn, &principal, org.id, target_user.id).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn org_tokens_empty() {
    let ctx = TestDbCtx::new().await;

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    let tokens = Org::tokens(&ctx.conn, org.id)
        .await
        .expect("Query failed");

    assert!(tokens.is_empty());
}

#[tokio::test]
async fn org_tokens_multiple() {
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
        .expect("Failed to grant admin role");

    let principal = PrincipalIdentity::UserSession { user: user.clone() };

    // Create 3 org tokens
    for i in 0..3 {
        fixtures::api_key()
            .org(org.id)
            .description(Some(&format!("Org key {}", i)))
            .insert(&ctx.conn, &principal)
            .await
            .expect("Failed to create org token");
    }

    let tokens = Org::tokens(&ctx.conn, org.id)
        .await
        .expect("Query failed");

    assert_eq!(tokens.len(), 3);

    // Verify ordered by id desc (most recent first)
    for i in 0..tokens.len() - 1 {
        assert!(tokens[i].id > tokens[i + 1].id);
    }
}

#[tokio::test]
async fn is_user_admin_true() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Grant admin role
    fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin role");

    let is_admin = org
        .is_user_admin(&ctx.conn, user.id)
        .await
        .expect("Query failed");

    assert!(is_admin);
}

#[tokio::test]
async fn is_user_admin_false_member() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Grant member role (not admin)
    fixtures::org_role(org.id, user.id)
        .member()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant member role");

    let is_admin = org
        .is_user_admin(&ctx.conn, user.id)
        .await
        .expect("Query failed");

    assert!(!is_admin);
}

#[tokio::test]
async fn is_user_admin_false_no_membership() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // No role granted

    let is_admin = org
        .is_user_admin(&ctx.conn, user.id)
        .await
        .expect("Query failed");

    assert!(!is_admin);
}

#[tokio::test]
async fn is_user_admin_false_revoked() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    // Grant and then revoke admin role
    let role = fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin role");

    // Revoke manually by updating revoked_at
    use sea_orm::{ActiveModelTrait, Set};
    let mut active: OrgRoleActiveModel = role.into();
    active.revoked_at = Set(Some(chrono::Utc::now()));
    active
        .update(&ctx.conn)
        .await
        .expect("Failed to revoke");

    let is_admin = org
        .is_user_admin(&ctx.conn, user.id)
        .await
        .expect("Query failed");

    assert!(!is_admin);
}
