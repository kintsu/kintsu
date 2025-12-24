#![allow(clippy::result_large_err)]

use secrecy::ExposeSecret;

use kintsu_registry_core::ErrorResponse;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No authentication provided (set KINTSU_REGISTRY_TOKEN environment variable)")]
    NoAuth,
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("API Error: {0}")]
    Response(#[from] ErrorOrResponseError),
    #[error("Validation errors: {0}")]
    Validation(#[from] validator::ValidationErrors),
    #[error("{0}")]
    Fs(#[from] kintsu_fs::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum ErrorOrResponseError {
    #[error("{0}")]
    ErrorString(String),
    #[error("{0:?}")]
    ResponseError(ErrorResponse),

    #[error("{}: {source:?}", status.as_u16())]
    WithStatus {
        #[source]
        source: Box<ErrorOrResponseError>,
        status: reqwest::StatusCode,
    },
}

impl ErrorOrResponseError {
    pub fn with_status(
        self,
        status: reqwest::StatusCode,
    ) -> Self {
        Self::WithStatus {
            source: Box::new(self),
            status,
        }
    }
}

pub struct RegistryClient {
    client: reqwest::Client,
    base_url: url::Url,
    token: Option<secrecy::SecretString>,
}

impl RegistryClient {
    pub fn new(
        base_url: &str,
        token: Option<secrecy::SecretString>,
    ) -> Result<Self, Error> {
        let base_url = url::Url::parse(base_url)?;
        let client = reqwest::Client::new();

        Ok(Self {
            client,
            base_url,
            token,
        })
    }

    pub fn url(
        &self,
        path: &str,
    ) -> url::Url {
        self.base_url.join(path).unwrap()
    }

    pub async fn perform<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::Request,
    ) -> Result<T, Error> {
        let resp = self.client.execute(req).await?;

        let status = resp.status();
        let body = resp.bytes().await?;

        if status.is_success() {
            let parsed: T = serde_json::from_slice(&body)?;
            Ok(parsed)
        } else {
            Err(Self::handle_response_with_errors(status, body).await)
        }
    }

    pub async fn perform_authenticated<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::Request,
    ) -> Result<T, Error> {
        let Some(token) = &self.token else {
            return Err(Error::NoAuth);
        };

        let mut req = req;

        req.headers_mut().insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token.expose_secret())
                .parse()
                .unwrap(),
        );

        self.perform(req).await
    }

    async fn handle_response_with_errors(
        status: reqwest::StatusCode,
        body: bytes::Bytes,
    ) -> Error {
        (if !body.is_empty() {
            match serde_json::from_slice::<ErrorResponse>(&body) {
                Ok(err_resp) => ErrorOrResponseError::ResponseError(err_resp),
                Err(parse_err) => {
                    tracing::trace! {
                        "Failed to parse error response: {}",
                        parse_err,
                    }
                    let error_str = String::from_utf8_lossy(&body);
                    ErrorOrResponseError::ErrorString(error_str.to_string())
                },
            }
        } else {
            ErrorOrResponseError::ErrorString("empty response body".to_string())
        })
        .with_status(status)
        .into()
    }

    pub async fn publish_compiled_package(
        &self,
        mut manifest: kintsu_manifests::package::PackageManifest,
        package_data: std::sync::Arc<dyn kintsu_fs::FileSystem>,
        root_path: impl AsRef<std::path::Path>,
    ) -> Result<(), Error> {
        let package_name = manifest.package.name.clone();

        manifest.prepare_publish()?;

        let package_data = kintsu_fs::memory::MemoryFileSystem::extract_from(
            &package_data,
            &root_path,
            &["/**/*.ks", "/schema.toml", "/**/*.md", "/**/*.txt"],
            &Vec::<String>::new(),
        )
        .await?;

        let body = kintsu_registry_core::models::PublishPackageRequest {
            manifest,
            package_data,
        };

        let mut request =
            reqwest::Request::new(reqwest::Method::POST, self.url("/packages/publish"));

        *request.body_mut() = Some(reqwest::Body::from(serde_json::to_vec(&body)?));

        request.headers_mut().insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );

        let published = self
            .perform_authenticated::<kintsu_registry_core::models::Version>(request)
            .await?;

        tracing::info!("Published {}@{}", package_name, published.qualified_version);
        tracing::info!(
            "Package URL: {}",
            self.url(&format!(
                "/packages/{}/{}",
                package_name, published.qualified_version
            ))
        );

        Ok(())
    }
}
