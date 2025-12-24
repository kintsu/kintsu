//! Basic integration tests to verify test infrastructure works correctly

mod common;

use common::fixtures;
use kintsu_registry_db::tst::TestDbCtx;

#[tokio::test]
async fn test_user_fixture_insert() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .gh_login("alice")
        .email("alice@example.com")
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert user");

    assert_eq!(user.gh_login, "alice");
    assert_eq!(user.email, "alice@example.com");
    assert!(user.id > 0);
}

#[tokio::test]
async fn test_org_fixture_insert() {
    let ctx = TestDbCtx::new().await;

    let org = fixtures::org()
        .name("acme-corp")
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert org");

    assert_eq!(org.name, "acme-corp");
    assert!(org.id > 0);
}

#[tokio::test]
async fn test_package_fixture_insert() {
    let ctx = TestDbCtx::new().await;

    let pkg = fixtures::package()
        .name("my-package")
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert package");

    assert_eq!(pkg.name, "my-package");
    assert!(pkg.id > 0);
}

#[tokio::test]
async fn test_version_fixture_insert() {
    let ctx = TestDbCtx::new().await;

    // First create a package
    let pkg = fixtures::package()
        .name("versioned-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert package");

    // Create user as publisher
    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert user");

    // Now create a version
    let ver = fixtures::version(pkg.id)
        .version("1.2.3")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert version");

    assert_eq!(ver.qualified_version.to_string(), "1.2.3");
    assert_eq!(ver.package, pkg.id);
    assert_eq!(ver.publishing_user_id, Some(user.id));
}

#[tokio::test]
async fn test_org_role_fixture_insert() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert user");

    let org = fixtures::org()
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert org");

    let role = fixtures::org_role(org.id, user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert org role");

    assert_eq!(role.org_id, org.id);
    assert_eq!(role.user_id, user.id);
    assert!(role.revoked_at.is_none());
}

#[tokio::test]
async fn test_schema_role_fixture_insert() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert user");

    let pkg = fixtures::package()
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert package");

    let role = fixtures::schema_role(pkg.id)
        .user(user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert schema role");

    assert_eq!(role.package, pkg.id);
    assert_eq!(role.user_id, Some(user.id));
    assert!(role.revoked_at.is_none());
}
