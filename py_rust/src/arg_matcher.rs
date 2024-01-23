use crate::{
    args::{self, get_version_info},
    init,
    prelude::*,
    read, render, replace_matcher,
};

pub fn arg_matcher(arg: args::Args) -> Result<(), Zerr> {
    match &arg.command {
        args::Command::Render(render) => {
            render::render(&arg, render)?;
            Ok(())
        }
        args::Command::ReadConfig(read_config) => Ok(read::read_config(&arg, read_config)?),
        args::Command::ReadVar(read_var) => Ok(read::read_var(&arg, read_var)?),
        args::Command::Init(init) => Ok(init::init(init)?),
        args::Command::ReplaceMatcher(replace) => Ok(replace_matcher::replace(&arg, replace)?),
        args::Command::Version { output_format: _ } => {
            println!("zetch {}", get_version_info());
            Ok(())
        }
    }
}
