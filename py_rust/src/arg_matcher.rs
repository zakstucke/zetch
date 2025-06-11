use crate::{
    args::{self, get_version_info},
    init,
    prelude::*,
    read_write, render, replace_matcher, var,
};

pub fn arg_matcher(arg: args::Args) -> Result<(), Report<Zerr>> {
    match &arg.command {
        args::Command::Render(render) => {
            render::render(&arg, render)?;
            Ok(())
        }
        args::Command::Var(read_var) => Ok(var::read_var(&arg, read_var)?),
        args::Command::Init(init) => Ok(init::init(init)?),
        args::Command::ReplaceMatcher(replace) => Ok(replace_matcher::replace(&arg, replace)?),
        args::Command::Read(fargs) => Ok(read_write::handle_file_cmd(&arg, fargs.into())?),
        args::Command::Put(fargs) => Ok(read_write::handle_file_cmd(&arg, fargs.into())?),
        args::Command::Del(fargs) => Ok(read_write::handle_file_cmd(&arg, fargs.into())?),
        args::Command::Version { output_format: _ } => {
            println!("zetch {}", get_version_info());
            Ok(())
        }
    }
}
