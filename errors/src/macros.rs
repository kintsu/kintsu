/// Declares a domain error enum with consistent structure.
/// Each variant includes an error code, message template, optional help text,
/// optional severity override, and optional fields.
#[macro_export]
macro_rules! define_domain_errors {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident {
                    code: ($domain:ident, $category:ident, $seq:literal),
                    message: $msg:literal
                    $(, help: $help:literal)?
                    $(, severity: $severity:ident)?
                    $(, fields: { $($field:ident: $ftype:ty),* $(,)? })?
                    $(,)?
                }
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        $vis enum $name {
            $(
                $(#[$variant_meta])*
                $variant {
                    $($($field: $ftype,)*)?
                    span: Option<$crate::Span>,
                },
            )*
        }

        impl $name {
            pub const fn error_code(&self) -> $crate::ErrorCode {
                match self {
                    $(
                        Self::$variant { .. } => $crate::ErrorCode::new(
                            $crate::Domain::$domain,
                            $crate::Category::$category,
                            $seq
                        ),
                    )*
                }
            }

            pub fn message(&self) -> String {
                match self {
                    $(
                        Self::$variant { $($($field,)*)? .. } => {
                            format!($msg $(, $($field = $field),*)?)
                        }
                    )*
                }
            }

            pub fn help_text(&self) -> Option<&'static str> {
                match self {
                    $(
                        Self::$variant { .. } => $crate::define_domain_errors!(@help $($help)?),
                    )*
                }
            }

            pub fn severity(&self) -> $crate::Severity {
                match self {
                    $(
                        Self::$variant { .. } => $crate::define_domain_errors!(@severity $($severity)?),
                    )*
                }
            }

            pub fn span(&self) -> Option<$crate::Span> {
                match self {
                    $(Self::$variant { span, .. } => *span,)*
                }
            }

            pub fn with_span(mut self, new_span: $crate::Span) -> Self {
                match &mut self {
                    $(Self::$variant { span, .. } => *span = Some(new_span),)*
                }
                self
            }

            pub fn with_span_opt(mut self, new_span: Option<$crate::Span>) -> Self {
                if let Some(s) = new_span {
                    match &mut self {
                        $(Self::$variant { span, .. } => *span = Some(s),)*
                    }
                }
                self
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.message())
            }
        }

        impl std::error::Error for $name {}
    };

    (@help) => { None };
    (@help $help:literal) => { Some($help) };
    (@severity) => { $crate::Severity::Error };
    (@severity $severity:ident) => { $crate::Severity::$severity };
}

#[cfg(test)]
mod tests {
    use crate::{Category, Domain, ErrorCode, Severity, Span};

    define_domain_errors! {
        /// Test error enum
        pub enum TestError {
            /// Simple error with no fields
            SimpleError {
                code: (TR, Resolution, 1),
                message: "simple error occurred",
            },

            /// Error with fields
            WithFields {
                code: (TR, Resolution, 2),
                message: "type '{name}' not found",
                help: "check the type name",
                fields: { name: String },
            },

            /// Warning severity
            SomeWarning {
                code: (TR, Warning, 1),
                message: "this is a warning",
                severity: Warning,
            },
        }
    }

    #[test]
    fn test_error_code() {
        let err = TestError::SimpleError { span: None };
        assert_eq!(err.error_code().to_string(), "KTR1001");
    }

    #[test]
    fn test_message_with_fields() {
        let err = TestError::WithFields {
            name: "User".to_string(),
            span: None,
        };
        assert_eq!(err.message(), "type 'User' not found");
    }

    #[test]
    fn test_help_text() {
        let err1 = TestError::SimpleError { span: None };
        assert_eq!(err1.help_text(), None);

        let err2 = TestError::WithFields {
            name: "X".to_string(),
            span: None,
        };
        assert_eq!(err2.help_text(), Some("check the type name"));
    }

    #[test]
    fn test_severity() {
        let err1 = TestError::SimpleError { span: None };
        assert_eq!(err1.severity(), Severity::Error);

        let err2 = TestError::SomeWarning { span: None };
        assert_eq!(err2.severity(), Severity::Warning);
    }

    #[test]
    fn test_with_span() {
        let err = TestError::SimpleError { span: None };
        let err = err.with_span(Span::new(10, 20));
        assert_eq!(err.span(), Some(Span::new(10, 20)));
    }
}
