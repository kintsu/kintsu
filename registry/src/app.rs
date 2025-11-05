use crate::oauth::AuthClient;
use actix_web::{
    App, HttpServer, cookie,
    web::{self},
};
use secrecy::ExposeSecret;
use utoipa::{
    Modify, OpenApi, PartialSchema,
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
};
use utoipa_actix_web::AppExt;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};

use crate::routes::*;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "kintsu-registry", description = "Kintsu Registry API")
    ),
    modifiers(&SecurityAddon),
    modifiers(&SharedErrorsAddon),
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(
        &self,
        openapi: &mut utoipa::openapi::OpenApi,
    ) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("apikey"))),
        );
        // components.add_security_scheme(
        //     "session",
        //     SecurityScheme::
        // );
    }
}

struct SharedErrorsAddon;

impl Modify for SharedErrorsAddon {
    fn modify(
        &self,
        openapi: &mut utoipa::openapi::OpenApi,
    ) {
        let components = openapi.components.as_mut().unwrap();
        components
            .schemas
            .insert("ErrorResponse".into(), crate::ErrorResponse::schema());
    }
}

pub async fn start_server(config: crate::config::Config) -> crate::Result<()> {
    let db = web::Data::new(config.database.connect().await?);
    let client = web::Data::new(AuthClient::new(config.gh)?);
    let addr = config.addr;
    let session_config = web::Data::new(config.session);
    let cookie_key = web::Data::new(cookie::Key::derive_from(
        session_config.key.expose_secret().as_bytes(),
    ));

    tracing::info!(
        "starting server on {}://{addr}",
        if config.insecure {
            "http"
        } else {
            "https"
        }
    );
    HttpServer::new(move || {
        App::new()
            .into_utoipa_app()
            .openapi(ApiDoc::openapi())
            .app_data(session_config.clone())
            .app_data(db.clone())
            .app_data(client.clone())
            .app_data(cookie_key.clone())
            // Auth routes
            .service(auth::callback)
            .service(auth::whoami)
            .service(auth::logout)
            .service(auth::create_auth_token)
            .service(auth::revoke_auth_token)
            .service(auth::get_user_tokens)
            .service(auth::redirect_to_login)
            // Org routes
            .service(org::get_org_by_id)
            .service(org::check_org_exists)
            .service(org::get_my_orgs)
            .service(org::create_org_token)
            .service(org::get_org_tokens)
            .service(org::discover_orgs)
            .service(org::import_org)
            // Package routes
            .service(packages::get_package_version)
            .service(packages::download_package_version)
            .service(packages::get_package_total_downloads)
            .service(packages::get_package_download_history)
            .service(packages::list_packages)
            .service(packages::search_packages)
            .service(packages::list_package_versions)
            .service(packages::get_package_publishers)
            .openapi_service(|api| Redoc::with_url("/redoc", api))
            .openapi_service(|api| {
                RapiDoc::with_openapi("/api-docs/openapi.json", api).path("/rapidoc")
            })
            .into_app()
    })
    .bind(addr)?
    .run()
    .await?;

    Ok(())
}
