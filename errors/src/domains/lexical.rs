//! Lexical errors (KLX) - ERR-0002
//! Errors during tokenization of source text.

define_domain_errors! {
    /// Lexical analysis errors (KLX domain)
    pub enum LexicalError {
        /// KLX0001: Invalid character in source
        UnknownCharacter {
            code: (LX, Syntax, 1),
            message: "invalid character '{ch}'",
            help: "remove or replace this character",
            fields: { ch: char },
        },

        /// KLX0002: Integer literal parsing failed
        InvalidIntegerLiteral {
            code: (LX, Syntax, 2),
            message: "invalid integer literal: {reason}",
            help: "use a valid decimal integer",
            fields: { reason: String },
        },

        /// KLX0003: Float literal parsing failed
        InvalidFloatLiteral {
            code: (LX, Syntax, 3),
            message: "invalid float literal: {reason}",
            help: "ensure digits on both sides of decimal point",
            fields: { reason: String },
        },

        /// KLX0004: Boolean literal parsing failed
        InvalidBooleanLiteral {
            code: (LX, Syntax, 4),
            message: "invalid boolean literal: expected 'true' or 'false'",
            help: "use lowercase 'true' or 'false'",
        },

        /// KLX0005: Unterminated string literal
        UnterminatedString {
            code: (LX, Syntax, 5),
            message: "unterminated string literal",
            help: "add closing quote",
        },

        /// KLX0006: Invalid escape sequence
        InvalidEscapeSequence {
            code: (LX, Syntax, 6),
            message: "invalid escape sequence '\\{ch}'",
            help: "valid escapes: \\n, \\r, \\t, \\\\, \\\"",
            fields: { ch: char },
        },

        /// KLX0007: Empty token stream
        EmptyTokens {
            code: (LX, Syntax, 7),
            message: "empty token stream",
            help: "file contains no valid tokens",
        },

        /// KLX9001: Unknown lexing error (internal)
        UnknownLexingError {
            code: (LX, Internal, 1),
            message: "unknown lexing error: {reason}",
            fields: { reason: String },
        },
    }
}

impl LexicalError {
    pub fn unknown_char(ch: char) -> Self {
        Self::UnknownCharacter { ch, span: None }
    }

    pub fn parse_int(reason: impl Into<String>) -> Self {
        Self::InvalidIntegerLiteral {
            reason: reason.into(),
            span: None,
        }
    }

    pub fn parse_float(reason: impl Into<String>) -> Self {
        Self::InvalidFloatLiteral {
            reason: reason.into(),
            span: None,
        }
    }

    pub fn unknown(reason: impl Into<String>) -> Self {
        Self::UnknownLexingError {
            reason: reason.into(),
            span: None,
        }
    }

    /// Alias for unknown - wraps a lexing error from the tokenizer.
    pub fn lexer_error(reason: impl Into<String>) -> Self {
        Self::unknown(reason)
    }
}
