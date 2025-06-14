use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};

use super::static_var::CtxStaticVar;
use crate::{
    coerce::{Coerce, coerce},
    prelude::*,
    utils::cmd::run_cmd,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CtxEnvVar {
    pub env_name: Option<String>,
    pub default: Option<CtxStaticVar>,
    pub coerce: Option<Coerce>,
}

impl CtxEnvVar {
    pub fn read(
        &self,
        key_name: &str,
        default_banned: bool,
    ) -> Result<serde_json::Value, Report<Zerr>> {
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
                            ));
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
    pub fn read(&self, config_path: &Path) -> Result<serde_json::Value, Report<Zerr>> {
        let config_dir = config_path.parent().ok_or_else(|| {
            zerr!(
                Zerr::InternalError,
                "Failed to get parent dir of config file: {}",
                config_path.display()
            )
        })?;

        let cmd_out = run_cmd(config_dir, &self.commands, &[])?;
        let last_stdout = cmd_out.last_stdout();

        // Prevent empty output:
        if last_stdout.trim().is_empty() {
            let mut e = zerr!(
                Zerr::UserCommandError,
                "Implicit None. Final cli command returned nothing.",
            );
            for output in cmd_out.iter_formatted_commands_and_outputs() {
                e = e.attach_printable(output);
            }
            return Err(e);
        }

        coerce(
            &serde_json::Value::String(last_stdout.trim().to_string()),
            &self.coerce,
        )
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
