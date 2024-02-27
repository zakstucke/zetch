use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;
use regex::Regex;

use super::conf::Config;
use crate::prelude::*;

// Include the schema in the binary to use at runtime:
static JSON_SCHEMA: &str = include_str!(r"./schema.json");

pub fn pre_validate(value: &serde_json::Value) -> Result<(), Zerr> {
    let state = run_against_schema(value)?;
    if !state.is_strictly_valid() {
        let mut report = zerr!(Zerr::ConfigInvalid, "Config validation failed.");
        for err in state.errors {
            report = report.attach_printable(format_err(err));
        }

        if !state.missing.is_empty() {
            for missing in state.missing {
                report = report.attach_printable(format!("Missing: {}", missing));
            }
            report = report.change_context(Zerr::InternalError).attach_printable(
                "Missing errors most likely an issue with the internal json schema for the config. First arose when task shared $def was improperly configured in json schema.",
            );
        }

        return Err(report);
    }

    Ok(())
}

/// Extra validation & cleaning to do on the created config object.
pub fn post_validate(conf: &mut Config, config_path: &Path) -> Result<(), Zerr> {
    // Make sure at least one matcher has been provided:
    if conf.matchers.is_empty() {
        return Err(zerr!(
            Zerr::ConfigInvalid,
            "[matchers]: must have at least one template matcher. e.g. ['zetch']",
        ));
    }

    // Make sure matchers only contain numbers, lowercase letters, dashes and underscores:
    let matchers_regex = Regex::new(r"^[a-z0-9_-]+$").unwrap();
    for matcher in conf.matchers.iter() {
        if matcher.is_empty() {
            return Err(zerr!(
                Zerr::ConfigInvalid,
                "[matchers]: cannot be empty string."
            ));
        }

        if !matchers_regex.is_match(matcher) {
            return Err(zerr!(
                Zerr::ConfigInvalid,
                "[matchers]: lowercase letters, numbers, dashes and underscores only in matchers (a-z 0-9 _ -). Not: '{}'",
                matcher
            ));
        }
    }

    // ignore_files and engine.custom_extensions should be resolved relative to the config file, so rewrite the paths if needed and make sure they exist:
    let validate_and_rewrite = |in_path: String| -> Result<String, Zerr> {
        // Make relative to config file if not absolute:
        let path = if !PathBuf::from(&in_path).is_absolute() {
            config_path
                .parent()
                .unwrap()
                .join(in_path)
                .to_str()
                .unwrap()
                .to_string()
        } else {
            in_path
        };

        // Make sure exists:
        if !PathBuf::from(&path).exists() {
            return Err(zerr!(Zerr::ConfigInvalid, "Path '{}' does not exist. Note relative paths are resolved from the config file directory.", path));
        }

        Ok(path)
    };

    for ignore_file in conf.ignore_files.iter_mut() {
        *ignore_file = validate_and_rewrite(ignore_file.clone())?;

        // Make sure is a file:
        if !PathBuf::from(&ignore_file).is_file() {
            return Err(zerr!(
                Zerr::ConfigInvalid,
                "Path '{}' is not a file.",
                ignore_file
            ));
        }
    }

    for user_extension in conf.engine.custom_extensions.iter_mut() {
        *user_extension = validate_and_rewrite(user_extension.clone())?;

        let path = PathBuf::from(&user_extension);
        // If it's a dir, make sure it has an __init__.py file:
        if path.is_dir() {
            let init_file = path.join("__init__.py");
            if !init_file.exists() {
                return Err(zerr!(
                    Zerr::ConfigInvalid,
                    "Custom extension '{}' is a directory but does not contain an __init__.py file, not a valid package.",
                    user_extension
                ));
            }
        } else {
            // Otherwise make sure its a .py file:
            let extension = path.extension().unwrap_or_default();
            if extension != "py" {
                return Err(zerr!(
                    Zerr::ConfigInvalid,
                    "Custom extension '{}' is not a .py file.",
                    user_extension
                ));
            }
        }
    }

    Ok(())
}

static COERCE_MSG: &str = "Expected one of ['json', 'str', 'int', 'float', 'bool'].";

/// Because we're hacking together toml validation using a json parser, format the errors a bit more applicably where possible.
fn format_err(err: Box<dyn valico::common::error::ValicoError>) -> String {
    // Want the actual detail, only use title if detail is missing (crates cli seems to state title is always available but detail not so. But detail seems to always be there.)
    let info = if let Some(detail) = err.get_detail() {
        detail
    } else {
        err.get_title()
    };

    let mut loc_parts = err
        .get_path()
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let mut desc = info.to_string();

    if let Some(extra) = err_extra_property(&desc) {
        desc = format!("Unknown property: '{}'.", extra);
    } else if let Some(invalid_type) = err_invalid_type(&desc) {
        desc = format!(
            "Expected {}.",
            match invalid_type.as_str() {
                "object" => "a table".to_string(),
                "array" => "an array".to_string(),
                _ => format!("a {}", invalid_type),
            }
        );
    } else if desc.contains("Enum conditions are not met") && loc_parts.last() == Some(&"coerce") {
        desc = COERCE_MSG.to_string();
    } else if desc.contains("OneOf conditions are not met") {
        // The only time a oneOf exists is for the CtxStaticVar used in static vars, env defaults and light values. Each of which will only fail if coerce has been specified wrong:
        loc_parts.push("coerce");
        desc = COERCE_MSG.to_string();
    }

    let loc_str = if loc_parts.is_empty() {
        "[root]: ".to_string()
    } else {
        format!("[{}]: ", loc_parts.join("."))
    };

    format!(
        "{}{}{}",
        loc_str,
        desc,
        if desc.ends_with('.') { "" } else { "." }
    )
}

static RE_EXTRA_PROP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"Additional property '([^']*)' is not allowed").expect("Invalid regex pattern")
});

fn err_extra_property(desc: &str) -> Option<String> {
    RE_EXTRA_PROP
        .captures(desc)
        .map(|caps| caps.get(1).unwrap().as_str().to_string())
}

static RE_INVALID_TYPE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"The value must be ([^']*)").expect("Invalid regex pattern"));

fn err_invalid_type(desc: &str) -> Option<String> {
    RE_INVALID_TYPE
        .captures(desc)
        .map(|caps| caps.get(1).unwrap().as_str().to_string())
}

fn run_against_schema(
    json: &serde_json::Value,
) -> Result<valico::json_schema::ValidationState, Zerr> {
    let json_schema: serde_json::Value =
        serde_json::from_str(JSON_SCHEMA).change_context(Zerr::InternalError)?;
    let mut scope = valico::json_schema::Scope::new();
    let schema = scope
        .compile_and_return(json_schema, true)
        .change_context(Zerr::InternalError)?;
    Ok(schema.validate(json))
}
