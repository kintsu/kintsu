use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, Message, ResponseActFuture,
    Supervised, fut::wrap_future,
};
use kintsu_registry_auth::AuditEvent;
use serde_jsonlines::WriteExt;
use std::{
    future::Future,
    io::Write,
    pin::Pin,
    sync::{Arc, RwLock},
};
use tokio::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Event reporting error: {message}")]
    InternalError { message: String },
    #[error("Actor mailbox error: {0}")]
    MailboxError(String),
    #[error("Event system not initialized")]
    NotInitialized,
}

type BoxFuture<'e, T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'e>>;

const MAX_BATCH_SIZE: usize = 100;
const BATCH_FLUSH_INTERVAL_MS: u64 = 100;

static EVENT_SYSTEM: RwLock<Option<EventSystem>> = RwLock::new(None);

/// Trait for event reporters that can emit events individually or in batches
pub trait EventReporter: Send + Sync {
    fn emit<'e>(
        &'e self,
        event: &'e AuditEvent,
    ) -> BoxFuture<'e, Result<(), Error>>;

    fn emit_batch<'e>(
        &'e self,
        events: &'e Vec<AuditEvent>,
    ) -> BoxFuture<'e, Result<(), Error>> {
        Box::pin(async move {
            for event in events {
                self.emit(event).await?;
            }
            Ok(())
        })
    }
}

/// Message to flush all pending events
#[derive(Message)]
#[rtype(result = "()")]
struct FlushEvents;

/// Message to process a batch of events
#[derive(Message)]
#[rtype(result = "()")]
struct ProcessBatch(Vec<AuditEvent>);

/// EventCollector actor that accumulates events and sends batches to the executor
pub struct EventCollector {
    buffer: Vec<AuditEvent>,
    executor: Addr<EventExecutor>,
    max_batch_size: usize,
}

impl EventCollector {
    pub fn new(
        executor: Addr<EventExecutor>,
        max_batch_size: usize,
    ) -> Self {
        Self {
            buffer: Vec::with_capacity(max_batch_size),
            executor,
            max_batch_size,
        }
    }

    fn maybe_flush(
        &mut self,
        ctx: &mut Context<Self>,
    ) {
        if self.buffer.len() >= self.max_batch_size {
            self.flush(ctx);
        }
    }

    fn flush(
        &mut self,
        _ctx: &mut Context<Self>,
    ) {
        if self.buffer.is_empty() {
            return;
        }

        let events = std::mem::replace(&mut self.buffer, Vec::with_capacity(self.max_batch_size));

        self.executor.do_send(ProcessBatch(events));
    }
}

impl Actor for EventCollector {
    type Context = Context<Self>;

    fn started(
        &mut self,
        ctx: &mut Self::Context,
    ) {
        tracing::debug!("EventCollector started");

        ctx.run_interval(
            Duration::from_millis(BATCH_FLUSH_INTERVAL_MS),
            |act, ctx| {
                act.flush(ctx);
            },
        );
    }

    fn stopping(
        &mut self,
        ctx: &mut Self::Context,
    ) -> actix::Running {
        tracing::debug!(
            "EventCollector stopping, flushing {} events",
            self.buffer.len()
        );
        self.flush(ctx);
        actix::Running::Stop
    }

    fn stopped(
        &mut self,
        _ctx: &mut Self::Context,
    ) {
        tracing::debug!("EventCollector stopped");
    }
}

impl Supervised for EventCollector {
    fn restarting(
        &mut self,
        _ctx: &mut Context<EventCollector>,
    ) {
        tracing::warn!(
            "EventCollector restarting, preserving {} events in buffer",
            self.buffer.len()
        );
        // Buffer is preserved across restarts
    }
}

impl Handler<AuditEvent> for EventCollector {
    type Result = ();

    fn handle(
        &mut self,
        msg: AuditEvent,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        self.buffer.push(msg);
        self.maybe_flush(ctx);
    }
}

impl Handler<FlushEvents> for EventCollector {
    type Result = ();

    fn handle(
        &mut self,
        _msg: FlushEvents,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        self.flush(ctx);
    }
}

/// EventExecutor actor that processes batches using configured reporters
pub struct EventExecutor {
    reporters: Vec<Arc<dyn EventReporter>>,
}

impl EventExecutor {
    pub fn new(reporters: Vec<Box<dyn EventReporter>>) -> Self {
        Self {
            reporters: reporters
                .into_iter()
                .map(|r| Arc::from(r))
                .collect(),
        }
    }
}

impl Actor for EventExecutor {
    type Context = Context<Self>;

    fn started(
        &mut self,
        _ctx: &mut Self::Context,
    ) {
        tracing::debug!(
            "EventExecutor started with {} reporters",
            self.reporters.len()
        );
    }

    fn stopped(
        &mut self,
        _ctx: &mut Self::Context,
    ) {
        tracing::debug!("EventExecutor stopped");
    }
}

impl Supervised for EventExecutor {}

impl Handler<ProcessBatch> for EventExecutor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(
        &mut self,
        msg: ProcessBatch,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let events = msg.0;
        tracing::trace!("Processing batch of {} events", events.len());

        // Clone Arc pointers to reporters so they can be moved into async block
        let reporters = self.reporters.clone();

        // Create async future that processes all reporters
        let fut = async move {
            for reporter in reporters.iter() {
                match reporter.emit_batch(&events).await {
                    Ok(_) => {},
                    Err(err) => {
                        tracing::error!("Event reporter batch emit error: {err:#?}");
                    },
                }
            }
        };

        // Convert the future into an ActorFuture using wrap_future
        Box::pin(wrap_future::<_, Self>(fut).map(|_, _, _| ()))
    }
}

/// System for managing event collection and execution
pub struct EventSystem {
    collector: Addr<EventCollector>,
    executor: Addr<EventExecutor>,
}

impl EventSystem {
    pub fn new(reporters: Vec<Box<dyn EventReporter>>) -> Self {
        Self::with_batch_size(reporters, MAX_BATCH_SIZE)
    }

    pub fn with_batch_size(
        reporters: Vec<Box<dyn EventReporter>>,
        max_batch_size: usize,
    ) -> Self {
        let executor = EventExecutor::new(reporters).start();
        let collector = EventCollector::new(executor.clone(), max_batch_size).start();

        Self {
            collector,
            executor,
        }
    }

    pub fn emit(
        &self,
        event: AuditEvent,
    ) {
        self.collector.do_send(event);
    }

    pub async fn flush(&self) -> Result<(), Error> {
        self.collector
            .send(FlushEvents)
            .await
            .map_err(|e| Error::MailboxError(e.to_string()))
    }

    pub fn collector(&self) -> &Addr<EventCollector> {
        &self.collector
    }

    pub async fn shutdown(self) -> Result<(), Error> {
        tracing::info!("Shutting down event system");

        self.flush().await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        drop(self.collector);
        drop(self.executor);

        tracing::info!("Event system shutdown complete");
        Ok(())
    }
}

pub struct LogEventReporter;

impl EventReporter for LogEventReporter {
    fn emit(
        &self,
        event: &AuditEvent,
    ) -> BoxFuture<'_, Result<(), Error>> {
        let as_json = serde_json::to_string(event);
        Box::pin(async move {
            println!(
                "{}",
                as_json.map_err(|e| {
                    Error::InternalError {
                        message: e.to_string(),
                    }
                })?
            );
            Ok(())
        })
    }

    fn emit_batch<'e>(
        &'e self,
        events: &'e Vec<AuditEvent>,
    ) -> BoxFuture<'e, Result<(), Error>> {
        Box::pin(async move {
            std::io::stdout()
                .write_json_lines(events)
                .map_err(|e| {
                    Error::InternalError {
                        message: e.to_string(),
                    }
                })?;
            Ok(())
        })
    }
}

pub struct TracingEventReporter;

impl TracingEventReporter {
    fn one(event: &AuditEvent) {
        tracing::info!(
            event_timestamp = %event.timestamp,
            principal_type = ?event.principal_type,
            principal_id = event.principal_id,
            allowed = event.allowed,
            reason = %event.reason,
            "audit_event"
        );
    }
}

impl EventReporter for TracingEventReporter {
    fn emit<'e>(
        &'e self,
        event: &'e AuditEvent,
    ) -> BoxFuture<'e, Result<(), Error>> {
        Box::pin(async move {
            Self::one(event);
            Ok(())
        })
    }

    fn emit_batch<'e>(
        &'e self,
        events: &'e Vec<AuditEvent>,
    ) -> BoxFuture<'e, Result<(), Error>> {
        Box::pin(async move {
            for event in events {
                Self::one(event);
            }
            Ok(())
        })
    }
}

pub struct NoOpReporter;

impl EventReporter for NoOpReporter {
    fn emit<'e>(
        &'e self,
        _: &'e AuditEvent,
    ) -> BoxFuture<'e, Result<(), Error>> {
        Box::pin(async move { Ok(()) })
    }
}

pub struct MultiEventReporter {
    reporters: Vec<Arc<dyn EventReporter>>,
}

impl MultiEventReporter {
    pub fn new(reporters: Vec<Arc<dyn EventReporter>>) -> Self {
        Self { reporters }
    }
}

impl EventReporter for MultiEventReporter {
    fn emit<'e>(
        &'e self,
        event: &'e AuditEvent,
    ) -> BoxFuture<'e, Result<(), Error>> {
        let reporters = self.reporters.clone();
        Box::pin(async move {
            for reporter in reporters.iter() {
                reporter.emit(event).await?;
            }
            Ok(())
        })
    }

    fn emit_batch<'e>(
        &'e self,
        events: &'e Vec<AuditEvent>,
    ) -> BoxFuture<'e, Result<(), Error>> {
        let reporters = self.reporters.clone();
        Box::pin(async move {
            for reporter in reporters.iter() {
                reporter.emit_batch(events).await?;
            }
            Ok(())
        })
    }
}

pub async fn init(reporters: Vec<Box<dyn EventReporter>>) {
    let mut system = EVENT_SYSTEM.write().unwrap();
    if system.is_some() {
        tracing::warn!("Event system already started, replacing existing system");
    }
    *system = Some(EventSystem::new(reporters));
    tracing::info!("Event system started");
    drop(system);
}

pub async fn start<F: FnOnce() -> Fut, Fut: Future<Output = R>, R>(
    reporters: Vec<Box<dyn EventReporter>>,
    handle: F,
) -> Fut::Output {
    init(reporters).await;

    let out = handle().await;

    shutdown().await.unwrap();

    out
}

/// Emit an event to the event system.
/// When compiled with the `test` feature, this becomes a no-op that always succeeds.
#[cfg(not(feature = "test"))]
pub fn emit_event(event: AuditEvent) -> Result<(), Error> {
    match EVENT_SYSTEM.read().unwrap().as_ref() {
        Some(sys) => {
            sys.emit(event);
            Ok(())
        },
        None => Err(Error::NotInitialized),
    }
}

/// Test mode: emit_event is a no-op that always succeeds
#[cfg(feature = "test")]
pub fn emit_event(_event: AuditEvent) -> Result<(), Error> {
    Ok(())
}

pub async fn flush() -> Result<(), Error> {
    let system = {
        let system = EVENT_SYSTEM.read().unwrap();
        system
            .as_ref()
            .map(|s| s.collector().clone())
    };

    match system {
        Some(collector) => {
            collector
                .send(FlushEvents)
                .await
                .map_err(|e| Error::MailboxError(e.to_string()))
        },
        None => Err(Error::NotInitialized),
    }
}

pub async fn shutdown() -> Result<(), Error> {
    flush().await?;

    let system = EVENT_SYSTEM.write().unwrap().take();

    match system {
        Some(sys) => {
            tracing::info!("Shutting down global event system");
            sys.shutdown().await?;
            tracing::info!("Global event system shutdown complete");
            Ok(())
        },
        None => {
            tracing::warn!("Event system not initialized, nothing to shutdown");
            Ok(())
        },
    }
}
