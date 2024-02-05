use std::collections::HashSet;

use crate::{
    args::{ReadOutputFormat, ReadVarCommand},
    config::final_config_path,
    prelude::*,
};

/// Read a finalised config variable.
pub fn read_var(args: &crate::args::Args, read: &ReadVarCommand) -> Result<(), Zerr> {
    let raw_conf = crate::config::RawConfig::from_toml(&final_config_path(&args.config, None)?)?;

    let all_context_keys = raw_conf.all_context_keys();

    let conf = crate::config::process(
        raw_conf,
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
            all_context_keys.join(", ")
        ))
    }
}
