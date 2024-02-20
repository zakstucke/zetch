use std::collections::{BTreeMap, HashMap};

use serde::{Serialize, Serializer};

/// For use with serde's [serialize_with] attribute.
///
/// Will make a hashmap alphabetically ordered by key on serialization.
///
/// Source: https://stackoverflow.com/questions/42723065/how-to-sort-hashmap-keys-when-serializing-with-serde
pub fn ordered_map_serializer<S, K: Ord + Serialize, V: Serialize>(
    value: &HashMap<K, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}
