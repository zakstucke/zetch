use std::str::FromStr;

use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    arg_matcher::arg_matcher,
    args::{self, get_py_args},
    prelude::*,
};

// If one of these is the first argument, won't auto assume example_project_py_rs render subcommand
const ROOT_ARGS: &[&str] = &[
    "-h",
    "--help",
    "help",
    "-V",
    "--version",
    "version",
    // Including delete in here as its a del alias but has_subcommand() doesn't seem to work with aliases:
    "delete",
];
const DEFAULT_SUBCOMMAND: &str = "render";

pub fn run() -> Result<(), Report<Zerr>> {
    let mut py_args = get_py_args()?;

    // Clap doesn't support default subcommands but we want to DEFAULT_SUBCOMMAND by
    // default for convenience, so we just preprocess the arguments accordingly before passing them to Clap.
    let arg1 = py_args.get(1);
    let add = {
        if let Some(arg1) = arg1 {
            // If the first argument isn't already a subcommand, and isn't a specific root arg/option, true:
            !args::Command::has_subcommand(arg1) && !ROOT_ARGS.contains(&arg1.as_str())
        } else {
            true
        }
    };
    if add {
        py_args.insert(1, DEFAULT_SUBCOMMAND.into());
    }

    let args = args::Args::parse_from(py_args);

    // Setup logging:
    if args.log_level_args.verbose {
        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_level(true)
                    .with_target(false)
                    .with_file(true)
                    .with_line_number(true)
                    .with_ansi(true)
                    .with_file(true),
            )
            .with(EnvFilter::from_str("RUST_LOG=trace").change_context(Zerr::InternalError)?)
            .init();
    } else if !args.log_level_args.silent {
        // If its read, put, delete or var subcommands, stdout is important, so only show error!() in default mode:
        let level_from = if matches!(
            &args.command,
            args::Command::Read(_)
                | args::Command::Put(_)
                | args::Command::Del(_)
                | args::Command::Var(_)
        ) {
            tracing::Level::ERROR
        } else {
            tracing::Level::INFO
        };

        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_level(true)
                    .with_target(false)
                    .with_file(true)
                    .with_line_number(true)
                    .with_ansi(true)
                    .with_file(true),
            )
            .with(
                EnvFilter::from_str(&format!("RUST_LOG={level_from}"))
                    .change_context(Zerr::InternalError)?,
            )
            .init();
    }

    let result = arg_matcher(args);

    debug!(
        "{}",
        GLOBAL_TIME_RECORDER
            .format_verbose()
            .change_context(Zerr::InternalError)?
    );

    result
}
