use crate::prelude::*;
use error_stack::{Context, Report};
use strum::Display;

#[derive(Debug, Display)]
pub enum TomlErr {
    /// When the contents can't be parsed as toml.
    TomlInvalid,
    /// When a user provided path in a toml file (i.e. config file) doesn't match the contents.
    TomlReadPathError,
    /// Something happened that shouldn't have.
    InternalError,
}

impl Context for TomlErr {}

/// Update a toml file with a json patch and remove some paths
/// * `initial` - The toml input as a string
/// * `update` - A json patch to apply to the toml input, overwrites anything existing, adds anything missing from the patch
/// * `remove` - A list of paths to remove from the toml, applied after update. E.g. [["ctx", "foo"], ["ctx", "bar"]] would remove ctx.foo and ctx.bar from the toml file.
pub fn update(
    initial: &str,
    update: Option<serde_json::Value>,
    remove: Option<Vec<Vec<String>>>,
) -> Result<String, TomlErr> {
    let mut jsonified: serde_json::Value =
        toml::from_str(initial).change_context(TomlErr::TomlInvalid)?;

    if let Some(update) = update {
        json_patch::merge(&mut jsonified, &update);
    }

    if let Some(remove) = remove {
        for path in remove {
            if !path.is_empty() {
                let mut depth_obj = &mut jsonified;
                for (index, key) in path.iter().enumerate() {
                    if index == path.len() - 1 {
                        if let Some(obj) = depth_obj.as_object_mut() {
                            obj.remove(key);
                        }
                    } else if let Some(obj) = depth_obj.get_mut(key) {
                        depth_obj = obj;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    toml::to_string_pretty(&jsonified).change_context(TomlErr::InternalError)
}

/// Returns the portion of a toml file requested. Returning it as a serde_json::Value.
/// E.g. `["foo", "bar", "baz"]` would return the value of `toml["foo"]["bar"]["baz"]
/// If a key is an integer, and the current object is an array, it will return the value at that index.
pub fn read(src: &str, path: &[&str]) -> Result<serde_json::Value, TomlErr> {
    // Remove any erroneous empty "" strings:
    let path: Vec<&str> = path.iter().filter(|s| !s.is_empty()).copied().collect();

    let mut active: Option<serde_json::Value> =
        Some(toml::from_str(src).change_context(TomlErr::TomlInvalid)?);
    for (index, key) in path.iter().enumerate() {
        let mut err: Option<String> = None;
        match active.take().unwrap() {
            serde_json::Value::Object(mut obj) => {
                if let Some(value) = obj.remove(*key) {
                    active = Some(value);
                } else {
                    err = Some(format!(
                        "Key '{}' not found in active table. Avail keys: '{}'.",
                        key,
                        obj.keys()
                            .collect::<Vec<&String>>()
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    ));
                }
            }
            serde_json::Value::Array(mut arr) => {
                if let Ok(index) = key.parse::<usize>() {
                    if arr.len() > index {
                        active = Some(arr.swap_remove(index));
                    } else {
                        err = Some(format!(
                            "Index '{}' is outside the bounds of the array (len {}).",
                            index,
                            arr.len()
                        ));
                    }
                } else {
                    err = Some(format!(
                        "Table key '{}' cannot be found. Active element is an array.",
                        key
                    ));
                }
            }
            other_type => {
                err = Some(format!(
                    "Key '{}' not found in active element. Active element is of type: '{}'.",
                    key,
                    match other_type {
                        serde_json::Value::Null => "null",
                        serde_json::Value::Bool(_) => "bool",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::String(_) => "string",
                        serde_json::Value::Array(_) => "array",
                        serde_json::Value::Object(_) => "object",
                    }
                ))
            }
        }

        if let Some(err) = err {
            return Err(
                Report::new(TomlErr::TomlReadPathError).attach_printable(format!(
                    "Failed to read toml path: '{}'. Failed at: '{}' with error: '{}'.",
                    path.join("."),
                    if index != 0 {
                        path[0..index].join(".")
                    } else {
                        "root".to_string()
                    },
                    err
                )),
            );
        }
    }

    Ok(active.unwrap())
}
