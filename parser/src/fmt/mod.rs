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
    let config = Arc::new(FormatConfig::new(config_dir).into_diagnostic()?);
    let mut futs = vec![];
    for t in targets {
        let config = config.clone();
        futs.push(Box::pin(async move { format_file(&config, t, dry).await }));
    }
    for f in futures_util::future::join_all(futs).await {
        f?;
    }
    Ok(())
}
