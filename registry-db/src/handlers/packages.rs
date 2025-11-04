use crate::{
    Error, Result,
    models::{
        Entity, Page, Paginated, api_key::ApiKey, package::Package, scopes::Permission, user::User,
        version::Version,
    },
    schema::{package, schema_admin, users, version},
};
use chrono::Utc;
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

fn check_scope_match(
    api_key: &ApiKey,
    package_name: &str,
) -> bool {
    api_key.scopes.iter().any(|scope| {
        if scope.pattern.ends_with('*') {
            let prefix = scope.pattern.trim_end_matches('*');
            package_name.starts_with(prefix)
        } else {
            scope.pattern == package_name
        }
    })
}

pub async fn upload_package_version(
    conn: &mut AsyncPgConnection,
    api_key: &ApiKey,
    package_name: &str,
    version_str: &str,
    checksum: &str,
    description: Option<&str>,
    homepage: Option<&str>,
    license: &str,
    readme: &str,
    repository: &str,
    keywords: Vec<String>,
) -> Result<Version> {
    let scope_matches = check_scope_match(api_key, package_name);
    let has_permission = api_key
        .permissions
        .contains(&Permission::PublishPackage);

    if !scope_matches || !has_permission {
        return Err(Error::Unauthorized(format!(
            "Token does not have publish permission for package '{}'. Scope match: {}, Has permission: {}",
            package_name, scope_matches, has_permission
        )));
    }

    let publishing_user_id = api_key
        .user_id
        .ok_or_else(|| Error::Unauthorized("Only users can publish packages".into()))?;

    let pkg = package::table
        .filter(package::name.eq(package_name))
        .first::<Package>(conn)
        .await
        .optional()?;

    let package_id = if let Some(pkg) = pkg {
        let is_admin = schema_admin::table
            .filter(schema_admin::package.eq(pkg.id))
            .filter(
                schema_admin::user_id
                    .eq(publishing_user_id)
                    .or(schema_admin::org_id.eq(api_key.org_id)),
            )
            .filter(schema_admin::revoked_at.is_null())
            .count()
            .get_result::<i64>(conn)
            .await?
            > 0;

        if !is_admin {
            return Err(Error::Unauthorized("Not package owner".into()));
        }

        pkg.id
    } else {
        let new_pkg = diesel::insert_into(package::table)
            .values(package::name.eq(package_name))
            .get_result::<Package>(conn)
            .await?;

        diesel::insert_into(schema_admin::table)
            .values((
                schema_admin::package.eq(new_pkg.id),
                schema_admin::user_id.eq(Some(publishing_user_id)),
                schema_admin::org_id.eq(api_key.org_id),
            ))
            .execute(conn)
            .await?;

        new_pkg.id
    };

    let new_version = diesel::insert_into(version::table)
        .values((
            version::package.eq(package_id),
            version::qualified_version.eq(version_str),
            version::checksum.eq(checksum),
            version::description.eq(description),
            version::homepage.eq(homepage),
            version::license.eq(license),
            version::readme.eq(readme),
            version::repository.eq(repository),
            version::keywords.eq(keywords),
            version::publishing_user_id.eq(Some(publishing_user_id)),
            version::publishing_org_id.eq(api_key.org_id),
        ))
        .get_result::<Version>(conn)
        .await?;

    Ok(new_version)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VersionWithPublisher {
    #[serde(flatten)]
    pub version: Version,
    pub publisher: Entity,
}

pub async fn get_package_version(
    conn: &mut AsyncPgConnection,
    package_name: &str,
    version_str: &str,
) -> Result<VersionWithPublisher> {
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

    let ver = if version_str == "latest" {
        let mut all_versions = Version::query()
            .filter(version::package.eq(pkg.id))
            .filter(version::yanked_at.is_null())
            .load(conn)
            .await?;

        if all_versions.is_empty() {
            return Err(Error::NotFound("No versions found for package".into()));
        }

        // descending sort
        all_versions.sort_by(|a, b| b.qualified_version.cmp(&a.qualified_version));

        let latest_stable = all_versions
            .iter()
            .find(|v| v.qualified_version.is_stable());

        let selected = latest_stable
            .or_else(|| Some(&all_versions[0]))
            .ok_or_else(|| Error::NotFound("No valid versions found".into()))?;

        selected.clone()
    } else {
        Version::query()
            .filter(version::package.eq(pkg.id))
            .filter(version::qualified_version.eq(version_str))
            .first(conn)
            .await
            .map_err(|e| {
                match e {
                    diesel::result::Error::NotFound => {
                        Error::NotFound(format!("Version '{}' not found", version_str))
                    },
                    e => Error::Database(e),
                }
            })?
    };

    let publisher = if let Some(user_id) = ver.publishing_user_id {
        let user = users::table
            .filter(users::id.eq(user_id))
            .first::<User>(conn)
            .await?;
        Entity::User(user)
    } else if let Some(org_id) = ver.publishing_org_id {
        let org = crate::schema::org::table
            .filter(crate::schema::org::id.eq(org_id))
            .first::<crate::models::org::Org>(conn)
            .await?;
        Entity::Org(org)
    } else {
        return Err(Error::Database(diesel::result::Error::NotFound));
    };

    Ok(VersionWithPublisher {
        version: ver,
        publisher,
    })
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

pub async fn yank_version(
    conn: &mut AsyncPgConnection,
    api_key: &ApiKey,
    package_name: &str,
    version_str: &str,
) -> Result<()> {
    let scope_matches = check_scope_match(api_key, package_name);
    let has_permission = api_key
        .permissions
        .contains(&Permission::YankPackage);

    if !scope_matches || !has_permission {
        return Err(Error::Unauthorized(format!(
            "Token does not have yank permission for package '{}'",
            package_name
        )));
    }

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

    let updated = diesel::update(version::table)
        .filter(version::package.eq(pkg.id))
        .filter(version::qualified_version.eq(version_str))
        .set(version::yanked_at.eq(Some(Utc::now())))
        .execute(conn)
        .await?;

    if updated == 0 {
        return Err(Error::NotFound(format!(
            "Version '{}' not found",
            version_str
        )));
    }

    Ok(())
}

pub async fn list_package_versions(
    conn: &mut AsyncPgConnection,
    package_name: &str,
    page: Page,
) -> Result<Paginated<Version>> {
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

    let total_items = version::table
        .filter(version::package.eq(pkg.id))
        .count()
        .get_result::<i64>(conn)
        .await?;

    let items = Version::query()
        .filter(version::package.eq(pkg.id))
        .order(version::created_at.desc())
        .limit(page.size)
        .offset((page.number - 1) * page.size)
        .load(conn)
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
