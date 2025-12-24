use std::fmt::Debug;

const SCOPES: &[&str] = &["user:email", "read:user", "read:org"];

pub fn scopes() -> String {
    SCOPES.join("%20")
}

fn default_base_url() -> url::Url {
    url::Url::parse("https://github.com").unwrap()
}

fn default_api_url() -> url::Url {
    url::Url::parse("https://api.github.com").unwrap()
}

#[derive(validator::Validate, serde::Deserialize, Debug)]
pub struct GhClientConfig {
    /// env: GH_CLIENT_ID
    #[serde(alias = "ID")]
    pub id: String,
    /// env: GH_CLIENT_SECRET
    #[serde(alias = "SECRET")]
    pub secret: secrecy::SecretString,
}

#[derive(validator::Validate, serde::Deserialize, Debug)]
pub struct GhOauthConfig {
    /// env: GH_BASE_URL
    #[serde(default = "default_base_url", alias = "BASE_URL")]
    pub base_url: url::Url,

    #[serde(default = "default_api_url", alias = "API_URL")]
    pub api_url: url::Url,

    #[serde(alias = "CLIENT")]
    pub client: GhClientConfig,
}
