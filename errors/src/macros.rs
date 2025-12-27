/// Declares a domain error enum with consistent structure.
/// Each variant includes an error code, message template, optional help text,
/// optional severity override, and optional fields.
///
/// Generated constructors return `ErrorBuilder<Unspanned, Self>` requiring
/// either `.at(span)` or `.unlocated()` before `.build()`.
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

            #[doc(hidden)]
            pub fn set_span(&mut self, new_span: $crate::Span) {
                match self {
                    $(Self::$variant { span, .. } => *span = Some(new_span),)*
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.message())
            }
        }

        impl std::error::Error for $name {}

        impl $crate::builder::DomainError for $name {
            fn into_compiler_error(self) -> $crate::CompilerError {
                $crate::CompilerError::from(self)
            }

            fn with_span(mut self, span: $crate::Span) -> Self {
                self.set_span(span);
                self
            }
        }
    };

    (@help) => { None };
    (@help $help:literal) => { Some($help) };
    (@severity) => { $crate::Severity::Error };
    (@severity $severity:ident) => { $crate::Severity::$severity };
}

#[cfg(test)]
mod tests {
    use crate::{InternalError, Severity, Span};

    // Test the generated methods using a real domain error type

    #[test]
    fn test_error_code() {
        let err = InternalError::internal("test")
            .unlocated()
            .build();
        assert_eq!(err.error_code().to_string(), "KIN9001");
    }

    #[test]
    fn test_message_with_fields() {
        let err = InternalError::internal("something failed")
            .unlocated()
            .build();
        assert_eq!(err.message(), "internal error: something failed");
    }

    #[test]
    fn test_help_text() {
        let err = InternalError::internal("test")
            .unlocated()
            .build();
        assert_eq!(
            err.help_text(),
            Some("this is a compiler bug - please report it")
        );
    }

    #[test]
    fn test_severity() {
        let err = InternalError::internal("test")
            .unlocated()
            .build();
        assert_eq!(err.severity(), Severity::Error);
    }

    #[test]
    fn test_with_span() {
        let err = InternalError::internal("test")
            .at(Span::new(10, 20))
            .build();
        assert_eq!(err.span(), Some(Span::new(10, 20)));
    }
}
