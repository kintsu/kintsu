#![allow(clippy::result_large_err)]

use clap::Parser;
use human_panic::{Metadata, setup_panic};
use kintsu_cli::cli::Cli;
use miette::GraphicalReportHandler;
use std::process::ExitCode;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<ExitCode, ()> {
    // Setup miette for fancy error display
    miette::set_hook(Box::new(|_| {
        Box::new(
            GraphicalReportHandler::new()
                .with_theme(miette::GraphicalTheme::unicode())
                .with_context_lines(5),
        )
    }))
    .ok(); // Ignore if already set

    setup_panic!(
        Metadata::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .homepage(env!("CARGO_PKG_HOMEPAGE"))
            .support(
                "Please open an issue on github. Attach the outputs of the above referenced report file."
            ).authors(env!("CARGO_PKG_AUTHORS"))
    );

    let cli = Cli::parse();

    let log_level: tracing::Level = cli.log_level.clone().into();

    let indicatif_layer = tracing_indicatif::IndicatifLayer::new();

    let layer = tracing_subscriber::registry()
        .with({
            let mut layer =
                tracing_subscriber::fmt::layer().with_writer(indicatif_layer.get_stderr_writer());

            #[cfg(test)]
            {
                layer = layer.with_file(true).with_line_number(true);
            }

            #[cfg(not(test))]
            {
                layer = layer
                    .with_file(false)
                    .with_line_number(false);
            }

            layer
        })
        .with(indicatif_layer)
        .with(LevelFilter::from_level(log_level));

    layer
        .try_init()
        .expect("Unable to initialize logging layer");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");

    Ok(match rt.block_on(cli.run()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Convert to CompilerError for structured error codes
            let compiler_error: kintsu_errors::CompilerError = e.into();
            let report = compiler_error.to_report();
            eprintln!("{report:?}");
            ExitCode::FAILURE
        },
    })
}
