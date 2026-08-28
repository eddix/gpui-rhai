use std::collections::BTreeMap;

use rhai::{Array, CustomType, Dynamic, FLOAT, INT, ImmutableString, Map, TypeBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A typed reference to a Rust-owned resource that must not be copied into Rhai.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct OpaqueHandle {
    kind: String,
    id: u64,
}

impl OpaqueHandle {
    #[must_use]
    pub fn new(kind: impl Into<String>, id: u64) -> Self {
        Self {
            kind: kind.into(),
            id,
        }
    }

    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }
}

impl CustomType for OpaqueHandle {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("OpaqueHandle")
            .with_get("kind", |handle: &mut Self| {
                ImmutableString::from(handle.kind.clone())
            })
            .with_fn("to_string", |handle: &mut Self| {
                format!("{}#{}", handle.kind, handle.id)
            });
    }
}

/// Data allowed to cross a capability or semantic-event boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum UiValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Self>),
    Map(BTreeMap<String, Self>),
    Handle(OpaqueHandle),
}

impl UiValue {
    /// Convert a Rhai value into the restricted capability data model.
    ///
    /// # Errors
    ///
    /// Returns [`UiValueError`] when the value contains a custom type such as a
    /// `UiNode` or callback, or when one of its descendants is unsupported.
    pub fn from_dynamic(value: Dynamic) -> Result<Self, UiValueError> {
        Self::from_dynamic_at(value, "$".to_owned())
    }

    fn from_dynamic_at(value: Dynamic, path: String) -> Result<Self, UiValueError> {
        if value.is_unit() {
            return Ok(Self::Null);
        }
        if value.is::<bool>() {
            return Ok(Self::Bool(value.cast::<bool>()));
        }
        if value.is::<INT>() {
            return Ok(Self::Integer(value.cast::<INT>()));
        }
        if value.is::<FLOAT>() {
            return Ok(Self::Float(value.cast::<FLOAT>()));
        }
        if value.is::<ImmutableString>() {
            return Ok(Self::String(value.cast::<ImmutableString>().to_string()));
        }
        if value.is::<Array>() {
            let values = value.cast::<Array>();
            return values
                .into_iter()
                .enumerate()
                .map(|(index, value)| Self::from_dynamic_at(value, format!("{path}[{index}]")))
                .collect::<Result<Vec<_>, _>>()
                .map(Self::Array);
        }
        if value.is::<Map>() {
            let values = value.cast::<Map>();
            return values
                .into_iter()
                .map(|(key, value)| {
                    let key = key.to_string();
                    let value = Self::from_dynamic_at(value, format!("{path}.{key}"))?;
                    Ok((key, value))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()
                .map(Self::Map);
        }
        if value.is::<OpaqueHandle>() {
            return Ok(Self::Handle(value.cast::<OpaqueHandle>()));
        }

        Err(UiValueError::UnsupportedType {
            path,
            type_name: value.type_name().to_owned(),
        })
    }

    pub fn into_dynamic(self) -> Dynamic {
        match self {
            Self::Null => Dynamic::UNIT,
            Self::Bool(value) => Dynamic::from(value),
            Self::Integer(value) => Dynamic::from(value),
            Self::Float(value) => Dynamic::from(value),
            Self::String(value) => Dynamic::from(value),
            Self::Array(values) => {
                Dynamic::from_array(values.into_iter().map(Self::into_dynamic).collect())
            }
            Self::Map(values) => {
                let map = values
                    .into_iter()
                    .map(|(key, value)| (key.into(), value.into_dynamic()))
                    .collect::<Map>();
                Dynamic::from_map(map)
            }
            Self::Handle(handle) => Dynamic::from(handle),
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum UiValueError {
    #[error("unsupported Rhai type `{type_name}` at {path}")]
    UnsupportedType { path: String, type_name: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UiNode;

    #[test]
    fn nested_values_round_trip() {
        let value = UiValue::Map(BTreeMap::from([
            ("enabled".to_owned(), UiValue::Bool(true)),
            (
                "items".to_owned(),
                UiValue::Array(vec![UiValue::Integer(1), UiValue::String("two".to_owned())]),
            ),
            (
                "image".to_owned(),
                UiValue::Handle(OpaqueHandle::new("image", 7)),
            ),
        ]));

        assert_eq!(
            UiValue::from_dynamic(value.clone().into_dynamic()).unwrap(),
            value
        );
    }

    #[test]
    fn ui_nodes_cannot_cross_capability_boundary() {
        let error = UiValue::from_dynamic(Dynamic::from(UiNode::text("no"))).unwrap_err();
        assert!(matches!(
            error,
            UiValueError::UnsupportedType { ref path, .. } if path == "$"
        ));
    }
}
