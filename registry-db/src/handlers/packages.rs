use crate::{
    Error, Result,
    models::{
        Entity, Page, Paginated,
        org::Org,
        package::Package,
        user::User,
        version::{QualifiedPackageVersion, Version},
    },
    schema::{org, package, users, version},
};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

#[derive(Debug, Clone, Copy)]
pub enum PackageOrderingField {
    Name,
    DownloadCount,
}

#[derive(Debug, Clone, Copy)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy)]
pub struct PackageOrdering {
    pub field: PackageOrderingField,
    pub direction: OrderDirection,
}

pub async fn list_packages(
    conn: &mut AsyncPgConnection,
    page: Page,
    ordering: PackageOrdering,
) -> Result<Paginated<Package>> {
    let mut query = package::table.into_boxed();

    query = match (ordering.field, ordering.direction) {
        (PackageOrderingField::Name, OrderDirection::Asc) => query.order(package::name.asc()),
        (PackageOrderingField::Name, OrderDirection::Desc) => query.order(package::name.desc()),
        (PackageOrderingField::DownloadCount, OrderDirection::Asc) => {
            query.order(package::id.asc())
        },
        (PackageOrderingField::DownloadCount, OrderDirection::Desc) => {
            query.order(package::id.desc())
        },
    };

    let total_items = package::table
        .count()
        .get_result::<i64>(conn)
        .await?;

    let items = query
        .limit(page.size)
        .offset((page.number - 1) * page.size)
        .load::<Package>(conn)
        .await?;

    let total_pages = (total_items + page.size - 1) / page.size;
    let next_page = if page.number < total_pages {
        Some(page.number + 1)
    } else {
        None
    };

    Ok(Paginated {
        items,
        page,
        next_page,
        total_items,
        total_pages,
    })
}

pub async fn search_packages(
    conn: &mut AsyncPgConnection,
    query_str: &str,
    page: Page,
    ordering: PackageOrdering,
) -> Result<Paginated<Package>> {
    let search_pattern = format!("%{}%", query_str);

    let mut query = package::table
        .filter(package::name.ilike(&search_pattern))
        .into_boxed();

    query = match (ordering.field, ordering.direction) {
        (PackageOrderingField::Name, OrderDirection::Asc) => query.order(package::name.asc()),
        (PackageOrderingField::Name, OrderDirection::Desc) => query.order(package::name.desc()),
        (PackageOrderingField::DownloadCount, OrderDirection::Asc) => {
            query.order(package::id.asc())
        },
        (PackageOrderingField::DownloadCount, OrderDirection::Desc) => {
            query.order(package::id.desc())
        },
    };

    let total_items = package::table
        .filter(package::name.ilike(&search_pattern))
        .count()
        .get_result::<i64>(conn)
        .await?;

    let items = query
        .limit(page.size)
        .offset((page.number - 1) * page.size)
        .load::<Package>(conn)
        .await?;

    let total_pages = (total_items + page.size - 1) / page.size;
    let next_page = if page.number < total_pages {
        Some(page.number + 1)
    } else {
        None
    };

    Ok(Paginated {
        items,
        page,
        next_page,
        total_items,
        total_pages,
    })
}

macro_rules! version_list_query {
    ($pkg:expr, $filter_user_id:expr, $filter_org_id:expr) => {{
        let mut query = Version::query()
            .filter(version::package.eq($pkg.id))
            .left_join(
                users::table.on(users::id
                    .nullable()
                    .eq(version::publishing_user_id)),
            )
            .left_join(
                org::table.on(org::id
                    .nullable()
                    .eq(version::publishing_org_id)),
            )
            .into_boxed();

        // Apply publisher filters
        if let Some(user_id) = $filter_user_id {
            query = query.filter(version::publishing_user_id.eq(user_id));
        }
        if let Some(org_id) = $filter_org_id {
            query = query.filter(version::publishing_org_id.eq(org_id));
        }

        query
    }};
}

pub async fn list_package_versions(
    conn: &mut AsyncPgConnection,
    package_name: &str,
    page: Page,
    filter_user_id: Option<i64>,
    filter_org_id: Option<i64>,
) -> Result<Paginated<QualifiedPackageVersion>> {
    let pkg = package::table
        .filter(package::name.eq(package_name))
        .first::<Package>(conn)
        .await
        .map_err(|e| {
            match e {
                diesel::result::Error::NotFound => {
                    Error::NotFound(format!("Package '{}' not found", package_name))
                },
                e => Error::Database(e),
            }
        })?;

    let count = version_list_query!(pkg, filter_user_id, filter_org_id).count();

    let items = version_list_query!(pkg, filter_user_id, filter_org_id)
        .order(version::created_at.desc())
        .limit(page.size)
        .offset((page.number - 1) * page.size)
        .select((
            Version::as_select(),
            Option::<User>::as_select(),
            Option::<Org>::as_select(),
        ));

    let (total_items, items): (i64, Vec<(Version, Option<User>, Option<Org>)>) = tokio::try_join!(
        count.get_result::<i64>(conn),
        items.load::<(Version, Option<User>, Option<Org>)>(conn)
    )?;

    let total_pages = (total_items + page.size - 1) / page.size;
    let next_page = if page.number < total_pages {
        Some(page.number + 1)
    } else {
        None
    };

    Ok(Paginated {
        items: items
            .into_iter()
            .map(|(ver, pub_user, pub_org)| {
                QualifiedPackageVersion {
                    package: pkg.clone(),
                    version: ver,
                    publisher: pub_user
                        .map(Entity::User)
                        .or(pub_org.map(Entity::Org))
                        .unwrap(),
                }
            })
            .collect(),
        page,
        next_page,
        total_items,
        total_pages,
    })
}

/// Get unique publishers of a package, ordered by their latest published version
pub async fn get_package_publishers(
    conn: &mut AsyncPgConnection,
    package_name: &str,
) -> Result<Vec<Entity>> {
    let pkg = package::table
        .filter(package::name.eq(package_name))
        .first::<Package>(conn)
        .await
        .map_err(|e| {
            match e {
                diesel::result::Error::NotFound => {
                    Error::NotFound(format!("Package '{}' not found", package_name))
                },
                e => Error::Database(e),
            }
        })?;

    let users = users::table
        .distinct()
        .filter(
            users::id.nullable().eq_any(
                version::table
                    .filter(version::package.eq(pkg.id))
                    .filter(version::publishing_user_id.is_not_null())
                    .select(version::publishing_user_id)
                    .order_by(version::created_at.desc()),
            ),
        )
        .select(User::as_select())
        .load::<User>(conn)
        .await?;

    let orgs = org::table
        .distinct()
        .filter(
            org::id.nullable().eq_any(
                version::table
                    .filter(version::package.eq(pkg.id))
                    .filter(version::publishing_org_id.is_not_null())
                    .select(version::publishing_org_id)
                    .order_by(version::created_at.desc()),
            ),
        )
        .select(Org::as_select())
        .load::<Org>(conn)
        .await?;

    let mut publishers = Vec::new();

    for user in users {
        publishers.push(Entity::User(user));
    }
    for org in orgs {
        publishers.push(Entity::Org(org));
    }

    Ok(publishers)
}
