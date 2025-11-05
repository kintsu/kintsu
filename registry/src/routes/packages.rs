use crate::DbPool;
use actix_web::{
    Responder, get,
    web::{self, Redirect},
};

const PACKAGES: &str = "packages";

/// Get package version metadata
#[utoipa::path(
    tag = PACKAGES,
    params(
        ("name" = String, Path, description = "Package name"),
        ("version" = String, Path, description = "Version string or 'latest'"),
    ),
    responses(
        (status = 200, description = "Package version metadata", body = kintsu_registry_db::models::version::QualifiedPackageVersion),
        (status = 404, description = "Package or version not found", body = crate::ErrorResponse),
    )
)]
#[get("/package/{name}/{version}")]
pub async fn get_package_version(
    path: web::Path<(String, String)>,
    pool: DbPool,
) -> crate::Result<impl Responder> {
    let (name, version) = path.into_inner();
    let mut conn = pool.get().await?;

    let qualified = kintsu_registry_db::models::version::Version::get_package_version(
        &mut conn, &name, &version,
    )
    .await?;

    Ok(web::Json(qualified))
}

/// Download a package version
#[utoipa::path(
    tag = PACKAGES,
    params(
        ("name" = String, Path, description = "Package name"),
        ("version" = String, Path, description = "Version string or 'latest'"),
    ),
    responses(
        (status = 302, description = "Redirect to package download URL"),
        (status = 404, description = "Package or version not found", body = crate::ErrorResponse),
    )
)]
#[get("/package/{name}/{version}/download")]
pub async fn download_package_version(
    path: web::Path<(String, String)>,
    pool: DbPool,
) -> crate::Result<impl Responder> {
    let (name, version) = path.into_inner();
    let mut conn = pool.get().await?;

    let qualified = kintsu_registry_db::models::version::Version::get_package_version(
        &mut conn, &name, &version,
    )
    .await?;

    // Increment download count asynchronously (fire and forget)
    let version_id = qualified.version.id;
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        if let Ok(mut conn) = pool_clone.get().await {
            let _ = kintsu_registry_db::models::version::Version::increment_download_count(
                &mut conn, version_id,
            )
            .await;
        }
    });

    // TODO: Generate signed CDN URL from storage backend
    // For now, construct a simple URL based on package metadata
    let download_url = format!(
        "https://cdn.kintsu.dev/packages/{}/{}/{}.tar.gz",
        name, qualified.version.qualified_version, qualified.version.checksum
    );

    Ok(Redirect::to(download_url))
}

/// Get total download count for a package
#[utoipa::path(
    tag = PACKAGES,
    params(
        ("name" = String, Path, description = "Package name"),
    ),
    responses(
        (status = 200, description = "Total download count", body = crate::models::DownloadStats),
        (status = 404, description = "Package not found", body = crate::ErrorResponse),
    )
)]
#[get("/package-analytics/{name}/downloads/all")]
pub async fn get_package_total_downloads(
    name: web::Path<String>,
    pool: DbPool,
) -> crate::Result<impl Responder> {
    let mut conn = pool.get().await?;

    let total =
        kintsu_registry_db::models::package::Package::get_package_download_count(&mut conn, &name)
            .await?;

    Ok(web::Json(crate::models::DownloadStats {
        package: name.into_inner(),
        total_downloads: total,
    }))
}

/// Get 90-day download history for a package
#[utoipa::path(
    tag = PACKAGES,
    params(
        ("name" = String, Path, description = "Package name"),
    ),
    responses(
        (status = 200, description = "90-day download history", body = Vec<kintsu_registry_db::models::package::DownloadHistory>),
        (status = 404, description = "Package not found", body = crate::ErrorResponse),
    )
)]
#[get("/package-analytics/{name}/downloads/history")]
pub async fn get_package_download_history(
    name: web::Path<String>,
    pool: DbPool,
) -> crate::Result<impl Responder> {
    let mut conn = pool.get().await?;

    let history =
        kintsu_registry_db::models::package::Package::package_download_history(&mut conn, &name)
            .await?;

    Ok(web::Json(history))
}

/// List packages with pagination and ordering
#[utoipa::path(
    tag = PACKAGES,
    params(
        ("page" = Option<i64>, Query, description = "Page number (default: 1)"),
        ("size" = Option<i64>, Query, description = "Page size (default: 20)"),
        ("order_by" = Option<String>, Query, description = "Order by field: name or downloads (default: name)"),
        ("order_dir" = Option<String>, Query, description = "Order direction: asc or desc (default: asc)"),
    ),
    responses(
        (status = 200, description = "Paginated list of packages", body = kintsu_registry_db::models::Paginated<kintsu_registry_db::models::package::Package>),
        (status = 400, description = "Invalid query parameters", body = crate::ErrorResponse),
    )
)]
#[get("/packages")]
pub async fn list_packages(
    query: web::Query<ListPackagesQuery>,
    pool: DbPool,
) -> crate::Result<impl Responder> {
    use kintsu_registry_db::{
        handlers::packages::{OrderDirection, PackageOrdering, PackageOrderingField},
        models::Page,
    };

    let mut conn = pool.get().await?;

    let page = Page {
        number: query.page.unwrap_or(1),
        size: query.size.unwrap_or(20),
    };

    let ordering = PackageOrdering {
        field: match query.order_by.as_deref() {
            Some("downloads") => PackageOrderingField::DownloadCount,
            _ => PackageOrderingField::Name,
        },
        direction: match query.order_dir.as_deref() {
            Some("desc") => OrderDirection::Desc,
            _ => OrderDirection::Asc,
        },
    };

    let paginated =
        kintsu_registry_db::handlers::packages::list_packages(&mut conn, page, ordering).await?;

    Ok(web::Json(paginated))
}

/// Search packages by name
#[utoipa::path(
    tag = PACKAGES,
    params(
        ("q" = String, Query, description = "Search query"),
        ("page" = Option<i64>, Query, description = "Page number (default: 1)"),
        ("size" = Option<i64>, Query, description = "Page size (default: 20)"),
        ("order_by" = Option<String>, Query, description = "Order by field: name or downloads (default: name)"),
        ("order_dir" = Option<String>, Query, description = "Order direction: asc or desc (default: asc)"),
    ),
    responses(
        (status = 200, description = "Paginated search results", body = kintsu_registry_db::models::Paginated<kintsu_registry_db::models::package::Package>),
        (status = 400, description = "Invalid query parameters", body = crate::ErrorResponse),
    )
)]
#[get("/packages/search")]
pub async fn search_packages(
    query: web::Query<SearchPackagesQuery>,
    pool: DbPool,
) -> crate::Result<impl Responder> {
    use kintsu_registry_db::{
        handlers::packages::{OrderDirection, PackageOrdering, PackageOrderingField},
        models::Page,
    };

    let mut conn = pool.get().await?;

    let page = Page {
        number: query.page.unwrap_or(1),
        size: query.size.unwrap_or(20),
    };

    let ordering = PackageOrdering {
        field: match query.order_by.as_deref() {
            Some("downloads") => PackageOrderingField::DownloadCount,
            _ => PackageOrderingField::Name,
        },
        direction: match query.order_dir.as_deref() {
            Some("desc") => OrderDirection::Desc,
            _ => OrderDirection::Asc,
        },
    };

    let paginated = kintsu_registry_db::handlers::packages::search_packages(
        &mut conn, &query.q, page, ordering,
    )
    .await?;

    Ok(web::Json(paginated))
}

#[derive(serde::Deserialize)]
pub struct ListPackagesQuery {
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub order_by: Option<String>,
    pub order_dir: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct SearchPackagesQuery {
    pub q: String,
    #[serde(flatten)]
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub order_by: Option<String>,
    pub order_dir: Option<String>,
}

/// List versions of a package with optional filtering by publisher
#[utoipa::path(
    tag = PACKAGES,
    params(
        ("name" = String, Path, description = "Package name"),
        ("page" = Option<i64>, Query, description = "Page number (default: 1)"),
        ("size" = Option<i64>, Query, description = "Page size (default: 20)"),
        ("user_id" = Option<i64>, Query, description = "Filter by user publisher ID"),
        ("org_id" = Option<i64>, Query, description = "Filter by organization publisher ID"),
    ),
    responses(
        (status = 200, description = "Paginated list of versions", body = kintsu_registry_db::models::Paginated<kintsu_registry_db::models::version::QualifiedPackageVersion>),
        (status = 404, description = "Package not found", body = crate::ErrorResponse),
        (status = 400, description = "Invalid query parameters", body = crate::ErrorResponse),
    )
)]
#[get("/packages/{name}/versions")]
pub async fn list_package_versions(
    name: web::Path<String>,
    query: web::Query<ListVersionsQuery>,
    pool: DbPool,
) -> crate::Result<impl Responder> {
    use kintsu_registry_db::models::Page;

    let mut conn = pool.get().await?;

    let page = Page {
        number: query.page.unwrap_or(1),
        size: query.size.unwrap_or(20),
    };

    let paginated = kintsu_registry_db::handlers::packages::list_package_versions(
        &mut conn,
        &name,
        page,
        query.user_id,
        query.org_id,
    )
    .await?;

    Ok(web::Json(paginated))
}

/// Get unique publishers of a package
#[utoipa::path(
    tag = PACKAGES,
    params(
        ("name" = String, Path, description = "Package name"),
    ),
    responses(
        (status = 200, description = "List of unique publishers ordered by latest published version", body = Vec<kintsu_registry_db::models::Entity>),
        (status = 404, description = "Package not found", body = crate::ErrorResponse),
    )
)]
#[get("/packages/{name}/publishers")]
pub async fn get_package_publishers(
    name: web::Path<String>,
    pool: DbPool,
) -> crate::Result<impl Responder> {
    let mut conn = pool.get().await?;

    let publishers =
        kintsu_registry_db::handlers::packages::get_package_publishers(&mut conn, &name).await?;

    Ok(web::Json(publishers))
}

#[derive(serde::Deserialize)]
pub struct ListVersionsQuery {
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub user_id: Option<i64>,
    pub org_id: Option<i64>,
}
