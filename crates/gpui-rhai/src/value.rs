use std::collections::BTreeMap;

use rhai::{Array, CustomType, Dynamic, FLOAT, INT, ImmutableString, Map, TypeBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum UiValuePathSegment {
    Key(String),
    Index(usize),
    /// Select one map inside an array by a stable string field.
    Item {
        key_field: String,
        key: String,
    },
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct UiValuePath(Vec<UiValuePathSegment>);

impl<'de> Deserialize<'de> for UiValuePath {
    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'de>,
    {
        let segments = Vec::<UiValuePathSegment>::deserialize(deserializer)?;
        Self::new(segments).map_err(serde::de::Error::custom)
    }
}

impl UiValuePath {
    /// Construct a bounded nested value path.
    ///
    /// # Errors
    ///
    /// Returns [`UiValuePathError::InvalidPath`] for more than 64 segments or
    /// empty/oversized map keys.
    pub fn new(segments: Vec<UiValuePathSegment>) -> Result<Self, UiValuePathError> {
        if segments.len() > 64
            || segments.iter().any(|segment| match segment {
                UiValuePathSegment::Key(key) => key.is_empty() || key.len() > 256,
                UiValuePathSegment::Index(_) => false,
                UiValuePathSegment::Item { key_field, key } => {
                    key_field.is_empty()
                        || key_field.len() > 256
                        || key.is_empty()
                        || key.len() > 256
                }
            })
        {
            return Err(UiValuePathError::InvalidPath);
        }
        Ok(Self(segments))
    }

    #[must_use]
    pub fn segments(&self) -> &[UiValuePathSegment] {
        &self.0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

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
#[derive(Clone, Debug, PartialEq)]
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

#[derive(Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum UiValueRef<'a> {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(&'a str),
    Array(&'a [UiValue]),
    Map(&'a BTreeMap<String, UiValue>),
    Handle(&'a OpaqueHandle),
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum UiValueOwned {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<UiValue>),
    Map(BTreeMap<String, UiValue>),
    Handle(OpaqueHandle),
}

impl Serialize for UiValue {
    fn serialize<Serializer>(
        &self,
        serializer: Serializer,
    ) -> Result<Serializer::Ok, Serializer::Error>
    where
        Serializer: serde::Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        match self {
            Self::Null => UiValueRef::Null,
            Self::Bool(value) => UiValueRef::Bool(*value),
            Self::Integer(value) => UiValueRef::Integer(*value),
            Self::Float(value) => UiValueRef::Float(*value),
            Self::String(value) => UiValueRef::String(value),
            Self::Array(value) => UiValueRef::Array(value),
            Self::Map(value) => UiValueRef::Map(value),
            Self::Handle(value) => UiValueRef::Handle(value),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for UiValue {
    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'de>,
    {
        let value = match UiValueOwned::deserialize(deserializer)? {
            UiValueOwned::Null => Self::Null,
            UiValueOwned::Bool(value) => Self::Bool(value),
            UiValueOwned::Integer(value) => Self::Integer(value),
            UiValueOwned::Float(value) => Self::Float(value),
            UiValueOwned::String(value) => Self::String(value),
            UiValueOwned::Array(value) => Self::Array(value),
            UiValueOwned::Map(value) => Self::Map(value),
            UiValueOwned::Handle(value) => Self::Handle(value),
        };
        value.validate().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

impl UiValue {
    const MAX_DEPTH: usize = 64;
    const MAX_ITEMS: usize = 1_000_000;
    const MAX_BYTES: usize = 16 * 1024 * 1024;

    /// Validate the complete durable value domain, including values constructed
    /// directly by Rust or deserialized without passing through Rhai.
    ///
    /// # Errors
    ///
    /// Returns a path-aware non-finite or resource-limit violation.
    pub fn validate(&self) -> Result<(), UiValueError> {
        let mut items = 0usize;
        let mut bytes = 0usize;
        self.validate_at("$", 0, &mut items, &mut bytes)
    }

    fn validate_at(
        &self,
        path: &str,
        depth: usize,
        items: &mut usize,
        bytes: &mut usize,
    ) -> Result<(), UiValueError> {
        if depth > Self::MAX_DEPTH {
            return Err(UiValueError::Limit {
                path: path.to_owned(),
                resource: "depth",
                limit: Self::MAX_DEPTH,
            });
        }
        *items = items.saturating_add(1);
        if *items > Self::MAX_ITEMS {
            return Err(UiValueError::Limit {
                path: path.to_owned(),
                resource: "items",
                limit: Self::MAX_ITEMS,
            });
        }
        match self {
            Self::Float(value) if !value.is_finite() => {
                return Err(UiValueError::NonFiniteFloat {
                    path: path.to_owned(),
                });
            }
            Self::String(value) => *bytes = bytes.saturating_add(value.len()),
            Self::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    value.validate_at(&format!("{path}[{index}]"), depth + 1, items, bytes)?;
                }
            }
            Self::Map(values) => {
                for (key, value) in values {
                    *bytes = bytes.saturating_add(key.len());
                    value.validate_at(&format!("{path}.{key}"), depth + 1, items, bytes)?;
                }
            }
            Self::Null | Self::Bool(_) | Self::Integer(_) | Self::Float(_) | Self::Handle(_) => {}
        }
        if *bytes > Self::MAX_BYTES {
            return Err(UiValueError::Limit {
                path: path.to_owned(),
                resource: "bytes",
                limit: Self::MAX_BYTES,
            });
        }
        Ok(())
    }

    /// Read one nested map/array path.
    ///
    /// # Errors
    ///
    /// Returns a precise type, missing-key, or out-of-bounds path error.
    pub fn get_path(&self, path: &UiValuePath) -> Result<&Self, UiValuePathError> {
        let mut current = self;
        for (depth, segment) in path.segments().iter().enumerate() {
            current = match (current, segment) {
                (Self::Map(values), UiValuePathSegment::Key(key)) => {
                    values
                        .get(key)
                        .ok_or_else(|| UiValuePathError::MissingKey {
                            depth,
                            key: key.clone(),
                        })?
                }
                (Self::Array(values), UiValuePathSegment::Index(index)) => values
                    .get(*index)
                    .ok_or(UiValuePathError::IndexOutOfBounds {
                        depth,
                        index: *index,
                        len: values.len(),
                    })?,
                (Self::Array(values), UiValuePathSegment::Item { key_field, key }) => {
                    &values[Self::keyed_item_index(values, depth, key_field, key)?]
                }
                (value, segment) => {
                    return Err(UiValuePathError::TypeMismatch {
                        depth,
                        expected: match segment {
                            UiValuePathSegment::Key(_) => "map",
                            UiValuePathSegment::Index(_) | UiValuePathSegment::Item { .. } => {
                                "array"
                            }
                        },
                        actual: value.kind_name(),
                    });
                }
            };
        }
        Ok(current)
    }

    /// Replace one existing nested map/array path.
    ///
    /// An empty path replaces the complete value. This operation never grows
    /// arrays or creates missing map keys.
    ///
    /// # Errors
    ///
    /// Returns a precise type, missing-key, or out-of-bounds path error.
    pub fn set_path(&mut self, path: &UiValuePath, value: Self) -> Result<(), UiValuePathError> {
        let Some((last, parents)) = path.segments().split_last() else {
            *self = value;
            return Ok(());
        };
        let parent = Self::get_path_mut(self, parents)?;
        match (parent, last) {
            (Self::Map(values), UiValuePathSegment::Key(key)) => {
                let slot = values
                    .get_mut(key)
                    .ok_or_else(|| UiValuePathError::MissingKey {
                        depth: parents.len(),
                        key: key.clone(),
                    })?;
                *slot = value;
            }
            (Self::Array(values), UiValuePathSegment::Index(index)) => {
                let len = values.len();
                let slot = values
                    .get_mut(*index)
                    .ok_or(UiValuePathError::IndexOutOfBounds {
                        depth: parents.len(),
                        index: *index,
                        len,
                    })?;
                *slot = value;
            }
            (Self::Array(values), UiValuePathSegment::Item { key_field, key }) => {
                let index = Self::keyed_item_index(values, parents.len(), key_field, key)?;
                values[index] = value;
            }
            (parent, segment) => {
                return Err(UiValuePathError::TypeMismatch {
                    depth: parents.len(),
                    expected: match segment {
                        UiValuePathSegment::Key(_) => "map",
                        UiValuePathSegment::Index(_) | UiValuePathSegment::Item { .. } => "array",
                    },
                    actual: parent.kind_name(),
                });
            }
        }
        Ok(())
    }

    fn get_path_mut<'a>(
        mut current: &'a mut Self,
        segments: &[UiValuePathSegment],
    ) -> Result<&'a mut Self, UiValuePathError> {
        for (depth, segment) in segments.iter().enumerate() {
            current = match (current, segment) {
                (Self::Map(values), UiValuePathSegment::Key(key)) => values
                    .get_mut(key)
                    .ok_or_else(|| UiValuePathError::MissingKey {
                        depth,
                        key: key.clone(),
                    })?,
                (Self::Array(values), UiValuePathSegment::Index(index)) => {
                    let len = values.len();
                    values
                        .get_mut(*index)
                        .ok_or(UiValuePathError::IndexOutOfBounds {
                            depth,
                            index: *index,
                            len,
                        })?
                }
                (Self::Array(values), UiValuePathSegment::Item { key_field, key }) => {
                    let index = Self::keyed_item_index(values, depth, key_field, key)?;
                    &mut values[index]
                }
                (value, segment) => {
                    return Err(UiValuePathError::TypeMismatch {
                        depth,
                        expected: match segment {
                            UiValuePathSegment::Key(_) => "map",
                            UiValuePathSegment::Index(_) | UiValuePathSegment::Item { .. } => {
                                "array"
                            }
                        },
                        actual: value.kind_name(),
                    });
                }
            };
        }
        Ok(current)
    }

    fn keyed_item_index(
        values: &[Self],
        depth: usize,
        key_field: &str,
        key: &str,
    ) -> Result<usize, UiValuePathError> {
        let mut found = None;
        for (index, item) in values.iter().enumerate() {
            let Self::Map(fields) = item else {
                return Err(UiValuePathError::InvalidKeyedItem {
                    depth,
                    index,
                    reason: "item is not a map",
                });
            };
            let Some(item_key) = fields.get(key_field) else {
                return Err(UiValuePathError::InvalidKeyedItem {
                    depth,
                    index,
                    reason: "key field is missing",
                });
            };
            let Self::String(item_key) = item_key else {
                return Err(UiValuePathError::InvalidKeyedItem {
                    depth,
                    index,
                    reason: "key field is not a string",
                });
            };
            if item_key == key && found.replace(index).is_some() {
                return Err(UiValuePathError::DuplicateItemKey {
                    depth,
                    key_field: key_field.to_owned(),
                    key: key.to_owned(),
                });
            }
        }
        found.ok_or_else(|| UiValuePathError::MissingItem {
            depth,
            key_field: key_field.to_owned(),
            key: key.to_owned(),
        })
    }

    const fn kind_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::Integer(_) => "integer",
            Self::Float(_) => "float",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Map(_) => "map",
            Self::Handle(_) => "handle",
        }
    }

    /// Convert a Rhai value into the restricted capability data model.
    ///
    /// # Errors
    ///
    /// Returns [`UiValueError`] when the value contains a custom type such as a
    /// `UiNode` or callback, or when one of its descendants is unsupported.
    pub fn from_dynamic(value: Dynamic) -> Result<Self, UiValueError> {
        let value = Self::from_dynamic_at(value, "$".to_owned())?;
        value.validate()?;
        Ok(value)
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
    #[error("non-finite float at {path}")]
    NonFiniteFloat { path: String },
    #[error("UiValue {resource} limit {limit} exceeded at {path}")]
    Limit {
        path: String,
        resource: &'static str,
        limit: usize,
    },
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum UiValuePathError {
    #[error("UiValue path must have at most 64 segments and 1-256 byte keys")]
    InvalidPath,
    #[error("UiValue path segment {depth} is missing map key `{key}`")]
    MissingKey { depth: usize, key: String },
    #[error("UiValue path segment {depth} index {index} is outside array length {len}")]
    IndexOutOfBounds {
        depth: usize,
        index: usize,
        len: usize,
    },
    #[error("UiValue path segment {depth} requires {expected}, got {actual}")]
    TypeMismatch {
        depth: usize,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("UiValue path segment {depth} keyed item {index} is invalid: {reason}")]
    InvalidKeyedItem {
        depth: usize,
        index: usize,
        reason: &'static str,
    },
    #[error("UiValue path segment {depth} has no item where `{key_field}` is `{key}`")]
    MissingItem {
        depth: usize,
        key_field: String,
        key: String,
    },
    #[error("UiValue path segment {depth} has duplicate items where `{key_field}` is `{key}`")]
    DuplicateItemKey {
        depth: usize,
        key_field: String,
        key: String,
    },
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

    #[test]
    fn non_finite_numbers_are_rejected_from_rhai_and_serialization() {
        assert!(matches!(
            UiValue::from_dynamic(Dynamic::from(f64::NAN)),
            Err(UiValueError::NonFiniteFloat { .. })
        ));
        assert!(serde_json::to_string(&UiValue::Float(f64::INFINITY)).is_err());
        assert!(serde_json::from_str::<UiValue>(r#"{"type":"float","value":1e999}"#).is_err());
    }

    #[test]
    fn bounded_paths_read_and_replace_existing_nested_values() {
        let path = UiValuePath::new(vec![
            UiValuePathSegment::Key("items".to_owned()),
            UiValuePathSegment::Index(1),
            UiValuePathSegment::Key("name".to_owned()),
        ])
        .unwrap();
        let mut value = UiValue::Map(BTreeMap::from([(
            "items".to_owned(),
            UiValue::Array(vec![
                UiValue::Map(BTreeMap::from([(
                    "name".to_owned(),
                    UiValue::String("first".to_owned()),
                )])),
                UiValue::Map(BTreeMap::from([(
                    "name".to_owned(),
                    UiValue::String("second".to_owned()),
                )])),
            ]),
        )]));
        assert_eq!(
            value.get_path(&path).unwrap(),
            &UiValue::String("second".to_owned())
        );
        value
            .set_path(&path, UiValue::String("updated".to_owned()))
            .unwrap();
        assert_eq!(
            value.get_path(&path).unwrap(),
            &UiValue::String("updated".to_owned())
        );
        assert!(
            value
                .get_path(&UiValuePath::new(vec![UiValuePathSegment::Index(0)]).unwrap())
                .is_err()
        );
    }

    #[test]
    fn keyed_item_paths_survive_array_reorder() {
        let path = UiValuePath::new(vec![
            UiValuePathSegment::Key("items".to_owned()),
            UiValuePathSegment::Item {
                key_field: "id".to_owned(),
                key: "second".to_owned(),
            },
            UiValuePathSegment::Key("name".to_owned()),
        ])
        .unwrap();
        let item = |id: &str, name: &str| {
            UiValue::Map(BTreeMap::from([
                ("id".to_owned(), UiValue::String(id.to_owned())),
                ("name".to_owned(), UiValue::String(name.to_owned())),
            ]))
        };
        let mut value = UiValue::Map(BTreeMap::from([(
            "items".to_owned(),
            UiValue::Array(vec![item("first", "Alpha"), item("second", "Beta")]),
        )]));
        assert_eq!(
            value.get_path(&path).unwrap(),
            &UiValue::String("Beta".to_owned())
        );
        if let UiValue::Map(root) = &mut value
            && let Some(UiValue::Array(items)) = root.get_mut("items")
        {
            items.reverse();
        }
        assert_eq!(
            value.get_path(&path).unwrap(),
            &UiValue::String("Beta".to_owned())
        );
    }
}
