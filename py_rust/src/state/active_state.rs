use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use bitbazaar::timeit;
use parking_lot::Mutex;
use tempfile::NamedTempFile;

use super::parent_state::load_parent_state;
use crate::{args::Command, config::conf::Config, prelude::*};

#[derive(Debug)]
pub struct State {
    pub args: crate::args::Args,

    /// Raw config decoded from the config file.
    pub conf: Config,
    pub final_config_path: PathBuf,

    /// True if this state has been loaded in from a parent process:
    /// The currently loaded values, this starts of empty until explicitly loaded using load_var() or load_all_vars():
    pub ctx: HashMap<String, serde_json::Value>,

    /// True if --light or --superlight
    pub light: bool,

    /// True if --superlight
    pub superlight: bool,

    // Storing the cached state file to prevent being dropped too early:
    // Bit of a hack, but mutex easier than making state mutable where this needs to be set:
    pub cached_state_file: Mutex<Option<NamedTempFile>>,
}

impl State {
    /// Creates initial state for the command.
    /// This will not process any context by default. It may not be needed.
    /// Will automatically run the pre-tasks if needed.
    pub fn new(args: &crate::args::Args) -> Result<State, Zerr> {
        // If running as a subprocess of a zetch command and is applicable (i.e. in a post task),
        // then use some of its state (ctx/config) to prevent recursion errors and avoid unnecessary processing):
        let state = if let Some(parent_shared_state) = load_parent_state()? {
            Self {
                args: args.clone(),
                conf: parent_shared_state.conf,
                ctx: parent_shared_state.ctx,
                final_config_path: parent_shared_state.final_config_path,
                light: false,
                superlight: false,
                cached_state_file: Mutex::new(None),
            }
        } else {
            let final_config_path = build_final_config_path(
                &args.config,
                if let crate::args::Command::Render(render) = &args.command {
                    Some(&render.root)
                } else {
                    None
                },
            )?;

            let conf = timeit!("Config processing", {
                Config::from_toml(&final_config_path)
            })?;

            // Run the pre-tasks if applicable to the active command.
            // Note this won't be run if in child process (which makes sense), due to above return.
            let command_expecting_tasks = match &args.command {
                crate::args::Command::Render(_) | crate::args::Command::Var(_) => true,
                crate::args::Command::Read(_)
                | crate::args::Command::Put(_)
                | crate::args::Command::Del(_)
                | crate::args::Command::Init(_)
                | crate::args::Command::ReplaceMatcher(_)
                | crate::args::Command::Version { .. } => false,
            };

            // Set light to true if --light or --superlight, and superlight to true if --superlight:
            let (light, superlight) = if let crate::args::Command::Render(render) = &args.command {
                (render.light || render.superlight, render.superlight)
            } else {
                (false, false)
            };

            // Run pre-tasks if the right type of command and not running in light/superlight mode:
            if command_expecting_tasks && !light {
                conf.tasks.run_pre(&final_config_path)?;
            }

            Self {
                args: args.clone(),
                conf,
                ctx: HashMap::new(),
                final_config_path,
                light,
                superlight,
                cached_state_file: Mutex::new(None),
            }
        };

        Ok(state)
    }

    /// Load a new context var, returning a reference to the value, and storing in state.ctx.
    /// This will also internally manage running pre tasks.
    pub fn load_var(
        &mut self,
        var: &str,
        default_banned: bool, // TODO want to internalise into state
    ) -> Result<&serde_json::Value, Zerr> {
        // If already exists use:
        if self.ctx.contains_key(var) {
            return Ok(self.ctx.get(var).unwrap());
        }

        let new_value = {
            if let Some(value) = self.conf.context.stat.get(var) {
                value.read()
            } else if let Some(value) = self.conf.context.env.get(var) {
                value.read(var, default_banned)
            } else if let Some(value) = self.conf.context.cli.get(var) {
                // In light mode use the user provided default or an empty string, rather than running a user command:
                if self.light {
                    if let Some(light_val) = &value.light {
                        Ok(light_val.read()?)
                    } else {
                        Ok(serde_json::Value::String("".to_string()))
                    }
                } else {
                    value.read(&self.final_config_path)
                }
            } else {
                // Otherwise something wrong in userland:
                return Err(zerr!(
                    Zerr::ReadVarMissing,
                    "Context variable '{}' not found in finalised config. All context keys: '{}'.",
                    var,
                    self.conf.ctx_keys().join(", ")
                ));
            }
        }
        .change_context(Zerr::ContextLoadError)
        .attach_printable_lazy(|| format!("Ctx var: '{}'", var))?;

        // Add to ctx and return reference:
        self.ctx.insert(var.to_string(), new_value);
        Ok(self.ctx.get(var).unwrap())
    }

    /// Load all context vars.
    pub fn load_all_vars(&mut self) -> Result<(), Zerr> {
        timeit!(
            "Context value extraction (including user task & cli env scripting)",
            {
                // Static vars:
                for key in self
                    .conf
                    .context
                    .stat
                    .keys()
                    .cloned()
                    .collect::<Vec<String>>()
                {
                    self.load_var(&key, false)?;
                }

                // Env vars:
                // If some env defaults banned, validate list and convert to a hashset for faster lookup:
                let banned_env_defaults: Option<HashSet<String>> = if let Command::Render(
                    render_args,
                ) = &self.args.command
                {
                    if let Some(banned) = render_args.ban_defaults.as_ref() {
                        // If no vars provided, ban all defaults:
                        if banned.is_empty() {
                            Some(
                                self.conf
                                    .context
                                    .env
                                    .keys()
                                    .cloned()
                                    .collect::<HashSet<String>>(),
                            )
                        } else {
                            let banned_env_defaults: HashSet<String> =
                                banned.iter().cloned().collect();
                            // Make sure they are all valid env context keys:
                            for key in banned_env_defaults.iter() {
                                if !self.conf.context.env.contains_key(key) {
                                    // Printing the env keys in the error, want them alphabetically sorted:
                                    let mut env_keys = self
                                        .conf
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

                for key in self
                    .conf
                    .context
                    .env
                    .keys()
                    .cloned()
                    .collect::<Vec<String>>()
                {
                    self.load_var(
                        &key,
                        // Check if the default is banned:
                        if let Some(banned) = banned_env_defaults.as_ref() {
                            banned.contains(key.as_str())
                        } else {
                            false
                        },
                    )?;
                }

                // External commands can be extremely slow compared to the rest of the library,
                // try and remedy a bit by running them in parallel:
                let mut handles = vec![];
                for key in self
                    .conf
                    .context
                    .cli
                    .keys()
                    .cloned()
                    .collect::<Vec<String>>()
                {
                    // can't use load_var() as wanting to make parallel, so repeating logic here in a way that can be executed in parallel:

                    // If already exists skip:
                    if self.ctx.contains_key(&key) {
                        continue;
                    }

                    let var = self.conf.context.cli.get(&key).unwrap();
                    // If light mode, need to use the light user replacement otherwise an empty string: (no need for threads)
                    if self.light {
                        let value = if let Some(light_val) = &var.light {
                            light_val.read()?
                        } else {
                            serde_json::Value::String("".to_string())
                        };
                        self.ctx.insert(key, value);
                    } else {
                        let final_config_path = self.final_config_path.to_path_buf();
                        let var = var.clone();
                        handles.push(std::thread::spawn(
                            move || -> Result<(String, serde_json::Value), Zerr> {
                                timeit!(format!("Cli var processing: '{}'", &key).as_str(), {
                                    Ok((key, var.read(&final_config_path)?))
                                })
                            },
                        ));
                    }
                }

                for handle in handles {
                    match handle.join() {
                        Ok(fn_result) => {
                            let (key, value) = fn_result?;
                            self.ctx.insert(key, value);
                        }
                        Err(thread_err) => {
                            return Err(
                                zerr!(Zerr::InternalError, "Error reading thread result.",)
                                    .attach_printable(format!("Thread error: {:?}", thread_err)),
                            );
                        }
                    }
                }

                Ok(())
            }
        )?;

        Ok(())
    }
}

/// Get the final config path, errors if path doesn't exist.
/// For render subcommand usage, if the config path is relative and doesn't exist to run directory, will also check relative to root directory.
fn build_final_config_path(base_path: &Path, render_root: Option<&Path>) -> Result<PathBuf, Zerr> {
    if base_path.exists() {
        return Ok(base_path.to_path_buf());
    }

    // Try also reading relative to the render root (if render subcommand):
    if let Some(render_root) = render_root {
        if base_path.is_relative() {
            let maybe_config_path = render_root.join(base_path);
            if maybe_config_path.exists() {
                return Ok(maybe_config_path);
            }
        }
    };

    Err(zerr!(
        Zerr::ConfigInvalid,
        "State file not found at '{}'.",
        base_path.display()
    ))
}
