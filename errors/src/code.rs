use std::fmt;

/// Two-letter domain identifier per ERR-0001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    /// Lexical errors (KLX) - ERR-0002
    LX,
    /// Parsing errors (KPR) - ERR-0003
    PR,
    /// Namespace errors (KNS) - ERR-0004
    NS,
    /// Type definition errors (KTY) - ERR-0005
    TY,
    /// Type resolution errors (KTR) - ERR-0006
    TR,
    /// Union errors (KUN) - ERR-0007
    UN,
    /// Metadata errors (KMT) - ERR-0008
    MT,
    /// Tagging errors (KTG) - ERR-0009
    TG,
    /// Type expression errors (KTE) - ERR-0010
    TE,
    /// Package errors (KPK) - ERR-0011
    PK,
    /// Registry errors (KRG) - ERR-0012
    RG,
    /// Filesystem errors (KFS) - ERR-0013
    FS,
    /// Internal errors (KIN) - ERR-0014
    IN,
}

impl Domain {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::LX => "LX",
            Self::PR => "PR",
            Self::NS => "NS",
            Self::TY => "TY",
            Self::TR => "TR",
            Self::UN => "UN",
            Self::MT => "MT",
            Self::TG => "TG",
            Self::TE => "TE",
            Self::PK => "PK",
            Self::RG => "RG",
            Self::FS => "FS",
            Self::IN => "IN",
        }
    }
}

impl fmt::Display for Domain {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Category digit (0-9) per ERR-0001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Category {
    /// Syntax errors (malformed input)
    Syntax = 0,
    /// Resolution errors (failed lookups)
    Resolution = 1,
    /// Validation errors (constraint violations)
    Validation = 2,
    /// Conflict errors (duplicate definitions)
    Conflict = 3,
    /// Missing errors (required element absent)
    Missing = 4,
    /// Cycle errors (circular dependencies)
    Cycle = 5,
    /// Compatibility errors (version mismatches)
    Compatibility = 6,
    /// Reserved for future use
    Reserved = 7,
    /// Warning (non-fatal issues)
    Warning = 8,
    /// Internal errors (compiler bugs)
    Internal = 9,
}

impl Category {
    pub const fn as_digit(&self) -> u8 {
        *self as u8
    }
}

impl fmt::Display for Category {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_digit())
    }
}

/// Compositional error code - K[Domain][Category][Seq]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode {
    pub domain: Domain,
    pub category: Category,
    pub sequence: u16,
}

impl ErrorCode {
    pub const fn new(
        domain: Domain,
        category: Category,
        sequence: u16,
    ) -> Self {
        Self {
            domain,
            category,
            sequence,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "K{}{}{:03}",
            self.domain,
            self.category.as_digit(),
            self.sequence
        )
    }
}

/// Error severity levels per RFC-0023
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Severity {
    /// Compilation fails
    #[default]
    Error,
    /// Compilation succeeds, shown by default
    Warning,
    /// Shown only with --verbose
    Info,
    /// IDE-only hints
    Hint,
}

impl Severity {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        }
    }

    pub const fn is_fatal(&self) -> bool {
        matches!(self, Self::Error)
    }
}

impl fmt::Display for Severity {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_display() {
        let code = ErrorCode::new(Domain::TR, Category::Resolution, 1);
        assert_eq!(code.to_string(), "KTR1001");

        let code = ErrorCode::new(Domain::LX, Category::Syntax, 1);
        assert_eq!(code.to_string(), "KLX0001");

        let code = ErrorCode::new(Domain::IN, Category::Internal, 1);
        assert_eq!(code.to_string(), "KIN9001");
    }

    #[test]
    fn severity_properties() {
        assert!(Severity::Error.is_fatal());
        assert!(!Severity::Warning.is_fatal());
        assert!(!Severity::Info.is_fatal());
        assert!(!Severity::Hint.is_fatal());
    }
}
