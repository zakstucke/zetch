use super::py_yaml::{YamlLoc, YamlUpdate, py_modify_yaml};
use crate::{
    prelude::*,
    read_write::traverser::{TravNode, Traversable, Traverser},
};

pub struct YamlRoot {
    pub file_contents: String,
    pub root_value: serde_yaml::Value,
}

impl YamlRoot {
    pub fn new(file_contents: &str) -> Result<Self, Report<Zerr>> {
        Ok(Self {
            file_contents: file_contents.to_string(),
            root_value: serde_yaml::from_str(file_contents).change_context(Zerr::InternalError)?,
        })
    }

    pub fn build_active(&mut self) -> Result<YamlActive<'_>, Report<Zerr>> {
        YamlActive::new(&mut self.file_contents, &mut self.root_value)
    }
}

pub struct YamlActive<'t> {
    value: &'t mut serde_yaml::Value,
    updates: Vec<YamlUpdate>,
    cur_path: Vec<YamlLoc>,
    // Yaml updates will be batched at the end and processed from python.
    // hence we'll modify the root directly during the traversers finish() method.
    root: &'t mut String,
}

impl<'t> YamlActive<'t> {
    pub fn new(
        file_contents: &'t mut String,
        root_value: &'t mut serde_yaml::Value,
    ) -> Result<Self, Report<Zerr>> {
        Ok(Self {
            value: root_value,
            updates: vec![],
            cur_path: vec![],
            root: file_contents,
        })
    }

    fn into_parts(
        self,
    ) -> (
        &'t mut serde_yaml::Value,
        Vec<YamlUpdate>,
        Vec<YamlLoc>,
        &'t mut String,
    ) {
        (self.value, self.updates, self.cur_path, self.root)
    }

    fn from_parts(
        value: &'t mut serde_yaml::Value,
        updates: Vec<YamlUpdate>,
        cur_path: Vec<YamlLoc>,
        root: &'t mut String,
    ) -> Self {
        Self {
            value,
            updates,
            cur_path,
            root,
        }
    }
}

pub type YamlTraverser<'t, 'r> = Traverser<YamlActive<'t>>;

impl<'t, 'r> Traversable<'r> for YamlTraverser<'t, 'r> {
    fn active(&self) -> Result<TravNode, Report<Zerr>> {
        self.with_active(|active| to_trav_node(active.value))
    }

    fn active_as_serde(&self) -> Result<serde_json::Value, Report<Zerr>> {
        self.with_active(|active| {
            serde_json::to_value(&active.value).change_context(Zerr::InternalError)
        })
    }

    fn array_enter(&self, index: usize) -> Result<(), Report<Zerr>> {
        self.replace_active(|active| {
            let (value, updates, cur_path, root) = active.into_parts();
            with_array(value, |arr| {
                if arr.len() > index {
                    let mut active =
                        YamlActive::from_parts(&mut arr[index], updates, cur_path, root);
                    active.cur_path.push(YamlLoc::Index(index));
                    Ok(active)
                } else {
                    Err(zerr_int!("Index is out of bounds."))
                }
            })
        })
    }

    fn array_set_index(&self, index: usize, json_str: &'r str) -> Result<(), Report<Zerr>> {
        self.with_active(|active| {
            with_array(active.value, |arr| {
                arr[index] = serde_json::from_str(json_str).change_context(Zerr::InternalError)?;

                // The output file contents is handled separately with batched updates:
                let mut path = active.cur_path.clone();
                path.push(YamlLoc::Index(index));
                active
                    .updates
                    .push(YamlUpdate::Put(path, json_str.to_string()));

                Ok(())
            })
        })
    }

    fn array_len(&self) -> Result<usize, Report<Zerr>> {
        self.with_active(|active| with_array(active.value, |arr| Ok(arr.len())))
    }

    fn array_push(&self, json_str: &'r str) -> Result<(), Report<Zerr>> {
        self.with_active(|active| {
            with_array(active.value, |arr| {
                let old_len = arr.len();

                arr.push(serde_json::from_str(json_str).change_context(Zerr::InternalError)?);

                // The output file contents is handled separately with batched updates:
                let mut path = active.cur_path.clone();
                path.push(YamlLoc::Index(old_len));
                active
                    .updates
                    .push(YamlUpdate::Put(path, json_str.to_string()));

                Ok(())
            })
        })
    }

    fn array_delete_index(&self, index: usize) -> Result<(), Report<Zerr>> {
        self.with_active(|active| {
            with_array(active.value, |arr| {
                arr.remove(index);

                // The output file contents is handled separately with batched updates:
                let mut path = active.cur_path.clone();
                path.push(YamlLoc::Index(index));
                active.updates.push(YamlUpdate::Delete(path));

                Ok(())
            })
        })
    }

    fn object_enter(&self, key: &str) -> Result<(), Report<Zerr>> {
        self.replace_active(|active| {
            let (value, updates, cur_path, root) = active.into_parts();
            with_object(value, |obj| {
                if let Some(inner) = obj.get_mut(key) {
                    let mut active = YamlActive::from_parts(inner, updates, cur_path, root);
                    active.cur_path.push(YamlLoc::Key(key.to_string()));
                    Ok(active)
                } else {
                    Err(zerr_int!("Key does not exist in mapping."))
                }
            })
        })
    }

    fn object_key_exists(&self, key: &str) -> Result<bool, Report<Zerr>> {
        self.with_active(|active| with_object(active.value, |obj| Ok(obj.contains_key(key))))
    }

    fn object_set_key(&self, key: &'r str, json_str: &'r str) -> Result<(), Report<Zerr>> {
        self.with_active(|active| {
            with_object(active.value, |obj| {
                obj.insert(
                    serde_yaml::Value::String(key.to_string()),
                    serde_json::from_str(json_str).change_context(Zerr::InternalError)?,
                );

                // The output file contents is handled separately with batched updates:
                let mut path = active.cur_path.clone();
                path.push(YamlLoc::Key(key.to_string()));
                active
                    .updates
                    .push(YamlUpdate::Put(path, json_str.to_string()));

                Ok(())
            })
        })
    }

    fn object_delete_key(&self, key: &str) -> Result<(), Report<Zerr>> {
        self.with_active(|active| {
            with_object(active.value, |obj| {
                obj.remove(key);

                // The output file contents is handled separately with batched updates:
                let mut path = active.cur_path.clone();
                path.push(YamlLoc::Key(key.to_string()));
                active.updates.push(YamlUpdate::Delete(path));

                Ok(())
            })
        })
    }

    fn finish(&self) -> Result<(), Report<Zerr>> {
        // Need to update the roots contents by calling python with the batched updates:
        self.with_active(|active| {
            if !active.updates.is_empty() {
                *active.root =
                    py_modify_yaml(active.root.to_string(), std::mem::take(&mut active.updates))?;
            }
            Ok(())
        })
    }
}

/// Repeated code for getting the active array value and doing something with it.
///
/// Should already be sure it's an array, errors handled outside.
fn with_array<'t, T>(
    value: &'t mut serde_yaml::Value,
    cb: impl FnOnce(&'t mut Vec<serde_yaml::Value>) -> Result<T, Report<Zerr>>,
) -> Result<T, Report<Zerr>> {
    match value {
        serde_yaml::Value::Sequence(arr) => cb(arr),
        // Tagged contains nested Value that needs recursing:
        serde_yaml::Value::Tagged(tagged) => with_array(&mut tagged.value, cb),
        _ => Err(zerr_int!("Value is not an array.")),
    }
}

/// Repeated code for getting the active object value and doing something with it.
///
/// Should already be sure it's an object, errors handled outside.
fn with_object<'t, T>(
    value: &'t mut serde_yaml::Value,
    cb: impl FnOnce(&'t mut serde_yaml::Mapping) -> Result<T, Report<Zerr>>,
) -> Result<T, Report<Zerr>> {
    match value {
        serde_yaml::Value::Mapping(obj) => cb(obj),
        // Tagged contains nested Value that needs recursing:
        serde_yaml::Value::Tagged(tagged) => with_object(&mut tagged.value, cb),
        _ => Err(zerr_int!("Value is not an object.")),
    }
}

fn to_trav_node(value: &serde_yaml::Value) -> Result<TravNode, Report<Zerr>> {
    Ok(match value {
        serde_yaml::Value::Sequence(_) => TravNode::Array,
        serde_yaml::Value::Mapping(_) => TravNode::Object,
        serde_yaml::Value::Tagged(tagged) => to_trav_node(&tagged.value)?,
        _ => TravNode::Other,
    })
}
