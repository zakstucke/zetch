use std::{borrow::BorrowMut, ops::Deref};

use toml_edit::{Formatted, Item, Table, Value};

use crate::{
    prelude::*,
    read_write::traverser::{TravNode, Traversable, Traverser},
};

pub type TomlTraverser<'t> = Traverser<Active<'t>>;

#[derive(Debug)]
enum Inner<'t> {
    Item(&'t mut Item),
    Value(&'t mut Value),
    Table(&'t mut Table),
}

pub struct Active<'t> {
    inner: Inner<'t>,
}

impl<'t> From<&'t mut Item> for Active<'t> {
    fn from(val: &'t mut Item) -> Self {
        Active {
            inner: Inner::Item(val),
        }
    }
}

impl<'t> From<&'t mut Value> for Active<'t> {
    fn from(val: &'t mut Value) -> Self {
        Active {
            inner: Inner::Value(val),
        }
    }
}

impl<'t> From<&'t mut Table> for Active<'t> {
    fn from(val: &'t mut Table) -> Self {
        Active {
            inner: Inner::Table(val),
        }
    }
}

impl<'t, 'r> Traversable<'r> for TomlTraverser<'t> {
    fn active(&self) -> Result<TravNode, Zerr> {
        self.with_active(|active| {
            let handle_val = |val: &Value| -> Result<TravNode, Zerr> {
                Ok(match val {
                    Value::Array(_) => TravNode::Array,
                    Value::InlineTable(_) => TravNode::Object,
                    _ => TravNode::Other,
                })
            };

            let node = match &active.inner {
                Inner::Item(item) => match item {
                    Item::Value(val) => handle_val(val)?,
                    Item::Table(_) => TravNode::Object,
                    Item::ArrayOfTables(_) => TravNode::Array,
                    Item::None => TravNode::Other,
                },
                Inner::Value(val) => handle_val(val)?,
                Inner::Table(_) => TravNode::Object,
            };

            Ok(node)
        })
    }

    fn active_as_serde(&self) -> Result<serde_json::Value, Zerr> {
        self.with_active(|active| -> Result<serde_json::Value, Zerr> {
            Ok(match &active.inner {
                Inner::Item(item) => item_to_serde(item)?,
                Inner::Value(val) => value_to_serde(val)?,
                Inner::Table(tab) => item_to_serde(&Item::Table(tab.deref().clone()))?,
            })
        })
    }

    fn array_enter(&self, index: usize) -> Result<(), Zerr> {
        self.replace_active(|active| {
            macro_rules! handle_arr {
                ($arr:expr) => {
                    if $arr.len() > index {
                        if let Some(inner) = $arr.get_mut(index) {
                            Ok(inner.into())
                        } else {
                            Err(zerr_int!("Index in bounds already checked..."))
                        }
                    } else {
                        Err(zerr_int!(
                            "Index: '{}' is out of array bounds (array len: '{}')",
                            index,
                            $arr.len()
                        ))
                    }
                };
            }

            let handle_val = |val: &'t mut Value| -> Result<Active<'t>, Zerr> {
                match val {
                    toml_edit::Value::Array(arr) => {
                        handle_arr!(arr)
                    }
                    _ => Err(zerr_int!()),
                }
            };

            match active.inner {
                Inner::Value(val) => handle_val(val),
                Inner::Item(item) => match item {
                    Item::Value(val) => handle_val(val),
                    Item::ArrayOfTables(arr) => handle_arr!(arr),
                    _ => Err(zerr_int!()),
                },
                Inner::Table(_) => Err(zerr_int!()),
            }
        })
    }

    fn array_set_index(&self, index: usize, json_str: &'r str) -> Result<(), Zerr> {
        self.with_active(|active| {
            macro_rules! handle_val {
                ($val:expr) => {{
                    match $val {
                        toml_edit::Value::Array(arr) => {
                            // Maintain surrounding comments etc if replacing:
                            let new_val = maintain_decor_val(
                                serde_to_value(
                                    serde_json::from_str::<serde_json::Value>(json_str)
                                        .change_context(Zerr::InternalError)?,
                                )?,
                                arr.get(index),
                            );
                            arr.replace(index, new_val);
                            Ok(())
                        }
                        _ => Err(zerr_int!()),
                    }
                }};
            }

            match &mut active.inner {
                Inner::Value(val) => handle_val!(val),
                Inner::Item(item) => match item {
                    Item::Value(val) => handle_val!(val),
                    Item::ArrayOfTables(arr) => {
                        let table = serde_to_table(
                            serde_json::from_str::<serde_json::Value>(json_str)
                                .change_context(Zerr::InternalError)?,
                        )?;

                        // No in built replace for array tables, have to hack it together by extracting then replacing:
                        let mut extracted: Vec<Table> = vec![];
                        for _ in 0..arr.len() {
                            extracted.push(arr.get(0).unwrap().clone());
                            arr.remove(0);
                        }
                        extracted[index] = table;
                        for tab in extracted {
                            arr.push(tab);
                        }

                        Ok(())
                    }
                    Item::None => Err(zerr_int!()),
                    Item::Table(_) => Err(zerr_int!()),
                },
                Inner::Table(_) => Err(zerr_int!()),
            }
        })
    }

    fn array_len(&self) -> Result<usize, Zerr> {
        self.with_active(|active| match &active.inner {
            Inner::Value(val) => match val {
                toml_edit::Value::Array(arr) => Ok(arr.len()),
                _ => Err(zerr_int!()),
            },
            Inner::Item(item) => match &item {
                Item::Value(val) => match val {
                    toml_edit::Value::Array(arr) => Ok(arr.len()),
                    _ => Err(zerr_int!()),
                },
                Item::ArrayOfTables(arr) => Ok(arr.len()),
                _ => Err(zerr_int!()),
            },
            Inner::Table(_) => Err(zerr_int!()),
        })
    }

    fn array_push(&self, json_str: &'r str) -> Result<(), Zerr> {
        self.with_active(|active| {
            macro_rules! handle_val {
                ($val:expr) => {
                    match $val {
                        toml_edit::Value::Array(arr) => {
                            arr.push(serde_to_value(
                                serde_json::from_str::<serde_json::Value>(json_str)
                                    .change_context(Zerr::InternalError)?,
                            )?);
                            Ok(())
                        }
                        _ => Err(zerr_int!()),
                    }
                };
            }

            match &mut active.inner {
                Inner::Value(val) => handle_val!(val),
                Inner::Item(item) => match item {
                    Item::Value(val) => handle_val!(val),
                    Item::ArrayOfTables(arr) => {
                        let table = serde_to_table(
                            serde_json::from_str::<serde_json::Value>(json_str)
                                .change_context(Zerr::InternalError)?,
                        )?;
                        arr.push(table);
                        Ok(())
                    }
                    Item::None => Err(zerr_int!()),
                    Item::Table(_) => Err(zerr_int!()),
                },
                Inner::Table(_) => Err(zerr_int!()),
            }
        })
    }

    fn array_delete_index(&self, index: usize) -> Result<(), Zerr> {
        self.with_active(|active| match active.inner.borrow_mut() {
            Inner::Value(val) => match val {
                toml_edit::Value::Array(arr) => {
                    arr.remove(index);
                    Ok(())
                }
                _ => Err(zerr_int!()),
            },
            Inner::Item(item) => match item.borrow_mut() {
                Item::Value(val) => match val {
                    toml_edit::Value::Array(arr) => {
                        arr.remove(index);
                        Ok(())
                    }
                    _ => Err(zerr_int!()),
                },
                Item::ArrayOfTables(arr) => {
                    arr.remove(index);
                    Ok(())
                }
                _ => Err(zerr_int!()),
            },
            Inner::Table(_) => Err(zerr_int!()),
        })
    }

    fn object_enter(&self, key: &str) -> Result<(), Zerr> {
        self.replace_active(|active| {
            macro_rules! handle_obj {
                ($obj:expr) => {
                    if let Some(inner) = $obj.get_mut(key) {
                        Ok(inner.into())
                    } else {
                        Err(zerr_int!("Key missing..."))
                    }
                };
            }

            let handle_val = |val: &'t mut Value| -> Result<Active<'t>, Zerr> {
                match val {
                    toml_edit::Value::InlineTable(table) => {
                        handle_obj!(table)
                    }
                    _ => Err(zerr_int!()),
                }
            };

            match active.inner {
                Inner::Value(val) => handle_val(val),
                Inner::Table(tab) => handle_obj!(tab),
                Inner::Item(item) => match item {
                    Item::Value(val) => handle_val(val),
                    Item::Table(tab) => handle_obj!(tab),
                    Item::None => Err(zerr_int!()),
                    Item::ArrayOfTables(_) => Err(zerr_int!()),
                },
            }
        })
    }

    fn object_key_exists(&self, key: &str) -> Result<bool, Zerr> {
        self.with_active(|active| {
            let handle_val = |val: &Value| -> Result<bool, Zerr> {
                match val {
                    toml_edit::Value::InlineTable(table) => Ok(table.get(key).is_some()),
                    _ => Err(zerr_int!()),
                }
            };

            match &active.inner {
                Inner::Value(val) => handle_val(val),
                Inner::Table(tab) => Ok(tab.get(key).is_some()),
                Inner::Item(item) => match item {
                    Item::Value(val) => handle_val(val),
                    Item::Table(tab) => Ok(tab.get(key).is_some()),
                    Item::ArrayOfTables(_) => Err(zerr_int!()),
                    Item::None => Err(zerr_int!()),
                },
            }
        })
    }

    fn object_set_key(&self, key: &'r str, json_str: &'r str) -> Result<(), Zerr> {
        self.with_active(|active| {
            let handle_val = |val: &mut Value| -> Result<(), Zerr> {
                match val {
                    toml_edit::Value::InlineTable(tab) => {
                        // Maintain surrounding comments etc if replacing:
                        let new_val = maintain_decor_val(
                            serde_to_value(
                                serde_json::from_str(json_str)
                                    .change_context(Zerr::InternalError)?,
                            )?,
                            tab.get(key),
                        );

                        // Keys in tables also have decor, maintain that if replacing:
                        let old_key_decor = tab.key_decor(key).cloned();
                        tab.insert(key, new_val);
                        if let Some(old_key_decor) = old_key_decor {
                            if let Some(new_key_decor) = tab.key_decor_mut(key) {
                                *new_key_decor = old_key_decor;
                            }
                        }

                        Ok(())
                    }
                    _ => Err(zerr_int!()),
                }
            };

            match &mut active.inner {
                Inner::Value(val) => handle_val(val),
                Inner::Table(tab) => {
                    // Maintain surrounding comments etc if replacing:
                    let new_item = maintain_decor_item(
                        Item::Value(serde_to_value(
                            serde_json::from_str(json_str).change_context(Zerr::InternalError)?,
                        )?),
                        tab.get(key),
                    );

                    // Keys in tables also have decor, maintain that if replacing:
                    let old_key_decor = tab.key_decor(key).cloned();
                    tab.insert(key, new_item);
                    if let Some(old_key_decor) = old_key_decor {
                        if let Some(new_key_decor) = tab.key_decor_mut(key) {
                            *new_key_decor = old_key_decor;
                        }
                    }

                    Ok(())
                }
                Inner::Item(item) => match item {
                    Item::Value(val) => handle_val(val),
                    Item::Table(tab) => {
                        tab.insert(
                            key,
                            Item::Value(serde_to_value(
                                serde_json::from_str(json_str)
                                    .change_context(Zerr::InternalError)?,
                            )?),
                        );
                        Ok(())
                    }
                    Item::None => Err(zerr_int!()),
                    Item::ArrayOfTables(_) => Err(zerr_int!()),
                },
            }
        })
    }

    fn object_delete_key(&self, key: &str) -> Result<(), Zerr> {
        self.with_active(|active| {
            let handle_val = |val: &mut Value| -> Result<(), Zerr> {
                match val {
                    toml_edit::Value::InlineTable(table) => {
                        table.remove(key);
                        Ok(())
                    }
                    _ => Err(zerr_int!()),
                }
            };

            match &mut active.inner {
                Inner::Value(val) => handle_val(val),
                Inner::Table(tab) => {
                    tab.remove(key);
                    Ok(())
                }
                Inner::Item(item) => match item {
                    Item::Value(val) => handle_val(val),
                    Item::Table(tab) => {
                        tab.remove(key);
                        Ok(())
                    }
                    Item::None => Err(zerr_int!()),
                    Item::ArrayOfTables(_) => Err(zerr_int!()),
                },
            }
        })
    }

    fn finish(&self) -> Result<(), Zerr> {
        Ok(())
    }
}

fn value_to_serde(value: &Value) -> Result<serde_json::Value, Zerr> {
    Ok(match value {
        Value::String(s) => serde_json::Value::String(s.value().clone()),
        Value::Integer(i) => serde_json::Value::Number(serde_json::Number::from(*i.value())),
        Value::Float(f) => serde_json::Value::Number(
            serde_json::Number::from_f64(*f.value()).ok_or_else(|| {
                zerr!(
                    Zerr::InternalError,
                    "Could not convert Value::Float to serde_json::Number."
                )
            })?,
        ),
        Value::Boolean(b) => serde_json::Value::Bool(*b.value()),
        Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        Value::Array(arr) => {
            let mut vec = Vec::new();
            for val in arr {
                vec.push(value_to_serde(val)?);
            }
            serde_json::Value::Array(vec)
        }
        Value::InlineTable(tab) => {
            let mut obj = serde_json::Map::new();
            for (key, val) in tab {
                obj.insert(key.to_string(), value_to_serde(val)?);
            }
            serde_json::Value::Object(obj)
        }
    })
}

// Convert a value to a table (not an inline one, a proper one), raising a user error if its not possible.
fn serde_to_table(val: serde_json::Value) -> Result<Table, Zerr> {
    Ok(match val {
        serde_json::Value::Object(obj) => {
            let mut table = Table::new();
            for (key, val) in obj {
                table.insert(&key, Item::Value(serde_to_value(val)?));
            }
            table
        }
        _ => Err(zerr!(
            Zerr::FileSyntaxError,
            "Value is not an object, can't convert to table where a table is required."
        ))?,
    })
}

fn item_to_serde(item: &Item) -> Result<serde_json::Value, Zerr> {
    Ok(match item {
        Item::None => serde_json::Value::Null,
        Item::Value(val) => value_to_serde(val)?,
        Item::Table(table) => {
            let mut obj = serde_json::Map::new();
            for (key, val) in table {
                obj.insert(key.to_string(), item_to_serde(val)?);
            }
            serde_json::Value::Object(obj)
        }
        Item::ArrayOfTables(arr) => {
            let mut vec = Vec::new();
            for table in arr {
                let mut obj = serde_json::Map::new();
                for (key, val) in table {
                    obj.insert(key.to_string(), item_to_serde(val)?);
                }
                vec.push(serde_json::Value::Object(obj));
            }
            serde_json::Value::Array(vec)
        }
    })
}

pub fn serde_to_value(val: serde_json::Value) -> Result<Value, Zerr> {
    Ok(match val {
        // User error as null can't be represented in toml:
        serde_json::Value::Null => Err(zerr!(
            Zerr::FileSyntaxError,
            "Null value not supported by toml format."
        ))?,
        serde_json::Value::Bool(b) => Value::Boolean(Formatted::new(b)),
        serde_json::Value::String(s) => Value::String(Formatted::new(s)),
        serde_json::Value::Number(n) => {
            if let Some(n) = n.as_i64() {
                Value::Integer(Formatted::new(n))
            } else if let Some(n) = n.as_f64() {
                Value::Float(Formatted::new(n))
            } else {
                return Err(zerr!(
                    Zerr::InternalError,
                    "Could not convert serde_json::Number to toml_edit::Value. Number: {:?}",
                    n
                ));
            }
        }
        serde_json::Value::Array(arr) => {
            let mut array = toml_edit::Array::default();
            for val in arr {
                array.push(serde_to_value(val)?);
            }
            Value::Array(array)
        }
        serde_json::Value::Object(obj) => {
            let mut table = toml_edit::InlineTable::default();
            for (key, val) in obj {
                table.insert(key, serde_to_value(val)?);
            }
            Value::InlineTable(table)
        }
    })
}

/// When replacing an item (arr or object), may be able to maintain some of the decor (e.g. comments) attached to the old.
fn maintain_decor_item(mut new: Item, old: Option<&Item>) -> Item {
    if let Some(old) = old {
        match (&mut new, old) {
            // Both values
            (Item::Value(new), Item::Value(old)) => {
                *new.decor_mut() = old.decor().clone();
            }
            // Both tables
            (Item::Table(new), Item::Table(old)) => {
                *new.decor_mut() = old.decor().clone();
            }
            // New is a value, old was a table
            (Item::Value(new), Item::Table(old)) => {
                *new.decor_mut() = old.decor().clone();
            }
            // New is a table, old was a value
            (Item::Table(new), Item::Value(old)) => {
                *new.decor_mut() = old.decor().clone();
            }

            // Arrays of tables don't seem to have decor:
            // (Item::Value(_), Item::ArrayOfTables(_)) => todo!(),
            // (Item::Table(_), Item::ArrayOfTables(_)) => todo!(),
            // (Item::ArrayOfTables(_), Item::Value(_)) => todo!(),
            // (Item::ArrayOfTables(_), Item::Table(_)) => todo!(),
            // (Item::ArrayOfTables(_), Item::ArrayOfTables(_)) => todo!(),

            // Anything with None obviously has no decor:
            // (Item::None, Item::None) => todo!(),
            // (Item::None, Item::Value(_)) => todo!(),
            // (Item::None, Item::Table(_)) => todo!(),
            // (Item::None, Item::ArrayOfTables(_)) => todo!(),
            // (Item::Value(_), Item::None) => todo!(),
            // (Item::Table(_), Item::None) => todo!(),
            // (Item::ArrayOfTables(_), Item::None) => todo!(),
            _ => {}
        }
    }

    new
}

/// When replacing a value, may be able to maintain some of the decor (e.g. comments) attached to the old.
fn maintain_decor_val(mut new: Value, old: Option<&Value>) -> Value {
    if let Some(old) = old {
        *new.decor_mut() = old.decor().clone();
    }
    new
}
