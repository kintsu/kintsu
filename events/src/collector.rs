use crate::{Diagnostic, DiagnosticBundle, reporter::DiagnosticReporter};
use actix::{Actor, Context, Handler, Message, MessageResult, Supervised};
use std::sync::Arc;

#[derive(Message)]
#[rtype(result = "()")]
pub struct EmitDiagnostic(pub Diagnostic);

#[derive(Message)]
#[rtype(result = "()")]
pub struct EmitBatch(pub Vec<Diagnostic>);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Flush;

#[derive(Message)]
#[rtype(result = "DiagnosticBundle")]
pub struct TakeBundle;

pub struct DiagnosticCollector {
    bundle: DiagnosticBundle,
    reporters: Vec<Arc<dyn DiagnosticReporter>>,
}

impl DiagnosticCollector {
    pub fn new(reporters: Vec<Box<dyn DiagnosticReporter>>) -> Self {
        Self {
            bundle: DiagnosticBundle::new(),
            reporters: reporters
                .into_iter()
                .map(Arc::from)
                .collect(),
        }
    }

    fn emit_to_reporters(
        &self,
        diagnostic: &Diagnostic,
    ) {
        for reporter in &self.reporters {
            if let Err(e) = reporter.emit(diagnostic) {
                tracing::error!("reporter emit failed: {}", e);
            }
        }
    }

    fn flush_reporters(&self) {
        for reporter in &self.reporters {
            if let Err(e) = reporter.flush() {
                tracing::error!("reporter flush failed: {}", e);
            }
        }
    }
}

impl Actor for DiagnosticCollector {
    type Context = Context<Self>;

    fn started(
        &mut self,
        _ctx: &mut Self::Context,
    ) {
        tracing::debug!("DiagnosticCollector started");
    }

    fn stopped(
        &mut self,
        _ctx: &mut Self::Context,
    ) {
        tracing::debug!(
            "DiagnosticCollector stopped with {} errors, {} warnings",
            self.bundle.error_count(),
            self.bundle.warning_count()
        );
    }
}

impl Supervised for DiagnosticCollector {}

impl Handler<EmitDiagnostic> for DiagnosticCollector {
    type Result = ();

    fn handle(
        &mut self,
        msg: EmitDiagnostic,
        _ctx: &mut Self::Context,
    ) {
        let diagnostic = msg.0;
        self.emit_to_reporters(&diagnostic);
        self.bundle.push(diagnostic);
    }
}

impl Handler<EmitBatch> for DiagnosticCollector {
    type Result = ();

    fn handle(
        &mut self,
        msg: EmitBatch,
        _ctx: &mut Self::Context,
    ) {
        for diagnostic in msg.0 {
            self.emit_to_reporters(&diagnostic);
            self.bundle.push(diagnostic);
        }
    }
}

impl Handler<TakeBundle> for DiagnosticCollector {
    type Result = MessageResult<TakeBundle>;

    fn handle(
        &mut self,
        _msg: TakeBundle,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        MessageResult(std::mem::take(&mut self.bundle))
    }
}

impl Handler<Flush> for DiagnosticCollector {
    type Result = ();

    fn handle(
        &mut self,
        _msg: Flush,
        _ctx: &mut Self::Context,
    ) {
        self.flush_reporters();
    }
}
