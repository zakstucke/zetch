use crate::{
    args::{ReadOutputFormat, VarCommand},
    prelude::*,
    state::State,
};

/// Read a finalised config variable.
pub fn read_var(args: &crate::args::Args, read: &VarCommand) -> Result<(), Zerr> {
    let mut state = State::new(args)?;

    // Only need to load the specific target:
    let target = state.load_var(read.var.as_str(), false, false)?;

    // Handle different output formats:
    match read.output {
        ReadOutputFormat::Raw => match target {
            serde_json::Value::String(s) => println!("{}", s),
            target => println!(
                "{}",
                serde_json::to_string(&target).change_context(Zerr::InternalError)?
            ),
        },
        ReadOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&target).change_context(Zerr::InternalError)?
        ),
    }
    Ok(())
}
