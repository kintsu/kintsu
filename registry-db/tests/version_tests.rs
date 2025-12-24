//! Version Engine Tests
//!
//! Tests for registry-db/src/engine/version.rs
//! Covers version lookup, latest versions, downloads, dependencies, dependents.

mod common;

use common::fixtures;
use kintsu_registry_db::{engine::version::QualifiedPackageVersion, entities::*, tst::TestDbCtx};

#[tokio::test]
async fn lookup_by_id_found() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("lookup-ver-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let ver = fixtures::version(pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create version");

    let found = Version::by_id(&ctx.conn, ver.id)
        .await
        .expect("Lookup failed");

    assert_eq!(found.id, ver.id);
    assert_eq!(found.qualified_version.to_string(), "1.0.0");
}

#[tokio::test]
async fn lookup_by_id_not_found() {
    let ctx = TestDbCtx::new().await;

    let result = Version::by_id(&ctx.conn, 999999).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn lookup_by_name_and_version() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("my-named-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::version(pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create version");

    let found = Version::by_name_and_version(&ctx.conn, "my-named-pkg", "1.0.0")
        .await
        .expect("Lookup failed");

    assert_eq!(found.qualified_version.to_string(), "1.0.0");
}

#[tokio::test]
async fn lookup_version_not_found() {
    let ctx = TestDbCtx::new().await;

    let result = Version::by_name_and_version(&ctx.conn, "nonexistent-pkg", "1.0.0").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn exists_true() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("exists-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::version(pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create version");

    let exists = Version::exists(&ctx.conn, "exists-pkg", "1.0.0")
        .await
        .expect("Check failed");

    assert!(exists);
}

#[tokio::test]
async fn exists_false() {
    let ctx = TestDbCtx::new().await;

    let exists = Version::exists(&ctx.conn, "nonexistent-pkg", "1.0.0")
        .await
        .expect("Check failed");

    assert!(!exists);
}

#[tokio::test]
async fn latest_all_stable() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("latest-stable-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // Create versions in order - latest by ID should be 1.2.0
    for ver in ["1.0.0", "1.1.0", "1.2.0"] {
        fixtures::version(pkg.id)
            .version(ver)
            .publisher_user(user.id)
            .insert(&ctx.conn)
            .await
            .expect("Failed to create version");
    }

    let latest = Version::get_latest_versions(&ctx.conn, pkg.id)
        .await
        .expect("Failed to get latest");

    assert_eq!(latest.latest_version.to_string(), "1.2.0");
    assert_eq!(
        latest.latest_stable.map(|v| v.to_string()),
        Some("1.2.0".to_string())
    );
}

#[tokio::test]
async fn latest_stable_and_prerelease() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("latest-pre-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // Create versions - last one is prerelease
    for ver in ["0.9.0", "1.0.0", "1.1.0", "2.0.0-beta.0"] {
        fixtures::version(pkg.id)
            .version(ver)
            .publisher_user(user.id)
            .insert(&ctx.conn)
            .await
            .expect("Failed to create version");
    }

    let latest = Version::get_latest_versions(&ctx.conn, pkg.id)
        .await
        .expect("Failed to get latest");

    // Latest by ID is 2.0.0-beta.0
    assert_eq!(latest.latest_version.to_string(), "2.0.0-beta.0");
    // Latest stable should be 1.1.0
    assert_eq!(
        latest.latest_stable.map(|v| v.to_string()),
        Some("1.1.0".to_string())
    );
}

#[tokio::test]
async fn latest_no_stable() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("latest-nostable-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // Only prereleases
    for ver in ["1.0.0-alpha.0", "1.0.0-beta.0"] {
        fixtures::version(pkg.id)
            .version(ver)
            .publisher_user(user.id)
            .insert(&ctx.conn)
            .await
            .expect("Failed to create version");
    }

    let latest = Version::get_latest_versions(&ctx.conn, pkg.id)
        .await
        .expect("Failed to get latest");

    assert_eq!(latest.latest_version.to_string(), "1.0.0-beta.0");
    assert!(latest.latest_stable.is_none());
}

#[tokio::test]
async fn latest_no_versions() {
    let ctx = TestDbCtx::new().await;

    let pkg = fixtures::package()
        .name("empty-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let result = Version::get_latest_versions(&ctx.conn, pkg.id).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn get_version_specific() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .gh_login("publisher")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("specific-ver-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::version(pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create version");

    let qpv = Version::get_package_version(&ctx.conn, "specific-ver-pkg", "1.0.0")
        .await
        .expect("Failed to get version");

    assert_eq!(qpv.package.name, "specific-ver-pkg");
    assert_eq!(qpv.version.qualified_version.to_string(), "1.0.0");
}

#[tokio::test]
async fn get_version_latest() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("latest-ver-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    for ver in ["1.0.0", "1.1.0", "2.0.0-beta.0"] {
        fixtures::version(pkg.id)
            .version(ver)
            .publisher_user(user.id)
            .insert(&ctx.conn)
            .await
            .expect("Failed to create version");
    }

    let qpv = Version::get_package_version(&ctx.conn, "latest-ver-pkg", "latest")
        .await
        .expect("Failed to get latest");

    // Should return latest stable (1.1.0)
    assert_eq!(qpv.version.qualified_version.to_string(), "1.1.0");
}

#[tokio::test]
async fn get_version_latest_only_prerelease() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("only-pre-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    for ver in ["0.1.0-alpha.0", "0.1.0-beta.0"] {
        fixtures::version(pkg.id)
            .version(ver)
            .publisher_user(user.id)
            .insert(&ctx.conn)
            .await
            .expect("Failed to create version");
    }

    let qpv = Version::get_package_version(&ctx.conn, "only-pre-pkg", "latest")
        .await
        .expect("Failed to get latest");

    // Should return latest prerelease when no stable exists
    assert_eq!(qpv.version.qualified_version.to_string(), "0.1.0-beta.0");
}

#[tokio::test]
async fn get_version_invalid_version_string() {
    let ctx = TestDbCtx::new().await;

    fixtures::package()
        .name("invalid-ver-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let result =
        Version::get_package_version(&ctx.conn, "invalid-ver-pkg", "invalid-version").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn increment_new_download() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("download-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let ver = fixtures::version(pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create version");

    Version::increment_download_count(&ctx.conn, ver.id)
        .await
        .expect("Failed to increment");

    // Verify count is 1
    let count = Package::get_package_download_count(&ctx.conn, "download-pkg")
        .await
        .expect("Failed to get count");

    assert_eq!(count, 1);
}

#[tokio::test]
async fn increment_existing_download() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("multi-download-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let ver = fixtures::version(pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create version");

    // Increment multiple times on same day
    for _ in 0..5 {
        Version::increment_download_count(&ctx.conn, ver.id)
            .await
            .expect("Failed to increment");
    }

    let count = Package::get_package_download_count(&ctx.conn, "multi-download-pkg")
        .await
        .expect("Failed to get count");

    assert_eq!(count, 5);
}

#[tokio::test]
async fn dependencies_empty() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("no-deps-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let ver = fixtures::version(pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .dependencies(vec![])
        .insert(&ctx.conn)
        .await
        .expect("Failed to create version");

    let deps = ver
        .dependencies(&ctx.conn)
        .await
        .expect("Failed to get dependencies");

    assert!(deps.is_empty());
}

#[tokio::test]
async fn dependencies_multiple() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    // Create dependency packages
    let dep_pkg1 = fixtures::package()
        .name("aaa-dep")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create dep1 pkg");

    let dep_pkg2 = fixtures::package()
        .name("bbb-dep")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create dep2 pkg");

    let dep1 = fixtures::version(dep_pkg1.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create dep1");

    let dep2 = fixtures::version(dep_pkg2.id)
        .version("2.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create dep2");

    // Create main package with dependencies
    let main_pkg = fixtures::package()
        .name("main-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create main pkg");

    let main_ver = fixtures::version(main_pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .dependencies(vec![dep1.id, dep2.id])
        .insert(&ctx.conn)
        .await
        .expect("Failed to create main version");

    let deps = main_ver
        .dependencies(&ctx.conn)
        .await
        .expect("Failed to get dependencies");

    assert_eq!(deps.len(), 2);

    // Should be ordered by package name
    assert_eq!(deps[0].package.name, "aaa-dep");
    assert_eq!(deps[1].package.name, "bbb-dep");
}

#[tokio::test]
async fn dependents_none() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("no-dependents-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let ver = fixtures::version(pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create version");

    let dependents = ver
        .dependents(&ctx.conn)
        .await
        .expect("Failed to get dependents");

    assert!(dependents.is_empty());
}

#[tokio::test]
async fn dependents_multiple() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    // Create base package
    let base_pkg = fixtures::package()
        .name("base-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create base pkg");

    let base_ver = fixtures::version(base_pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create base version");

    // Create packages that depend on base
    let dep_pkg1 = fixtures::package()
        .name("aaa-consumer")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create consumer1 pkg");

    let dep_pkg2 = fixtures::package()
        .name("bbb-consumer")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create consumer2 pkg");

    fixtures::version(dep_pkg1.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .dependencies(vec![base_ver.id])
        .insert(&ctx.conn)
        .await
        .expect("Failed to create consumer1");

    fixtures::version(dep_pkg2.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .dependencies(vec![base_ver.id])
        .insert(&ctx.conn)
        .await
        .expect("Failed to create consumer2");

    let dependents = base_ver
        .dependents(&ctx.conn)
        .await
        .expect("Failed to get dependents");

    assert_eq!(dependents.len(), 2);

    // Should be ordered by package name
    assert_eq!(dependents[0].package.name, "aaa-consumer");
    assert_eq!(dependents[1].package.name, "bbb-consumer");
}
