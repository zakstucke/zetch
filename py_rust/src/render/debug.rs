use crate::state::State;

#[derive(Debug, serde::Serialize)]
pub struct Debug {
    pub state: State,
    pub written: Vec<String>,
    pub identical: Vec<String>,
    pub matched_templates: Vec<String>,
    pub lockfile_modified: bool,
}
