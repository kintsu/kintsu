use std::{sync::Arc, time::Instant};

use console::Style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

#[derive(Clone)]
pub struct CompilationProgress {
    multi: Arc<MultiProgress>,
    start_time: Instant,
    enabled: bool,
}

impl CompilationProgress {
    pub fn new(enabled: bool) -> Self {
        Self {
            multi: Arc::new(MultiProgress::new()),
            start_time: Instant::now(),
            enabled,
        }
    }

    pub fn maybe<F: Fn(&Self)>(
        &self,
        f: F,
    ) {
        if self.enabled {
            f(self);
        }
    }

    pub fn add_bar(
        &self,
        total: u64,
        prefix: &str,
    ) -> ProgressBar {
        if !self.enabled {
            return ProgressBar::hidden();
        }

        let bar = self.multi.add(ProgressBar::new(total));
        bar.set_style(
            ProgressStyle::with_template(
                "{prefix:>12.magenta.bold} [{bar:40}] {pos}/{len} {wide_msg}",
            )
            .unwrap()
            .progress_chars("=> "),
        );
        bar.set_prefix(prefix.to_string());
        bar
    }

    pub fn add_spinner(
        &self,
        prefix: &str,
    ) -> ProgressBar {
        if !self.enabled {
            return ProgressBar::hidden();
        }

        let spinner = self.multi.add(ProgressBar::new_spinner());
        spinner.set_style(
            ProgressStyle::with_template("{prefix:>12.magenta.bold} {spinner} {wide_msg}").unwrap(),
        );
        spinner.set_prefix(prefix.to_string());
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        spinner
    }

    pub fn println(
        &self,
        prefix: &str,
        message: &str,
    ) {
        if !self.enabled {
            return;
        }

        let cyan_bold = Style::new().cyan().bold();
        let line = format!("{:>12} {}", cyan_bold.apply_to(prefix), message);

        if let Some(bar) = self.multi.add(ProgressBar::hidden()).into() {
            bar.println(line);
        }
    }

    pub fn finish(&self) {
        if !self.enabled {
            return;
        }

        let elapsed = self.start_time.elapsed();
        let secs = elapsed.as_secs_f64();

        let cyan_bold = Style::new().cyan().bold();
        println!(
            "{:>12} compilation in {:.3} seconds",
            cyan_bold.apply_to("Finished"),
            secs
        );
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for CompilationProgress {
    fn default() -> Self {
        Self::new(false)
    }
}
