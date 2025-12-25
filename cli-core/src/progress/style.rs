pub mod colors {
    use console::Style;

    pub fn prefix() -> Style {
        Style::new().cyan().bold()
    }

    pub fn progress() -> Style {
        Style::new().magenta().bold()
    }

    pub fn success() -> Style {
        Style::new().green().bold()
    }

    pub fn warning() -> Style {
        Style::new().yellow().bold()
    }

    pub fn error() -> Style {
        Style::new().red().bold()
    }
}

pub mod templates {
    use indicatif::ProgressStyle;

    pub fn bar() -> ProgressStyle {
        ProgressStyle::with_template("{prefix:>12.magenta.bold} [{bar:40}] {pos}/{len} {wide_msg}")
            .unwrap()
            .progress_chars("=> ")
    }

    pub fn spinner() -> ProgressStyle {
        ProgressStyle::with_template("{prefix:>12.magenta.bold} {spinner} {wide_msg}").unwrap()
    }
}

pub mod prefixes {
    pub const INITIALIZING: &str = "Initializing";
    pub const ANALYZING: &str = "Analyzing";
    pub const PROCESSING: &str = "Processing";
    pub const FINISHED: &str = "Finished";

    pub const COMPILING: &str = "Compiling";
    pub const RESOLVING: &str = "Resolving";
    pub const LOADING: &str = "Loading";

    pub const FORMATTING: &str = "Formatting";
    pub const CHECKING: &str = "Checking";

    pub const PREPARING: &str = "Preparing";
    pub const UPLOADING: &str = "Uploading";
    pub const PUBLISHING: &str = "Publishing";
    pub const PUBLISHED: &str = "Published";

    pub const GENERATING: &str = "Generating";
    pub const WRITING: &str = "Writing";

    pub const SUCCESS: &str = "Success";
    pub const WARNING: &str = "Warning";
    pub const ERROR: &str = "Error";
}
