use super::{raise_invalid_path, traverser::TravNode};
use crate::{
    args::{FileCommand, ReadOutputFormat},
    prelude::*,
};

/// Handle reads prints the result to stdout in the user specified format.
///
/// Note the file should already be checked to be valid for the given type and so the initial load should raise InternalError if it fails (aka it shouldn't fail.)
pub fn handle_read<'r>(
    _args: &crate::args::Args,
    fargs: &FileCommand,
    path: &[&'r str],
    mut manager: super::langs::Manager<'r>,
) -> Result<(), Zerr> {
    let trav = manager.traverser()?;

    for key in path.iter() {
        match trav.active()? {
            TravNode::Array => {
                let index = trav.key_as_index(key)?;

                if index >= trav.array_len()? {
                    return Err(
                        raise_invalid_path!(path, path.len() - 1, trav.active_as_serde()?)
                            .attach_printable(format!("Array index '{}' is out of bounds.", key)),
                    );
                }

                trav.array_enter(index)?;
            }
            TravNode::Object => {
                if !trav.object_key_exists(key)? {
                    return Err(
                        raise_invalid_path!(path, path.len() - 1, trav.active_as_serde()?)
                            .attach_printable(format!("Object key '{}' does not exist.", key)),
                    );
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

    let as_serde = trav.active_as_serde()?;

    trav.finish()?;

    // Handle different output formats:
    match fargs.output {
        ReadOutputFormat::Raw => match as_serde {
            serde_json::Value::String(s) => println!("{}", s),
            as_serde => println!(
                "{}",
                serde_json::to_string(&as_serde).change_context(Zerr::InternalError)?
            ),
        },
        ReadOutputFormat::Json => println!(
            "{}",
            serde_json::to_string(&as_serde).change_context(Zerr::InternalError)?
        ),
    }

    Ok(())
}
