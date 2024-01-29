use std::{fs, path::Path};

use crate::{init::update_schema_directive_if_needed, prelude::*};

/// Reads & pre-parses the config and updates managed sections, returns updated to save and use if changes needed.
///
/// E.g. currently just updates the schema directive if needs changing.
pub fn read_and_auto_update(config_path: &Path) -> Result<String, Zerr> {
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
