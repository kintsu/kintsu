use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use tokio::sync::{
    Mutex, RwLock,
    mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::ctx::{cache::SchemaCache, registry::TypeRegistry};

use super::{
    loader::{CompilationTask, DependencyTaskResult},
    progress::CompilationProgress,
    state::SharedCompilationState,
};

#[derive(Clone)]
pub(super) struct CoordinatorState {
    pending_tasks: Arc<AtomicUsize>,
    errors: Arc<Mutex<Vec<crate::Error>>>,
    state: Arc<RwLock<SharedCompilationState>>,
    progress: CompilationProgress,
}

impl CoordinatorState {
    pub fn new(
        state: Arc<RwLock<SharedCompilationState>>,
        progress: CompilationProgress,
    ) -> Self {
        Self {
            pending_tasks: Arc::new(AtomicUsize::new(0)),
            errors: Arc::new(Mutex::new(Vec::new())),
            state,
            progress,
        }
    }

    pub fn increment_pending(&self) {
        self.pending_tasks
            .fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement_pending(&self) -> usize {
        self.pending_tasks
            .fetch_sub(1, Ordering::SeqCst)
            .saturating_sub(1)
    }

    pub fn pending_count(&self) -> usize {
        self.pending_tasks.load(Ordering::SeqCst)
    }

    pub async fn record_error(
        &self,
        error: crate::Error,
    ) {
        self.errors.lock().await.push(error);
    }

    pub async fn take_first_error(&self) -> Option<crate::Error> {
        let mut l = self.errors.lock().await;

        if l.is_empty() {
            None
        } else {
            Some(l.remove(0))
        }
    }

    pub fn state(&self) -> &Arc<RwLock<SharedCompilationState>> {
        &self.state
    }

    pub fn progress(&self) -> &CompilationProgress {
        &self.progress
    }
}

pub(super) async fn dependency_worker(
    worker_id: usize,
    task_rx: Arc<tokio::sync::Mutex<UnboundedReceiver<CompilationTask>>>,
    result_tx: UnboundedSender<Result<DependencyTaskResult, crate::Error>>,
    coord_state: CoordinatorState,
    resolver: Arc<dyn super::resolver::PackageResolver>,
    cache: SchemaCache,
    registry: TypeRegistry,
) {
    tracing::debug!("Worker {} starting", worker_id);

    let mut processed_count = 0usize;

    loop {
        let task = {
            let mut rx = task_rx.lock().await;
            rx.recv().await
        };

        match task {
            Some(task) => {
                tracing::debug!("Worker {} processing {}", worker_id, task.package_name);

                let result = super::loader::DependencyLoader::process_dependency_task(
                    task,
                    coord_state.state().clone(),
                    resolver.clone(),
                    cache.clone(),
                    registry.clone(),
                )
                .await;

                if let Err(ref e) = result {
                    tracing::error!("Worker {} error processing task: {}", worker_id, e);
                }

                if result_tx.send(result).is_err() {
                    tracing::error!("Worker {} failed to send result: channel closed", worker_id);
                    break;
                }

                processed_count += 1;
            },
            None => {
                // Channel closed, exit
                break;
            },
        }
    }

    tracing::debug!(
        "Worker {} exiting after processing {} tasks",
        worker_id,
        processed_count
    );
}

pub(super) async fn dependency_coordinator(
    mut result_rx: UnboundedReceiver<Result<DependencyTaskResult, crate::Error>>,
    task_tx: UnboundedSender<CompilationTask>,
    completion_tx: UnboundedSender<()>,
    coord_state: CoordinatorState,
) {
    tracing::debug!("Coordinator starting");

    let resolving_spinner = coord_state
        .progress()
        .add_spinner("Resolving");

    resolving_spinner.set_message("dependencies");

    let mut loaded_count = 0usize;

    while let Some(result_or_error) = result_rx.recv().await {
        match result_or_error {
            Ok(DependencyTaskResult::Loaded(result)) => {
                loaded_count += 1;
                resolving_spinner.set_message(format!(
                    "{}@{} ({} loaded)",
                    result.package_name, result.version, loaded_count
                ));

                {
                    let mut guard = coord_state.state().write().await;
                    guard
                        .dependencies
                        .insert(result.package_name.clone(), result.schema.clone());
                    guard
                        .loaded_versions
                        .insert(result.package_name.clone(), result.version.clone());
                    guard
                        .processing_set
                        .remove(&result.package_name);
                }

                let (transitive_deps, _) =
                    super::loader::DependencyLoader::collect_transitive_deps(
                        &result.schema,
                        &result.resolved_path,
                        coord_state.state(),
                        &result.dependency_chain,
                        &result.package_name,
                    )
                    .await;

                for _ in transitive_deps.iter() {
                    coord_state.increment_pending();
                }

                for sub_task in transitive_deps {
                    if task_tx.send(sub_task).is_err() {
                        tracing::error!("Failed to send sub-task: channel closed");
                        coord_state.decrement_pending();
                    }
                }

                let remaining = coord_state.decrement_pending();

                tracing::trace!(
                    "Task completed: {}, remaining: {}",
                    result.package_name,
                    remaining
                );

                if remaining == 0 {
                    tracing::info!("All dependencies loaded, signaling completion");
                    let _ = completion_tx.send(());
                    drop(task_tx);
                    break;
                }
            },

            Ok(DependencyTaskResult::AlreadyLoaded) => {
                let remaining = coord_state.decrement_pending();

                tracing::trace!("Task already loaded, remaining: {}", remaining);

                if remaining == 0 {
                    tracing::info!("All dependencies loaded, signaling completion");
                    let _ = completion_tx.send(());
                    drop(task_tx);
                    break;
                }
            },

            Err(e) => {
                tracing::error!("Coordinator received error: {}", e);
                coord_state.record_error(e).await;

                let remaining = coord_state.decrement_pending();

                if remaining == 0 {
                    tracing::info!("All tasks complete (with errors), signaling completion");
                    let _ = completion_tx.send(());
                    drop(task_tx);
                    break;
                }
            },
        }
    }

    resolving_spinner.finish_with_message(format!(
        "{} {}",
        loaded_count,
        if loaded_count != 1 {
            "dependencies"
        } else {
            "dependency"
        }
    ));

    tracing::debug!("Coordinator exiting");
}
