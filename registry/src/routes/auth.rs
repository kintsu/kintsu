use crate::{WebPgPool, config::SessionConfig, oauth::AuthClient, session::SessionData};
use actix_web::{
    Responder, cookie, get, post,
    web::{self, Redirect},
};
use secrecy::SecretString;

const AUTH: &str = "auth";

#[derive(serde::Deserialize)]
struct CallbackQuery {
    code: SecretString,
}

#[utoipa::path(
    tag = AUTH,
)]
#[get("/auth/callback")]
pub async fn callback(
    req: actix_web::HttpRequest,
    client: web::Data<AuthClient>,
    cookie_key: web::Data<cookie::Key>,
    session_config: web::Data<SessionConfig>,
    code: web::Query<CallbackQuery>,
    pool: WebPgPool,
) -> crate::Result<impl Responder> {
    let token = client
        .exchange_token(code.into_inner().code)
        .await?;

    let user = client
        .saturate_user_data(&token.access_token)
        .await?;

    let mut conn = pool.get().await?;

    let created = kintsu_registry_db::handlers::auth::create_or_update_user_from_oauth(
        &mut conn,
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
        (status = 200, description = "Logout user"),
    )
)]
#[get("/auth/logout")]
pub async fn logout(session: crate::session::SessionData) -> impl Responder {
    "200".to_string()
}

#[utoipa::path(
    tag = AUTH,
    responses(
        (status = 200, description = "Successfully created auth token", body = [String]),
    )
)]
#[post("/auth/token")]
pub async fn create_auth_token(pool: WebPgPool) -> impl Responder {
    "200".to_string()
}

#[utoipa::path(
    tag = AUTH,
    responses(
        (status = 307),
    )
)]
#[post("/auth/login")]
pub async fn redirect_to_login(client: web::Data<AuthClient>) -> impl Responder {
    Redirect::to(client.login_url.to_string())
}
