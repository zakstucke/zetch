use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use super::{context::Context, engine::Engine, tasks::Tasks};
use crate::{init::update_schema_directive_if_needed, prelude::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
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
    #[serde(default = "Tasks::default")]
    pub tasks: Tasks,
}

fn default_matchers() -> Vec<String> {
    vec!["zetch".into()]
}

impl Config {
    pub fn ctx_keys(&self) -> Vec<&str> {
        let mut keys = Vec::new();

        for (key, _) in self.context.stat.iter() {
            keys.push(key.as_str());
        }

        for (key, _) in self.context.env.iter() {
            keys.push(key.as_str());
        }

        for (key, _) in self.context.cli.iter() {
            keys.push(key.as_str());
        }

        keys
    }

    pub fn from_toml(config_path: &Path) -> Result<Self, Report<Zerr>> {
        Config::from_toml_inner(config_path).attach_printable_lazy(|| {
            format!(
                "Error reading config file from '{}'.",
                config_path.display()
            )
        })
    }

    fn from_toml_inner(config_path: &Path) -> Result<Self, Report<Zerr>> {
        let contents = autoupdate(config_path)?;

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
        let mut config: Config =
            serde_json::from_value(json).change_context(Zerr::InternalError)?;

        super::validate::post_validate(&mut config, config_path)?;

        Ok(config)
    }
}

/// Reads & pre-parses the config and updates managed sections, returns updated to save and use if changes needed.
///
/// E.g. currently just updates the schema directive if needs changing.
fn autoupdate(config_path: &Path) -> Result<String, Report<Zerr>> {
    let mut contents = fs::read_to_string(config_path).change_context(Zerr::InternalError)?;
    let mut updated = false;

    // Handle the schema directive:
    if let Some(new_contents) = update_schema_directive_if_needed(&contents) {
        // Re-write schema to file first:
        contents = new_contents;
        updated = true;
    }

    if updated {
        fs::write(config_path, &contents).change_context(Zerr::InternalError)?;
    }

    Ok(contents)
}
