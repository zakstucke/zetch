use std::collections::HashSet;

use crate::{
    args::{ReadOutputFormat, VarCommand},
    prelude::*,
};

/// Read a finalised config variable.
pub fn read_var(args: &crate::args::Args, read: &VarCommand) -> Result<(), Zerr> {
    // Note conf.context will only contain variable requested, rest won't be processed,
    // use conf.raw to see what's actually in the config file.
    let conf = crate::config::load(
        args,
        None,
        // Don't need to compute all of them, just the one being printed:
        Some(HashSet::from_iter([read.var.as_str()])),
        false,
    )?;

    if let Some(target) = conf.context.get(read.var.as_str()) {
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
    } else {
        Err(zerr!(
            Zerr::ReadVarMissing,
            "Context variable '{}' not found in finalised config. All context keys: '{}'.",
            read.var,
            conf.raw.all_context_keys().join(", ")
        ))
    }
}
