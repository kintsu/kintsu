use crate::oauth::AuthClient;
use actix_web::{
    App, HttpServer, cookie,
    web::{self},
};
use secrecy::ExposeSecret;
use std::sync::Arc;
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
    modifiers(&SecurityAddon, &SharedErrorsAddon),
)]
pub struct ApiDoc;

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

#[macro_export]
macro_rules! bind_app {
    (
        $session_config: ident,
        $db: ident,
        $s3: ident,
        $client: ident,
        $cookie_key: ident,
    ) => {
        move || {
            App::new()
                .into_utoipa_app()
                .openapi(ApiDoc::openapi())
                .app_data($session_config.clone())
                .app_data($db.clone())
                .app_data($client.clone())
                .app_data($cookie_key.clone())
                .app_data($s3.clone())
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
                .service(org::grant_org_role)
                .service(org::revoke_org_role)
                // Favourites routes
                .service(favourites::list_favourites)
                .service(favourites::create_favourite)
                .service(favourites::delete_favourite)
                .service(favourites::org_favourite_count)
                .service(favourites::package_favourite_count)
                // Package routes
                .service(packages::publish_package)
                .service(packages::get_package_version)
                .service(packages::get_package_dependencies)
                .service(packages::package_declarations)
                .service(packages::get_dependent_packages)
                .service(packages::download_package_version)
                .service(packages::get_package_total_downloads)
                .service(packages::get_package_download_history)
                .service(packages::list_packages)
                .service(packages::search_packages)
                .service(packages::list_package_versions)
                .service(packages::get_package_publishers)
                .service(packages::grant_package_role)
                .service(packages::revoke_package_role)
                // Docs
                .openapi_service(|api| Redoc::with_url("/redoc", api))
                .openapi_service(|api| {
                    RapiDoc::with_openapi("/api-docs/openapi.json", api).path("/rapidoc")
                })
                .into_app()
        }
    };
}

pub async fn start_server(config: crate::config::Config) -> crate::Result<()> {
    let db = web::Data::new(config.database.connect().await?);
    let client = web::Data::new(AuthClient::new(config.gh)?);
    let addr = config.addr;
    let session_config = web::Data::new(config.session);
    let cookie_key = web::Data::new(cookie::Key::derive_from(
        session_config.key.expose_secret().as_bytes(),
    ));

    let s3 = web::Data::new(
        kintsu_registry_storage::s3::S3Storage::<kintsu_parser::declare::DeclarationVersion>::managed(
            &config.s3,
        )
        .await,
    );

    tracing::info!(
        "starting server on {}://{addr}",
        if config.insecure {
            "http"
        } else {
            "https"
        }
    );

    let server = HttpServer::new(bind_app!(session_config, db, s3, client, cookie_key,));

    let server_fut = {
        if config.insecure {
            server.bind(&addr)?.run()
        } else {
            let tls_config = build_tls_config(&config.tls)?;
            server
                .bind_rustls_0_23(&addr, tls_config)?
                .run()
        }
    };

    let (server_exit,) = tokio::join!(server_fut);

    kintsu_registry_events::shutdown()
        .await
        .unwrap();

    Ok(server_exit?)
}

fn build_tls_config(tls: &crate::config::TlsConfig) -> crate::Result<rustls::ServerConfig> {
    use std::sync::Arc;

    if !tls.is_configured() {
        return Err(crate::Error::TlsConfig(
            "TLS enabled but no certificate/key configured. \
             Set tls.cert_file + tls.key_file or tls.certificate + tls.key"
                .into(),
        ));
    }

    let cert_chain = tls.load_cert_chain()?;
    let private_key = tls.load_private_key()?;

    let config = if tls.require_client_cert {
        let client_ca_roots = tls.load_client_ca_roots()?;
        let client_cert_verifier =
            rustls::server::WebPkiClientVerifier::builder(Arc::new(client_ca_roots))
                .build()
                .map_err(|e| {
                    crate::Error::TlsConfig(format!("failed to build client verifier: {}", e))
                })?;

        tracing::info!("mTLS enabled - requiring client certificates");

        rustls::ServerConfig::builder()
            .with_client_cert_verifier(client_cert_verifier)
            .with_single_cert(cert_chain, private_key)?
    } else {
        tracing::warn!(
            "client certificate verification disabled - \
             consider enabling tls.require_client_cert for origin server security"
        );

        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)?
    };

    Ok(config)
}
