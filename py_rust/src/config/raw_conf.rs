use std::{collections::HashMap, path::Path};

use bitbazaar::{
    cli::{execute_bash, CmdErr, CmdOut},
    timeit,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::{coerce, engine::Engine, src_read::read_and_auto_update};
use crate::prelude::*;

// String literal of json, str, int, float, bool:
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Coerce {
    Json,
    Str,
    Int,
    Float,
    Bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CtxStaticVar {
    pub value: serde_json::Value,
    pub coerce: Option<Coerce>,
}

impl CtxStaticVar {
    pub fn consume(self) -> Result<serde_json::Value, Zerr> {
        coerce(self.value, self.coerce)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CtxEnvVar {
    pub env_name: Option<String>,
    pub default: Option<serde_json::Value>,
    pub coerce: Option<Coerce>,
}

impl CtxEnvVar {
    pub fn consume(self, key_name: &str, default_banned: bool) -> Result<serde_json::Value, Zerr> {
        let env_name = match self.env_name {
            Some(env_name) => env_name,
            None => key_name.to_string(),
        };

        let value = match std::env::var(&env_name) {
            Ok(value) => value,
            Err(_) => {
                if self.default.is_some() && default_banned {
                    return Err(zerr!(
                        Zerr::ContextLoadError,
                        "Could not find environment variable '{}' and the default has been banned using the 'ban-defaults' cli option.",
                        env_name
                    ));
                } else {
                    match self.default {
                        Some(value) => return Ok(value),
                        None => {
                            return Err(zerr!(
                                Zerr::ContextLoadError,
                                "Could not find environment variable '{}' and no default provided.",
                                env_name
                            ))
                        }
                    }
                }
            }
        };

        let value = serde_json::Value::String(value);

        coerce(value, self.coerce)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CtxCliVar {
    pub commands: Vec<String>,
    pub coerce: Option<Coerce>,
    pub initial: Option<serde_json::Value>,
}

impl CtxCliVar {
    pub fn consume(self) -> Result<serde_json::Value, Zerr> {
        let commands = self.commands;

        let runner = |command: &str| -> Result<CmdOut, Zerr> {
            debug!("Running command: {}", command);

            let cmd_out = match timeit!(format!("Cmd: {}", command).as_str(), {
                execute_bash(command)
            }) {
                Ok(cmd_out) => Ok(cmd_out),
                Err(e) => match e.current_context() {
                    CmdErr::InternalError => Err(e.change_context(Zerr::InternalError)),
                    _ => Err(e.change_context(Zerr::UserCommandError)),
                },
            }?;

            if cmd_out.code != 0 {
                return Err(zerr!(
                    Zerr::UserCommandError,
                    "Returned non zero exit code: {}. Std output: {}",
                    cmd_out.code,
                    cmd_out.std_all()
                ));
            }

            Ok(cmd_out)
        };

        // Run each command before the last:
        for command in commands[..commands.len() - 1].iter() {
            runner(command).attach_printable_lazy(|| format!("Command: '{command}'"))?;
        }

        // Run the last and store its stdout as the value:
        let final_cmd = &commands[commands.len() - 1];
        let cmd_out =
            runner(final_cmd).attach_printable_lazy(|| format!("Command: '{final_cmd}'"))?;
        if cmd_out.stdout.trim().is_empty() {
            return Err(zerr!(
                Zerr::UserCommandError,
                "Implicit None. Final cli script returned nothing.",
            )
            .attach_printable(format!("Command: '{final_cmd}'")));
        }
        let value = serde_json::Value::String(cmd_out.stdout);

        coerce(value, self.coerce)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Context {
    #[serde(rename(deserialize = "static"))]
    #[serde(default = "HashMap::new")]
    pub stat: HashMap<String, CtxStaticVar>,

    #[serde(default = "HashMap::new")]
    pub env: HashMap<String, CtxEnvVar>,

    #[serde(default = "HashMap::new")]
    pub cli: HashMap<String, CtxCliVar>,
}

impl Context {
    pub fn default() -> Self {
        Self {
            stat: HashMap::new(),
            env: HashMap::new(),
            cli: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RawConfig {
    // All should be optional to allow empty config file, even though it wouldn't make too much sense!
    #[serde(default = "Context::default")]
    pub context: Context,
    #[serde(default = "Vec::new")]
    pub exclude: Vec<String>,
    #[serde(default = "Engine::default")]
    pub engine: Engine,
    #[serde(default = "Vec::new")]
    pub ignore_files: Vec<String>,
    #[serde(default = "default_matchers")]
    pub matchers: Vec<String>,
}

fn default_matchers() -> Vec<String> {
    vec!["zetch".into()]
}

impl RawConfig {
    pub fn all_context_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();

        for (key, _) in self.context.stat.iter() {
            keys.push(key.clone());
        }

        for (key, _) in self.context.env.iter() {
            keys.push(key.clone());
        }

        for (key, _) in self.context.cli.iter() {
            keys.push(key.clone());
        }

        keys
    }

    pub fn from_toml(config_path: &Path) -> Result<Self, Zerr> {
        RawConfig::from_toml_inner(config_path).attach_printable_lazy(|| {
            format!(
                "Error reading config file from '{}'.",
                config_path.display()
            )
        })
    }

    fn from_toml_inner(config_path: &Path) -> Result<Self, Zerr> {
        let contents = read_and_auto_update(config_path)?;

        // Decode directly the toml directly into serde/json, using that internally:
        let json: serde_json::Value = match toml::from_str(&contents) {
            Ok(toml) => toml,
            Err(e) => {
                return Err(zerr!(
                    Zerr::ConfigInvalid,
                    "Invalid toml formatting: '{}'.",
                    e
                ))
            }
        };

        // This will check against the json schema,
        // can produce much better errors than the toml decoder can, so prevalidate first:
        super::validate::pre_validate(&json)?;

        // Now deserialize after validation:
        let mut config: RawConfig =
            serde_json::from_value(json).change_context(Zerr::InternalError)?;

        super::validate::post_validate(&mut config, config_path)?;

        Ok(config)
    }
}