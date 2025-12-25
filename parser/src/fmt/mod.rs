use kintsu_cli_core::ProgressManager;
use kintsu_manifests::NewForConfig;
use miette::IntoDiagnostic;
use std::sync::Arc;

use crate::tokens::tokenize_with;

pub mod printer;
pub use printer::*;

fn default_width() -> usize {
    120
}
fn default_indent_width() -> usize {
    4
}
fn preserve_adjacent_blank_lines() -> bool {
    true
}
fn default_tabs() -> bool {
    true
}

#[derive(Clone, Debug, serde::Deserialize, validator::Validate)]
pub struct FormatConfig {
    #[serde(default = "default_width")]
    pub max_width: usize,
    #[serde(default = "default_tabs")]
    pub indent_with_tabs: bool,
    #[serde(default = "default_indent_width")]
    pub indent_width: usize,
    #[serde(default = "preserve_adjacent_blank_lines")]
    pub preserve_adjacent_blank_lines: bool,
}

impl NewForConfig for FormatConfig {
    const NAME: &'static str = "op-fmt";
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            max_width: default_width(),
            indent_with_tabs: default_tabs(),
            indent_width: default_indent_width(),
            preserve_adjacent_blank_lines: preserve_adjacent_blank_lines(),
        }
    }
}

async fn format_file(
    config: &FormatConfig,
    target: impl AsRef<std::path::Path>,
    dry: bool,
) -> miette::Result<Vec<miette::Report>> {
    let data = tokio::fs::read_to_string(&target)
        .await
        .into_diagnostic()?;

    let mut tokens = tokenize_with(&target, &data)?;
    let ast = crate::ast::AstStream::from_tokens_with(&target, &mut tokens)
        .map_err(|err| err.to_report(None, None, None))?;
    let formatted = crate::fmt::printer::print_ast(&ast, config);

    if !dry && data != formatted {
        tokio::fs::write(&target, formatted)
            .await
            .into_diagnostic()?;
    }

    Ok(Vec::new())
}

pub async fn fmt<S: AsRef<str>>(
    config_dir: Option<S>,
    targets: Vec<impl AsRef<std::path::Path> + Send + Sync>,
    dry: bool,
) -> miette::Result<()> {
    fmt_with_progress(config_dir, targets, dry, ProgressManager::disabled()).await
}

pub async fn fmt_with_progress<S: AsRef<str>>(
    config_dir: Option<S>,
    targets: Vec<impl AsRef<std::path::Path> + Send + Sync>,
    dry: bool,
    progress: ProgressManager,
) -> miette::Result<()> {
    let config = Arc::new(FormatConfig::new(config_dir).into_diagnostic()?);
    let bar = progress.add_bar(targets.len() as u64, kintsu_cli_core::prefixes::FORMATTING);

    let mut futs = vec![];
    for t in targets {
        let config = config.clone();
        let bar = bar.clone();
        futs.push(Box::pin(async move {
            let path_display = t.as_ref().display().to_string();
            let result = format_file(&config, &t, dry).await;
            bar.inc(1);
            bar.set_message(path_display);
            result
        }));
    }
    for f in futures_util::future::join_all(futs).await {
        f?;
    }

    bar.finish_and_clear();
    Ok(())
}
