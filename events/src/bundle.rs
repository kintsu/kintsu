use crate::Diagnostic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosticBundle {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<Diagnostic>,
}

impl DiagnosticBundle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty() && self.warnings.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    pub fn total_count(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }

    pub fn push_error(
        &mut self,
        diagnostic: Diagnostic,
    ) {
        self.errors.push(diagnostic);
    }

    pub fn push_warning(
        &mut self,
        diagnostic: Diagnostic,
    ) {
        self.warnings.push(diagnostic);
    }

    pub fn push(
        &mut self,
        diagnostic: Diagnostic,
    ) {
        if diagnostic.is_error() {
            self.errors.push(diagnostic);
        } else {
            self.warnings.push(diagnostic);
        }
    }

    pub fn merge(
        &mut self,
        other: DiagnosticBundle,
    ) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }

    pub fn print_to_stderr(&self) {
        for err in &self.errors {
            eprintln!("{:?}", err.to_report());
        }
        for warn in &self.warnings {
            eprintln!("{:?}", warn.to_report());
        }
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    pub fn to_json_compact(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

impl std::fmt::Display for DiagnosticBundle {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{} error(s), {} warning(s)",
            self.errors.len(),
            self.warnings.len()
        )
    }
}

impl std::error::Error for DiagnosticBundle {}

impl FromIterator<Diagnostic> for DiagnosticBundle {
    fn from_iter<T: IntoIterator<Item = Diagnostic>>(iter: T) -> Self {
        let mut bundle = DiagnosticBundle::new();
        for diagnostic in iter {
            bundle.push(diagnostic);
        }
        bundle
    }
}

impl Extend<Diagnostic> for DiagnosticBundle {
    fn extend<T: IntoIterator<Item = Diagnostic>>(
        &mut self,
        iter: T,
    ) {
        for diagnostic in iter {
            self.push(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kintsu_errors::{Category, Domain, ErrorCode, Severity};

    #[test]
    fn bundle_push_categorizes_correctly() {
        let mut bundle = DiagnosticBundle::new();

        bundle.push(Diagnostic::new(
            ErrorCode::new(Domain::TR, Category::Resolution, 1),
            "error msg",
            Severity::Error,
        ));

        bundle.push(Diagnostic::new(
            ErrorCode::new(Domain::UN, Category::Warning, 1),
            "warning msg",
            Severity::Warning,
        ));

        assert_eq!(bundle.error_count(), 1);
        assert_eq!(bundle.warning_count(), 1);
    }

    #[test]
    fn bundle_serializable() {
        let mut bundle = DiagnosticBundle::new();
        bundle.push(Diagnostic::new(
            ErrorCode::new(Domain::TR, Category::Resolution, 1),
            "test",
            Severity::Error,
        ));

        let json = bundle.to_json().unwrap();
        assert!(json.contains("errors"));
    }
}
