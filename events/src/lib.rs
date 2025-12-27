//! Kintsu Events - Unified diagnostic emission system
//!
//! Provides thread-safe diagnostic collection and reporting for the compiler.
//! Inspired by the `registry-events` actor pattern.

use actix::Addr;
use std::sync::RwLock;

mod bundle;
mod collector;
mod diagnostic;
mod reporter;

pub use bundle::DiagnosticBundle;
pub use collector::{DiagnosticCollector, EmitBatch, EmitDiagnostic, Flush, TakeBundle};
pub use diagnostic::{Diagnostic, DiagnosticLabel};
pub use reporter::{
    CollectingReporter, DiagnosticReporter, JsonLinesReporter, NoOpReporter, ReporterError,
    StderrReporter,
};

static DIAGNOSTIC_SYSTEM: RwLock<Option<Addr<DiagnosticCollector>>> = RwLock::new(None);

pub fn init(reporters: Vec<Box<dyn DiagnosticReporter>>) {
    use actix::Actor;

    let collector = DiagnosticCollector::new(reporters).start();
    let mut guard = DIAGNOSTIC_SYSTEM.write().unwrap();
    if guard.is_some() {
        tracing::warn!("diagnostic system already initialized, replacing");
    }
    *guard = Some(collector);
    tracing::debug!("diagnostic system initialized");
}

pub fn is_initialized() -> bool {
    DIAGNOSTIC_SYSTEM.read().unwrap().is_some()
}

pub fn emit(diagnostic: impl Into<Diagnostic>) {
    let guard = DIAGNOSTIC_SYSTEM.read().unwrap();
    if let Some(addr) = guard.as_ref() {
        addr.do_send(EmitDiagnostic(diagnostic.into()));
    } else {
        let diag = diagnostic.into();
        tracing::warn!("diagnostic system not initialized, printing directly");
        eprintln!("{:?}", diag.to_report());
    }
}

pub fn emit_error(err: impl Into<Diagnostic>) {
    emit(err);
}

pub fn emit_warning(warn: impl Into<Diagnostic>) {
    emit(warn);
}

pub fn emit_batch(diagnostics: Vec<Diagnostic>) {
    let guard = DIAGNOSTIC_SYSTEM.read().unwrap();
    if let Some(addr) = guard.as_ref() {
        addr.do_send(EmitBatch(diagnostics));
    } else {
        tracing::warn!("diagnostic system not initialized, printing directly");
        for diag in diagnostics {
            eprintln!("{:?}", diag.to_report());
        }
    }
}

#[allow(clippy::await_holding_lock)]
pub async fn take_bundle() -> DiagnosticBundle {
    let guard = DIAGNOSTIC_SYSTEM.read().unwrap();
    match guard.as_ref() {
        Some(addr) => {
            addr.send(TakeBundle)
                .await
                .unwrap_or_default()
        },
        None => DiagnosticBundle::new(),
    }
}

#[allow(clippy::await_holding_lock)]
pub async fn flush() {
    let guard = DIAGNOSTIC_SYSTEM.read().unwrap();
    if let Some(addr) = guard.as_ref() {
        let _ = addr.send(Flush).await;
    }
}

pub async fn shutdown() -> DiagnosticBundle {
    flush().await;
    let bundle = take_bundle().await;

    let mut guard = DIAGNOSTIC_SYSTEM.write().unwrap();
    *guard = None;

    tracing::debug!("diagnostic system shutdown");
    bundle
}

#[cfg(test)]
mod tests {
    use super::*;
    use kintsu_errors::{Category, Domain, ErrorCode, Severity};

    #[actix::test]
    async fn emit_collects_diagnostic() {
        // Ensure clean state from any previous tests
        if is_initialized() {
            let _ = shutdown().await;
        }

        init(vec![Box::new(NoOpReporter)]);

        emit(Diagnostic::new(
            ErrorCode::new(Domain::TR, Category::Resolution, 1),
            "test error",
            Severity::Error,
        ));

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let bundle = shutdown().await;
        assert_eq!(bundle.error_count(), 1);
    }

    #[actix::test]
    async fn bundle_separates_errors_and_warnings() {
        // Ensure clean state from any previous tests
        if is_initialized() {
            let _ = shutdown().await;
        }

        init(vec![Box::new(NoOpReporter)]);

        emit(Diagnostic::new(
            ErrorCode::new(Domain::TR, Category::Resolution, 1),
            "error",
            Severity::Error,
        ));

        emit(Diagnostic::new(
            ErrorCode::new(Domain::UN, Category::Warning, 1),
            "warning",
            Severity::Warning,
        ));

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let bundle = shutdown().await;
        assert_eq!(bundle.error_count(), 1);
        assert_eq!(bundle.warning_count(), 1);
    }
}
