use super::{filetype::FileType, langs, raise_invalid_path, traverser::TravNode};
use crate::{args::FileCommand, coerce::coerce, prelude::*};

static EMPTY_OBJ: &str = "{}";

/// Handle putting.
///
/// Note the file should already be checked to be valid for the given type and so the initial load should raise InternalError if it fails (aka it shouldn't fail.)
pub fn handle_put(
    fargs: &FileCommand,
    path: &[&str],
    to_write: String,
    ft: FileType,
    file_contents: String,
) -> Result<(), Zerr> {
    let mut manager = langs::Manager::new(ft, &file_contents)?;
    let to_write_val = coerce(serde_json::Value::String(to_write), &fargs.coerce)?;
    let to_write_json = serde_json::to_string(&to_write_val).change_context(Zerr::InternalError)?;

    let mut modified = false;

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

                    let to_add = if is_last { &to_write_json } else { EMPTY_OBJ };

                    // If push is true means was missing, either way need to add, if last always replacing/adding:
                    if is_last || push {
                        if push {
                            trav.array_push(to_add)?;
                            modified = true;
                        } else {
                            // Replacing an existing value, don't rewrite if identical value:
                            let existing = match trav.active_as_serde()? {
                                serde_json::Value::Array(mut arr) => arr.swap_remove(index),
                                _ => {
                                    return Err(zerr_int!("Couldn't extract existing array value."))
                                }
                            };
                            if existing != to_write_val {
                                trav.array_set_index(index, to_add)?;
                                modified = true;
                            }
                        }
                    }
                    trav.array_enter(index)?;
                }
                TravNode::Object => {
                    let existing = match trav.active_as_serde()? {
                        serde_json::Value::Object(mut obj) => obj.remove(*key),
                        _ => return Err(zerr_int!("Couldn't extract existing object value.")),
                    };

                    if is_last {
                        // Don't rewrite if the value already exists and is identical:
                        if existing.is_none() || (existing.unwrap() != to_write_val) {
                            trav.object_set_key(key, &to_write_json)?;
                            modified = true;
                        }
                    } else if existing.is_none() {
                        // Fill in intermediary objects in puts if missing:
                        trav.object_set_key(key, EMPTY_OBJ)?;
                        modified = true;
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

    // Rewrite the file only if modified, if not don't want to touch:
    if modified {
        std::fs::write(&fargs.filepath, manager.rewrite()?).change_context(Zerr::InternalError)?;
    }

    Ok(())
}
