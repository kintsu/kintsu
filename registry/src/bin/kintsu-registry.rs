use kintsu_manifests::NewForConfig;
use kintsu_registry::config::Config;
use tracing::Level;

#[actix_web::main]
async fn main() -> kintsu_registry::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let c = Config::new::<&str>(None)?;

    let event_reporter: Vec<Box<dyn kintsu_registry_events::EventReporter>> =
        vec![Box::new(kintsu_registry_events::TracingEventReporter)];

    kintsu_registry_events::start(event_reporter, move || {
        async { kintsu_registry::app::start_server(c).await }
    })
    .await
}
