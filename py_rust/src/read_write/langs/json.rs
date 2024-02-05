use fjson::ast::{Root, Value, ValueToken};

use crate::{
    prelude::*,
    read_write::traverser::{TravNode, Traversable, Traverser},
};

pub type JsonTraverser<'t, 'r> = Traverser<&'t mut ValueToken<'r>>;

impl<'t, 'r> Traversable<'r> for JsonTraverser<'t, 'r> {
    fn active(&self) -> Result<TravNode, Zerr> {
        self.with_active(|active| match active {
            fjson::ast::ValueToken::Array(_) => Ok(TravNode::Array),
            fjson::ast::ValueToken::Object(_) => Ok(TravNode::Object),
            _ => Ok(TravNode::Other),
        })
    }

    fn active_as_serde(&self) -> Result<serde_json::Value, Zerr> {
        let root = Root {
            meta_above: vec![],
            meta_below: vec![],
            value: Value {
                token: self.with_active(|active| Ok(active.clone()))?,
                comments: vec![],
            },
        };

        let mut jsonified = String::new();
        fjson::format::write_json_compact(&mut jsonified, &root)
            .change_context(Zerr::InternalError)?;
        serde_json::from_str(&jsonified).change_context(Zerr::InternalError)
    }

    fn array_enter(&self, index: usize) -> Result<(), Zerr> {
        self.replace_active(|active| {
            if let fjson::ast::ValueToken::Array(arr) = active {
                let mut value_index = 0;
                for info in arr.iter_mut() {
                    if let fjson::ast::ArrayValue::ArrayVal(val) = info {
                        if value_index == index {
                            return Ok(&mut val.token);
                        }
                        value_index += 1;
                    }
                }
                Err(zerr!(
                    Zerr::InternalError,
                    "Index: '{}' is out of array bounds (array len: '{}')",
                    index,
                    value_index
                ))
            } else {
                Err(zerr!(Zerr::InternalError, "Active value is not an array."))
            }
        })
    }

    fn array_set_index(&self, index: usize, json_str: &'r str) -> Result<(), Zerr> {
        self.with_active(|active| {
            if let fjson::ast::ValueToken::Array(arr) = active {
                let mut value_index = 0;
                for info in arr.iter_mut() {
                    if let fjson::ast::ArrayValue::ArrayVal(val) = info {
                        if value_index == index {
                            val.token = json_str_to_token(json_str)?;
                            return Ok(());
                        }
                        value_index += 1;
                    }
                }

                Err(zerr!(
                    Zerr::InternalError,
                    "Index: '{}' is out of array bounds (array len: '{}')",
                    index,
                    value_index
                ))
            } else {
                Err(zerr!(Zerr::InternalError, "Active value is not an array."))
            }
        })
    }

    fn array_len(&self) -> Result<usize, Zerr> {
        self.with_active(|active| {
            if let fjson::ast::ValueToken::Array(arr) = active {
                let mut value_index = 0;
                for info in arr.iter() {
                    if let fjson::ast::ArrayValue::ArrayVal(_) = info {
                        value_index += 1;
                    }
                }
                Ok(value_index)
            } else {
                Err(zerr!(Zerr::InternalError, "Active value is not an array."))
            }
        })
    }

    fn array_push(&self, json_str: &'r str) -> Result<(), Zerr> {
        self.with_active(|active| {
            if let fjson::ast::ValueToken::Array(arr) = active {
                arr.push(fjson::ast::ArrayValue::ArrayVal(fjson::ast::Value {
                    token: json_str_to_token(json_str)?,
                    comments: vec![],
                }));
                Ok(())
            } else {
                Err(zerr!(Zerr::InternalError, "Active value is not an array."))
            }
        })
    }

    fn array_delete_index(&self, index: usize) -> Result<(), Zerr> {
        self.with_active(|active| {
            if let fjson::ast::ValueToken::Array(arr) = active {
                let mut value_index = 0;
                for info in arr.iter_mut() {
                    if let fjson::ast::ArrayValue::ArrayVal(_) = info {
                        if value_index == index {
                            arr.remove(value_index);
                            return Ok(());
                        }
                        value_index += 1;
                    }
                }
                Err(zerr!(
                    Zerr::InternalError,
                    "Index: '{}' is out of array bounds (array len: '{}')",
                    index,
                    value_index
                ))
            } else {
                Err(zerr!(Zerr::InternalError, "Active value is not an array."))
            }
        })
    }

    fn object_enter(&self, key: &str) -> Result<(), Zerr> {
        self.replace_active(|active| {
            if let fjson::ast::ValueToken::Object(obj) = active {
                for info in obj.iter_mut() {
                    if let fjson::ast::ObjectValue::KeyVal(k, val) = info {
                        if *k == key {
                            return Ok(&mut val.token);
                        }
                    }
                }
                Err(zerr!(
                    Zerr::InternalError,
                    "Key: '{}' does not exist in object.",
                    key
                ))
            } else {
                Err(zerr!(Zerr::InternalError, "Active value is not an object."))
            }
        })
    }

    fn object_key_exists(&self, key: &str) -> Result<bool, Zerr> {
        self.with_active(|active| {
            if let fjson::ast::ValueToken::Object(obj) = active {
                for info in obj.iter() {
                    if let fjson::ast::ObjectValue::KeyVal(k, _) = info {
                        if *k == key {
                            return Ok(true);
                        }
                    }
                }
                Ok(false)
            } else {
                Err(zerr!(Zerr::InternalError, "Active value is not an object."))
            }
        })
    }

    fn object_set_key(&self, key: &'r str, json_str: &'r str) -> Result<(), Zerr> {
        self.with_active(|active| {
            if let fjson::ast::ValueToken::Object(obj) = active {
                // Replace if already exists to keep ordering: (will return if replaced)
                for info in obj.iter_mut() {
                    if let fjson::ast::ObjectValue::KeyVal(k, val) = info {
                        if *k == key {
                            val.token = json_str_to_token(json_str)?;
                            return Ok(());
                        }
                    }
                }

                // Otherwise at to the end:
                obj.push(fjson::ast::ObjectValue::KeyVal(
                    key,
                    fjson::ast::Value {
                        token: json_str_to_token(json_str)?,
                        comments: vec![],
                    },
                ));
                Ok(())
            } else {
                Err(zerr!(Zerr::InternalError, "Active value is not an object."))
            }
        })
    }

    fn object_delete_key(&self, key: &str) -> Result<(), Zerr> {
        self.with_active(|active| {
            if let fjson::ast::ValueToken::Object(obj) = active {
                for (index, info) in obj.iter_mut().enumerate() {
                    if let fjson::ast::ObjectValue::KeyVal(k, _) = info {
                        if *k == key {
                            obj.remove(index);
                            return Ok(());
                        }
                    }
                }
                Err(zerr!(
                    Zerr::InternalError,
                    "Key: '{}' does not exist in object.",
                    key
                ))
            } else {
                Err(zerr!(Zerr::InternalError, "Active value is not an object."))
            }
        })
    }

    fn finish(&self) -> Result<(), Zerr> {
        Ok(())
    }
}

fn json_str_to_token(json_str: &str) -> Result<ValueToken<'_>, Zerr> {
    let root = fjson::ast::parse(json_str).change_context(Zerr::InternalError)?;
    Ok(root.value.token)
}
