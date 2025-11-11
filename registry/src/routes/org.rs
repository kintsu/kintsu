use crate::{DbConn, session::SessionData};
use actix_web::{Responder, get, post, web};
use kintsu_registry_db::entities::Org;
use validator::Validate;

const ORGS: &str = "orgs";

/// Get organization by ID
#[utoipa::path(
    tag = ORGS,
    params(
        ("id" = i64, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Organization metadata", body = kintsu_registry_db::entities::Org),
        (status = 404, description = "Organization not found", body = crate::ErrorResponse),
    )
)]
#[get("/org/{id}")]
pub async fn get_org_by_id(
    org_id: web::Path<i64>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let org = kintsu_registry_db::entities::Org::by_id(conn.as_ref(), *org_id)
        .await?
        .ok_or_else(|| {
            crate::Error::Database(kintsu_registry_db::Error::NotFound(
                "Organization not found".into(),
            ))
        })?;

    Ok(web::Json(org))
}

/// Check if an organization name is already taken
#[utoipa::path(
    tag = ORGS,
    params(
        ("name" = String, Path, description = "Organization name to check"),
    ),
    responses(
        (status = 200, description = "Organization existence check", body = inline(Object), example = json!({"exists": true, "name": "acme-corp"})),
        (status = 400, description = "Invalid organization name", body = crate::ErrorResponse),
    )
)]
#[get("/orgs/exists/{name}")]
pub async fn check_org_exists(
    name: web::Path<String>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let name = name.into_inner();

    // Validate org name format (1-39 chars per GitHub limits)
    if name.is_empty() || name.len() > 39 {
        return Err(crate::Error::Database(
            kintsu_registry_db::Error::Validation(
                "Organization name must be between 1 and 39 characters".into(),
            ),
        ));
    }

    // Use the Org model to check existence by name
    use kintsu_registry_db::entities::Org;
    let exists = Org::by_name(conn.as_ref(), &name)
        .await?
        .is_some();

    Ok(web::Json(serde_json::json!({
        "exists": exists,
        "name": name,
    })))
}

/// Get current user's organizations
#[utoipa::path(
    tag = ORGS,
    responses(
        (status = 200, description = "User's organizations", body = Vec<kintsu_registry_db::engine::OrgWithAdmin>),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
    ),
    security(("session" = []))
)]
#[get("/orgs/mine")]
pub async fn get_my_orgs(
    session: SessionData,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let orgs = session.user.user.orgs(conn.as_ref()).await?;

    Ok(web::Json(orgs))
}

/// Create an API token for an organization
#[utoipa::path(
    tag = ORGS,
    params(
        ("id" = i64, Path, description = "Organization ID"),
    ),
    request_body = kintsu_registry_core::models::CreateTokenRequest,
    responses(
        (status = 200, description = "Successfully created org token", body = kintsu_registry_db::engine::OneTimeApiKey),
        (status = 400, description = "Invalid request", body = crate::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
        (status = 403, description = "User is not an org admin", body = crate::ErrorResponse),
    ),
    security(("session" = []))
)]
#[post("/org/{id}/tokens")]
pub async fn create_org_token(
    org_id: web::Path<i64>,
    session: SessionData,
    conn: DbConn,
    req: web::Json<kintsu_registry_core::models::CreateTokenRequest>,
) -> crate::Result<impl Responder> {
    use chrono::Duration;

    req.validate()?;

    let expires = chrono::Utc::now() + Duration::days(req.expires_in_days.unwrap_or(90));

    // User::request_org_token validates admin status via NewApiKey::qualify
    let one_time = session
        .user
        .user
        .request_org_token(
            conn.as_ref(),
            req.description.clone(),
            req.scopes.clone(),
            req.permissions.clone(),
            expires,
            *org_id,
        )
        .await?;

    Ok(web::Json(one_time))
}

/// Get all API tokens for an organization (admin only)
#[utoipa::path(
    tag = ORGS,
    params(
        ("id" = i64, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "List of org API tokens", body = Vec<kintsu_registry_db::entities::ApiKey>),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
        (status = 403, description = "User is not an org admin", body = crate::ErrorResponse),
        (status = 404, description = "Organization not found", body = crate::ErrorResponse),
    ),
    security(("session" = []))
)]
#[get("/org/{id}/tokens")]
pub async fn get_org_tokens(
    org_id: web::Path<i64>,
    session: SessionData,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    // Verify user is org admin
    let org = kintsu_registry_db::entities::Org::by_id(conn.as_ref(), *org_id)
        .await?
        .ok_or_else(|| {
            crate::Error::Database(kintsu_registry_db::Error::NotFound(
                "Organization not found".into(),
            ))
        })?;

    org.must_be_admin(conn.as_ref(), session.user.user.id)
        .await?;

    // Get org tokens
    let tokens = kintsu_registry_db::entities::Org::tokens(conn.as_ref(), *org_id).await?;

    Ok(web::Json(tokens))
}

/// Discover GitHub organizations where user has admin access
#[utoipa::path(
    tag = ORGS,
    responses(
        (status = 200, description = "List of candidate organizations", body = Vec<kintsu_registry_core::models::CandidateOrg>),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
        (status = 502, description = "GitHub API error", body = crate::ErrorResponse),
    ),
    security(("session" = []))
)]
#[get("/orgs/discover")]
pub async fn discover_orgs(
    session: SessionData,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    use secrecy::SecretString;

    // Build Octocrab client with user's token
    let github = octocrab::Octocrab::builder()
        .personal_token(SecretString::new(session.token.clone().into_boxed_str()))
        .build()?;

    // Fetch user's organizations from GitHub
    let gh_orgs = github
        .current()
        .list_org_memberships_for_authenticated_user()
        .per_page(100)
        .send()
        .await?;

    // Filter to only admin orgs
    let admin_orgs: Vec<_> = gh_orgs
        .items
        .into_iter()
        .filter(|membership| membership.role == "admin")
        .collect();

    let existing_orgs = Org::exists_bulk(
        conn.as_ref(),
        &admin_orgs
            .iter()
            .map(|org| org.organization.login.as_str())
            .collect::<Vec<_>>(),
    )
    .await?;

    // Build response
    let candidates: Vec<kintsu_registry_core::models::CandidateOrg> = admin_orgs
        .into_iter()
        .map(|membership| {
            let gh_id = membership.organization.id.0 as i32;
            kintsu_registry_core::models::CandidateOrg {
                gh_id,
                name: membership.organization.login.clone(),
                avatar_url: membership
                    .organization
                    .avatar_url
                    .to_string(),
                is_imported: existing_orgs.contains(&membership.organization.login),
            }
        })
        .collect();

    Ok(web::Json(candidates))
}

/// Import a GitHub organization into the registry
#[utoipa::path(
    tag = ORGS,
    request_body = kintsu_registry_core::models::ImportOrgRequest,
    responses(
        (status = 200, description = "Organization imported successfully", body = kintsu_registry_db::entities::Org),
        (status = 400, description = "Invalid request", body = crate::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
        (status = 403, description = "User is not an admin of the organization", body = crate::ErrorResponse),
        (status = 409, description = "Organization already imported", body = crate::ErrorResponse),
        (status = 502, description = "GitHub API error", body = crate::ErrorResponse),
    ),
    security(("session" = []))
)]
#[post("/orgs/import")]
pub async fn import_org(
    session: SessionData,
    conn: DbConn,
    req: web::Json<kintsu_registry_core::models::ImportOrgRequest>,
) -> crate::Result<impl Responder> {
    use secrecy::SecretString;

    // Validate request
    req.validate()?;

    // Build Octocrab client with user's token
    let github = octocrab::Octocrab::builder()
        .personal_token(SecretString::new(session.token.clone().into()))
        .build()?;

    // Fetch organization details from GitHub
    let gh_org = github.orgs(&req.org_name).get().await?;

    // Verify user has admin role in this organization
    // octocrab doesn't have a direct method, so we check if user can access org members (admin-only)
    let is_admin = github
        .orgs(&req.org_name)
        .list_members()
        .send()
        .await
        .is_ok();

    if !is_admin {
        return Err(crate::Error::Database(
            kintsu_registry_db::Error::Unauthorized(format!(
                "User is not an admin of organization '{}'",
                req.org_name
            )),
        ));
    }

    // Import organization to database
    let org = kintsu_registry_db::engine::org::import_organization(
        conn.as_ref(),
        gh_org.id.0 as i32,
        gh_org.login,
        gh_org.avatar_url.to_string(),
        session.user.user.id,
    )
    .await?;

    Ok(web::Json(org))
}
