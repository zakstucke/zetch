use std::collections::HashSet;

use crate::{
    args::{ReadConfigCommand, ReadOutputFormat, ReadVarCommand},
    config::final_config_path,
    prelude::*,
    utils::toml::TomlErr,
};

/// Read specific contents from the config file. Prints as json.
pub fn read_config(args: &crate::args::Args, read: &ReadConfigCommand) -> Result<(), Zerr> {
    // This will error if can't be found:
    let config_path = final_config_path(&args.config, None)?;

    // Should have been validated to exist already so internal error on problem:
    let toml_contents = std::fs::read_to_string(config_path).change_context(Zerr::InternalError)?;

    let target = match crate::utils::toml::read(
        &toml_contents,
        &read.path.split('.').collect::<Vec<&str>>(),
    ) {
        Ok(target) => target,
        Err(e) => {
            let mapped = match e.current_context() {
                TomlErr::TomlInvalid => Zerr::ConfigInvalid,
                TomlErr::TomlReadPathError => Zerr::TomlReadPathError,
                TomlErr::InternalError => Zerr::InternalError,
            };
            return Err(e.change_context(mapped));
        }
    };

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
