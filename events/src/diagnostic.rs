use kintsu_errors::{CompilerError, DiagnosticBuilder, ErrorCode, Severity, Span};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Serializable span representation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpanRepr {
    start: usize,
    end: usize,
}

impl From<Span> for SpanRepr {
    fn from(s: Span) -> Self {
        Self {
            start: s.start,
            end: s.end,
        }
    }
}

impl From<SpanRepr> for Span {
    fn from(s: SpanRepr) -> Self {
        Self::new(s.start, s.end)
    }
}

/// Serializable severity representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SeverityRepr {
    Error,
    Warning,
    Info,
    Hint,
}

impl From<Severity> for SeverityRepr {
    fn from(s: Severity) -> Self {
        match s {
            Severity::Error => Self::Error,
            Severity::Warning => Self::Warning,
            Severity::Info => Self::Info,
            Severity::Hint => Self::Hint,
        }
    }
}

impl From<SeverityRepr> for Severity {
    fn from(s: SeverityRepr) -> Self {
        match s {
            SeverityRepr::Error => Self::Error,
            SeverityRepr::Warning => Self::Warning,
            SeverityRepr::Info => Self::Info,
            SeverityRepr::Hint => Self::Hint,
        }
    }
}

/// Serializable diagnostic for JSON output
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagnosticRepr {
    code: String,
    message: String,
    severity: SeverityRepr,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<SpanRepr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    help: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    labels: Vec<DiagnosticLabelRepr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagnosticLabelRepr {
    span: SpanRepr,
    message: String,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: ErrorCode,
    pub message: String,
    pub severity: Severity,
    pub span: Option<Span>,
    pub source_name: Option<String>,
    pub source_content: Option<Arc<str>>,
    pub help: Option<String>,
    pub labels: Vec<DiagnosticLabel>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub message: String,
}

impl Serialize for Diagnostic {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer, {
        let repr = DiagnosticRepr {
            code: self.code.to_string(),
            message: self.message.clone(),
            severity: self.severity.into(),
            span: self.span.map(|s| s.into()),
            source_name: self.source_name.clone(),
            help: self.help.clone(),
            labels: self
                .labels
                .iter()
                .map(|l| {
                    DiagnosticLabelRepr {
                        span: l.span.into(),
                        message: l.message.clone(),
                    }
                })
                .collect(),
        };
        repr.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Diagnostic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>, {
        use serde::de::Error;

        let repr = DiagnosticRepr::deserialize(deserializer)?;

        // Parse error code from string like "KTR1001"
        let code = parse_error_code(&repr.code)
            .ok_or_else(|| D::Error::custom(format!("invalid error code: {}", repr.code)))?;

        Ok(Self {
            code,
            message: repr.message,
            severity: repr.severity.into(),
            span: repr.span.map(|s| s.into()),
            source_name: repr.source_name,
            source_content: None, // source content not serialized
            help: repr.help,
            labels: repr
                .labels
                .into_iter()
                .map(|l| {
                    DiagnosticLabel {
                        span: l.span.into(),
                        message: l.message,
                    }
                })
                .collect(),
        })
    }
}

fn parse_error_code(s: &str) -> Option<ErrorCode> {
    use kintsu_errors::{Category, Domain};

    if !s.starts_with('K') || s.len() < 5 {
        return None;
    }

    // Domain is 2 chars after K
    let domain_str = s.get(1..3)?;
    let domain = match domain_str {
        "LX" => Domain::LX,
        "PR" => Domain::PR,
        "NS" => Domain::NS,
        "TY" => Domain::TY,
        "TR" => Domain::TR,
        "UN" => Domain::UN,
        "MT" => Domain::MT,
        "TG" => Domain::TG,
        "TE" => Domain::TE,
        "PK" => Domain::PK,
        "RG" => Domain::RG,
        "FS" => Domain::FS,
        "IN" => Domain::IN,
        _ => return None,
    };

    // Category is single digit after domain
    let category_char = s.chars().nth(3)?;
    let category_digit = category_char.to_digit(10)?;
    let category = match category_digit {
        0 => Category::Syntax,
        1 => Category::Resolution,
        2 => Category::Validation,
        3 => Category::Conflict,
        4 => Category::Missing,
        5 => Category::Cycle,
        6 => Category::Compatibility,
        7 => Category::Reserved,
        8 => Category::Warning,
        9 => Category::Internal,
        _ => return None,
    };

    // Sequence is remaining digits
    let sequence: u16 = s.get(4..)?.parse().ok()?;

    Some(ErrorCode::new(domain, category, sequence))
}

impl Diagnostic {
    pub fn new(
        code: ErrorCode,
        message: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            severity,
            span: None,
            source_name: None,
            source_content: None,
            help: None,
            labels: Vec::new(),
        }
    }

    pub fn with_span(
        mut self,
        span: Span,
    ) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_source(
        mut self,
        name: impl Into<String>,
        content: impl Into<Arc<str>>,
    ) -> Self {
        self.source_name = Some(name.into());
        self.source_content = Some(content.into());
        self
    }

    pub fn with_help(
        mut self,
        help: impl Into<String>,
    ) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn add_label(
        mut self,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        self.labels.push(DiagnosticLabel {
            span,
            message: message.into(),
        });
        self
    }

    pub fn to_report(&self) -> miette::Report {
        let mut builder = DiagnosticBuilder::new(self.code, &self.message, self.severity);

        if let Some(span) = &self.span {
            builder = builder.span(*span);
        }

        if let (Some(name), Some(content)) = (&self.source_name, &self.source_content) {
            builder = builder.source(name, content.as_ref());
        }

        if let Some(help) = &self.help {
            builder = builder.help(help.as_str());
        }

        // Add secondary labels for multi-location highlighting
        for label in &self.labels {
            builder = builder.secondary_label(label.span, &label.message);
        }

        builder.into_report()
    }

    pub fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }

    pub fn is_warning(&self) -> bool {
        matches!(self.severity, Severity::Warning)
    }
}

impl From<CompilerError> for Diagnostic {
    fn from(err: CompilerError) -> Self {
        let (source_name, source_content) = err
            .extract_source()
            .map(|(p, s)| (Some(p.display().to_string()), Some(Arc::from(s))))
            .unwrap_or((None, None));

        // Extract secondary labels from the error
        let labels = err
            .extract_secondary_labels()
            .into_iter()
            .map(|(span, message)| DiagnosticLabel { span, message })
            .collect();

        Self {
            code: err.error_code(),
            message: err.message(),
            severity: err.severity(),
            span: err.extract_deepest_span(),
            source_name,
            source_content,
            help: err.help_text().map(String::from),
            labels,
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kintsu_errors::Category;

    #[test]
    fn diagnostic_serializable() {
        let diag = Diagnostic::new(
            ErrorCode::new(kintsu_errors::Domain::TR, Category::Resolution, 1),
            "undefined type",
            Severity::Error,
        );

        let json = serde_json::to_string(&diag).unwrap();
        assert!(json.contains("KTR1001"));
        assert!(json.contains("undefined type"));
    }
}
