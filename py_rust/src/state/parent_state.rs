use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use super::State;
use crate::{
    config::{conf::Config, tasks::parent_task_active},
    prelude::*,
};

pub static CACHED_STATE_ENV_VAR: &str = "ZETCH_TMP_STORED_CONFIG_PATH";

/// The parts of State that should be stored between parent/child.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredParentState {
    pub conf: Config,
    pub ctx: HashMap<String, serde_json::Value>,
    pub final_config_path: PathBuf,
}

/// Cache the config in a temporary file, used in e.g. subcommands that might read the config.
///
/// Returns the PathBuf to the temporary file.
pub fn store_parent_state(state: &State) -> Result<PathBuf, Zerr> {
    let state = StoredParentState {
        conf: state.conf.clone(),
        ctx: state.ctx.clone(),
        final_config_path: state.final_config_path.clone(),
    };

    let temp = NamedTempFile::new().change_context(Zerr::InternalError)?;
    serde_json::to_writer(&temp, &state).change_context(Zerr::InternalError)?;
    Ok(temp.path().to_path_buf())
}

/// Load the cached state if it's available, return None otherwise.
pub fn load_parent_state() -> Result<Option<StoredParentState>, Zerr> {
    // If not in a task, parent state shouldn't be set or used:
    if !parent_task_active() {
        return Ok(None);
    }

    if let Ok(path) = std::env::var(CACHED_STATE_ENV_VAR) {
        let path = Path::new(&path);
        if path.exists() {
            let contents = std::fs::read_to_string(path).change_context(Zerr::InternalError)?;
            return Ok(Some(
                serde_json::from_str(&contents).change_context(Zerr::InternalError)?,
            ));
        }
    }
    Ok(None)
}
