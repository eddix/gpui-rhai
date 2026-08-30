use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use rhai::{Array, Dynamic, FLOAT, FnPtr, INT, ImmutableString, Map};
use serde::{Deserialize, Serialize};

use crate::{AssetId, ElementRef, Length, NativeSignal, OpaqueHandle, Style, UiNode, UiValue};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValueSchema {
    Null,
    Bool,
    Integer {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<INT>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<INT>,
    },
    Float {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<FLOAT>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<FLOAT>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exclusive_min: Option<FLOAT>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exclusive_max: Option<FLOAT>,
    },
    Number {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<FLOAT>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<FLOAT>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exclusive_min: Option<FLOAT>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exclusive_max: Option<FLOAT>,
    },
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
    OneOf {
        variants: Vec<Self>,
    },
    Node,
    Callback,
    Style,
    Length,
    UiValue,
    Asset,
    Signal,
    Ref,
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
    pub const fn integer() -> Self {
        Self::Integer {
            min: None,
            max: None,
        }
    }

    #[must_use]
    pub const fn bounded_integer(min: Option<INT>, max: Option<INT>) -> Self {
        Self::Integer { min, max }
    }

    #[must_use]
    pub const fn float() -> Self {
        Self::Float {
            min: None,
            max: None,
            exclusive_min: None,
            exclusive_max: None,
        }
    }

    #[must_use]
    pub const fn bounded_float(min: Option<FLOAT>, max: Option<FLOAT>) -> Self {
        Self::Float {
            min,
            max,
            exclusive_min: None,
            exclusive_max: None,
        }
    }

    #[must_use]
    pub const fn number() -> Self {
        Self::Number {
            min: None,
            max: None,
            exclusive_min: None,
            exclusive_max: None,
        }
    }

    #[must_use]
    pub const fn bounded_number(min: Option<FLOAT>, max: Option<FLOAT>) -> Self {
        Self::Number {
            min,
            max,
            exclusive_min: None,
            exclusive_max: None,
        }
    }

    #[must_use]
    pub const fn positive_number() -> Self {
        Self::Number {
            min: None,
            max: None,
            exclusive_min: Some(0.0),
            exclusive_max: None,
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
    pub fn one_of(variants: impl IntoIterator<Item = Self>) -> Self {
        Self::OneOf {
            variants: variants.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn object(fields: BTreeMap<String, ObjectField>) -> Self {
        Self::Object {
            fields,
            allow_unknown: false,
        }
    }

    /// Validate the schema itself before it is used for props or events.
    ///
    /// # Errors
    ///
    /// Returns [`SchemaDefinitionError`] for inverted/non-finite bounds, an
    /// empty union, duplicate enum values, or an invalid nested definition.
    pub fn validate_definition(&self) -> Result<(), SchemaDefinitionError> {
        self.validate_definition_at("$")
    }

    fn validate_definition_at(&self, path: &str) -> Result<(), SchemaDefinitionError> {
        match self {
            Self::Integer { min, max } => validate_integer_bounds(*min, *max, path),
            Self::Float {
                min,
                max,
                exclusive_min,
                exclusive_max,
            }
            | Self::Number {
                min,
                max,
                exclusive_min,
                exclusive_max,
            } => validate_float_bounds(*min, *max, *exclusive_min, *exclusive_max, path),
            Self::String { allowed } => {
                let mut unique = BTreeSet::new();
                if let Some(duplicate) = allowed.iter().find(|value| !unique.insert(*value)) {
                    return Err(SchemaDefinitionError::new(
                        path,
                        format!("allowed string value `{duplicate}` is duplicated"),
                    ));
                }
                Ok(())
            }
            Self::Array { items, .. } => items.validate_definition_at(&format!("{path}.items")),
            Self::Map { values } => values.validate_definition_at(&format!("{path}.values")),
            Self::Object { fields, .. } => {
                for (name, field) in fields {
                    field
                        .schema
                        .validate_definition_at(&format!("{path}.fields.{name}"))?;
                }
                Ok(())
            }
            Self::Optional { value } => value.validate_definition_at(&format!("{path}.value")),
            Self::OneOf { variants } => {
                if variants.is_empty() {
                    return Err(SchemaDefinitionError::new(
                        path,
                        "one_of must contain at least one variant",
                    ));
                }
                for (index, variant) in variants.iter().enumerate() {
                    variant.validate_definition_at(&format!("{path}.variants[{index}]"))?;
                }
                Ok(())
            }
            Self::Handle { kind } if kind.trim().is_empty() => Err(SchemaDefinitionError::new(
                path,
                "handle kind cannot be empty",
            )),
            Self::Null
            | Self::Bool
            | Self::Node
            | Self::Callback
            | Self::Style
            | Self::Length
            | Self::UiValue
            | Self::Asset
            | Self::Signal
            | Self::Ref
            | Self::Handle { .. } => Ok(()),
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
            Self::Integer { min, max } => validate_integer(value, *min, *max, path, issues),
            Self::Float {
                min,
                max,
                exclusive_min,
                exclusive_max,
            } => validate_float(
                value,
                *min,
                *max,
                *exclusive_min,
                *exclusive_max,
                path,
                issues,
            ),
            Self::Number {
                min,
                max,
                exclusive_min,
                exclusive_max,
            } => validate_number(
                value,
                *min,
                *max,
                *exclusive_min,
                *exclusive_max,
                path,
                issues,
            ),
            Self::String { allowed } => validate_string(value, allowed, path, issues),
            Self::Array { items, max_items } => {
                validate_array(value, items, *max_items, path, issues);
            }
            Self::Map { values } => validate_map(value, values, path, issues),
            Self::Object {
                fields,
                allow_unknown,
            } => validate_object_value(value, fields, *allow_unknown, path, issues),
            Self::Optional { value: schema } => {
                if !value.is_unit() {
                    schema.validate_at(value, path, issues);
                }
            }
            Self::OneOf { variants } => validate_one_of(value, variants, path, issues),
            Self::Node => expect_type(value.is::<UiNode>(), value, path, "UiNode", issues),
            Self::Callback => expect_type(
                value.is::<FnPtr>() || value.is::<crate::NativeHandlerRef>(),
                value,
                path,
                "callback or NativeHandlerRef",
                issues,
            ),
            Self::Style => expect_type(value.is::<Style>(), value, path, "Style", issues),
            Self::Length => validate_length(value, path, issues),
            Self::UiValue => {
                if let Err(error) = UiValue::from_dynamic(value.clone()) {
                    issues.push(SchemaIssue::new(path, error.to_string()));
                }
            }
            Self::Asset => expect_type(value.is::<AssetId>(), value, path, "AssetId", issues),
            Self::Signal => expect_type(
                value.is::<NativeSignal>(),
                value,
                path,
                "NativeSignal",
                issues,
            ),
            Self::Ref => expect_type(value.is::<ElementRef>(), value, path, "ElementRef", issues),
            Self::Handle { kind } => validate_handle(value, kind, path, issues),
        }
    }
}

fn validate_integer(
    value: &Dynamic,
    min: Option<INT>,
    max: Option<INT>,
    path: &str,
    issues: &mut Vec<SchemaIssue>,
) {
    if value.is::<INT>() {
        validate_integer_value(value.clone_cast::<INT>(), min, max, path, issues);
    } else {
        expect_type(false, value, path, "integer", issues);
    }
}

fn validate_float(
    value: &Dynamic,
    min: Option<FLOAT>,
    max: Option<FLOAT>,
    exclusive_min: Option<FLOAT>,
    exclusive_max: Option<FLOAT>,
    path: &str,
    issues: &mut Vec<SchemaIssue>,
) {
    if value.is::<FLOAT>() {
        validate_float_value(
            value.clone_cast::<FLOAT>(),
            min,
            max,
            exclusive_min,
            exclusive_max,
            path,
            issues,
        );
    } else {
        expect_type(false, value, path, "float", issues);
    }
}

fn validate_number(
    value: &Dynamic,
    min: Option<FLOAT>,
    max: Option<FLOAT>,
    exclusive_min: Option<FLOAT>,
    exclusive_max: Option<FLOAT>,
    path: &str,
    issues: &mut Vec<SchemaIssue>,
) {
    let number = if value.is::<INT>() {
        Some(integer_as_float(value.clone_cast::<INT>()))
    } else if value.is::<FLOAT>() {
        Some(value.clone_cast::<FLOAT>())
    } else {
        None
    };
    if let Some(number) = number {
        validate_float_value(number, min, max, exclusive_min, exclusive_max, path, issues);
    } else {
        expect_type(false, value, path, "number", issues);
    }
}

fn validate_string(value: &Dynamic, allowed: &[String], path: &str, issues: &mut Vec<SchemaIssue>) {
    if value.is::<ImmutableString>() {
        let actual = value.clone_cast::<ImmutableString>();
        if !allowed.is_empty() && !allowed.iter().any(|allowed| allowed == actual.as_str()) {
            issues.push(SchemaIssue::new(
                path,
                format!("expected one of [{}], got `{actual}`", allowed.join(", ")),
            ));
        }
    } else {
        expect_type(false, value, path, "string", issues);
    }
}

fn validate_array(
    value: &Dynamic,
    items: &ValueSchema,
    max_items: Option<usize>,
    path: &str,
    issues: &mut Vec<SchemaIssue>,
) {
    if !value.is::<Array>() {
        expect_type(false, value, path, "array", issues);
        return;
    }
    let values = value.clone_cast::<Array>();
    if let Some(max_items) = max_items
        && values.len() > max_items
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

fn validate_map(value: &Dynamic, values: &ValueSchema, path: &str, issues: &mut Vec<SchemaIssue>) {
    if !value.is::<Map>() {
        expect_type(false, value, path, "map", issues);
        return;
    }
    for (key, item) in value.clone_cast::<Map>() {
        values.validate_at(&item, &format!("{path}.{key}"), issues);
    }
}

fn validate_object_value(
    value: &Dynamic,
    fields: &BTreeMap<String, ObjectField>,
    allow_unknown: bool,
    path: &str,
    issues: &mut Vec<SchemaIssue>,
) {
    if value.is::<Map>() {
        validate_object(
            &value.clone_cast::<Map>(),
            fields,
            allow_unknown,
            path,
            issues,
        );
    } else {
        expect_type(false, value, path, "object", issues);
    }
}

fn validate_one_of(
    value: &Dynamic,
    variants: &[ValueSchema],
    path: &str,
    issues: &mut Vec<SchemaIssue>,
) {
    let mut branch_issues = Vec::with_capacity(variants.len());
    for variant in variants {
        let mut candidate = Vec::new();
        variant.validate_at(value, path, &mut candidate);
        if candidate.is_empty() {
            return;
        }
        branch_issues.push(candidate);
    }
    let summary = branch_issues
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let messages = candidate
                .iter()
                .map(|issue| issue.message.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("variant {index}: {messages}")
        })
        .collect::<Vec<_>>()
        .join("; ");
    issues.push(SchemaIssue::new(
        path,
        format!("value did not match any one_of variant ({summary})"),
    ));
}

fn validate_length(value: &Dynamic, path: &str, issues: &mut Vec<SchemaIssue>) {
    if value.is::<Length>() {
        let length = value.clone_cast::<Length>();
        if let Err(error) = length.validate() {
            issues.push(SchemaIssue::new(path, error.to_string()));
        }
    } else {
        expect_type(false, value, path, "Length", issues);
    }
}

fn validate_handle(value: &Dynamic, kind: &str, path: &str, issues: &mut Vec<SchemaIssue>) {
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

fn validate_integer_bounds(
    min: Option<INT>,
    max: Option<INT>,
    path: &str,
) -> Result<(), SchemaDefinitionError> {
    if min.zip(max).is_some_and(|(min, max)| min > max) {
        Err(SchemaDefinitionError::new(
            path,
            format!("integer minimum {min:?} exceeds maximum {max:?}"),
        ))
    } else {
        Ok(())
    }
}

fn validate_float_bounds(
    min: Option<FLOAT>,
    max: Option<FLOAT>,
    exclusive_min: Option<FLOAT>,
    exclusive_max: Option<FLOAT>,
    path: &str,
) -> Result<(), SchemaDefinitionError> {
    if min.is_some_and(|value| !value.is_finite())
        || max.is_some_and(|value| !value.is_finite())
        || exclusive_min.is_some_and(|value| !value.is_finite())
        || exclusive_max.is_some_and(|value| !value.is_finite())
    {
        return Err(SchemaDefinitionError::new(
            path,
            "numeric bounds must be finite",
        ));
    }
    if min.is_some() && exclusive_min.is_some() {
        return Err(SchemaDefinitionError::new(
            path,
            "numeric schema cannot define both min and exclusive_min",
        ));
    }
    if max.is_some() && exclusive_max.is_some() {
        return Err(SchemaDefinitionError::new(
            path,
            "numeric schema cannot define both max and exclusive_max",
        ));
    }
    let lower = min.or(exclusive_min);
    let upper = max.or(exclusive_max);
    let empty = lower.zip(upper).is_some_and(|(lower, upper)| {
        matches!(lower.total_cmp(&upper), Ordering::Greater)
            || (matches!(lower.total_cmp(&upper), Ordering::Equal)
                && (exclusive_min.is_some() || exclusive_max.is_some()))
    });
    if empty {
        Err(SchemaDefinitionError::new(
            path,
            format!("numeric lower bound {lower:?} does not precede upper bound {upper:?}"),
        ))
    } else {
        Ok(())
    }
}

fn validate_integer_value(
    value: INT,
    min: Option<INT>,
    max: Option<INT>,
    path: &str,
    issues: &mut Vec<SchemaIssue>,
) {
    if let Some(min) = min
        && value < min
    {
        issues.push(SchemaIssue::new(
            path,
            format!("expected integer >= {min}, got {value}"),
        ));
    }
    if let Some(max) = max
        && value > max
    {
        issues.push(SchemaIssue::new(
            path,
            format!("expected integer <= {max}, got {value}"),
        ));
    }
}

fn validate_float_value(
    value: FLOAT,
    min: Option<FLOAT>,
    max: Option<FLOAT>,
    exclusive_min: Option<FLOAT>,
    exclusive_max: Option<FLOAT>,
    path: &str,
    issues: &mut Vec<SchemaIssue>,
) {
    if !value.is_finite() {
        issues.push(SchemaIssue::new(path, "number must be finite"));
        return;
    }
    if let Some(min) = min
        && value < min
    {
        issues.push(SchemaIssue::new(
            path,
            format!("expected number >= {min}, got {value}"),
        ));
    }
    if let Some(max) = max
        && value > max
    {
        issues.push(SchemaIssue::new(
            path,
            format!("expected number <= {max}, got {value}"),
        ));
    }
    if let Some(min) = exclusive_min
        && value <= min
    {
        issues.push(SchemaIssue::new(
            path,
            format!("expected number > {min}, got {value}"),
        ));
    }
    if let Some(max) = exclusive_max
        && value >= max
    {
        issues.push(SchemaIssue::new(
            path,
            format!("expected number < {max}, got {value}"),
        ));
    }
}

fn integer_as_float(value: INT) -> FLOAT {
    value
        .to_string()
        .parse()
        .expect("an integer always has a finite float representation")
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SchemaDefinitionError {
    pub path: String,
    pub message: String,
}

impl SchemaDefinitionError {
    #[must_use]
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for SchemaDefinitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for SchemaDefinitionError {}

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

    #[test]
    fn bounds_and_unions_report_precise_failures() {
        let schema = ValueSchema::one_of([
            ValueSchema::bounded_integer(Some(1), Some(3)),
            ValueSchema::enumeration(["auto"]),
        ]);
        schema.validate_definition().unwrap();
        schema.validate(&Dynamic::from(2_i64)).unwrap();
        schema.validate(&Dynamic::from("auto")).unwrap();

        let error = schema.validate(&Dynamic::from(9_i64)).unwrap_err();
        assert_eq!(error.issues.len(), 1);
        assert!(error.issues[0].message.contains("integer <= 3"));
        assert!(error.issues[0].message.contains("expected string"));
    }

    #[test]
    fn invalid_definitions_are_rejected() {
        assert!(matches!(
            ValueSchema::bounded_integer(Some(4), Some(2)).validate_definition(),
            Err(SchemaDefinitionError { ref path, .. }) if path == "$"
        ));
        assert!(ValueSchema::one_of([]).validate_definition().is_err());
        assert!(
            ValueSchema::bounded_number(Some(f64::NAN), None)
                .validate_definition()
                .is_err()
        );
        ValueSchema::positive_number()
            .validate(&Dynamic::from(0.5_f64))
            .unwrap();
        assert!(
            ValueSchema::positive_number()
                .validate(&Dynamic::from(0_i64))
                .is_err()
        );
    }

    #[test]
    fn length_ui_value_and_asset_are_distinct() {
        ValueSchema::Length
            .validate(&Dynamic::from(Length::pixels(12.0).unwrap()))
            .unwrap();
        ValueSchema::UiValue
            .validate(&Dynamic::from_map(Map::from_iter([(
                "nested".into(),
                Dynamic::from_array(vec![Dynamic::from(1_i64)]),
            )])))
            .unwrap();
        ValueSchema::Asset
            .validate(&Dynamic::from(AssetId::parse("app/check").unwrap()))
            .unwrap();
        assert!(
            ValueSchema::UiValue
                .validate(&Dynamic::from(UiNode::text("not data")))
                .is_err()
        );
    }
}
