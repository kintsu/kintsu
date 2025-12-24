//! Favourites Engine Tests
//!
//! Tests for registry-db/src/engine/favourites.rs
//! Covers listing, creating, and deleting user favourites.

mod common;

use common::fixtures;
use kintsu_registry_db::{
    Error,
    engine::{
        Page,
        favourites::{
            FavouriteEntity, FavouriteTarget, create_favourite, delete_favourite, list_favourites,
        },
    },
    entities::*,
    tst::TestDbCtx,
};
use sea_orm::EntityTrait;

#[tokio::test]
async fn list_favourites_empty() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let page = Page {
        number: 1,
        size: 20,
    };
    let result = list_favourites(&ctx.conn, user.id, page)
        .await
        .expect("Failed to list favourites");

    assert!(result.items.is_empty());
    assert_eq!(result.total_items, 0);
    assert_eq!(result.total_pages, 0);
}

#[tokio::test]
async fn list_favourites_packages() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg1 = fixtures::package()
        .name("fav-pkg-1")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package 1");

    let pkg2 = fixtures::package()
        .name("fav-pkg-2")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package 2");

    // Favourite both packages
    create_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg1.id))
        .await
        .expect("Failed to favourite package 1");

    create_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg2.id))
        .await
        .expect("Failed to favourite package 2");

    let page = Page {
        number: 1,
        size: 20,
    };
    let result = list_favourites(&ctx.conn, user.id, page)
        .await
        .expect("Failed to list favourites");

    assert_eq!(result.items.len(), 2);
    assert_eq!(result.total_items, 2);
    assert_eq!(result.total_pages, 1);

    for item in &result.items {
        assert!(matches!(&item.entity, FavouriteEntity::Package(_)));
    }
}

#[tokio::test]
async fn list_favourites_orgs() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org1 = fixtures::org()
        .name("fav-org-1")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org 1");

    let org2 = fixtures::org()
        .name("fav-org-2")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org 2");

    create_favourite(&ctx.conn, user.id, FavouriteTarget::Org(org1.id))
        .await
        .expect("Failed to favourite org 1");

    create_favourite(&ctx.conn, user.id, FavouriteTarget::Org(org2.id))
        .await
        .expect("Failed to favourite org 2");

    let page = Page {
        number: 1,
        size: 20,
    };
    let result = list_favourites(&ctx.conn, user.id, page)
        .await
        .expect("Failed to list favourites");

    assert_eq!(result.items.len(), 2);

    for item in &result.items {
        assert!(matches!(&item.entity, FavouriteEntity::Org(_)));
    }
}

#[tokio::test]
async fn list_favourites_mixed() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("mixed-fav-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let org = fixtures::org()
        .name("mixed-fav-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    create_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg.id))
        .await
        .expect("Failed to favourite package");

    create_favourite(&ctx.conn, user.id, FavouriteTarget::Org(org.id))
        .await
        .expect("Failed to favourite org");

    let page = Page {
        number: 1,
        size: 20,
    };
    let result = list_favourites(&ctx.conn, user.id, page)
        .await
        .expect("Failed to list favourites");

    assert_eq!(result.items.len(), 2);
    assert_eq!(result.total_items, 2);

    let has_package = result
        .items
        .iter()
        .any(|i| matches!(&i.entity, FavouriteEntity::Package(_)));
    let has_org = result
        .items
        .iter()
        .any(|i| matches!(&i.entity, FavouriteEntity::Org(_)));

    assert!(has_package);
    assert!(has_org);
}

#[tokio::test]
async fn list_favourites_pagination_first_page() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    // Create 15 packages and favourite them
    for i in 0..15 {
        let pkg = fixtures::package()
            .name(&format!("page-pkg-{}", i))
            .insert(&ctx.conn)
            .await
            .expect("Failed to create package");

        create_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg.id))
            .await
            .expect("Failed to favourite package");
    }

    let page = Page {
        number: 1,
        size: 10,
    };
    let result = list_favourites(&ctx.conn, user.id, page)
        .await
        .expect("Failed to list favourites");

    assert_eq!(result.items.len(), 10);
    assert_eq!(result.total_items, 15);
    assert_eq!(result.total_pages, 2);
    assert_eq!(result.next_page, Some(2));
}

#[tokio::test]
async fn list_favourites_pagination_last_page() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    // Create 15 packages and favourite them
    for i in 0..15 {
        let pkg = fixtures::package()
            .name(&format!("last-pkg-{}", i))
            .insert(&ctx.conn)
            .await
            .expect("Failed to create package");

        create_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg.id))
            .await
            .expect("Failed to favourite package");
    }

    let page = Page {
        number: 2,
        size: 10,
    };
    let result = list_favourites(&ctx.conn, user.id, page)
        .await
        .expect("Failed to list favourites");

    assert_eq!(result.items.len(), 5);
    assert_eq!(result.total_items, 15);
    assert!(result.next_page.is_none());
}

#[tokio::test]
async fn list_favourites_empty_page() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    // Create only 5 favourites
    for i in 0..5 {
        let pkg = fixtures::package()
            .name(&format!("empty-page-pkg-{}", i))
            .insert(&ctx.conn)
            .await
            .expect("Failed to create package");

        create_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg.id))
            .await
            .expect("Failed to favourite package");
    }

    // Request page 10 (doesn't exist)
    let page = Page {
        number: 10,
        size: 10,
    };
    let result = list_favourites(&ctx.conn, user.id, page)
        .await
        .expect("Failed to list favourites");

    assert!(result.items.is_empty());
    assert_eq!(result.total_items, 5);
    assert_eq!(result.total_pages, 1);
}

#[tokio::test]
async fn create_favourite_package_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("create-fav-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let favourite = create_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg.id))
        .await
        .expect("Failed to create favourite");

    assert_eq!(favourite.user_id, user.id);
    assert_eq!(favourite.package_id, Some(pkg.id));
    assert_eq!(favourite.org_id, None);

    // Verify in DB
    let from_db = UserFavouriteEntity::find_by_id(favourite.id)
        .one(&ctx.conn)
        .await
        .expect("DB error")
        .expect("Favourite not found");

    assert_eq!(from_db.package_id, Some(pkg.id));
}

#[tokio::test]
async fn create_favourite_org_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .name("create-fav-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    let favourite = create_favourite(&ctx.conn, user.id, FavouriteTarget::Org(org.id))
        .await
        .expect("Failed to create favourite");

    assert_eq!(favourite.user_id, user.id);
    assert_eq!(favourite.org_id, Some(org.id));
    assert_eq!(favourite.package_id, None);
}

#[tokio::test]
async fn create_favourite_package_not_found() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let result = create_favourite(&ctx.conn, user.id, FavouriteTarget::Package(99999)).await;

    assert!(matches!(result, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn create_favourite_org_not_found() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let result = create_favourite(&ctx.conn, user.id, FavouriteTarget::Org(99999)).await;

    assert!(matches!(result, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn create_favourite_duplicate() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("dup-fav-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // First favourite - should succeed
    create_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg.id))
        .await
        .expect("Failed to create first favourite");

    // Second favourite - should fail (duplicate)
    let result = create_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg.id)).await;

    // Should get a DB constraint error
    assert!(result.is_err());
}

#[tokio::test]
async fn delete_favourite_package_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("del-fav-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    let favourite = create_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg.id))
        .await
        .expect("Failed to create favourite");

    // Delete the favourite
    delete_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg.id))
        .await
        .expect("Failed to delete favourite");

    // Verify removed from DB
    let from_db = UserFavouriteEntity::find_by_id(favourite.id)
        .one(&ctx.conn)
        .await
        .expect("DB error");

    assert!(from_db.is_none());
}

#[tokio::test]
async fn delete_favourite_org_success() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let org = fixtures::org()
        .name("del-fav-org")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create org");

    create_favourite(&ctx.conn, user.id, FavouriteTarget::Org(org.id))
        .await
        .expect("Failed to create favourite");

    delete_favourite(&ctx.conn, user.id, FavouriteTarget::Org(org.id))
        .await
        .expect("Failed to delete favourite");

    // Verify by listing - should be empty
    let page = Page {
        number: 1,
        size: 10,
    };
    let result = list_favourites(&ctx.conn, user.id, page)
        .await
        .expect("Failed to list favourites");

    assert!(result.items.is_empty());
}

#[tokio::test]
async fn delete_favourite_not_found() {
    let ctx = TestDbCtx::new().await;

    let user = fixtures::user()
        .insert(&ctx.conn)
        .await
        .expect("Failed to create user");

    let pkg = fixtures::package()
        .name("not-fav-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // Try to delete a favourite that doesn't exist
    let result = delete_favourite(&ctx.conn, user.id, FavouriteTarget::Package(pkg.id)).await;

    assert!(matches!(result, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn delete_favourite_wrong_user() {
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

    let pkg = fixtures::package()
        .name("wrong-user-pkg")
        .insert(&ctx.conn)
        .await
        .expect("Failed to create package");

    // user1 favourites the package
    create_favourite(&ctx.conn, user1.id, FavouriteTarget::Package(pkg.id))
        .await
        .expect("Failed to create favourite");

    // user2 tries to delete user1's favourite
    let result = delete_favourite(&ctx.conn, user2.id, FavouriteTarget::Package(pkg.id)).await;

    // Should get NotFound since user2 doesn't have this favourite
    assert!(matches!(result, Err(Error::NotFound(_))));

    // Verify user1's favourite still exists
    let page = Page {
        number: 1,
        size: 10,
    };
    let user1_favs = list_favourites(&ctx.conn, user1.id, page)
        .await
        .expect("Failed to list favourites");

    assert_eq!(user1_favs.items.len(), 1);
}
