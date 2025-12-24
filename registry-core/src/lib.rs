use std::collections::HashMap;
use utoipa::ToSchema;

pub mod models;

#[derive(Debug, thiserror::Error, serde::Serialize, serde::Deserialize, ToSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PackagingError {
    #[error("package data is empty")]
    EmptyPackageData,
    #[error("invalid file '{path}': {reason}")]
    InvalidFile { path: String, reason: String },
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PublicErrorType {
    InternalServerError,

    CodeExchangeError,

    SessionError,
    InvalidCookie,
    Unauthorized,
    Forbidden,
    NotFound,

    InvalidToken,
    TokenExpired,

    AuthorizationRequired,

    Validation,

    ManifestError,

    Multiple,

    PackagingError(PackagingError),
}

impl Into<&'static str> for &PublicErrorType {
    fn into(self) -> &'static str {
        match self {
            PublicErrorType::InternalServerError => "internal-server-error",
            PublicErrorType::CodeExchangeError => "code-exchange-error",
            PublicErrorType::SessionError => "session-error",
            PublicErrorType::InvalidCookie => "invalid-cookie",
            PublicErrorType::Unauthorized => "unauthorized",
            PublicErrorType::Forbidden => "forbidden",
            PublicErrorType::NotFound => "not-found",
            PublicErrorType::InvalidToken => "invalid-token",
            PublicErrorType::TokenExpired => "token-expired",
            PublicErrorType::AuthorizationRequired => "authorization-required",
            PublicErrorType::Validation => "validation-error",
            PublicErrorType::ManifestError => "manifest-error",
            PublicErrorType::Multiple => "multiple-errors",
            PublicErrorType::PackagingError(_) => "packaging-error",
        }
    }
}

impl std::fmt::Debug for PublicErrorType {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let s: &'static str = self.into();
        write!(f, "{}", s)
    }
}

impl std::fmt::Display for PublicErrorType {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let s: &'static str = self.into();
        write!(f, "{}", s)
    }
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ErrorResponse {
    pub error: PublicErrorType,
    pub errors: Vec<PublicErrorType>,
    pub error_description: Option<String>,
    pub error_uri: Option<String>,
    pub validation: Option<HashMap<String, Vec<String>>>,
}

impl ErrorResponse {
    pub fn internal() -> Self {
        Self {
            error: PublicErrorType::InternalServerError,
            errors: vec![],
            error_description: None,
            error_uri: None,
            validation: None,
        }
    }

    pub fn from_public_error(
        error: PublicErrorType,
        desc: Option<String>,
    ) -> Self {
        Self {
            error,
            errors: vec![],
            error_description: desc,
            error_uri: None,
            validation: None,
        }
    }

    pub fn from_public_errors(
        errors: Vec<PublicErrorType>,
        desc: Option<String>,
    ) -> Self {
        Self {
            error: PublicErrorType::Multiple,
            errors,
            error_description: desc,
            error_uri: None,
            validation: None,
        }
    }
}
