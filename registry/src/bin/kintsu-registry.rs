use kintsu_manifests::NewForConfig;
use kintsu_registry::config::Config;
use tracing::Level;

#[tokio::main]
async fn main() -> kintsu_registry::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let c = Config::new::<&str>(None).unwrap();

    kintsu_registry::app::start_server(c).await
}
