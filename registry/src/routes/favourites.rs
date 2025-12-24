use crate::{DbConn, principal::Principal, session::SessionData};
use actix_web::{Responder, delete, get, post, web};
use kintsu_registry_core::models::{
    CreateFavouriteRequest, DeleteFavouriteRequest, FavouriteTargetRequest, FavouritesCount,
};
use kintsu_registry_db::engine::{self, FavouriteTarget};
use validator::Validate;

const FAVOURITES: &str = "favourites";

#[derive(serde::Deserialize)]
pub struct ListFavouritesQuery {
    pub page: Option<i64>,
    pub size: Option<i64>,
}

#[utoipa::path(
    tag = FAVOURITES,
    params(
        ("page" = Option<i64>, Query, description = "Page number (default: 1)"),
        ("size" = Option<i64>, Query, description = "Page size (default: 20)"),
    ),
    responses(
        (status = 200, description = "User favourites retrieved successfully", body = kintsu_registry_db::engine::Paginated<kintsu_registry_db::engine::FavouriteWithEntity>),
        (status = 401, description = "Unauthorized - session required", body = crate::ErrorResponse),
        (status = 400, description = "Invalid query parameters", body = crate::ErrorResponse),
    ),
    security(("session" = []))
)]
#[get("/favourites")]
pub async fn list_favourites(
    principal: Principal,
    query: web::Query<ListFavouritesQuery>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let user_id = principal
        .user()
        .ok_or_else(|| crate::Error::AuthorizationRequired)?
        .id;

    let page = kintsu_registry_db::engine::Page {
        number: query.page.unwrap_or(1),
        size: query.size.unwrap_or(20),
    };
    page.validate()?;

    let favourites = engine::list_favourites(conn.as_ref(), user_id, page).await?;
    Ok(web::Json(favourites))
}

/// Create a user favourite
#[utoipa::path(
    tag = FAVOURITES,
    request_body = CreateFavouriteRequest,
    responses(
        (status = 201, description = "Favourite created successfully", body = kintsu_registry_db::entities::UserFavourite),
        (status = 401, description = "Unauthorized - session required", body = crate::ErrorResponse),
        (status = 404, description = "Package or organization not found", body = crate::ErrorResponse),
        (status = 409, description = "Favourite already exists", body = crate::ErrorResponse),
    ),
    security(("session" = []))
)]
#[post("/favourites")]
pub async fn create_favourite(
    session: SessionData,
    body: web::Json<CreateFavouriteRequest>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    // Validate request
    body.validate().map_err(|e| {
        crate::Error::Database(kintsu_registry_db::Error::Validation(format!(
            "Invalid request: {}",
            e
        )))
    })?;

    // Convert request target to engine target
    let target = match &body.target {
        FavouriteTargetRequest::Package(id) => FavouriteTarget::Package(*id),
        FavouriteTargetRequest::Org(id) => FavouriteTarget::Org(*id),
    };

    // Create the favourite
    let favourite = engine::create_favourite(conn.as_ref(), session.user.user.id, target).await?;

    Ok(web::Json(favourite))
}

/// Delete a user favourite
#[utoipa::path(
    tag = FAVOURITES,
    request_body = DeleteFavouriteRequest,
    responses(
        (status = 204, description = "Favourite deleted successfully"),
        (status = 401, description = "Unauthorized - session required", body = crate::ErrorResponse),
        (status = 404, description = "Favourite not found", body = crate::ErrorResponse),
    ),
    security(("session" = []))
)]
#[delete("/favourites")]
pub async fn delete_favourite(
    session: SessionData,
    body: web::Json<DeleteFavouriteRequest>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    // Validate request
    body.validate().map_err(|e| {
        crate::Error::Database(kintsu_registry_db::Error::Validation(format!(
            "Invalid request: {}",
            e
        )))
    })?;

    // Convert request target to engine target
    let target = match &body.target {
        FavouriteTargetRequest::Package(id) => FavouriteTarget::Package(*id),
        FavouriteTargetRequest::Org(id) => FavouriteTarget::Org(*id),
    };

    // Delete the favourite
    engine::delete_favourite(conn.as_ref(), session.user.user.id, target).await?;

    Ok(actix_web::HttpResponse::NoContent().finish())
}

/// Number of times an organization has been favourited
#[utoipa::path(
    tag = FAVOURITES,
    params(
        ("id" = i64, Path, description = "Organization id to check"),
    ),
    responses(
        (status = 200, description = "Organization existence check", body = FavouritesCount),
        (status = 400, description = "Invalid organization", body = crate::ErrorResponse),
    )
)]
#[get("/favourites/orgs/{id}")]
pub async fn org_favourite_count(
    id: web::Path<i64>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let count =
        kintsu_registry_db::engine::favourites::org_favourite_count(conn.as_ref(), *id).await?;
    Ok(web::Json(FavouritesCount { count }))
}

/// Number of times a package has been favourited
#[utoipa::path(
    tag = FAVOURITES,
    params(
        ("id" = i64, Path, description = "Package id to check"),
    ),
    responses(
        (status = 200, description = "Package existence check", body = FavouritesCount),
        (status = 400, description = "Invalid package", body = crate::ErrorResponse),
    )
)]
#[get("/favourites/packages/{id}")]
pub async fn package_favourite_count(
    id: web::Path<i64>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let count =
        kintsu_registry_db::engine::favourites::package_favourite_count(conn.as_ref(), *id).await?;
    Ok(web::Json(FavouritesCount { count }))
}
