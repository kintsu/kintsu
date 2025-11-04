use crate::oauth::config::scopes;

mod config;

pub use config::GhOauthConfig;
use octocrab::models::Author;
use reqwest::Method;
use secrecy::{ExposeSecret, SecretString};

pub struct AuthClient {
    config: config::GhOauthConfig,
    client: reqwest::Client,
    pub(crate) login_url: url::Url,
}

impl AuthClient {
    pub fn new(config: config::GhOauthConfig) -> crate::Result<Self> {
        Ok(Self {
            login_url: Self::create_login_url(config.base_url.clone(), &config.client.id),
            client: reqwest::ClientBuilder::new().build()?,
            config,
        })
    }

    fn create_login_url(
        base_url: url::Url,
        client_id: &str,
    ) -> url::Url {
        let query = format!(
            "/login/oauth/authorize?client_id={}&scope={}",
            client_id,
            scopes()
        );
        base_url.join(&query).unwrap()
    }

    pub async fn exchange_token(
        &self,
        code: SecretString,
    ) -> Result<ValidExchangeResponse, crate::Error> {
        let mut url = self
            .config
            .base_url
            .join("/login/oauth/access_token")
            .unwrap();

        let mut query = url.query_pairs_mut();

        query.append_pair("code", code.expose_secret());
        query.append_pair("client_id", &self.config.client.id);
        query.append_pair("client_secret", self.config.client.secret.expose_secret());
        query.append_pair("accept", "json");

        drop(query);

        let mut request = reqwest::Request::new(Method::POST, url);

        let headers_mut = request.headers_mut();
        headers_mut.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        let resp = self
            .client
            .execute(request)
            .await?
            .error_for_status()?;

        let body = resp.bytes().await?;

        Ok(serde_json::from_slice::<ExchangeResponse>(&body)?.into_result()?)
    }

    pub async fn saturate_user_data(
        &self,
        access_token: &SecretString,
    ) -> crate::Result<Author> {
        Ok(octocrab::Octocrab::builder()
            .personal_token(access_token.clone())
            .build()?
            .current()
            .user()
            .await?)
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
pub enum ExchangeResponse {
    Valid(ValidExchangeResponse),
    Invalid {
        error: String,
        error_description: Option<String>,
        error_uri: Option<String>,
    },
}

impl ExchangeResponse {
    pub fn into_result(self) -> crate::Result<ValidExchangeResponse> {
        match self {
            ExchangeResponse::Valid(v) => Ok(v),
            ExchangeResponse::Invalid {
                error,
                error_description,
                error_uri,
            } => {
                Err(crate::Error::TokenExchangeError {
                    error,
                    error_description,
                    error_uri,
                })
            },
        }
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct ValidExchangeResponse {
    token_type: String,
    #[serde(deserialize_with = "de_str_split")]
    scope: Vec<String>,
    pub(crate) access_token: SecretString,
}

fn de_str_split<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>, {
    let s: &str = serde::Deserialize::deserialize(deserializer)?;
    Ok(s.split(',').map(|s| s.to_string()).collect())
}
