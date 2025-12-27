use crate::Diagnostic;
use std::io::Write;

#[derive(Debug, thiserror::Error)]
pub enum ReporterError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub trait DiagnosticReporter: Send + Sync {
    fn emit(
        &self,
        diagnostic: &Diagnostic,
    ) -> Result<(), ReporterError>;

    fn flush(&self) -> Result<(), ReporterError> {
        Ok(())
    }
}

pub struct StderrReporter;

impl DiagnosticReporter for StderrReporter {
    fn emit(
        &self,
        diagnostic: &Diagnostic,
    ) -> Result<(), ReporterError> {
        eprintln!("{:?}", diagnostic.to_report());
        Ok(())
    }

    fn flush(&self) -> Result<(), ReporterError> {
        std::io::stderr().flush()?;
        Ok(())
    }
}

pub struct NoOpReporter;

impl DiagnosticReporter for NoOpReporter {
    fn emit(
        &self,
        _: &Diagnostic,
    ) -> Result<(), ReporterError> {
        Ok(())
    }
}

pub struct JsonLinesReporter<W: Write + Send + Sync> {
    writer: std::sync::Mutex<W>,
}

impl<W: Write + Send + Sync> JsonLinesReporter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: std::sync::Mutex::new(writer),
        }
    }
}

impl<W: Write + Send + Sync> DiagnosticReporter for JsonLinesReporter<W> {
    fn emit(
        &self,
        diagnostic: &Diagnostic,
    ) -> Result<(), ReporterError> {
        let json = serde_json::to_string(diagnostic)?;
        let mut w = self.writer.lock().unwrap();
        writeln!(w, "{}", json)?;
        Ok(())
    }

    fn flush(&self) -> Result<(), ReporterError> {
        let mut w = self.writer.lock().unwrap();
        w.flush()?;
        Ok(())
    }
}

pub struct CollectingReporter {
    diagnostics: std::sync::Mutex<Vec<Diagnostic>>,
}

impl CollectingReporter {
    pub fn new() -> Self {
        Self {
            diagnostics: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn take(&self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics.lock().unwrap())
    }
}

impl Default for CollectingReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticReporter for CollectingReporter {
    fn emit(
        &self,
        diagnostic: &Diagnostic,
    ) -> Result<(), ReporterError> {
        self.diagnostics
            .lock()
            .unwrap()
            .push(diagnostic.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kintsu_errors::{Category, Domain, ErrorCode, Severity};

    #[test]
    fn collecting_reporter_collects() {
        let reporter = CollectingReporter::new();

        reporter
            .emit(&Diagnostic::new(
                ErrorCode::new(Domain::TR, Category::Resolution, 1),
                "test",
                Severity::Error,
            ))
            .unwrap();

        let collected = reporter.take();
        assert_eq!(collected.len(), 1);
    }

    #[test]
    fn json_lines_reporter_formats() {
        let mut buffer = Vec::new();
        {
            let reporter = JsonLinesReporter::new(&mut buffer);
            reporter
                .emit(&Diagnostic::new(
                    ErrorCode::new(Domain::TR, Category::Resolution, 1),
                    "test",
                    Severity::Error,
                ))
                .unwrap();
        }

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("KTR1001"));
        assert!(output.ends_with('\n'));
    }
}
