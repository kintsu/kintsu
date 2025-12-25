pub mod args;
pub mod progress;

pub use args::WithProgressConfig;
pub use indicatif::ProgressBar;
pub use progress::{CompilationProgress, ProgressManager, colors, prefixes, templates};
