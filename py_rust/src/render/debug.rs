use std::collections::HashMap;

use crate::config::conf::Config;

#[derive(Debug, serde::Serialize)]
pub struct Debug {
    pub conf: Config,
    pub ctx: HashMap<String, serde_json::Value>,
    pub written: Vec<String>,
    pub identical: Vec<String>,
    pub matched_templates: Vec<String>,
    pub lockfile_modified: bool,
}
