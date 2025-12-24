use divan::{Bencher, black_box};
use kintsu_registry_auth::{AuditEvent, Policy, PolicyCheck, PrincipalType};
use kintsu_registry_events::{
    EventReporter, EventSystem, MultiEventReporter, NoOpReporter, TracingEventReporter,
};
use std::sync::Arc;

// Helper to create test event
fn create_test_event() -> AuditEvent {
    AuditEvent::builder()
        .timestamp(Default::default())
        .principal_type(PrincipalType::UserSession)
        .principal_id(12345)
        .event_type(serde_json::json!({"action": "test"}))
        .allowed(true)
        .reason("test benchmark")
        .policy_checks(vec![PolicyCheck {
            policy: Policy::ExplicitPermission,
            passed: true,
            details: "test".to_string(),
        }])
        .request_id("req-123")
        .ip_address("127.0.0.1")
        .user_agent("bench/1.0")
        .build()
}

// Helper to create batch of events
fn create_event_batch(size: usize) -> Vec<AuditEvent> {
    (0..size)
        .map(|_| create_test_event())
        .collect()
}

#[divan::bench(name = "noop reporter - single emit")]
fn noop_single_emit(bencher: Bencher) {
    let reporter = NoOpReporter;
    let event = create_test_event();

    bencher.bench(|| {
        black_box(futures::executor::block_on(async {
            reporter
                .emit(black_box(event.clone()))
                .await
                .unwrap()
        }));
    });
}

#[divan::bench(
    name = "noop reporter - batch emit",
    args = [10, 100, 1000, 10_000, 100_000],
)]
fn noop_batch_emit(
    bencher: Bencher,
    batch_size: usize,
) {
    let reporter = NoOpReporter;
    let events = create_event_batch(batch_size);

    bencher.bench(|| {
        black_box(futures::executor::block_on(async {
            reporter
                .emit_batch(black_box(events.clone()))
                .await
                .unwrap()
        }));
    });
}

#[divan::bench(name = "tracing reporter - single emit")]
fn tracing_single_emit(bencher: Bencher) {
    let reporter = TracingEventReporter;
    let event = create_test_event();

    bencher.bench(|| {
        black_box(futures::executor::block_on(async {
            reporter
                .emit(black_box(event.clone()))
                .await
                .unwrap()
        }));
    });
}

#[divan::bench(
    name = "tracing reporter - batch emit",
    args = [10, 100, 1000, 10_000, 100_000],
)]
fn tracing_batch_emit(
    bencher: Bencher,
    batch_size: usize,
) {
    let reporter = TracingEventReporter;
    let events = create_event_batch(batch_size);

    bencher.bench(|| {
        black_box(futures::executor::block_on(async {
            reporter
                .emit_batch(black_box(events.clone()))
                .await
                .unwrap()
        }));
    });
}

#[divan::bench(name = "multi reporter - 2 noop reporters")]
fn multi_reporter_noop(bencher: Bencher) {
    let reporters: Vec<Arc<dyn EventReporter>> =
        vec![Arc::new(NoOpReporter), Arc::new(NoOpReporter)];
    let multi = MultiEventReporter::new(reporters);
    let event = create_test_event();

    bencher.bench(|| {
        black_box(futures::executor::block_on(async {
            multi
                .emit(black_box(event.clone()))
                .await
                .unwrap()
        }));
    });
}

#[divan::bench(
    name = "event system throughput - single events",
    args = [10, 100, 1000, 10_000, 100_000],
)]
fn event_system_single_throughput(
    bencher: Bencher,
    event_count: usize,
) {
    // Create a tokio runtime that persists across iterations
    let rt = tokio::runtime::Runtime::new().unwrap();

    bencher
        .with_inputs(|| {
            // Setup: create system with noop reporter within tokio + actix context
            let _guard = rt.enter();
            let system = actix::System::new()
                .block_on(async { EventSystem::new(vec![Box::new(NoOpReporter)]) });
            let events: Vec<_> = (0..event_count)
                .map(|_| create_test_event())
                .collect();
            (system, events)
        })
        .bench_values(|(system, events)| {
            // Benchmark the emission and flush time
            let _guard = rt.enter();
            for event in events {
                system.emit(black_box(event));
            }

            // Cleanup - flush and wait
            rt.block_on(async {
                system.flush().await.ok();
            });
        });
}

#[divan::bench(
    name = "event system batching efficiency",
    args = [10, 100, 1000, 10_000, 100_000],
)]
fn event_system_batch_efficiency(
    bencher: Bencher,
    batch_size: usize,
) {
    // Create a tokio runtime that persists across iterations
    let rt = tokio::runtime::Runtime::new().unwrap();

    bencher
        .with_inputs(|| {
            // Setup: create system with specified batch size within tokio + actix context
            let _guard = rt.enter();
            let system = actix::System::new().block_on(async {
                EventSystem::with_batch_size(vec![Box::new(NoOpReporter)], batch_size)
            });
            let events: Vec<_> = (0..batch_size * 2)
                .map(|_| create_test_event())
                .collect();
            (system, events)
        })
        .bench_values(|(system, events)| {
            // Benchmark emission and flushing
            let _guard = rt.enter();
            for event in events {
                system.emit(black_box(event));
            }

            rt.block_on(async {
                system.flush().await.ok();
            });
        });
}

#[divan::bench(name = "event creation overhead")]
fn event_creation() {
    black_box(create_test_event());
}

#[divan::bench(
    name = "event batch creation",
    args = [10, 50, 100, 500],
)]
fn event_batch_creation(batch_size: usize) {
    black_box(create_event_batch(black_box(batch_size)));
}

fn main() {
    divan::main();
}
