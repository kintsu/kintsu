use secrecy::SecretString;
use serde::Deserialize;

fn default_domain() -> String {
    "kintsu.dev".into()
}

#[derive(Deserialize, Debug)]
pub struct SessionConfig {
    #[serde(alias = "DOMAIN", default = "default_domain")]
    pub domain: String,

    #[serde(alias = "KEY")]
    pub key: SecretString,
}
