use std::{cell::RefCell, ops::DerefMut};

use crate::prelude::*;

#[derive(Debug)]
pub enum TravNode {
    Array,
    Object,
    Other,
}

pub struct Traverser<V> {
    active: RefCell<Option<V>>,
}

impl<V> Traverser<V> {
    pub fn new(active: V) -> Self {
        Self {
            active: RefCell::new(Some(active)),
        }
    }

    pub fn replace_active(&self, cb: impl FnOnce(V) -> Result<V, Zerr>) -> Result<(), Zerr> {
        let new_active = {
            let active = self.active.borrow_mut().take();
            if let Some(active) = active {
                cb(active)?
            } else {
                return Err(zerr!(
                    Zerr::InternalError,
                    "Active value in traverser is None, this should never happen."
                ));
            }
        };
        *self.active.borrow_mut() = Some(new_active);
        Ok(())
    }

    pub fn with_active<R>(&self, cb: impl FnOnce(&mut V) -> Result<R, Zerr>) -> Result<R, Zerr> {
        let mut active = self.active.borrow_mut();
        if let Some(active) = active.deref_mut() {
            cb(active)
        } else {
            Err(zerr!(
                Zerr::InternalError,
                "Active value in traverser is None, this should never happen."
            ))
        }
    }
}

pub trait Traversable<'r> {
    /// Get the active value as a TravNode, indicating if it's an array, object or something else.
    fn active(&self) -> Result<TravNode, Zerr>;

    /// Get the active value as a serde_json::Value, needing for error printing and outputting partials.
    fn active_as_serde(&self) -> Result<serde_json::Value, Zerr>;

    /// Move active to an array child at the given index.
    /// Already checked:
    /// - active is currently an array
    /// - index exists in current array
    /// Raise InternalErr on any problems.
    fn array_enter(&self, index: usize) -> Result<(), Zerr>;

    /// Replace a value in the active array.
    /// Already checked:
    /// - active is currently an array
    /// - Index is within bounds
    /// Raise InternalErr on any problems.
    fn array_set_index(&self, index: usize, json_str: &'r str) -> Result<(), Zerr>;

    /// Get the length of the active array.
    /// Already checked:
    /// - active is currently an array
    /// Raise InternalErr on any problems.
    fn array_len(&self) -> Result<usize, Zerr>;

    /// Push a value to the active array.
    /// Already checked:
    /// - active is currently an array
    /// Raise InternalErr on any problems.
    fn array_push(&self, json_str: &'r str) -> Result<(), Zerr>;

    /// Delete an index from an active array.
    /// Already checked:
    /// - active is currently an array
    /// - index exists in current array
    /// Raise InternalErr on any problems.
    fn array_delete_index(&self, index: usize) -> Result<(), Zerr>;

    /// Move active to an object child with the given key.
    /// Already checked:
    /// - active is currently an object
    /// - key exists in current object
    /// Raise InternalErr on any problems.
    fn object_enter(&self, key: &str) -> Result<(), Zerr>;

    /// Check if a key exists in an active object.
    /// Already checked:
    /// - active is currently an object
    /// Raise InternalErr on any problems.
    fn object_key_exists(&self, key: &str) -> Result<bool, Zerr>;

    /// Set a new value for a key in an active object.
    /// Already checked:
    /// - active is currently an object
    /// Raise InternalErr on any problems.
    fn object_set_key(&self, key: &'r str, json_str: &'r str) -> Result<(), Zerr>;

    /// Delete a key from an active object.
    /// Already checked:
    /// - active is currently an object
    /// - key exists in current object
    /// Raise InternalErr on any problems.
    fn object_delete_key(&self, key: &str) -> Result<(), Zerr>;

    /// Helper to convert a key to an index, erroring with all needed context if the key isn't a number.
    fn key_as_index(&self, key: &str) -> Result<usize, Zerr> {
        key.parse::<usize>()
            .change_context(Zerr::InternalError)
            .attach_printable(format!("Array index '{}' is not a number.", key))
    }

    /// Run any finalization needed at the end of the traverser's usage.
    fn finish(&self) -> Result<(), Zerr>;
}
