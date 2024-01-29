use std::collections::{HashMap, HashSet};

use serde::Serialize;
use tracing::debug;

use super::{engine::Engine, raw_conf::RawConfig};
use crate::{args::RenderCommand, prelude::*};

#[derive(Debug, Serialize)]
pub struct Config {
    pub raw: RawConfig,
    pub context: HashMap<String, serde_json::Value>,
    pub exclude: Vec<String>,
    pub engine: Engine,
    pub ignore_files: Vec<String>,
    pub matchers: Vec<String>,
}

pub fn process(
    raw: RawConfig,
    render_args: Option<&RenderCommand>,
    only_ctx_keys: Option<HashSet<&str>>,
    use_cli_initials: bool,
) -> Result<Config, Zerr> {
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
    let banned_env_defaults: Option<HashSet<String>> = if let Some(render_args) = render_args {
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
    let mut handles = vec![];
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
                continue;
            }
        }

        handles.push(std::thread::spawn(
            move || -> Result<(String, serde_json::Value), Zerr> {
                let value = value
                    .consume()
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
    };

    debug!("Processed config: \n{:#?}", config);

    Ok(config)
}
