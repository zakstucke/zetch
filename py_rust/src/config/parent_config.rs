use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

use super::{tasks::parent_task_active, Config};
use crate::prelude::*;

pub static CACHED_CONFIG_ENV_VAR: &str = "ZETCH_TMP_STORED_CONFIG_PATH";

/// Cache the config in a temporary file, used in e.g. subcommands that might read the config.
///
/// Returns the PathBuf to the temporary file.
pub fn store_parent_config(config: &Config) -> Result<PathBuf, Zerr> {
    let temp = NamedTempFile::new().change_context(Zerr::InternalError)?;
    serde_json::to_writer(&temp, config).change_context(Zerr::InternalError)?;
    Ok(temp.path().to_path_buf())
}

/// Load the cached config if it's available, return None otherwise.
pub fn load_parent_config() -> Result<Option<Config>, Zerr> {
    // If not in a task, parent config shouldn't be set or used:
    if !parent_task_active() {
        return Ok(None);
    }

    if let Ok(path) = std::env::var(CACHED_CONFIG_ENV_VAR) {
        let path = Path::new(&path);
        if path.exists() {
            let contents = std::fs::read_to_string(path).change_context(Zerr::InternalError)?;
            let mut config: Config =
                serde_json::from_str(&contents).change_context(Zerr::InternalError)?;
            config.from_tmp_cache = true;
            return Ok(Some(config));
        }
    }
    Ok(None)
}
