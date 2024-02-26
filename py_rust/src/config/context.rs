use std::{collections::HashMap, path::Path};

use bitbazaar::cli::{Bash, BashErr};
use serde::{Deserialize, Serialize};

use super::static_var::CtxStaticVar;
use crate::{
    coerce::{coerce, Coerce},
    prelude::*,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CtxEnvVar {
    pub env_name: Option<String>,
    pub default: Option<CtxStaticVar>,
    pub coerce: Option<Coerce>,
}

impl CtxEnvVar {
    pub fn read(&self, key_name: &str, default_banned: bool) -> Result<serde_json::Value, Zerr> {
        let env_name = match &self.env_name {
            Some(env_name) => env_name,
            None => key_name,
        };

        let value = match std::env::var(env_name) {
            Ok(value) => value,
            Err(_) => {
                if self.default.is_some() && default_banned {
                    return Err(zerr!(
                        Zerr::ContextLoadError,
                        "Could not find environment variable '{}' and the default has been banned using the 'ban-defaults' cli option.",
                        env_name
                    ));
                } else {
                    match &self.default {
                        Some(value) => return value.read(),
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

        coerce(&serde_json::Value::String(value), &self.coerce)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CtxCliVar {
    pub commands: Vec<String>,
    pub coerce: Option<Coerce>,
    pub light: Option<CtxStaticVar>,
}

impl CtxCliVar {
    pub fn read(&self, config_path: &Path) -> Result<serde_json::Value, Zerr> {
        let config_dir = config_path.parent().ok_or_else(|| {
            zerr!(
                Zerr::InternalError,
                "Failed to get parent dir of config file: {}",
                config_path.display()
            )
        })?;

        let mut bash = Bash::new().chdir(config_dir);
        for command in self.commands.iter() {
            bash = bash.cmd(command);
        }
        let cmd_out = match bash.run() {
            Ok(cmd_out) => Ok(cmd_out),
            Err(e) => match e.current_context() {
                BashErr::InternalError(_) => Err(e.change_context(Zerr::InternalError)),
                _ => Err(e.change_context(Zerr::UserCommandError)),
            },
        }?;
        cmd_out.throw_on_bad_code(Zerr::UserCommandError)?;

        // Prevent empty output:
        let last_cmd_out = cmd_out.last_stdout();
        if last_cmd_out.trim().is_empty() {
            return Err(zerr!(
                Zerr::UserCommandError,
                "Implicit None. Final cli command returned nothing.",
            )
            .attach_printable(cmd_out.fmt_attempted_commands()));
        }

        coerce(&serde_json::Value::String(last_cmd_out), &self.coerce)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Context {
    #[serde(rename(deserialize = "static"))]
    #[serde(default = "HashMap::new")]
    pub stat: HashMap<String, CtxStaticVar>,

    #[serde(default = "HashMap::new")]
    pub env: HashMap<String, CtxEnvVar>,

    #[serde(default = "HashMap::new")]
    pub cli: HashMap<String, CtxCliVar>,
}
