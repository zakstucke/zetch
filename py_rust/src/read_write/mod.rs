mod delete;
mod filetype;
mod langs;
mod put;
mod read;
mod traverser;

use self::{delete::handle_delete, filetype::get_filetype, put::handle_put, read::handle_read};
use crate::{args::FileCommand, coerce::coerce, prelude::*};

pub fn handle_file_cmd(args: &crate::args::Args, fargs: &FileCommand) -> Result<(), Zerr> {
    let cmd_type = if fargs.put.is_some() && fargs.delete {
        return Err(zerr!(
            Zerr::FileCmdUsageError,
            "Only one of '--write' or '--delete' can be specified, read is inferred when neither are specified."
        ));
    } else if fargs.delete {
        CommandType::Delete
    } else if let Some(content) = fargs.put.clone() {
        CommandType::Put(content)
    } else {
        CommandType::Read
    };

    // Read the file:
    let file_contents =
        std::fs::read_to_string(&fargs.filepath).change_context(Zerr::FileNotFound)?;

    let manager = langs::Manager::new(get_filetype(args, fargs, &file_contents)?, &file_contents)?;

    // Convert the . separated path into a Vec<&str>:
    let path = fargs.path.split('.').collect::<Vec<&str>>();

    // Zetch should be used for reading and writing to parts of files, not creating, deleting, reading full files which are very easy to do outside of zetch:
    if path.is_empty() {
        return Err(zerr!(Zerr::FilePathError, "Path cannot be empty."));
    }

    match cmd_type {
        CommandType::Delete => handle_delete(args, fargs, &path, manager)?,
        CommandType::Put(to_write) => {
            // Coerce the type then convert into a json string, complex borrowing & cross compatibility semantics require this:
            let to_put_str =
                serde_json::to_string(&coerce(serde_json::Value::String(to_write), &fargs.coerce)?)
                    .change_context(Zerr::InternalError)?;
            handle_put(args, fargs, &path, &to_put_str, manager)?
        }
        CommandType::Read => handle_read(args, fargs, &path, manager)?,
    }

    Ok(())
}

#[derive(Debug)]
enum CommandType {
    Read,
    Put(String),
    Delete,
}

/// Simplifies creating path errs:
macro_rules! raise_invalid_path {
    ($path:expr, $cur_index:expr, $parent:expr) => {
        zerr!(
            Zerr::FilePathError,
            "Invalid key '{}' at path location '{}'. Parent value below.",
            $path[$cur_index],
            $path[..$cur_index].join(".")
        )
        .attach_printable($parent)
    };
}

pub(crate) use raise_invalid_path;
