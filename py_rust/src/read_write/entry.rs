use super::{
    delete::handle_delete, filetype::get_filetype, put::handle_put, read::handle_read,
    source::Source,
};
use crate::{
    args::{DelCommand, FileSharedArgs, PutCommand, ReadCommand},
    prelude::*,
};

pub enum FileCommand<'a> {
    Read(&'a ReadCommand),
    Put(&'a PutCommand),
    Delete(&'a DelCommand),
}

impl<'a> FileCommand<'a> {
    fn shared(&self) -> &FileSharedArgs {
        match self {
            FileCommand::Read(cmd) => &cmd.shared,
            FileCommand::Put(cmd) => &cmd.shared,
            FileCommand::Delete(cmd) => &cmd.shared,
        }
    }
}

impl<'a> From<&'a ReadCommand> for FileCommand<'a> {
    fn from(cmd: &'a ReadCommand) -> Self {
        FileCommand::Read(cmd)
    }
}

impl<'a> From<&'a PutCommand> for FileCommand<'a> {
    fn from(cmd: &'a PutCommand) -> Self {
        FileCommand::Put(cmd)
    }
}

impl<'a> From<&'a DelCommand> for FileCommand<'a> {
    fn from(cmd: &'a DelCommand) -> Self {
        FileCommand::Delete(cmd)
    }
}

pub fn handle_file_cmd(args: &crate::args::Args, fargs: FileCommand) -> Result<(), Zerr> {
    let sargs = fargs.shared();

    let mut source = Source::new(&sargs.source)?;
    let file_contents = source.read()?;

    // Convert the . separated path into a Vec<&str>:
    let path = sargs.content_path.split('.').collect::<Vec<&str>>();

    // Zetch should be used for reading and writing to parts of files, not creating, deleting, reading full files which are very easy to do outside of zetch:
    if path.is_empty() {
        return Err(zerr!(Zerr::FilePathError, "Path cannot be empty."));
    }

    let ft = get_filetype(args, sargs, &file_contents, &source)?;
    match fargs {
        FileCommand::Delete(dargs) => handle_delete(dargs, &path, ft, file_contents, source)?,
        FileCommand::Put(pargs) => handle_put(pargs, &path, ft, file_contents, source)?,
        FileCommand::Read(rargs) => handle_read(rargs, &path, ft, file_contents)?,
    }

    Ok(())
}
