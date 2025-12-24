mod database;
mod session;
mod tls;

pub use database::DatabaseConfig;
pub use session::SessionConfig;
pub use tls::TlsConfig;

use serde::Deserialize;
use validator::Validate;

fn default_addr() -> String {
    "127.0.0.1:8000".into()
}

#[derive(Deserialize, Debug, Validate)]
pub struct Config {
    #[serde(default = "default_addr", alias = "ADDR")]
    pub(crate) addr: String,

    #[serde(default)]
    pub(crate) insecure: bool,

    #[serde(default, alias = "TLS")]
    pub(crate) tls: TlsConfig,

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
