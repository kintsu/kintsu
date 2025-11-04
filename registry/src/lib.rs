use actix_web::{ResponseError, web};
use diesel_async::pooled_connection::{PoolError, bb8::RunError};
use kintsu_registry_db::AsyncConnectionPool;

pub mod app;
pub mod config;
mod oauth;
pub mod routes;
mod sealed;
pub(crate) mod session;

pub type WebPgPool = web::Data<AsyncConnectionPool>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("database error: {0:?}")]
    Database(#[from] kintsu_registry_db::Error),

    #[error("missing `web::Data<{data}>`")]
    MissingData { data: String },

    #[error("unknown json error: {0:?}")]
    Json(#[from] serde_json::Error),

    #[error("request error: {0:?}")]
    RequestError(#[from] reqwest::Error),

    #[error("github error: {0:?}")]
    Octocrab(#[from] octocrab::Error),

    #[error(
        "token exchange error: {error}, description: {error_description:?}, uri: {error_uri:?}"
    )]
    TokenExchangeError {
        error: String,
        error_description: Option<String>,
        error_uri: Option<String>,
    },

    #[error("database error: {0:?}")]
    PoolError(#[from] PoolError),

    #[error("bb8 pool error: {0:?}")]
    Bb8(#[from] RunError),

    #[error("io error: {0:?}")]
    IoError(#[from] std::io::Error),

    #[error("session error: {cause}")]
    SessionError { cause: String },

    #[error("cookie parse error: {0:?}")]
    CookieParseError(#[from] actix_web::cookie::ParseError),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(serde::Serialize)]
struct ErrorResponse {
    error: String,
    error_description: Option<String>,
    error_uri: Option<String>,
}
impl ErrorResponse {
    fn internal() -> Self {
        Self {
            error: "internal_server_error".to_string(),
            error_description: None,
            error_uri: None,
        }
    }
}

impl Error {
    pub(crate) fn session(cause: impl Into<String>) -> Self {
        Error::SessionError {
            cause: cause.into(),
        }
    }

    pub(crate) fn missing_data(data: impl Into<String>) -> Self {
        Error::MissingData { data: data.into() }
    }

    fn to_error_response(&self) -> ErrorResponse {
        match self {
            Error::SessionError { cause } => {
                ErrorResponse {
                    error: "session_error".to_string(),
                    error_description: Some(cause.clone()),
                    error_uri: None,
                }
            },
            Error::CookieParseError(..) => {
                ErrorResponse {
                    error: "cookie_parse_error".to_string(),
                    error_description: Some("Bad request cookies".to_string()),
                    error_uri: None,
                }
            },
            // todo: add db error unauthorized
            Error::TokenExchangeError {
                error,
                error_description,
                error_uri,
            } => {
                ErrorResponse {
                    error: error.clone(),
                    error_description: error_description.clone(),
                    error_uri: error_uri.clone(),
                }
            },
            _ => ErrorResponse::internal(),
        }
    }
}
impl ResponseError for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Error::Json(_) => actix_web::http::StatusCode::BAD_REQUEST,
            Error::Octocrab(_) => actix_web::http::StatusCode::BAD_GATEWAY,
            Error::RequestError(_) => actix_web::http::StatusCode::BAD_GATEWAY,
            Error::TokenExchangeError { .. } => actix_web::http::StatusCode::UNAUTHORIZED,
            Error::SessionError { .. } => actix_web::http::StatusCode::UNAUTHORIZED,
            Error::CookieParseError { .. } => actix_web::http::StatusCode::BAD_REQUEST,
            Error::Database(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::PoolError(_) | Error::Bb8(_) => {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            },
            Error::IoError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            Error::MissingData { .. } => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        let response = self.to_error_response();
        actix_web::HttpResponse::build(self.status_code()).json(response)
    }
}
