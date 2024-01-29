mod coerce;
mod engine;
mod process;
mod raw_conf;
mod src_read;
mod validate;

use std::path::{Path, PathBuf};

pub use coerce::coerce;
pub use engine::{register_py_func, PY_CONTEXT};
pub use process::{process, Config};
pub use raw_conf::RawConfig;

use crate::prelude::*;

/// Get the final config path, errors if path doesn't exist.
/// For render subcommand usage, if the config path is relative and doesn't exist to run directory, will also check relative to root directory.
pub fn final_config_path(config: &Path, render_root: Option<&Path>) -> Result<PathBuf, Zerr> {
    if config.exists() {
        return Ok(config.to_path_buf());
    }

    // Try also reading relative to the render root (if render subcommand):
    if let Some(render_root) = render_root {
        if config.is_relative() {
            let maybe_config_path = render_root.join(config);
            if maybe_config_path.exists() {
                return Ok(maybe_config_path);
            }
        }
    };

    Err(zerr!(
        Zerr::ConfigInvalid,
        "Failed to read config file at '{}'.",
        config.display()
    ))
}
