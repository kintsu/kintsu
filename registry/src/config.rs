use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use validator::Validate;

fn default_db_pool_size() -> u32 {
    10
}

#[derive(Deserialize, Debug)]
pub(crate) struct DatabaseConfig {
    #[serde(alias = "URL")]
    pub(crate) url: SecretString,
    #[serde(alias = "POOL_SIZE", default = "default_db_pool_size")]
    pub(crate) pool_size: u32,
}

impl DatabaseConfig {
    pub async fn connect(&self) -> crate::Result<sea_orm::DatabaseConnection> {
        let database_url = self.url.expose_secret();
        let db = sea_orm::Database::connect(database_url).await?;
        Ok(db)
    }
}

pub fn default_domain() -> String {
    "kintsu.dev".into()
}

#[derive(Deserialize, Debug)]
pub struct SessionConfig {
    #[serde(alias = "DOMAIN", default = "default_domain")]
    pub domain: String,

    #[serde(alias = "KEY")]
    pub key: SecretString,
}

fn default_addr() -> String {
    "127.0.0.1:8000".into()
}

#[derive(Deserialize, Debug, validator::Validate)]
pub struct Config {
    #[serde(default = "default_addr", alias = "ADDR")]
    pub(crate) addr: String,

    #[serde(default)]
    pub(crate) insecure: bool,

    #[validate(nested)]
    #[serde(alias = "GH")]
    pub(crate) gh: crate::oauth::GhOauthConfig,

    #[serde(alias = "DATABASE")]
    pub(crate) database: DatabaseConfig,

    #[serde(alias = "SESSION")]
    pub(crate) session: SessionConfig,

    #[serde(alias = "S3")]
    pub(crate) s3: kintsu_registry_storage::Config,
}

impl kintsu_manifests::NewForConfig for Config {
    const NAME: &'static str = "registry";
    const ENV: &'static str = "KS";
}
