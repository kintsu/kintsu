use sea_orm::ConnectOptions;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

fn default_pool_size() -> u32 {
    10
}

#[derive(Deserialize, Debug)]
pub struct DatabaseConfig {
    #[serde(alias = "URL")]
    pub(crate) url: SecretString,
    #[allow(unused)]
    #[serde(alias = "POOL_SIZE", default = "default_pool_size")]
    pub(crate) pool_size: u32,
}

impl DatabaseConfig {
    pub async fn connect(&self) -> crate::Result<sea_orm::DatabaseConnection> {
        let database_url = self.url.expose_secret();
        let mut opts = ConnectOptions::new(database_url.to_owned());

        opts.max_connections(self.pool_size);
        opts.sqlx_logging_level(tracing::log::LevelFilter::Debug);

        let db = sea_orm::Database::connect(opts).await?;
        Ok(db)
    }
}
