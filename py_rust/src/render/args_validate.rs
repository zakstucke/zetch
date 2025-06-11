use crate::{args::RenderCommand, prelude::*};

pub fn args_validate(args: &RenderCommand) -> Result<(), Report<Zerr>> {
    // Check the root path exists:
    if !args.root.exists() {
        return Err(zerr!(
            Zerr::RootError,
            "Root path does not exist: {}",
            args.root.display()
        ));
    }

    // Check the root path is a directory rather than a file:
    if !args.root.is_dir() {
        return Err(zerr!(
            Zerr::RootError,
            "Root path is not a directory: {}",
            args.root.display()
        ));
    }

    Ok(())
}
