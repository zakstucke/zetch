use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Engine {
    #[serde(default = "default_block_start")]
    pub block_start: String,
    #[serde(default = "default_block_end")]
    pub block_end: String,
    #[serde(default = "default_variable_start")]
    pub variable_start: String,
    #[serde(default = "default_variable_end")]
    pub variable_end: String,
    #[serde(default = "default_comment_start")]
    pub comment_start: String,
    #[serde(default = "default_comment_end")]
    pub comment_end: String,
    #[serde(default = "default_custom_extensions")]
    pub custom_extensions: Vec<String>,
}

impl Engine {
    pub fn default() -> Self {
        Self {
            // NOTE: when adding new, make sure to update schema.json and tests/helpers/types.py plus update tests.
            block_start: default_block_start(),
            block_end: default_block_end(),
            variable_start: default_variable_start(),
            variable_end: default_variable_end(),
            comment_start: default_comment_start(),
            comment_end: default_comment_end(),
            custom_extensions: default_custom_extensions(),
        }
    }
}

fn default_block_start() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "{%".to_string()
}

fn default_block_end() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "%}".to_string()
}

fn default_variable_start() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "{{".to_string()
}

fn default_variable_end() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "}}".to_string()
}

fn default_comment_start() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "{#".to_string()
}

fn default_comment_end() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "#}".to_string()
}

fn default_custom_extensions() -> Vec<String> {
    // NOTE: when changing make sure to update schema.json default for config hinting
    vec![]
}
