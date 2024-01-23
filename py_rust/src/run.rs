use bitbazaar::{
    logging::{create_subscriber, SubLayer, SubLayerFilter, SubLayerVariant},
    timing::GLOBAL_TIME_RECORDER,
};
use clap::{Parser, Subcommand};

use crate::{
    arg_matcher::arg_matcher,
    args::{self, get_py_args},
    prelude::*,
};

// If one of these is the first argument, won't auto assume example_project_py_rs render subcommand
const ROOT_ARGS: &[&str] = &["-h", "--help", "help", "-V", "--version", "version"];
const DEFAULT_SUBCOMMAND: &str = "render";

pub fn run() -> Result<(), Zerr> {
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

    // Setup global logging:
    let mut log_layers: Vec<SubLayer> = vec![];
    if args.log_level_args.verbose {
        log_layers.push(SubLayer {
            variant: SubLayerVariant::Stdout {},
            filter: SubLayerFilter::Above(tracing::Level::TRACE),
            ..Default::default()
        });
    } else if !args.log_level_args.silent {
        // If its a read command (i.e. the output is important, only show errors, to prevent polluting the output)
        if matches!(
            &args.command,
            args::Command::ReadConfig(_) | args::Command::ReadVar(_)
        ) {
            log_layers.push(SubLayer {
                variant: SubLayerVariant::Stdout {},
                filter: SubLayerFilter::Above(tracing::Level::ERROR),
                ..Default::default()
            });
        } else {
            // Otherwise by default show info and up:

            // For INFO, don't show the level:
            log_layers.push(SubLayer {
                variant: SubLayerVariant::Stdout {},
                filter: SubLayerFilter::Only(vec![tracing::Level::INFO]),
                include_lvl: false,
                ..Default::default()
            });

            // For the rest, show the level:
            log_layers.push(SubLayer {
                variant: SubLayerVariant::Stdout {},
                filter: SubLayerFilter::Above(tracing::Level::WARN),
                ..Default::default()
            });
        }
    }

    create_subscriber(log_layers)
        .change_context(Zerr::InternalError)?
        .into_global();

    let result = arg_matcher(args);

    debug!(
        "{}",
        GLOBAL_TIME_RECORDER
            .format_verbose()
            .change_context(Zerr::InternalError)?
    );

    result
}
