use super::{raise_invalid_path, traverser::TravNode};
use crate::{args::FileCommand, prelude::*};

static EMPTY_OBJ: &str = "{}";

/// Handle putting.
///
/// Note the file should already be checked to be valid for the given type and so the initial load should raise InternalError if it fails (aka it shouldn't fail.)
pub fn handle_put<'r>(
    _args: &crate::args::Args,
    fargs: &FileCommand,
    path: &[&'r str],
    to_write: &'r str,
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

                    let mut push = false;
                    match index.cmp(&trav.array_len()?) {
                        // Fine:
                        std::cmp::Ordering::Less => (),
                        // One greater than current max index, allow pushing onto end
                        std::cmp::Ordering::Equal => push = true,
                        // Out of bounds:
                        std::cmp::Ordering::Greater => {
                            return Err(raise_invalid_path!(
                                path,
                                path.len() - 1,
                                trav.active_as_serde()?
                            )
                            .attach_printable(format!("Array index '{}' is out of bounds.", key)));
                        }
                    }

                    let to_add = if is_last { to_write } else { EMPTY_OBJ };

                    // If push is true means was missing, either way need to add, if last always replacing/adding:
                    if is_last || push {
                        if push {
                            trav.array_push(to_add)?;
                        } else {
                            trav.array_set_index(index, to_add)?;
                        }
                    }

                    trav.array_enter(index)?;
                }
                TravNode::Object => {
                    if is_last {
                        trav.object_set_key(key, to_write)?;
                    } else if !trav.object_key_exists(key)? {
                        // Fill in intermediary objects in puts if missing:
                        trav.object_set_key(key, EMPTY_OBJ)?;
                    }

                    trav.object_enter(key)?;
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
