mod common;

use common::fixtures;
use kintsu_registry_db::{
    engine::{
        Entity as EngineEntity, OrderDirection, PackageOrdering, PackageOrderingField, Page,
        Paginated, PrincipalIdentity,
        package::{DownloadHistory, StagePublishPackage},
    },
    entities::*,
    tst::TestDbCtx,
};

#[tokio::test]
async fn lookup_by_id_found() {
    let ctx = TestDbCtx::new().await;

    let pkg = fixtures::package()
        .name("lookup-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let found = Package::by_id(&ctx.conn, pkg.id)
        .await
        .expect("Lookup failed");

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, pkg.id);
    assert_eq!(found.name, "lookup-pkg");
}

#[tokio::test]
async fn lookup_by_id_not_found() {
    let ctx = TestDbCtx::new().await;

    let found = Package::by_id(&ctx.conn, 999999)
        .await
        .expect("Lookup failed");

    assert!(found.is_none());
}

#[tokio::test]
async fn lookup_by_name_found() {
    let ctx = TestDbCtx::new().await;

    fixtures::package()
        .name("my-unique-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let found = Package::by_name(&ctx.conn, "my-unique-pkg")
        .await
        .expect("Lookup failed");

    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "my-unique-pkg");
}

#[tokio::test]
async fn lookup_by_name_not_found() {
    let ctx = TestDbCtx::new().await;

    let found = Package::by_name(&ctx.conn, "nonexistent-pkg")
        .await
        .expect("Lookup failed");

    assert!(found.is_none());
}

#[tokio::test]
async fn list_packages_empty() {
    let ctx = TestDbCtx::new().await;

    let page = Page {
        number: 1,
        size: 10,
    };
    let ordering = PackageOrdering {
        field: PackageOrderingField::Name,
        direction: OrderDirection::Asc,
    };

    let result = Package::list_packages(&ctx.conn, page, ordering)
        .await
        .expect("List failed");

    assert!(result.items.is_empty());
    assert_eq!(result.total_items, 0);
    assert!(result.next_page.is_none());
}

#[tokio::test]
async fn list_packages_paginated() {
    let ctx = TestDbCtx::new().await;

    // Create 25 packages
    for i in 0..25 {
        fixtures::package()
            .name(&format!("pkg-{:02}", i))
            .insert(&ctx.conn)
            .await
            .expect("Failed to create package");
    }

    let ordering = PackageOrdering {
        field: PackageOrderingField::Name,
        direction: OrderDirection::Asc,
    };

    // Page 1, size 10
    let page1 = Page {
        number: 1,
        size: 10,
    };
    let result1 = Package::list_packages(&ctx.conn, page1, ordering.clone())
        .await
        .expect("List failed");

    assert_eq!(result1.items.len(), 10);
    assert_eq!(result1.next_page, Some(2));
    assert_eq!(result1.total_items, 25);
    assert_eq!(result1.total_pages, 3);

    // Page 3, size 10
    let page3 = Page {
        number: 3,
        size: 10,
    };
    let result3 = Package::list_packages(&ctx.conn, page3, ordering)
        .await
        .expect("List failed");

    assert_eq!(result3.items.len(), 5);
    assert!(result3.next_page.is_none());
}

#[tokio::test]
async fn list_packages_order_by_name_asc() {
    let ctx = TestDbCtx::new().await;

    // Create packages in random order
    for name in ["zebra", "alpha", "beta"] {
        fixtures::package()
            .name(name)
            .insert(&ctx.conn)
            .await
            .expect("Failed to create package");
    }

    let page = Page {
        number: 1,
        size: 10,
    };
    let ordering = PackageOrdering {
        field: PackageOrderingField::Name,
        direction: OrderDirection::Asc,
    };

    let result = Package::list_packages(&ctx.conn, page, ordering)
        .await
        .expect("List failed");

    let names: Vec<&str> = result
        .items
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(names, vec!["alpha", "beta", "zebra"]);
}

#[tokio::test]
async fn list_packages_order_by_name_desc() {
    let ctx = TestDbCtx::new().await;

    for name in ["zebra", "alpha", "beta"] {
        fixtures::package()
            .name(name)
            .insert(&ctx.conn)
            .await
            .expect("Failed to create package");
    }

    let page = Page {
        number: 1,
        size: 10,
    };
    let ordering = PackageOrdering {
        field: PackageOrderingField::Name,
        direction: OrderDirection::Desc,
    };

    let result = Package::list_packages(&ctx.conn, page, ordering)
        .await
        .expect("List failed");

    let names: Vec<&str> = result
        .items
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(names, vec!["zebra", "beta", "alpha"]);
}

#[tokio::test]
async fn search_packages_match() {
    let ctx = TestDbCtx::new().await;

    for name in ["my-pkg", "my-lib", "other-pkg"] {
        fixtures::package()
            .name(name)
            .insert(&ctx.conn)
            .await
            .expect("Failed to create package");
    }

    let page = Page {
        number: 1,
        size: 10,
    };
    let ordering = PackageOrdering {
        field: PackageOrderingField::Name,
        direction: OrderDirection::Asc,
    };

    let result = Package::search_packages(&ctx.conn, "my", page, ordering)
        .await
        .expect("Search failed");

    assert_eq!(result.items.len(), 2);
    let names: Vec<&str> = result
        .items
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert!(names.contains(&"my-pkg"));
    assert!(names.contains(&"my-lib"));
}

#[tokio::test]
async fn search_packages_no_match() {
    let ctx = TestDbCtx::new().await;

    fixtures::package()
        .name("some-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let page = Page {
        number: 1,
        size: 10,
    };
    let ordering = PackageOrdering {
        field: PackageOrderingField::Name,
        direction: OrderDirection::Asc,
    };

    let result = Package::search_packages(&ctx.conn, "nonexistent", page, ordering)
        .await
        .expect("Search failed");

    assert!(result.items.is_empty());
}

#[tokio::test]
async fn user_admins_single() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // Grant admin role to user
    fixtures::schema_role(pkg.id)
        .user(user.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin");

    let admins = Package::user_admins(&ctx.conn, pkg.id)
        .await
        .expect("Failed to get admins");

    assert_eq!(admins.len(), 1);
    assert!(admins.contains(&user.id));
}

#[tokio::test]
async fn user_admins_multiple() {
    let ctx = TestDbCtx::new().await;

    let user1 = fixtures::user()
        .gh_login("admin1")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user1");

    let user2 = fixtures::user()
        .gh_login("admin2")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user2");

    let pkg = fixtures::package()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::schema_role(pkg.id)
        .user(user1.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin to user1");

    fixtures::schema_role(pkg.id)
        .user(user2.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin to user2");

    let admins = Package::user_admins(&ctx.conn, pkg.id)
        .await
        .expect("Failed to get admins");

    assert_eq!(admins.len(), 2);
    assert!(admins.contains(&user1.id));
    assert!(admins.contains(&user2.id));
}

#[tokio::test]
async fn org_admins_multiple() {
    let ctx = TestDbCtx::new().await;

    let org1 = fixtures::org()
        .name("org1")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org1");

    let org2 = fixtures::org()
        .name("org2")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org2");

    let pkg = fixtures::package()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::schema_role(pkg.id)
        .org(org1.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin to org1");

    fixtures::schema_role(pkg.id)
        .org(org2.id)
        .admin()
        .insert(&ctx.conn)
        .await
        .expect("Failed to grant admin to org2");

    let admins = Package::org_admins(&ctx.conn, pkg.id)
        .await
        .expect("Failed to get admins");

    assert_eq!(admins.len(), 2);
    assert!(admins.contains(&org1.id));
    assert!(admins.contains(&org2.id));
}

#[tokio::test]
async fn admins_exclude_revoked() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // Grant and revoke
    fixtures::schema_role(pkg.id)
        .user(user.id)
        .admin()
        .revoked()
        .insert(&ctx.conn)
        .await
        .expect("Failed to insert revoked role");

    let admins = Package::user_admins(&ctx.conn, pkg.id)
        .await
        .expect("Failed to get admins");

    assert!(admins.is_empty());
}

#[tokio::test]
async fn download_count_zero() {
    let ctx = TestDbCtx::new().await;

    fixtures::package()
        .name("no-downloads-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let count = Package::get_package_download_count(&ctx.conn, "no-downloads-pkg")
        .await
        .expect("Failed to get count");

    assert_eq!(count, 0);
}

#[tokio::test]
async fn download_count_aggregates() {
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

    let v1 = fixtures::version(pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create v1");

    let v2 = fixtures::version(pkg.id)
        .version("2.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create v2");

    // Add downloads
    fixtures::downloads(v1.id, 10)
        .insert(&ctx.conn)
        .await
        .expect("Failed to add v1 downloads");

    fixtures::downloads(v2.id, 25)
        .insert(&ctx.conn)
        .await
        .expect("Failed to add v2 downloads");

    let count = Package::get_package_download_count(&ctx.conn, "download-pkg")
        .await
        .expect("Failed to get count");

    assert_eq!(count, 35);
}

#[tokio::test]
async fn publishers_user_only() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("user-published-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    fixtures::version(pkg.id)
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create version");

    let publishers = Package::get_package_publishers(&ctx.conn, "user-published-pkg")
        .await
        .expect("Failed to get publishers");

    assert_eq!(publishers.len(), 1);
    match &publishers[0] {
        EngineEntity::User(u) => assert_eq!(u.id, user.id),
        _ => panic!("Expected User publisher"),
    }
}

#[tokio::test]
async fn publishers_user_and_org() {
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
        .name("mixed-publishers-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // User publishes v1
    fixtures::version(pkg.id)
        .version("1.0.0")
        .publisher_user(user.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create v1");

    // Org publishes v2
    fixtures::version(pkg.id)
        .version("1.0.1")
        .publisher_org(org.id)
        .insert(&ctx.conn)
        .await
        .expect("Failed to create v2");

    let publishers = Package::get_package_publishers(&ctx.conn, "mixed-publishers-pkg")
        .await
        .expect("Failed to get publishers");

    assert_eq!(publishers.len(), 2);

    let has_user = publishers
        .iter()
        .any(|p| matches!(p, EngineEntity::User(u) if u.id == user.id));
    let has_org = publishers
        .iter()
        .any(|p| matches!(p, EngineEntity::Org(o) if o.id == org.id));

    assert!(has_user, "Expected user publisher");
    assert!(has_org, "Expected org publisher");
}
