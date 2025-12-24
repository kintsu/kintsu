use actix_web::{ResponseError, web};
use kintsu_registry_auth::AuthorizationError;
use std::collections::HashMap;

pub(crate) mod apikey;
pub mod app;
pub mod config;
mod oauth;
pub mod principal;
pub(crate) mod resolver;
pub mod routes;
pub(crate) mod session;

pub type DbConn = web::Data<sea_orm::DatabaseConnection>;

pub use kintsu_registry_core::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("database error: {0:?}")]
    Database(#[from] kintsu_registry_db::Error),

    #[error("database connection error: {0:?}")]
    DatabaseConnect(#[from] sea_orm::DbErr),

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

    #[error("io error: {0:?}")]
    IoError(#[from] std::io::Error),

    #[error("session error: {cause}")]
    SessionError { cause: String },

    #[error("cookie parse error: {0:?}")]
    CookieParseError(#[from] actix_web::cookie::ParseError),

    #[error("authorization failure")]
    AuthorizationRequired,

    #[error("authorization error: {0}")]
    AuthorizationError(#[from] AuthorizationError),

    #[error("validation errors found")]
    ValidationErrors(#[from] validator::ValidationErrors),

    #[error("invalid packaging request: {0}")]
    PackagingError(#[from] PackagingError),

    #[error("many packaging errors: {0:?}")]
    PackagingErrors(Vec<PackagingError>),

    #[error("manifest error: {0}")]
    ManifestError(#[from] kintsu_manifests::Error),

    #[error("{0}")]
    StorageError(#[from] kintsu_registry_storage::StorageError),

    #[error("{0}")]
    CompileError(#[from] kintsu_parser::Error),

    #[error("multiple errors occurred: {0:?}")]
    Multiple(Vec<Error>),
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
                    errors: vec![],
                    error_description: Some("Validation errors found".to_string()),
                    error_uri: None,
                    validation: Some(validation),
                }
            },
            Error::AuthorizationRequired => {
                ErrorResponse::from_public_error(
                    PublicErrorType::Unauthorized,
                    Some("Authorization via Bearer token is required".to_string()),
                )
            },
            Error::ManifestError(err) => {
                ErrorResponse::from_public_error(
                    PublicErrorType::ManifestError,
                    Some(format!("Manifest error: {}", err)),
                )
            },
            Error::SessionError { cause } => {
                ErrorResponse::from_public_error(PublicErrorType::SessionError, Some(cause.clone()))
            },
            Error::CookieParseError(..) => {
                ErrorResponse::from_public_error(
                    PublicErrorType::InvalidCookie,
                    Some("Failed to parse cookie".to_string()),
                )
            },
            Error::Database(kintsu_registry_db::Error::Unauthorized(msg)) => {
                ErrorResponse::from_public_error(PublicErrorType::Unauthorized, Some(msg.clone()))
            },

            Error::Database(kintsu_registry_db::Error::InvalidToken) => {
                ErrorResponse::from_public_error(
                    PublicErrorType::InvalidToken,
                    Some("The provided token is invalid".to_string()),
                )
            },
            Error::Database(kintsu_registry_db::Error::TokenExpired) => {
                ErrorResponse::from_public_error(
                    PublicErrorType::TokenExpired,
                    Some("The provided token has expired".to_string()),
                )
            },
            Error::Database(kintsu_registry_db::Error::PackageVersionExists {
                package,
                version,
            }) => {
                ErrorResponse::from_public_error(
                    PublicErrorType::Validation,
                    Some(format!(
                        "Package {package} with version {version} already exists",
                    )),
                )
            },
            Error::Database(kintsu_registry_db::Error::Validation(error)) => {
                ErrorResponse::from_public_error(PublicErrorType::Validation, Some(error.clone()))
            },
            Error::Database(kintsu_registry_db::Error::AuthorizationDenied(auth_err))
            | Error::AuthorizationError(auth_err) => {
                ErrorResponse::from_public_error(
                    PublicErrorType::Forbidden,
                    Some(auth_err.to_string()),
                )
            },
            Error::PackagingError(err) => {
                ErrorResponse::from_public_error(
                    PublicErrorType::PackagingError(err.clone()),
                    Some(format!("Packaging error: {}", err)),
                )
            },
            Error::PackagingErrors(errs) => {
                ErrorResponse::from_public_errors(
                    errs.iter()
                        .cloned()
                        .map(PublicErrorType::PackagingError)
                        .collect(),
                    Some("Multiple packaging errors found.".into()),
                )
            },
            Error::TokenExchangeError {
                error,
                error_description,
                error_uri,
            } => {
                ErrorResponse {
                    error: PublicErrorType::CodeExchangeError,
                    errors: vec![],
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
            Error::Multiple(errs) => {
                // Return the highest status code among the errors
                errs.iter()
                    .map(|e| e.status_code())
                    .max()
                    .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
            },
            Error::Json(_)
            | Error::ValidationErrors(_)
            | Error::PackagingError(_)
            | Error::PackagingErrors(_)
            | Error::ManifestError(_)
            | Error::CookieParseError(_) | Error::CompileError(_) => actix_web::http::StatusCode::BAD_REQUEST,
            Error::TokenExchangeError { .. }
            | Error::Database(kintsu_registry_db::Error::Unauthorized(..))
            | Error::SessionError { .. }
            | Error::AuthorizationRequired => actix_web::http::StatusCode::UNAUTHORIZED,
            Error::Database(kintsu_registry_db::Error::AuthorizationDenied(..)) | Error::AuthorizationError(AuthorizationError::Denied{..}) => {
                actix_web::http::StatusCode::FORBIDDEN
            },
            Error::Database(kintsu_registry_db::Error::Conflict(..))
            | Error::Database(kintsu_registry_db::Error::PackageVersionExists { .. }) => {
                actix_web::http::StatusCode::CONFLICT
            },
            Error::Octocrab(_)
            | Error::RequestError(_)
            | Error::Database(_)
            | Error::IoError(_)
            // this is actually unreachable but jic
            | Error::DatabaseConnect(_) | Error::StorageError(_)
            | Error::MissingData { .. } | Error::AuthorizationError(AuthorizationError::NotApplicable {..}) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        let response = self.to_error_response();
        actix_web::HttpResponse::build(self.status_code()).json(response)
    }
}
