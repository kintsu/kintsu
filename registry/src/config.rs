use std::time::Duration;

use kintsu_registry_db::AsyncConnectionPool;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use validator::Validate;

use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, bb8::Pool},
};

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
    pub async fn connect(&self) -> crate::Result<AsyncConnectionPool> {
        let mgr = AsyncDieselConnectionManager::<AsyncPgConnection>::new(self.url.expose_secret());

        Pool::builder()
            .max_size(self.pool_size)
            .min_idle(Some(5))
            .max_lifetime(Some(Duration::from_secs(60 * 60 * 24)))
            .idle_timeout(Some(Duration::from_secs(60 * 2)))
            .build(mgr)
            .await
            .map_err(crate::Error::from)
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
}

impl kintsu_manifests::NewForConfig for Config {
    const NAME: &'static str = "registry";
    const ENV: &'static str = "KS";
}
