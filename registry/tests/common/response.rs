//! Test response wrapper with fluent assertions

use actix_web::{dev::ServiceResponse, http::StatusCode};
use kintsu_registry_core::{ErrorResponse, PublicErrorType};
use serde::de::DeserializeOwned;

/// Wrapper around ServiceResponse providing fluent assertions
pub struct TestResponse {
    status: StatusCode,
    body: Vec<u8>,
}

impl TestResponse {
    /// Create TestResponse from ServiceResponse
    pub(crate) async fn new(resp: ServiceResponse) -> Self {
        let status = resp.status();
        let body = actix_web::body::to_bytes(resp.into_body())
            .await
            .unwrap()
            .to_vec();
        Self { status, body }
    }

    /// Get the response status code
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Get the raw response body bytes
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Get the body as a string
    pub fn body_string(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    // Status assertions

    /// Assert status equals expected, returns self for chaining
    pub fn assert_status(
        self,
        expected: StatusCode,
    ) -> Self {
        assert_eq!(
            self.status,
            expected,
            "Expected status {expected}, got {}. Body: {}",
            self.status,
            self.body_string()
        );
        self
    }

    /// Assert status is 200 OK
    pub fn assert_ok(self) -> Self {
        self.assert_status(StatusCode::OK)
    }

    /// Assert status is 201 Created
    pub fn assert_created(self) -> Self {
        self.assert_status(StatusCode::CREATED)
    }

    /// Assert status is 204 No Content
    pub fn assert_no_content(self) -> Self {
        self.assert_status(StatusCode::NO_CONTENT)
    }

    /// Assert status is 400 Bad Request
    pub fn assert_bad_request(self) -> Self {
        self.assert_status(StatusCode::BAD_REQUEST)
    }

    /// Assert status is 401 Unauthorized
    pub fn assert_unauthorized(self) -> Self {
        self.assert_status(StatusCode::UNAUTHORIZED)
    }

    /// Assert status is 403 Forbidden
    pub fn assert_forbidden(self) -> Self {
        self.assert_status(StatusCode::FORBIDDEN)
    }

    /// Assert status is 404 Not Found
    pub fn assert_not_found(self) -> Self {
        self.assert_status(StatusCode::NOT_FOUND)
    }

    /// Assert status is 409 Conflict
    pub fn assert_conflict(self) -> Self {
        self.assert_status(StatusCode::CONFLICT)
    }

    // Body parsing

    /// Parse body as JSON, panics if parsing fails
    pub fn json<T: DeserializeOwned>(self) -> T {
        serde_json::from_slice(&self.body).unwrap_or_else(|e| {
            panic!(
                "Failed to parse response body as JSON: {}. Body: {}",
                e,
                self.body_string()
            )
        })
    }

    /// Parse body as ErrorResponse
    pub fn error_response(self) -> ErrorResponse {
        self.json()
    }

    // Error assertions

    /// Assert error type matches expected
    pub fn assert_error_type(
        self,
        expected: PublicErrorType,
    ) -> Self {
        let err: ErrorResponse = serde_json::from_slice(&self.body).unwrap_or_else(|e| {
            panic!(
                "Failed to parse error response: {}. Body: {}",
                e,
                self.body_string()
            )
        });
        assert_eq!(
            format!("{:?}", err.error),
            format!("{:?}", expected),
            "Expected error type {:?}, got {:?}",
            expected,
            err.error
        );
        self
    }

    /// Assert response contains a validation error for the specified field
    pub fn assert_validation_error(
        self,
        field: &str,
    ) -> Self {
        let err: ErrorResponse = serde_json::from_slice(&self.body).unwrap_or_else(|e| {
            panic!(
                "Failed to parse error response: {}. Body: {}",
                e,
                self.body_string()
            )
        });
        assert!(
            err.validation
                .as_ref()
                .map(|v| v.contains_key(field))
                .unwrap_or(false),
            "Expected validation error for field '{}', but validation map was: {:?}",
            field,
            err.validation
        );
        self
    }

    /// Assert response is an unauthorized error
    pub fn assert_unauthorized_error(self) -> Self {
        self.assert_unauthorized()
            .assert_error_type(PublicErrorType::Unauthorized)
    }

    /// Assert response is a forbidden error
    pub fn assert_forbidden_error(self) -> Self {
        self.assert_forbidden()
            .assert_error_type(PublicErrorType::Forbidden)
    }

    /// Assert response is an invalid token error
    pub fn assert_invalid_token_error(self) -> Self {
        self.assert_unauthorized()
            .assert_error_type(PublicErrorType::InvalidToken)
    }

    /// Assert response is a token expired error
    pub fn assert_token_expired_error(self) -> Self {
        self.assert_unauthorized()
            .assert_error_type(PublicErrorType::TokenExpired)
    }

    /// Assert error description contains expected substring
    pub fn assert_error_contains(
        self,
        substring: &str,
    ) -> Self {
        let err: ErrorResponse = serde_json::from_slice(&self.body).unwrap_or_else(|e| {
            panic!(
                "Failed to parse error response: {}. Body: {}",
                e,
                self.body_string()
            )
        });
        let desc = err.error_description.unwrap_or_default();
        assert!(
            desc.contains(substring),
            "Expected error description to contain '{}', but got: {}",
            substring,
            desc
        );
        self
    }
}

impl std::fmt::Debug for TestResponse {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("TestResponse")
            .field("status", &self.status)
            .field("body", &self.body_string())
            .finish()
    }
}
