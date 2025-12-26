//! Kintsu Registry Error System
//!
//! Registry-specific errors with PII isolation and API documentation support.
//!
//! # Design Principles
//!
//! - **PII Isolation**: Internal error details are never exposed to clients
//! - **Logging Safe**: Structured logging with separate client/internal messages
//! - **API Documentation**: All errors derive ToSchema for OpenAPI docs
//!
//! # Example
//!
//! ```
//! use kintsu_registry_errors::{ErrorResponse, RegistryError};
//!
//! let err = RegistryError::package_not_found("my-package");
//! assert_eq!(err.error_code().to_string(), "KRG1001");
//!
//! // Convert to API response (never exposes internal details)
//! #[cfg(feature = "api")]
//! let response: ErrorResponse = (&err).into();
//! ```

use kintsu_errors::{Category, Domain, ErrorCode, Severity};

#[cfg(feature = "api")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "api")]
use utoipa::ToSchema;

/// Registry operation errors.
///
/// These errors implement PII isolation - internal details are logged
/// but never sent to clients.
#[derive(Debug, Clone)]
pub enum RegistryError {
    /// KRG1001: Package not found
    PackageNotFound { name: String },

    /// KRG1002: Version not found
    VersionNotFound { name: String, version: String },

    /// KRG2001: Authentication failed
    AuthenticationFailed {
        reason: String,
        internal: Option<String>,
    },

    /// KRG2002: Authorisation failed
    AuthorisationFailed {
        action: String,
        name: String,
        internal: Option<String>,
    },

    /// KRG2003: Invalid package name
    InvalidPackageName { name: String, reason: String },

    /// KRG3001: Version already exists
    VersionAlreadyExists { name: String, version: String },

    /// KRG9001: Network error
    NetworkError {
        reason: String,
        internal: Option<String>,
    },

    /// KRG9002: Registry unavailable
    RegistryUnavailable {
        reason: String,
        internal: Option<String>,
    },
}

impl RegistryError {
    /// Returns the error code for this error.
    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::PackageNotFound { .. } => ErrorCode::new(Domain::RG, Category::Resolution, 1),
            Self::VersionNotFound { .. } => ErrorCode::new(Domain::RG, Category::Resolution, 2),
            Self::AuthenticationFailed { .. } => {
                ErrorCode::new(Domain::RG, Category::Validation, 1)
            },
            Self::AuthorisationFailed { .. } => ErrorCode::new(Domain::RG, Category::Validation, 2),
            Self::InvalidPackageName { .. } => ErrorCode::new(Domain::RG, Category::Validation, 3),
            Self::VersionAlreadyExists { .. } => ErrorCode::new(Domain::RG, Category::Conflict, 1),
            Self::NetworkError { .. } => ErrorCode::new(Domain::RG, Category::Internal, 1),
            Self::RegistryUnavailable { .. } => ErrorCode::new(Domain::RG, Category::Internal, 2),
        }
    }

    /// Returns the public error message safe for clients.
    pub fn public_message(&self) -> String {
        match self {
            Self::PackageNotFound { name } => {
                format!("package '{name}' not found in registry")
            },
            Self::VersionNotFound { name, version } => {
                format!("version '{version}' of package '{name}' not found")
            },
            Self::AuthenticationFailed { reason, .. } => {
                format!("authentication failed: {reason}")
            },
            Self::AuthorisationFailed { action, name, .. } => {
                format!("not authorised to {action} package '{name}'")
            },
            Self::InvalidPackageName { name, reason } => {
                format!("invalid package name '{name}': {reason}")
            },
            Self::VersionAlreadyExists { name, version } => {
                format!("version '{version}' of package '{name}' already exists")
            },
            Self::NetworkError { reason, .. } => {
                format!("network error: {reason}")
            },
            Self::RegistryUnavailable { reason, .. } => {
                format!("registry unavailable: {reason}")
            },
        }
    }

    /// Returns the internal error details for logging (may contain PII).
    pub fn internal_details(&self) -> Option<&str> {
        match self {
            Self::AuthenticationFailed { internal, .. }
            | Self::AuthorisationFailed { internal, .. }
            | Self::NetworkError { internal, .. }
            | Self::RegistryUnavailable { internal, .. } => internal.as_deref(),
            _ => None,
        }
    }

    /// Returns the error severity.
    pub fn severity(&self) -> Severity {
        Severity::Error
    }

    /// Returns help text if available.
    pub fn help_text(&self) -> Option<&'static str> {
        match self {
            Self::PackageNotFound { .. } => {
                Some("check the package name spelling or verify the package exists in the registry")
            },
            Self::VersionNotFound { .. } => {
                Some("check available versions or relax the version constraint")
            },
            Self::AuthenticationFailed { .. } => Some("run `kintsu login` to authenticate"),
            Self::AuthorisationFailed { .. } => {
                Some("request access from the package owner or organisation administrator")
            },
            Self::InvalidPackageName { .. } => {
                Some("package names must be lowercase alphanumeric with hyphens only")
            },
            Self::VersionAlreadyExists { .. } => {
                Some("increment the version number before publishing")
            },
            Self::NetworkError { .. } => Some("check your network connection and try again"),
            Self::RegistryUnavailable { .. } => {
                Some("the registry service may be experiencing issues; try again later")
            },
        }
    }

    /// Returns a kebab-case error slug for API responses.
    pub fn error_slug(&self) -> &'static str {
        match self {
            Self::PackageNotFound { .. } => "package-not-found",
            Self::VersionNotFound { .. } => "version-not-found",
            Self::AuthenticationFailed { .. } => "authentication-failed",
            Self::AuthorisationFailed { .. } => "authorisation-failed",
            Self::InvalidPackageName { .. } => "invalid-package-name",
            Self::VersionAlreadyExists { .. } => "version-already-exists",
            Self::NetworkError { .. } => "network-error",
            Self::RegistryUnavailable { .. } => "registry-unavailable",
        }
    }

    // Constructors

    pub fn package_not_found(name: impl Into<String>) -> Self {
        Self::PackageNotFound { name: name.into() }
    }

    pub fn version_not_found(
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self::VersionNotFound {
            name: name.into(),
            version: version.into(),
        }
    }

    pub fn authentication_failed(reason: impl Into<String>) -> Self {
        Self::AuthenticationFailed {
            reason: reason.into(),
            internal: None,
        }
    }

    pub fn authentication_failed_with_details(
        reason: impl Into<String>,
        internal: impl Into<String>,
    ) -> Self {
        Self::AuthenticationFailed {
            reason: reason.into(),
            internal: Some(internal.into()),
        }
    }

    pub fn authorisation_failed(
        action: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self::AuthorisationFailed {
            action: action.into(),
            name: name.into(),
            internal: None,
        }
    }

    pub fn authorisation_failed_with_details(
        action: impl Into<String>,
        name: impl Into<String>,
        internal: impl Into<String>,
    ) -> Self {
        Self::AuthorisationFailed {
            action: action.into(),
            name: name.into(),
            internal: Some(internal.into()),
        }
    }

    pub fn invalid_package_name(
        name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvalidPackageName {
            name: name.into(),
            reason: reason.into(),
        }
    }

    pub fn version_already_exists(
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self::VersionAlreadyExists {
            name: name.into(),
            version: version.into(),
        }
    }

    pub fn network_error(reason: impl Into<String>) -> Self {
        Self::NetworkError {
            reason: reason.into(),
            internal: None,
        }
    }

    pub fn network_error_with_details(
        reason: impl Into<String>,
        internal: impl Into<String>,
    ) -> Self {
        Self::NetworkError {
            reason: reason.into(),
            internal: Some(internal.into()),
        }
    }

    pub fn registry_unavailable(reason: impl Into<String>) -> Self {
        Self::RegistryUnavailable {
            reason: reason.into(),
            internal: None,
        }
    }

    pub fn registry_unavailable_with_details(
        reason: impl Into<String>,
        internal: impl Into<String>,
    ) -> Self {
        Self::RegistryUnavailable {
            reason: reason.into(),
            internal: Some(internal.into()),
        }
    }
}

impl std::fmt::Display for RegistryError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.public_message())
    }
}

impl std::error::Error for RegistryError {}

// API Response Types

/// Standard error response for API endpoints.
///
/// This struct is designed for OpenAPI documentation and never exposes
/// internal error details.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "api", derive(Serialize, Deserialize, ToSchema))]
pub struct ErrorResponse {
    /// Error type slug (kebab-case)
    pub error: String,

    /// Kintsu error code (K[XX][C][NNN])
    pub error_code: String,

    /// Human-readable description (safe for client display)
    pub error_description: String,

    /// Optional help text
    #[cfg_attr(feature = "api", serde(skip_serializing_if = "Option::is_none"))]
    pub help: Option<String>,

    /// Severity level
    pub severity: String,
}

impl From<&RegistryError> for ErrorResponse {
    fn from(err: &RegistryError) -> Self {
        Self {
            error: err.error_slug().to_string(),
            error_code: err.error_code().to_string(),
            error_description: err.public_message(),
            help: err.help_text().map(String::from),
            severity: err.severity().to_string(),
        }
    }
}

impl From<RegistryError> for ErrorResponse {
    fn from(err: RegistryError) -> Self {
        Self::from(&err)
    }
}

/// Type alias for Results using RegistryError.
pub type Result<T, E = RegistryError> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes() {
        assert_eq!(
            RegistryError::package_not_found("x")
                .error_code()
                .to_string(),
            "KRG1001"
        );
        assert_eq!(
            RegistryError::version_not_found("x", "1.0")
                .error_code()
                .to_string(),
            "KRG1002"
        );
        assert_eq!(
            RegistryError::authentication_failed("x")
                .error_code()
                .to_string(),
            "KRG2001"
        );
        assert_eq!(
            RegistryError::authorisation_failed("publish", "x")
                .error_code()
                .to_string(),
            "KRG2002"
        );
        assert_eq!(
            RegistryError::invalid_package_name("x", "y")
                .error_code()
                .to_string(),
            "KRG2003"
        );
        assert_eq!(
            RegistryError::version_already_exists("x", "1.0")
                .error_code()
                .to_string(),
            "KRG3001"
        );
        assert_eq!(
            RegistryError::network_error("x")
                .error_code()
                .to_string(),
            "KRG9001"
        );
        assert_eq!(
            RegistryError::registry_unavailable("x")
                .error_code()
                .to_string(),
            "KRG9002"
        );
    }

    #[test]
    fn public_messages() {
        let err = RegistryError::package_not_found("my-pkg");
        assert_eq!(
            err.public_message(),
            "package 'my-pkg' not found in registry"
        );
    }

    #[test]
    fn internal_details_isolation() {
        let err = RegistryError::authentication_failed_with_details(
            "invalid token",
            "user_id=12345, ip=192.168.1.1",
        );
        assert_eq!(err.public_message(), "authentication failed: invalid token");
        assert_eq!(
            err.internal_details(),
            Some("user_id=12345, ip=192.168.1.1")
        );
    }

    #[test]
    fn error_response_conversion() {
        let err = RegistryError::package_not_found("test-pkg");
        let response = ErrorResponse::from(&err);
        assert_eq!(response.error, "package-not-found");
        assert_eq!(response.error_code, "KRG1001");
        assert!(
            response
                .error_description
                .contains("test-pkg")
        );
    }
}
