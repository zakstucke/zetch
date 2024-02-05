use super::{raise_invalid_path, traverser::TravNode};
use crate::{args::FileCommand, prelude::*};

/// Handle deletions.
///
/// Note the file should already be checked to be valid for the given type and so the initial load should raise InternalError if it fails (aka it shouldn't fail.)
pub fn handle_delete<'r>(
    _args: &crate::args::Args,
    fargs: &FileCommand,
    path: &[&'r str],
    mut manager: super::langs::Manager<'r>,
) -> Result<(), Zerr> {
    // To reaccess root to compile, need to drop traverser, hence block:
    {
        let trav = manager.traverser()?;

        for (path_index, key) in path.iter().enumerate() {
            let is_last = path_index == path.len() - 1;

            match trav.active()? {
                TravNode::Array => {
                    let index = trav.key_as_index(key)?;
                    // Silently exit if out of bounds on delete:
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
                    // Silently exit if already missing on delete:
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
                    ))
                }
            }
        }

        trav.finish()?;
    };

    // Rewrite the file:
    std::fs::write(&fargs.filepath, manager.rewrite()?).change_context(Zerr::InternalError)?;

    Ok(())
}
