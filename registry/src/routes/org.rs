use crate::{DbConn, principal::Principal, session::SessionData};
use actix_web::{Responder, delete, get, post, web};
use kintsu_registry_core::models::{FavouritesCount, GrantOrgRoleRequest, RevokeOrgRoleRequest};
use kintsu_registry_db::{
    engine::fluent::AuthCheck,
    entities::{Org, Permission},
};
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
    principal: Principal,
    conn: DbConn,
    req: web::Json<kintsu_registry_core::models::CreateTokenRequest>,
) -> crate::Result<impl Responder> {
    use chrono::Duration;

    req.validate()?;

    let user = principal
        .user()
        .ok_or_else(|| crate::Error::AuthorizationRequired)?;

    let expires = chrono::Utc::now() + Duration::days(req.expires_in_days.unwrap_or(90));

    let req = req.into_inner();
    let one_time = user
        .request_org_token(
            conn.as_ref(),
            principal.as_ref(),
            req.description,
            req.scopes,
            req.permissions,
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
    principal: Principal,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let auth_result = AuthCheck::new(conn.as_ref(), principal.as_ref())
        .org(*org_id)
        .can_list_tokens()
        .await?;

    auth_result.require()?;

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
    principal: crate::principal::Principal,
    conn: DbConn,
    req: web::Json<kintsu_registry_core::models::ImportOrgRequest>,
) -> crate::Result<impl Responder> {
    use secrecy::SecretString;

    req.validate()?;

    if !principal.is_session() {
        return Err(crate::Error::Database(
            kintsu_registry_db::Error::Validation(
                "Organization import requires user session (not API key)".into(),
            ),
        ));
    }

    let github = octocrab::Octocrab::builder()
        .personal_token(SecretString::new(session.token.clone().into()))
        .build()?;

    let gh_org = github.orgs(&req.org_name).get().await?;

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

    let org = kintsu_registry_db::engine::org::import_organization(
        conn.as_ref(),
        principal.as_ref(),
        gh_org.id.0 as i32,
        gh_org.login,
        gh_org.avatar_url.to_string(),
    )
    .await?;

    Ok(web::Json(org))
}

#[utoipa::path(
    tag = ORGS,
    request_body = GrantOrgRoleRequest,
    responses(
        (status = 200, description = "Org role granted successfully", body = kintsu_registry_db::entities::OrgRole),
        (status = 400, description = "Invalid request", body = crate::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
        (status = 403, description = "Forbidden - insufficient permissions", body = crate::ErrorResponse),
        (status = 404, description = "Organization not found", body = crate::ErrorResponse),
    ),
    security(("api_key" = []), ("session" = []))
)]
#[post("/roles/org")]
pub async fn grant_org_role(
    principal: Principal,
    conn: DbConn,
    req: web::Json<GrantOrgRoleRequest>,
) -> crate::Result<impl Responder> {
    req.validate()?;

    let req = req.into_inner();

    let role = kintsu_registry_db::engine::org::grant_role(
        conn.as_ref(),
        principal.as_ref(),
        req.org_id,
        req.user_id,
        req.role,
    )
    .await?;

    Ok(web::Json(role))
}

#[utoipa::path(
    tag = ORGS,
    request_body = RevokeOrgRoleRequest,
    responses(
        (status = 204, description = "Org role revoked successfully"),
        (status = 400, description = "Invalid request", body = crate::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
        (status = 403, description = "Forbidden - insufficient permissions", body = crate::ErrorResponse),
        (status = 404, description = "Role not found", body = crate::ErrorResponse),
    ),
    security(("api_key" = []), ("session" = []))
)]
#[delete("/roles/org")]
pub async fn revoke_org_role(
    principal: Principal,
    conn: DbConn,
    req: web::Json<RevokeOrgRoleRequest>,
) -> crate::Result<impl Responder> {
    req.validate()?;

    let req = req.into_inner();

    kintsu_registry_db::engine::org::revoke_role(
        conn.as_ref(),
        principal.as_ref(),
        req.org_id,
        req.user_id,
    )
    .await?;

    Ok(actix_web::HttpResponse::NoContent().finish())
}
