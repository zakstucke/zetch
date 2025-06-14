use nondestructive::yaml::{Separator, ValueMut};

use crate::prelude::*;

/// An individual accessor for mapping or array in a yaml file.
#[derive(Clone)]
pub enum YamlLoc {
    Key(String),
    Index(usize),
}

/// An update that should be applied to a yaml document.
///
/// NOTES:
/// - Intermediary paths on a deletion should exist, shouldn't call into python if not needed!
/// - Intermediary paths on puts should be made in individual batch updates, as little logic in python as poss.
pub enum YamlUpdate {
    Delete(Vec<YamlLoc>),
    // Where the string is a json_str that will be decoded python side.
    Put(Vec<YamlLoc>, String),
}

/// TODO rename Delegate batched yaml updates to python ruamel.yaml library.
pub fn py_modify_yaml(src: String, updates: Vec<YamlUpdate>) -> Result<String, Report<Zerr>> {
    let mut doc = nondestructive::yaml::from_slice(&src).change_context(Zerr::InternalError)?;

    for update in updates {
        let path = match &update {
            YamlUpdate::Delete(path) => path,
            YamlUpdate::Put(path, _) => path,
        };

        let mut active_node = Some(doc.as_mut());

        for (path_index, field) in path.iter().enumerate() {
            let is_last = path_index == path.len() - 1;

            match field {
                YamlLoc::Key(key) => {
                    let mut mapping = active_node
                        .take()
                        .expect("Always replaced")
                        .into_mapping_mut()
                        .ok_or_else(|| zerr!(Zerr::InternalError, "TODO"))?;
                    if is_last {
                        match &update {
                            YamlUpdate::Delete(_) => {
                                let existed = mapping.remove(key);
                                if !existed {
                                    tracing::warn!("TODO");
                                }
                            }
                            YamlUpdate::Put(_, item) => {
                                let placeholder = mapping.insert(key, Separator::Auto);
                                build_value(
                                    &serde_json::from_str::<serde_json::Value>(item)
                                        .change_context(Zerr::InternalError)?,
                                    placeholder,
                                )?;
                            }
                        }
                    } else {
                        active_node = Some(
                            mapping
                                .get_into_mut(key)
                                .ok_or_else(|| zerr!(Zerr::InternalError, "TODO"))?,
                        );
                    }
                }
                YamlLoc::Index(index) => {
                    let mut sequence = active_node
                        .take()
                        .expect("Always replaced")
                        .into_sequence_mut()
                        .ok_or_else(|| zerr!(Zerr::InternalError, "TODO"))?;
                    if is_last {
                        match &update {
                            YamlUpdate::Delete(_) => {
                                // TODO check index
                                let existed = sequence.remove(*index);
                                if !existed {
                                    tracing::warn!("TODO");
                                }
                            }
                            YamlUpdate::Put(_, item) => {
                                let value_to_put = serde_json::from_str::<serde_json::Value>(item)
                                    .change_context(Zerr::InternalError)?;
                                // Either replace, or push to end if index is out of bounds:
                                if let Some(existing) = sequence.get_mut(*index) {
                                    build_value(&value_to_put, existing)?;
                                } else {
                                    let placeholder = sequence.push(Separator::Auto);
                                    build_value(&value_to_put, placeholder)?;
                                }
                            }
                        }
                    } else {
                        active_node = Some(
                            sequence
                                .get_into_mut(*index)
                                .ok_or_else(|| zerr!(Zerr::InternalError, "TODO"))?,
                        );
                    }
                }
            }
        }
    }

    Ok(doc.to_string())
}

fn build_value<'a>(json: &serde_json::Value, mut value: ValueMut<'a>) -> Result<(), Report<Zerr>> {
    match json {
        serde_json::Value::Null => value.set_null(nondestructive::yaml::Null::Keyword),
        serde_json::Value::Bool(bool) => value.set_bool(*bool),
        serde_json::Value::Number(num) => {
            if let Some(float) = num.as_f64() {
                value.set_f64(float);
            } else if let Some(bigint) = num.as_i128() {
                value.set_i128(bigint);
            } else if let Some(biguint) = num.as_u128() {
                value.set_u128(biguint);
            } else {
                return Err(zerr!(
                    Zerr::InternalError,
                    "Could not convert num to yaml: {:?}",
                    num
                ));
            }
        }
        serde_json::Value::String(string) => value.set_string(string),
        serde_json::Value::Array(array) => {
            let mut sequence = value.make_sequence();
            for item in array {
                let placeholder = sequence.push(Separator::Auto);
                build_value(item, placeholder)?;
            }
        }
        serde_json::Value::Object(object) => {
            let mut mapping = value.make_mapping();
            for (key, item) in object {
                let placeholder = mapping.insert(key, Separator::Auto);
                build_value(item, placeholder)?;
            }
        }
    }
    Ok(())
}
