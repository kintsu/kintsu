use actix_web::{ResponseError, web};
use diesel_async::pooled_connection::{PoolError, bb8::RunError};
use kintsu_registry_db::AsyncConnectionPool;
use std::collections::HashMap;
use utoipa::ToSchema;

pub(crate) mod apikey;
pub mod app;
pub mod config;
pub mod models;
mod oauth;
pub mod routes;
mod sealed;
pub(crate) mod session;

pub type DbPool = web::Data<AsyncConnectionPool>;

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

    #[error("authorization failure")]
    AuthorizationRequired,

    #[error("validation errors found")]
    ValidationErrors(#[from] validator::ValidationErrors),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(serde::Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PublicErrorType {
    InternalServerError,

    CodeExchangeError,

    SessionError,
    InvalidCookie,
    Unauthorized,

    InvalidToken,
    TokenExpired,

    AuthorizationRequired,

    Validation,
}

#[derive(serde::Serialize, ToSchema)]
struct ErrorResponse {
    error: PublicErrorType,
    error_description: Option<String>,
    error_uri: Option<String>,
    validation: Option<HashMap<String, Vec<String>>>,
}

impl ErrorResponse {
    fn internal() -> Self {
        Self {
            error: PublicErrorType::InternalServerError,
            error_description: None,
            error_uri: None,
            validation: None,
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
        tracing::error!("Handling error: {:?}", self);
        match self {
            Error::ValidationErrors(err) => {
                let mut validation = HashMap::new();

                for (field, errors) in err.field_errors().iter() {
                    let messages: Vec<String> = errors
                        .iter()
                        .map(|e| {
                            if let Some(message) = &e.message {
                                message.to_string()
                            } else {
                                format!("validation error on {}", field)
                            }
                        })
                        .collect();
                    validation.insert(field.to_string(), messages);
                }

                ErrorResponse {
                    error: PublicErrorType::Validation,
                    error_description: Some("Validation errors found".to_string()),
                    error_uri: None,
                    validation: Some(validation),
                }
            },
            Error::AuthorizationRequired => {
                ErrorResponse {
                    error: PublicErrorType::Unauthorized,
                    error_description: Some(
                        "Authorization via Bearer token is required".to_string(),
                    ),
                    error_uri: None,
                    validation: None,
                }
            },
            Error::SessionError { cause } => {
                ErrorResponse {
                    error: PublicErrorType::SessionError,
                    error_description: Some(cause.clone()),
                    error_uri: None,
                    validation: None,
                }
            },
            Error::CookieParseError(..) => {
                ErrorResponse {
                    error: PublicErrorType::InvalidCookie,
                    error_description: Some("Bad request cookies".to_string()),
                    error_uri: None,
                    validation: None,
                }
            },
            Error::Database(kintsu_registry_db::Error::Unauthorized(msg)) => {
                ErrorResponse {
                    error: PublicErrorType::Unauthorized,
                    error_description: Some(msg.clone()),
                    error_uri: None,
                    validation: None,
                }
            },

            Error::Database(kintsu_registry_db::Error::InvalidToken) => {
                ErrorResponse {
                    error: PublicErrorType::InvalidToken,
                    error_description: Some("The provided token is invalid".to_string()),
                    error_uri: None,
                    validation: None,
                }
            },
            Error::Database(kintsu_registry_db::Error::TokenExpired) => {
                ErrorResponse {
                    error: PublicErrorType::TokenExpired,
                    error_description: Some("The provided token has expired".to_string()),
                    error_uri: None,
                    validation: None,
                }
            },
            Error::Database(kintsu_registry_db::Error::Validation(error)) => {
                ErrorResponse {
                    error: PublicErrorType::Validation,
                    error_description: Some(error.clone()),
                    error_uri: None,
                    validation: None,
                }
            },
            Error::TokenExchangeError {
                error,
                error_description,
                error_uri,
            } => {
                ErrorResponse {
                    error: PublicErrorType::CodeExchangeError,
                    error_description: Some(match error_description {
                        Some(desc) => format!("{error}: {desc}"),
                        None => error.clone(),
                    }),
                    error_uri: error_uri.clone(),
                    validation: None,
                }
            },
            _ => ErrorResponse::internal(),
        }
    }
}
impl ResponseError for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Error::Json(_) | Error::ValidationErrors(_) => actix_web::http::StatusCode::BAD_REQUEST,
            Error::Octocrab(_) => actix_web::http::StatusCode::BAD_GATEWAY,
            Error::RequestError(_) => actix_web::http::StatusCode::BAD_GATEWAY,
            Error::TokenExchangeError { .. }
            | Error::Database(kintsu_registry_db::Error::Unauthorized(..))
            | Error::SessionError { .. }
            | Error::AuthorizationRequired => actix_web::http::StatusCode::UNAUTHORIZED,
            Error::Database(kintsu_registry_db::Error::Conflict(..)) => {
                actix_web::http::StatusCode::CONFLICT
            },
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
