use super::{
    filetype::FileType, langs, source::Source, traverser::TravNode, utils::raise_invalid_path,
};
use crate::{args::DelCommand, prelude::*};

/// Handle deletions.
///
/// Note the file should already be checked to be valid for the given type and so the initial load should raise InternalError if it fails (aka it shouldn't fail.)
pub fn handle_delete(
    _fargs: &DelCommand,
    path: &[&str],
    ft: FileType,
    file_contents: String,
    source: Source,
) -> Result<(), Report<Zerr>> {
    let mut manager = langs::Manager::new(ft, &file_contents)?;

    // To reaccess root to compile, need to drop traverser, hence block:
    {
        let trav = manager.traverser()?;

        for (path_index, key) in path.iter().enumerate() {
            let is_last = path_index == path.len() - 1;

            match trav.active()? {
                TravNode::Array => {
                    let index = trav.key_as_index(key)?;
                    // Silently exit if out of bounds on delete (don't want to modify file or error):
                    if index >= trav.array_len()? {
                        return Ok(());
                    }
                    if is_last {
                        trav.array_delete_index(index)?;
                    } else {
                        trav.array_enter(index)?;
                    }
                }
                TravNode::Object => {
                    // Silently exit if already missing on delete (don't want to modify file or error):
                    if !trav.object_key_exists(key)? {
                        return Ok(());
                    }
                    if is_last {
                        trav.object_delete_key(key)?;
                    } else {
                        trav.object_enter(key)?;
                    }
                }
                _ => {
                    return Err(raise_invalid_path!(
                        path,
                        path.len() - 1,
                        trav.active_as_serde()?
                    ));
                }
            }
        }

        trav.finish()?;
    };

    // Rewrite the file:
    source.write(&manager.rewrite()?)?;

    Ok(())
}
