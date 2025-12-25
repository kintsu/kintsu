use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use indicatif::{MultiProgress, ProgressBar};

use super::style::{colors, templates};

#[derive(Clone)]
pub struct ProgressManager {
    inner: Arc<ProgressManagerInner>,
}

struct ProgressManagerInner {
    multi: MultiProgress,
    start_time: Instant,
    enabled: bool,
    current_phase: RwLock<Option<String>>,
}

impl ProgressManager {
    /// Create a new progress manager
    pub fn new(enabled: bool) -> Self {
        Self {
            inner: Arc::new(ProgressManagerInner {
                multi: MultiProgress::new(),
                start_time: Instant::now(),
                enabled,
                current_phase: RwLock::new(None),
            }),
        }
    }

    /// Create a disabled (no-op) progress manager
    pub fn disabled() -> Self {
        Self::new(false)
    }

    /// Check if progress is enabled
    pub fn is_enabled(&self) -> bool {
        self.inner.enabled
    }

    /// Add a progress bar with count
    pub fn add_bar(
        &self,
        total: u64,
        prefix: &str,
    ) -> ProgressBar {
        if !self.inner.enabled {
            return ProgressBar::hidden();
        }

        let bar = self.inner.multi.add(ProgressBar::new(total));
        bar.set_style(templates::bar());
        bar.set_prefix(prefix.to_string());
        bar
    }

    /// Add a spinner (indeterminate progress)
    pub fn add_spinner(
        &self,
        prefix: &str,
    ) -> ProgressBar {
        if !self.inner.enabled {
            return ProgressBar::hidden();
        }

        let spinner = self
            .inner
            .multi
            .add(ProgressBar::new_spinner());
        spinner.set_style(templates::spinner());
        spinner.set_prefix(prefix.to_string());
        spinner.enable_steady_tick(Duration::from_millis(100));
        spinner
    }

    /// Print a styled message
    pub fn println(
        &self,
        prefix: &str,
        message: &str,
    ) {
        if !self.inner.enabled {
            return;
        }

        println!("{:>12} {}", colors::prefix().apply_to(prefix), message);
    }

    /// Transition to a new phase (prints phase header)
    pub fn transition_phase(
        &self,
        phase: &str,
    ) {
        if !self.inner.enabled {
            return;
        }

        if let Ok(mut current) = self.inner.current_phase.write() {
            *current = Some(phase.to_string());
        }
    }

    /// Get current phase name
    pub fn current_phase(&self) -> Option<String> {
        self.inner
            .current_phase
            .read()
            .ok()
            .and_then(|g| g.clone())
    }

    /// Complete with custom message
    pub fn complete(
        &self,
        message: impl AsRef<str>,
    ) {
        if !self.inner.enabled {
            return;
        }

        let elapsed = self.inner.start_time.elapsed();
        let secs = elapsed.as_secs_f64();

        println!(
            "{:>12} {} in {:.3} seconds",
            colors::prefix().apply_to("Finished"),
            message.as_ref(),
            secs
        );
    }

    /// Complete with default "Finished" message (for backwards compat)
    pub fn finish(&self) {
        self.complete("compilation");
    }

    /// Get elapsed time since creation
    pub fn elapsed(&self) -> Duration {
        self.inner.start_time.elapsed()
    }

    /// Execute a closure only if progress is enabled
    pub fn maybe<F: Fn(&Self)>(
        &self,
        f: F,
    ) {
        if self.inner.enabled {
            f(self);
        }
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Backwards-compatible type alias
pub type CompilationProgress = ProgressManager;
