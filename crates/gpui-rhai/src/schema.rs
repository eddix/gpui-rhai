use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use rhai::{Array, Dynamic, FLOAT, FnPtr, INT, ImmutableString, Map};
use serde::{Deserialize, Serialize};

use crate::{OpaqueHandle, Style, UiNode};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValueSchema {
    Null,
    Bool,
    Integer,
    Float,
    Number,
    String {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        allowed: Vec<String>,
    },
    Array {
        items: Box<Self>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_items: Option<usize>,
    },
    Map {
        values: Box<Self>,
    },
    Object {
        fields: BTreeMap<String, ObjectField>,
        #[serde(default)]
        allow_unknown: bool,
    },
    Optional {
        value: Box<Self>,
    },
    Node,
    Callback,
    Style,
    Handle {
        kind: String,
    },
}

impl ValueSchema {
    #[must_use]
    pub fn string() -> Self {
        Self::String {
            allowed: Vec::new(),
        }
    }

    #[must_use]
    pub fn enumeration(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::String {
            allowed: values.into_iter().map(Into::into).collect(),
        }
    }

    #[must_use]
    pub fn optional(value: Self) -> Self {
        Self::Optional {
            value: Box::new(value),
        }
    }

    #[must_use]
    pub fn object(fields: BTreeMap<String, ObjectField>) -> Self {
        Self::Object {
            fields,
            allow_unknown: false,
        }
    }

    /// Validate a Rhai runtime value and collect all detectable schema issues.
    ///
    /// # Errors
    ///
    /// Returns [`SchemaValidationError`] with precise value paths when one or
    /// more constraints fail.
    pub fn validate(&self, value: &Dynamic) -> Result<(), SchemaValidationError> {
        let mut issues = Vec::new();
        self.validate_at(value, "$", &mut issues);
        if issues.is_empty() {
            Ok(())
        } else {
            Err(SchemaValidationError { issues })
        }
    }

    fn validate_at(&self, value: &Dynamic, path: &str, issues: &mut Vec<SchemaIssue>) {
        match self {
            Self::Null => expect_type(value.is_unit(), value, path, "null", issues),
            Self::Bool => expect_type(value.is::<bool>(), value, path, "bool", issues),
            Self::Integer => {
                expect_type(value.is::<INT>(), value, path, "integer", issues);
            }
            Self::Float => {
                expect_type(value.is::<FLOAT>(), value, path, "float", issues);
            }
            Self::Number => expect_type(
                value.is::<INT>() || value.is::<FLOAT>(),
                value,
                path,
                "number",
                issues,
            ),
            Self::String { allowed } => {
                if !value.is::<ImmutableString>() {
                    expect_type(false, value, path, "string", issues);
                } else if !allowed.is_empty() {
                    let actual = value.clone_cast::<ImmutableString>();
                    if !allowed.iter().any(|allowed| allowed == actual.as_str()) {
                        issues.push(SchemaIssue::new(
                            path,
                            format!("expected one of [{}], got `{actual}`", allowed.join(", ")),
                        ));
                    }
                }
            }
            Self::Array { items, max_items } => {
                if !value.is::<Array>() {
                    expect_type(false, value, path, "array", issues);
                    return;
                }
                let values = value.clone_cast::<Array>();
                if let Some(max_items) = max_items
                    && values.len() > *max_items
                {
                    issues.push(SchemaIssue::new(
                        path,
                        format!("expected at most {max_items} items, got {}", values.len()),
                    ));
                }
                for (index, item) in values.iter().enumerate() {
                    items.validate_at(item, &format!("{path}[{index}]"), issues);
                }
            }
            Self::Map { values } => {
                if !value.is::<Map>() {
                    expect_type(false, value, path, "map", issues);
                    return;
                }
                for (key, item) in value.clone_cast::<Map>() {
                    values.validate_at(&item, &format!("{path}.{key}"), issues);
                }
            }
            Self::Object {
                fields,
                allow_unknown,
            } => {
                if !value.is::<Map>() {
                    expect_type(false, value, path, "object", issues);
                    return;
                }
                validate_object(
                    &value.clone_cast::<Map>(),
                    fields,
                    *allow_unknown,
                    path,
                    issues,
                );
            }
            Self::Optional { value: schema } => {
                if !value.is_unit() {
                    schema.validate_at(value, path, issues);
                }
            }
            Self::Node => {
                expect_type(value.is::<UiNode>(), value, path, "UiNode", issues);
            }
            Self::Callback => {
                expect_type(value.is::<FnPtr>(), value, path, "callback", issues);
            }
            Self::Style => {
                expect_type(value.is::<Style>(), value, path, "Style", issues);
            }
            Self::Handle { kind } => {
                if value.is::<OpaqueHandle>() {
                    let handle = value.clone_cast::<OpaqueHandle>();
                    if handle.kind() != kind {
                        issues.push(SchemaIssue::new(
                            path,
                            format!("expected `{kind}` handle, got `{}` handle", handle.kind()),
                        ));
                    }
                } else {
                    expect_type(false, value, path, &format!("{kind} handle"), issues);
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjectField {
    pub schema: ValueSchema,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<crate::UiValue>,
}

impl ObjectField {
    #[must_use]
    pub fn required(schema: ValueSchema) -> Self {
        Self {
            schema,
            required: true,
            sensitive: false,
            default: None,
        }
    }

    #[must_use]
    pub fn optional(schema: ValueSchema) -> Self {
        Self {
            schema,
            required: false,
            sensitive: false,
            default: None,
        }
    }

    #[must_use]
    pub fn sensitive(mut self) -> Self {
        self.sensitive = true;
        self
    }

    #[must_use]
    pub fn with_default(mut self, default: crate::UiValue) -> Self {
        self.default = Some(default);
        self
    }
}

fn validate_object(
    value: &Map,
    fields: &BTreeMap<String, ObjectField>,
    allow_unknown: bool,
    path: &str,
    issues: &mut Vec<SchemaIssue>,
) {
    let actual_keys = value
        .keys()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();

    for (name, field) in fields {
        match value.get(name.as_str()) {
            Some(value) => field
                .schema
                .validate_at(value, &format!("{path}.{name}"), issues),
            None if field.required => issues.push(SchemaIssue::new(
                format!("{path}.{name}"),
                "required field is missing",
            )),
            None => {}
        }
    }

    if !allow_unknown {
        for unknown in actual_keys.difference(&fields.keys().cloned().collect()) {
            issues.push(SchemaIssue::new(
                format!("{path}.{unknown}"),
                "unknown field",
            ));
        }
    }
}

fn expect_type(
    valid: bool,
    value: &Dynamic,
    path: &str,
    expected: &str,
    issues: &mut Vec<SchemaIssue>,
) {
    if !valid {
        issues.push(SchemaIssue::new(
            path,
            format!("expected {expected}, got {}", value.type_name()),
        ));
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SchemaIssue {
    pub path: String,
    pub message: String,
}

impl SchemaIssue {
    #[must_use]
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SchemaValidationError {
    pub issues: Vec<SchemaIssue>,
}

impl fmt::Display for SchemaValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, issue) in self.issues.iter().enumerate() {
            if index > 0 {
                formatter.write_str("; ")?;
            }
            write!(formatter, "{}: {}", issue.path, issue.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for SchemaValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn button_schema() -> ValueSchema {
        ValueSchema::object(BTreeMap::from([
            (
                "text".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "variant".to_owned(),
                ObjectField::optional(ValueSchema::enumeration(["primary", "secondary"])),
            ),
            (
                "on_click".to_owned(),
                ObjectField::optional(ValueSchema::Callback),
            ),
        ]))
    }

    #[test]
    fn objects_report_all_precise_paths() {
        let value = Dynamic::from_map(Map::from_iter([
            ("variant".into(), Dynamic::from("danger")),
            ("lable".into(), Dynamic::from("Save")),
        ]));
        let error = button_schema().validate(&value).unwrap_err();

        assert_eq!(
            error.issues,
            vec![
                SchemaIssue::new("$.text", "required field is missing"),
                SchemaIssue::new(
                    "$.variant",
                    "expected one of [primary, secondary], got `danger`",
                ),
                SchemaIssue::new("$.lable", "unknown field"),
            ]
        );
    }

    #[test]
    fn node_callback_and_handle_types_are_distinct() {
        ValueSchema::Node
            .validate(&Dynamic::from(UiNode::text("content")))
            .unwrap();
        ValueSchema::Callback
            .validate(&Dynamic::from(FnPtr::new("clicked").unwrap()))
            .unwrap();
        ValueSchema::Handle {
            kind: "image".to_owned(),
        }
        .validate(&Dynamic::from(OpaqueHandle::new("image", 42)))
        .unwrap();

        let error = ValueSchema::Handle {
            kind: "image".to_owned(),
        }
        .validate(&Dynamic::from(OpaqueHandle::new("task", 42)))
        .unwrap_err();
        assert_eq!(error.issues[0].path, "$".to_owned());
    }
}
