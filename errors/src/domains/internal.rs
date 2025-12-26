//! Internal errors (KIN) - ERR-0014
//! Compiler bugs and internal errors (not user errors).

define_domain_errors! {
    /// Internal errors (KIN domain)
    pub enum InternalError {
        /// KIN9001: Internal error (generic)
        InternalError {
            code: (IN, Internal, 1),
            message: "internal error: {reason}",
            help: "this is a compiler bug - please report it",
            fields: { reason: String },
        },

        /// KIN9002: Failed to create namespace context
        FailedToCreateNamespaceCtx {
            code: (IN, Internal, 2),
            message: "failed to create namespace context",
            help: "this is a compiler bug - please report it",
        },

        /// KIN9003: Unreachable code
        UnreachableCode {
            code: (IN, Internal, 3),
            message: "internal error: reached unreachable code: {location}",
            help: "this is a compiler bug - please report it",
            fields: { location: String },
        },

        /// KIN9004: Assertion failed
        AssertionFailed {
            code: (IN, Internal, 4),
            message: "internal assertion failed: {condition}",
            help: "this is a compiler bug - please report it",
            fields: { condition: String },
        },
    }
}

impl InternalError {
    pub fn internal(reason: impl Into<String>) -> Self {
        Self::InternalError {
            reason: reason.into(),
            span: None,
        }
    }

    pub fn failed_namespace_ctx() -> Self {
        Self::FailedToCreateNamespaceCtx { span: None }
    }

    pub fn unreachable(location: impl Into<String>) -> Self {
        Self::UnreachableCode {
            location: location.into(),
            span: None,
        }
    }

    pub fn assertion(condition: impl Into<String>) -> Self {
        Self::AssertionFailed {
            condition: condition.into(),
            span: None,
        }
    }
}
