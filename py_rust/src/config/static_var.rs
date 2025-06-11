use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    coerce::{coerce, Coerce},
    prelude::*,
};

#[derive(Clone, Debug, Serialize)]
pub struct CtxStaticVar {
    pub value: serde_json::Value,
    pub coerce: Option<Coerce>,
}

impl CtxStaticVar {
    pub fn read(&self) -> Result<serde_json::Value, Report<Zerr>> {
        coerce(&self.value, &self.coerce)
    }
}

impl<'de> Deserialize<'de> for CtxStaticVar {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize into a serde_json::Value first
        let mut value: serde_json::Value = Deserialize::deserialize(deserializer)?;

        // If an object, contains the value key, maybe the coerce key, and no other keys, treat as fully specified structure:
        if matches!(&value, serde_json::Value::Object(map) if map.contains_key("value")
        && (map.len() == 1 || (map.len() == 2 && map.contains_key("coerce"))))
        {
            let map = value.as_object_mut().unwrap();
            Ok(CtxStaticVar {
                value: map.remove("value").unwrap(),
                // Coerce may or may not be present:
                coerce: if let Some(coerce) = map.remove("coerce") {
                    // Might be null:
                    if coerce.is_null() {
                        None
                    } else {
                        Some(Coerce::deserialize(coerce).map_err(serde::de::Error::custom)?)
                    }
                } else {
                    None
                },
            })
        } else {
            // Otherwise, treat the user entered as the "value", with no coerce:
            Ok(CtxStaticVar {
                value,
                coerce: None,
            })
        }
    }
}
