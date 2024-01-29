use crate::config::Config;

#[derive(Debug, serde::Serialize)]
pub struct Debug {
    pub config: Config,
    pub written: Vec<String>,
    pub identical: Vec<String>,
    pub matched_templates: Vec<String>,
    pub lockfile_modified: bool,
}
