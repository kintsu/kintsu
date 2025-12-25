use crate::{DbConn, principal::Principal};
use actix_web::{
    Responder, delete, get, post,
    web::{self},
};
use kintsu_registry_core::models::{GrantSchemaRoleRequest, RevokeSchemaRoleRequest};
use kintsu_registry_db::engine::{OrderDirection, PackageOrdering, PackageOrderingField, Page};
use validator::Validate;

const PACKAGES: &str = "packages";

/// Get package version metadata
#[utoipa::path(
    tag = PACKAGES,
    params(
        ("name" = String, Path, description = "Package name"),
        ("version" = String, Path, description = "Version string or 'latest'"),
    ),
    responses(
        (status = 200, description = "Package version metadata", body = kintsu_registry_db::engine::version::QualifiedPackageVersion),
        (status = 404, description = "Package or version not found", body = crate::ErrorResponse),
    )
)]
#[get("/package/{name}/{version}")]
pub async fn get_package_version(
    path: web::Path<(String, String)>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let (name, version) = path.into_inner();

    let qualified =
        kintsu_registry_db::entities::Version::get_package_version(conn.as_ref(), &name, &version)
            .await?;

    Ok(web::Json(qualified))
}

#[utoipa::path(
    tag = PACKAGES,
    params(
        ("name" = String, Path, description = "Package name"),
        ("version" = String, Path, description = "Version string or 'latest'"),
    ),
    responses(
        (status = 200, description = "List of package dependents", body = Vec<kintsu_registry_db::engine::version::QualifiedPackageVersion>),
        (status = 404, description = "Package or version not found", body = crate::ErrorResponse),
    )
)]
#[get("/package/{name}/{version}/dependents")]
pub async fn get_dependent_packages(
    path: web::Path<(String, kintsu_manifests::version::VersionSerde)>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let (name, version) = path.into_inner();

    let found = kintsu_registry_db::entities::Version::by_name_and_version(
        conn.as_ref(),
        &name,
        &version.to_string(),
    )
    .await?;
    let dependents = found.dependents(conn.as_ref()).await?;

    Ok(web::Json(dependents))
}

#[utoipa::path(
    tag = PACKAGES,
    params(
        ("name" = String, Path, description = "Package name"),
        ("version" = String, Path, description = "Version string or 'latest'"),
    ),
    responses(
        (status = 200, description = "List of package dependencies", body = Vec<kintsu_registry_db::engine::version::QualifiedPackageVersion>),
        (status = 404, description = "Package or version not found", body = crate::ErrorResponse),
    )
)]
#[get("/package/{name}/{version}/dependencies")]
pub async fn get_package_dependencies(
    path: web::Path<(String, kintsu_manifests::version::VersionSerde)>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let (name, version) = path.into_inner();

    let found = kintsu_registry_db::entities::Version::by_name_and_version(
        conn.as_ref(),
        &name,
        &version.to_string(),
    )
    .await?;
    let dependencies = found.dependencies(conn.as_ref()).await?;

    Ok(web::Json(dependencies))
}

#[utoipa::path(
    tag = PACKAGES,
    params(
        ("name" = String, Path, description = "Package name"),
        ("version" = String, Path, description = "Version string or 'latest'"),
    ),
    responses(
        (status = 200, description = "Package declarations", body = kintsu_parser::declare::DeclarationVersion),
        (status = 404, description = "Package or version not found", body = crate::ErrorResponse),
    )
)]
#[get("/package/{name}/{version}/declarations")]
pub async fn package_declarations(
    path: web::Path<(String, String)>,
    conn: DbConn,
    storage: web::Data<kintsu_registry_db::PackageStorage>,
) -> crate::Result<impl Responder> {
    let (name, version) = path.into_inner();
    let version =
        kintsu_registry_db::entities::Version::by_name_and_version(conn.as_ref(), &name, &version)
            .await?;

    let version_id = version.id;

    tokio::spawn(async move {
        let _ = kintsu_registry_db::entities::Version::increment_download_count(
            conn.as_ref(),
            version_id,
        )
        .await;
    });

    let declarations = storage
        .get_declarations(
            &storage.path_for_declarations(&name, &version.qualified_version.to_string()),
            version.declarations_checksum.into(),
        )
        .await?;
    Ok(web::Json(declarations))
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
    conn: DbConn,
    storage: web::Data<kintsu_registry_db::PackageStorage>,
) -> crate::Result<impl Responder> {
    let (name, version) = path.into_inner();

    let version =
        kintsu_registry_db::entities::Version::by_name_and_version(conn.as_ref(), &name, &version)
            .await?;

    let version_id = version.id;

    tokio::spawn(async move {
        let _ = kintsu_registry_db::entities::Version::increment_download_count(
            conn.as_ref(),
            version_id,
        )
        .await;
    });

    let source = storage
        .get_source(
            &storage.path_for_source(&name, &version.qualified_version.to_string()),
            version.source_checksum.into(),
        )
        .await?;

    Ok(web::Json(source))
}

#[utoipa::path(
    tag = PACKAGES,
    request_body = GrantSchemaRoleRequest,
    responses(
        (status = 200, description = "Role granted successfully", body = kintsu_registry_db::entities::SchemaRole),
        (status = 400, description = "Invalid request", body = crate::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
        (status = 403, description = "Forbidden - insufficient permissions", body = crate::ErrorResponse),
        (status = 404, description = "Package not found", body = crate::ErrorResponse),
    ),
    security(("api_key" = []), ("session" = []))
)]
#[post("/roles/package")]
pub async fn grant_package_role(
    principal: Principal,
    conn: DbConn,
    req: web::Json<GrantSchemaRoleRequest>,
) -> crate::Result<impl Responder> {
    req.validate()?;

    let req = req.into_inner();
    let role = kintsu_registry_db::engine::schema_role::grant_role(
        conn.as_ref(),
        principal.as_ref(),
        &req.package_name,
        req.user_id,
        req.org_id,
        req.role,
    )
    .await?;

    Ok(web::Json(role))
}

#[utoipa::path(
    tag = PACKAGES,
    request_body = RevokeSchemaRoleRequest,
    responses(
        (status = 204, description = "Role revoked successfully"),
        (status = 400, description = "Invalid request", body = crate::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
        (status = 403, description = "Forbidden - insufficient permissions", body = crate::ErrorResponse),
        (status = 404, description = "Role not found", body = crate::ErrorResponse),
    ),
    security(("api_key" = []), ("session" = []))
)]
#[delete("/roles/package")]
pub async fn revoke_package_role(
    principal: Principal,
    conn: DbConn,
    req: web::Json<RevokeSchemaRoleRequest>,
) -> crate::Result<impl Responder> {
    req.validate()?;

    kintsu_registry_db::engine::schema_role::revoke_role(
        conn.as_ref(),
        principal.as_ref(),
        req.role_id,
    )
    .await?;

    Ok(actix_web::HttpResponse::NoContent().finish())
}

/// Get total download count for a package
#[utoipa::path(
    tag = PACKAGES,
    params(
        ("name" = String, Path, description = "Package name"),
    ),
    responses(
        (status = 200, description = "Total download count", body = kintsu_registry_core::models::DownloadStats),
        (status = 404, description = "Package not found", body = crate::ErrorResponse),
    )
)]
#[get("/package-analytics/{name}/downloads/all")]
pub async fn get_package_total_downloads(
    name: web::Path<String>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let total =
        kintsu_registry_db::entities::Package::get_package_download_count(conn.as_ref(), &name)
            .await?;

    Ok(web::Json(kintsu_registry_core::models::DownloadStats {
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
        (status = 200, description = "90-day download history", body = Vec<kintsu_registry_db::engine::DownloadHistory>),
        (status = 404, description = "Package not found", body = crate::ErrorResponse),
    )
)]
#[get("/package-analytics/{name}/downloads/history")]
pub async fn get_package_download_history(
    name: web::Path<String>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let history =
        kintsu_registry_db::entities::Package::package_download_history(conn.as_ref(), &name)
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
        (status = 200, description = "Paginated list of packages", body = kintsu_registry_db::engine::Paginated<kintsu_registry_db::entities::Package>),
        (status = 400, description = "Invalid query parameters", body = crate::ErrorResponse),
    )
)]
#[get("/packages")]
pub async fn list_packages(
    query: web::Query<ListPackagesQuery>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let page = Page {
        number: query.page.unwrap_or(1),
        size: query.size.unwrap_or(20),
    };
    page.validate()?;

    let ordering = PackageOrdering {
        field: query
            .order_by
            .unwrap_or(PackageOrderingField::Name),
        direction: query
            .order_dir
            .unwrap_or(OrderDirection::Asc),
    };

    let paginated =
        kintsu_registry_db::entities::Package::list_packages(conn.as_ref(), page, ordering).await?;

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
        (status = 200, description = "Paginated search results", body = kintsu_registry_db::engine::Paginated<kintsu_registry_db::entities::Package>),
        (status = 400, description = "Invalid query parameters", body = crate::ErrorResponse),
    )
)]
#[get("/packages/search")]
pub async fn search_packages(
    query: web::Query<SearchPackagesQuery>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let page = Page {
        number: query.page.unwrap_or(1),
        size: query.size.unwrap_or(20),
    };

    page.validate()?;

    let ordering = PackageOrdering {
        field: query
            .order_by
            .unwrap_or(PackageOrderingField::Name),
        direction: query
            .order_dir
            .unwrap_or(OrderDirection::Asc),
    };

    let paginated = kintsu_registry_db::entities::Package::search_packages(
        conn.as_ref(),
        &query.q,
        page,
        ordering,
    )
    .await?;

    Ok(web::Json(paginated))
}

#[derive(serde::Deserialize)]
pub struct ListPackagesQuery {
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub order_by: Option<PackageOrderingField>,
    pub order_dir: Option<OrderDirection>,
}

#[derive(serde::Deserialize)]
pub struct SearchPackagesQuery {
    pub q: String,
    #[serde(flatten)]
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub order_by: Option<PackageOrderingField>,
    pub order_dir: Option<OrderDirection>,
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
        (status = 200, description = "Paginated list of versions", body = kintsu_registry_db::engine::Paginated<kintsu_registry_db::engine::version::QualifiedPackageVersion>),
        (status = 404, description = "Package not found", body = crate::ErrorResponse),
        (status = 400, description = "Invalid query parameters", body = crate::ErrorResponse),
    )
)]
#[get("/packages/{name}/versions")]
pub async fn list_package_versions(
    name: web::Path<String>,
    query: web::Query<ListVersionsQuery>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let page = Page {
        number: query.page.unwrap_or(1),
        size: query.size.unwrap_or(20),
    };
    page.validate()?;

    let paginated = kintsu_registry_db::entities::Package::list_package_versions(
        conn.as_ref(),
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
        (status = 200, description = "List of unique publishers ordered by latest published version", body = Vec<kintsu_registry_db::engine::Entity>),
        (status = 404, description = "Package not found", body = crate::ErrorResponse),
    )
)]
#[get("/packages/{name}/publishers")]
pub async fn get_package_publishers(
    name: web::Path<String>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let publishers =
        kintsu_registry_db::entities::Package::get_package_publishers(conn.as_ref(), &name).await?;

    Ok(web::Json(publishers))
}

#[derive(serde::Deserialize)]
pub struct ListVersionsQuery {
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub user_id: Option<i64>,
    pub org_id: Option<i64>,
}

#[utoipa::path(
    tag = PACKAGES,
    responses(
        (status = 200, description = "Successfully published package", body = kintsu_registry_core::models::PublishPackageResponse),
        (status = 400, description = "Invalid package data", body = crate::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::ErrorResponse),
    ),
    security(("api_key" = []))
)]
#[post("/packages/publish")]
/// Publish a new package version.
/// This endpoint requires authentication via an API key with publish permissions.
/// In addition, manifest format must meet the following requirements:
/// - schema.toml at the root level
/// - schema/ directory with at least one schema file
/// - paths must be in UNIX format, no spaces or backslashes, special characters, snake case only.
/// - package name must be unique within the registry
/// - version must follow semantic versioning
pub async fn publish_package(
    conn: DbConn,
    storage: web::Data<kintsu_registry_db::PackageStorage>,
    principal: crate::principal::Principal,
    request: web::Json<kintsu_registry_core::models::PublishPackageRequest>,
) -> crate::Result<impl Responder> {
    request.validate()?;
    if let Err(err) = request.validate_publishing_package_data() {
        return Err(crate::Error::PackagingErrors(err));
    }

    let conn = conn.into_inner();

    let deps = kintsu_registry_db::engine::package::StagePublishPackage::manifest_dependencies(
        conn.as_ref(),
        request.manifest.dependencies(),
    )
    .await?;

    let transitive_deps = kintsu_registry_db::entities::Package::get_transitive_dependencies(
        conn.as_ref(),
        deps.clone(),
    )
    .await?
    .into_iter()
    .map(Into::into)
    .collect::<Vec<_>>();

    let deps_sources = storage.get_sources(transitive_deps).await?;
    let resolver = crate::resolver::InternalPackageResolver::new(
        deps_sources
            .into_iter()
            .map(|source| {
                (
                    (
                        source.package_name,
                        kintsu_manifests::version::VersionSerde(
                            kintsu_manifests::version::parse_version(&source.version).unwrap(),
                        ),
                    ),
                    source.fs,
                )
            })
            .collect(),
    );

    let ctx = kintsu_parser::ctx::CompileCtx::with_fs_and_config(
        std::sync::Arc::new(request.package_data.clone()),
        std::sync::Arc::new(resolver),
        "./",
        4,
        false,
    )
    .await?;

    ctx.finalize().await?;
    let declarations = ctx.emit_declarations().await?;

    let package = kintsu_registry_db::engine::package::StagePublishPackage::process(
        conn.as_ref(),
        principal.as_ref(),
        storage.into_inner(),
        request.0.package_data,
        request.0.manifest,
        declarations,
        deps,
    )
    .await?;

    Ok(web::Json(package))
}

#[post("/package/{name}/{version}/yank")]
pub async fn yank_package_version(
    path: web::Path<(String, String)>,
    conn: DbConn,
    principal: crate::principal::Principal,
) -> crate::Result<impl Responder> {
    let (name, version) = path.into_inner();

    kintsu_registry_db::entities::Package::yank_version(
        conn.as_ref(),
        principal.as_ref(),
        &name,
        &version,
    )
    .await?;

    Ok(actix_web::HttpResponse::NoContent().finish())
}
