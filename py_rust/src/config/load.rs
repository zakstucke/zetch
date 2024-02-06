use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use bitbazaar::timeit;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::{engine::Engine, raw_conf::RawConfig, tasks::Tasks};
use crate::{args::RenderCommand, config::parent_config::load_parent_config, prelude::*};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub raw: RawConfig,
    pub context: HashMap<String, serde_json::Value>,
    pub exclude: Vec<String>,
    pub engine: Engine,
    pub ignore_files: Vec<String>,
    pub matchers: Vec<String>,
    pub tasks: Tasks,
    pub final_config_path: PathBuf,
    pub cli_initials_used: bool,
    pub from_tmp_cache: bool,
}

pub fn load(
    args: &crate::args::Args,
    render_args: Option<&RenderCommand>,
    only_ctx_keys: Option<HashSet<&str>>,
    use_cli_initials: bool,
) -> Result<Config, Zerr> {
    // If running as a subprocess of a zetch command and is applicable (i.e. in a post task),
    // then use its config directly to prevent recursion errors and avoid unnecessary processing):
    if let Some(tmp_cached_config) = load_parent_config()? {
        return Ok(tmp_cached_config);
    }

    let final_config_path = build_final_config_path(
        &args.config,
        if let Some(render_args) = render_args {
            Some(&render_args.root)
        } else {
            None
        },
    )?;

    let raw = timeit!("Config processing", {
        RawConfig::from_toml(&final_config_path)
    })?;

    let config = timeit!(
        "Context value extraction (including user task & cli env scripting)",
        {
            // Before anything, run the pre-tasks:
            raw.tasks.run_pre(&final_config_path)?;

            let cloned_raw = raw.clone();
            let mut context: HashMap<String, serde_json::Value> = HashMap::new();

            for (key, value) in raw.context.stat {
                // Don't process if not needed:
                if let Some(only_ctx_keys) = only_ctx_keys.as_ref() {
                    if !only_ctx_keys.contains(key.as_str()) {
                        continue;
                    }
                }
                context.insert(
                    key.clone(),
                    value
                        .consume()
                        .change_context(Zerr::ContextLoadError)
                        .attach_printable_lazy(|| format!("Ctx static var: '{key}'"))?,
                );
            }

            // Now env vars:

            // If some env defaults banned, validate list and convert to a hashset for faster lookup:
            let banned_env_defaults: Option<HashSet<String>> = if let Some(render_args) =
                render_args
            {
                if let Some(banned) = render_args.ban_defaults.as_ref() {
                    // If no vars provided, ban all defaults:
                    if banned.is_empty() {
                        Some(raw.context.env.keys().cloned().collect::<HashSet<String>>())
                    } else {
                        let banned_env_defaults: HashSet<String> = banned.iter().cloned().collect();
                        // Make sure they are all valid env context keys:
                        for key in banned_env_defaults.iter() {
                            if !raw.context.env.contains_key(key) {
                                // Printing the env keys in the error, want them alphabetically sorted:
                                let mut env_keys = raw
                                    .context
                                    .env
                                    .keys()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<&str>>();
                                env_keys.sort_by_key(|name| name.to_lowercase());
                                return Err(zerr!(
                            Zerr::ContextLoadError,
                            "Unrecognized context.env var provided to '--ban-defaults': '{}'. All env vars in config: '{}'.",
                            key, env_keys.join(", ")
                        ));
                            }
                        }
                        Some(banned_env_defaults)
                    }
                } else {
                    None
                }
            } else {
                None
            };

            for (key, value) in raw.context.env {
                // Don't process if not needed:
                if let Some(only_ctx_keys) = only_ctx_keys.as_ref() {
                    if !only_ctx_keys.contains(key.as_str()) {
                        continue;
                    }
                }

                context.insert(
                    key.clone(),
                    value
                        .consume(
                            &key,
                            // Check if the default is banned:
                            if let Some(banned) = banned_env_defaults.as_ref() {
                                banned.contains(key.as_str())
                            } else {
                                false
                            },
                        )
                        .change_context(Zerr::ContextLoadError)
                        .attach_printable_lazy(|| format!("Ctx env var: '{key}'"))?,
                );
            }

            // External commands can be extremely slow compared to the rest of the library,
            // try and remedy a bit by running them in parallel:
            let mut cli_initials_used = false;
            let mut handles = vec![];
            let config_dir = final_config_path
                .parent()
                .ok_or_else(|| {
                    zerr!(
                        Zerr::InternalError,
                        "Failed to get parent dir of config file: {}",
                        final_config_path.display()
                    )
                })?
                .to_path_buf();
            for (key, value) in raw.context.cli {
                // Don't process if not needed:
                if let Some(only_ctx_keys) = only_ctx_keys.as_ref() {
                    if !only_ctx_keys.contains(key.as_str()) {
                        continue;
                    }
                }

                // If on the initial render, and the cli var has an initial value, use it instead of computing:
                if use_cli_initials {
                    if let Some(initial) = value.initial {
                        context.insert(key.clone(), initial);
                        cli_initials_used = true;
                        continue;
                    }
                }

                let config_dir_clone = config_dir.clone();
                handles.push(std::thread::spawn(
                    move || -> Result<(String, serde_json::Value), Zerr> {
                        let value = value
                            .consume(&config_dir_clone)
                            .change_context(Zerr::ContextLoadError)
                            .attach_printable_lazy(|| format!("Ctx cli var: '{key}'"))?;
                        Ok((key, value))
                    },
                ));
            }

            for handle in handles {
                let (key, value) = handle.join().unwrap()?;
                context.insert(key, value);
            }

            let config = Config {
                raw: cloned_raw,
                context,
                exclude: raw.exclude,
                engine: raw.engine,
                ignore_files: raw.ignore_files,
                matchers: raw.matchers,
                tasks: raw.tasks,
                final_config_path,
                cli_initials_used,
                from_tmp_cache: false,
            };

            Ok(config)
        }
    )?;

    debug!("Processed config: \n{:#?}", config);

    Ok(config)
}

/// Get the final config path, errors if path doesn't exist.
/// For render subcommand usage, if the config path is relative and doesn't exist to run directory, will also check relative to root directory.
fn build_final_config_path(config: &Path, render_root: Option<&Path>) -> Result<PathBuf, Zerr> {
    if config.exists() {
        return Ok(config.to_path_buf());
    }

    // Try also reading relative to the render root (if render subcommand):
    if let Some(render_root) = render_root {
        if config.is_relative() {
            let maybe_config_path = render_root.join(config);
            if maybe_config_path.exists() {
                return Ok(maybe_config_path);
            }
        }
    };

    Err(zerr!(
        Zerr::ConfigInvalid,
        "Config file not found at '{}'.",
        config.display()
    ))
}
