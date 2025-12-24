use crate::{
    DbConn, config::SessionConfig, oauth::AuthClient, principal::Principal, session::SessionData,
};
use actix_web::{
    Responder, cookie, delete, get, post,
    web::{self, Redirect},
};
use secrecy::SecretString;
use validator::Validate;

const AUTH: &str = "auth";

#[derive(serde::Deserialize)]
struct CallbackQuery {
    code: SecretString,
}

#[utoipa::path(
    tag = AUTH,
    responses(
        (status = 307, description = "Redirect to home page after successful authentication"),
        (status = 400, description = "Bad request", body = crate::ErrorResponse),
    )
)]
#[get("/auth/callback")]
pub async fn callback(
    req: actix_web::HttpRequest,
    client: web::Data<AuthClient>,
    cookie_key: web::Data<cookie::Key>,
    session_config: web::Data<SessionConfig>,
    code: web::Query<CallbackQuery>,
    conn: DbConn,
) -> crate::Result<impl Responder> {
    let token = client
        .exchange_token(code.into_inner().code)
        .await?;

    let user = client
        .saturate_user_data(&token.access_token)
        .await?;

    let created = kintsu_registry_db::engine::user::create_or_update_user_from_oauth(
        conn.as_ref(),
        user.id.0 as i32,
        &user.login,
        Some(user.avatar_url.as_str()),
        &user.email.unwrap(),
    )
    .await?;

    let mut session = SessionData::new(created.clone(), token.access_token);
    let mut jar = actix_web::cookie::CookieJar::new();
    session.jar(&mut jar, &cookie_key, session_config.domain.clone())?;

    let mut resp = Redirect::to("/").respond_to(&req);

    resp.headers_mut().append(
        actix_web::http::header::SET_COOKIE,
        jar.delta()
            .next()
            .unwrap()
            .encoded()
            .to_string()
            .parse()
            .unwrap(),
    );

    Ok(resp)
}

#[utoipa::path(
    tag = AUTH,
    responses(
        (status = 200, description = "Get current user info", body = crate::session::PublicData),
    )
)]
#[get("/auth/whoami")]
pub async fn whoami(session: crate::session::SessionData) -> impl Responder {
    web::Json(session.user)
}

#[utoipa::path(
    tag = AUTH,
    responses(
        (status = 307, description = "Redirect to home page after logout"),
    ),
    security(("session" = []))
)]
#[get("/auth/logout")]
pub async fn logout(
    req: actix_web::HttpRequest,
    session_config: web::Data<SessionConfig>,
) -> impl Responder {
    let cookie = crate::session::SessionData::removal_cookie(session_config.domain.clone());

    let mut resp = Redirect::to("/").respond_to(&req);

    resp.headers_mut().append(
        actix_web::http::header::SET_COOKIE,
        cookie.encoded().to_string().parse().unwrap(),
    );

    resp
}

#[utoipa::path(
    tag = AUTH,
    request_body = kintsu_registry_core::models::CreateTokenRequest,
    responses(
        (status = 200, description = "Successfully created auth token", body = kintsu_registry_db::engine::OneTimeApiKey),
        (status = 400, description = "Invalid request", body = crate::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
    ),
    security(("api_key" = []), ("session" = []))
)]
#[post("/auth/token")]
pub async fn create_auth_token(
    conn: DbConn,
    principal: Principal,
    req: web::Json<kintsu_registry_core::models::CreateTokenRequest>,
) -> crate::Result<impl Responder> {
    use chrono::Duration;

    req.validate()?;

    let user = principal
        .user()
        .ok_or_else(|| crate::Error::AuthorizationRequired)?;

    let expires = chrono::Utc::now() + Duration::days(req.expires_in_days.unwrap_or(90));

    let one_time = user
        .request_personal_token(
            conn.as_ref(),
            principal.as_ref(),
            req.description.clone(),
            req.scopes.clone(),
            req.permissions.clone(),
            expires,
        )
        .await?;

    Ok(web::Json(one_time))
}

#[utoipa::path(
    tag = AUTH,
    responses(
        (status = 200, description = "List of user API tokens", body = Vec<kintsu_registry_db::entities::ApiKey>),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
    ),
    security(("session" = []))
)]
#[get("/auth/tokens")]
pub async fn get_user_tokens(
    conn: DbConn,
    session: SessionData,
) -> crate::Result<impl Responder> {
    let tokens =
        kintsu_registry_db::entities::User::tokens(conn.as_ref(), session.user.user.id).await?;
    Ok(web::Json(tokens))
}

#[utoipa::path(
    tag = AUTH,
    responses(
        (status = 307),
    )
)]
#[get("/auth/login")]
pub async fn redirect_to_login(client: web::Data<AuthClient>) -> impl Responder {
    Redirect::to(client.login_url.to_string())
}

#[utoipa::path(
    tag = AUTH,
    responses(
        (status = 200, description = "Successfully revoked auth token"),
        (status = 401, description = "Unauthorized", body = crate::ErrorResponse),
        (status = 404, description = "Token not found", body = crate::ErrorResponse),
    ),
    security(("session" = []))
)]
#[delete("/auth/tokens/{id}")]
pub async fn revoke_auth_token(
    conn: DbConn,
    principal: Principal,
    id: web::Path<i64>,
) -> crate::Result<impl Responder> {
    let api_key =
        kintsu_registry_db::entities::ApiKey::by_id(conn.as_ref(), id.into_inner()).await?;

    api_key
        .revoke_token(conn.as_ref(), principal.as_ref())
        .await?;

    Ok(web::Json(()))
}
