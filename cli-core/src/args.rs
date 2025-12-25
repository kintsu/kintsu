use crate::progress::ProgressManager;

/// Shared progress configuration for CLI commands.
/// Use with `#[clap(flatten)]` in command arg structs.
#[derive(clap::Args, Debug, Clone, Default)]
pub struct WithProgressConfig {
    /// Disable progress output (useful for CI/scripts)
    #[clap(long, default_value_t = false, help = "disable progress output")]
    pub no_progress: bool,
}

impl WithProgressConfig {
    pub fn progress_enabled(&self) -> bool {
        !self.no_progress
    }

    pub fn create_manager(&self) -> ProgressManager {
        ProgressManager::new(self.progress_enabled())
    }
}
