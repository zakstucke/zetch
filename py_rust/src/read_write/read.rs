use super::{filetype::FileType, langs, traverser::TravNode, utils::raise_invalid_path};
use crate::{
    args::{ReadCommand, ReadOutputFormat},
    prelude::*,
};

/// Handle reads prints the result to stdout in the user specified format.
///
/// Note the file should already be checked to be valid for the given type and so the initial load should raise InternalError if it fails (aka it shouldn't fail.)
pub fn handle_read(
    fargs: &ReadCommand,
    path: &[&str],
    ft: FileType,
    file_contents: String,
) -> Result<(), Report<Zerr>> {
    let mut manager = langs::Manager::new(ft, &file_contents)?;

    let trav = manager.traverser()?;

    for key in path.iter() {
        match trav.active()? {
            TravNode::Array => {
                let index = trav.key_as_index(key)?;

                if index >= trav.array_len()? {
                    return Err(
                        raise_invalid_path!(path, path.len() - 1, trav.active_as_serde()?)
                            .attach_printable(format!("Array index '{key}' is out of bounds.")),
                    );
                }

                trav.array_enter(index)?;
            }
            TravNode::Object => {
                if !trav.object_key_exists(key)? {
                    return Err(
                        raise_invalid_path!(path, path.len() - 1, trav.active_as_serde()?)
                            .attach_printable(format!("Object key '{key}' does not exist.")),
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
            serde_json::Value::String(s) => println!("{s}"),
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
