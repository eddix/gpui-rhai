use std::collections::BTreeMap;

use rhai::{CustomType, Dynamic, EvalAltResult, FLOAT, INT, ImmutableString, TypeBuilder};
use thiserror::Error;

use crate::{ColorValue, ComponentInstancePath};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SignalKind {
    Bool,
    Integer,
    Float,
    OptionalFloat,
    String,
    Color,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SignalProperty {
    Opacity,
    TranslateX,
    TranslateY,
    Width,
    WidthOverride,
    Height,
    Background,
    TextColor,
    BorderColor,
}

impl SignalProperty {
    /// Parse one approved property name.
    ///
    /// # Errors
    ///
    /// Returns [`SignalError::UnsupportedProperty`] for properties that cannot
    /// use the native hot lane.
    pub fn parse(value: &str) -> Result<Self, SignalError> {
        match value {
            "opacity" => Ok(Self::Opacity),
            "translate_x" => Ok(Self::TranslateX),
            "translate_y" => Ok(Self::TranslateY),
            "width" => Ok(Self::Width),
            "width_override" => Ok(Self::WidthOverride),
            "height" => Ok(Self::Height),
            "background" => Ok(Self::Background),
            "text_color" => Ok(Self::TextColor),
            "border_color" => Ok(Self::BorderColor),
            _ => Err(SignalError::UnsupportedProperty(value.to_owned())),
        }
    }

    #[must_use]
    pub const fn signal_kind(self) -> SignalKind {
        match self {
            Self::Opacity | Self::TranslateX | Self::TranslateY | Self::Width | Self::Height => {
                SignalKind::Float
            }
            Self::WidthOverride => SignalKind::OptionalFloat,
            Self::Background | Self::TextColor | Self::BorderColor => SignalKind::Color,
        }
    }
}

impl SignalKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::OptionalFloat => "optional_float",
            Self::String => "string",
            Self::Color => "color",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SignalValue {
    Bool(bool),
    Integer(INT),
    Float(FLOAT),
    OptionalFloat(Option<FLOAT>),
    String(String),
    Color(ColorValue),
}

impl SignalValue {
    #[must_use]
    pub const fn kind(&self) -> SignalKind {
        match self {
            Self::Bool(_) => SignalKind::Bool,
            Self::Integer(_) => SignalKind::Integer,
            Self::Float(_) => SignalKind::Float,
            Self::OptionalFloat(_) => SignalKind::OptionalFloat,
            Self::String(_) => SignalKind::String,
            Self::Color(_) => SignalKind::Color,
        }
    }

    fn validate(&self) -> Result<(), SignalError> {
        match self {
            Self::Float(value) if !value.is_finite() => Err(SignalError::NonFiniteFloat),
            Self::OptionalFloat(Some(value)) if !value.is_finite() => {
                Err(SignalError::NonFiniteFloat)
            }
            _ => Ok(()),
        }
    }

    /// Convert a Rhai scalar into a typed hot value.
    ///
    /// # Errors
    ///
    /// Returns [`SignalError::UnsupportedValue`] for structural/custom values
    /// and [`SignalError::NonFiniteFloat`] for NaN or infinity.
    pub fn from_dynamic(value: Dynamic) -> Result<Self, SignalError> {
        if value.is::<bool>() {
            return Ok(Self::Bool(value.cast::<bool>()));
        }
        if value.is::<INT>() {
            return Ok(Self::Integer(value.cast::<INT>()));
        }
        if value.is::<FLOAT>() {
            let value = value.cast::<FLOAT>();
            return value
                .is_finite()
                .then_some(Self::Float(value))
                .ok_or(SignalError::NonFiniteFloat);
        }
        if value.is::<ImmutableString>() {
            return Ok(Self::String(value.cast::<ImmutableString>().to_string()));
        }
        if value.is::<ColorValue>() {
            return Ok(Self::Color(value.cast::<ColorValue>()));
        }
        Err(SignalError::UnsupportedValue(value.type_name().to_owned()))
    }

    /// Convert a Rhai value for an already typed signal.
    ///
    /// Optional-float signals accept either null or a finite float/number;
    /// other signal kinds retain the ordinary scalar conversion contract.
    ///
    /// # Errors
    ///
    /// Returns a type or finite-number error when the value cannot satisfy the
    /// signal's declared kind.
    pub fn from_dynamic_for_kind(value: Dynamic, kind: SignalKind) -> Result<Self, SignalError> {
        if kind == SignalKind::OptionalFloat {
            if value.is_unit() {
                return Ok(Self::OptionalFloat(None));
            }
            if value.is::<FLOAT>() {
                let value = value.cast::<FLOAT>();
                return value
                    .is_finite()
                    .then_some(Self::OptionalFloat(Some(value)))
                    .ok_or(SignalError::NonFiniteFloat);
            }
            if value.is::<INT>() {
                let value = value
                    .cast::<INT>()
                    .to_string()
                    .parse::<FLOAT>()
                    .map_err(|_| {
                        SignalError::UnsupportedValue("integer outside float range".to_owned())
                    })?;
                return Ok(Self::OptionalFloat(Some(value)));
            }
        }
        Self::from_dynamic(value)
    }

    pub fn into_dynamic(self) -> Dynamic {
        match self {
            Self::Bool(value) => Dynamic::from_bool(value),
            Self::Integer(value) => Dynamic::from_int(value),
            Self::Float(value) | Self::OptionalFloat(Some(value)) => Dynamic::from_float(value),
            Self::OptionalFloat(None) => Dynamic::UNIT,
            Self::String(value) => Dynamic::from(value),
            Self::Color(value) => Dynamic::from(value),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SignalId {
    component: ComponentInstancePath,
    incarnation: crate::ComponentIncarnation,
    key: String,
    kind: SignalKind,
}

impl SignalId {
    /// Construct a stable component/key/type identity.
    ///
    /// # Errors
    ///
    /// Returns [`SignalError::InvalidKey`] for unsafe keys.
    pub fn new(
        component: ComponentInstancePath,
        key: impl Into<String>,
        kind: SignalKind,
    ) -> Result<Self, SignalError> {
        let key = key.into();
        if key.is_empty()
            || key.len() > 128
            || !key.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
            })
        {
            return Err(SignalError::InvalidKey(key));
        }
        Ok(Self {
            component,
            incarnation: crate::ComponentIncarnation::unscoped(),
            key,
            kind,
        })
    }

    pub(crate) fn new_scoped(
        component: ComponentInstancePath,
        incarnation: crate::ComponentIncarnation,
        key: impl Into<String>,
        kind: SignalKind,
    ) -> Result<Self, SignalError> {
        let mut id = Self::new(component, key, kind)?;
        id.incarnation = incarnation;
        Ok(id)
    }

    #[must_use]
    pub const fn component(&self) -> &ComponentInstancePath {
        &self.component
    }

    #[must_use]
    pub const fn incarnation(&self) -> crate::ComponentIncarnation {
        self.incarnation
    }

    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    #[must_use]
    pub const fn kind(&self) -> SignalKind {
        self.kind
    }
}

/// Non-owning script/Rust reference to a runtime-managed native signal.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NativeSignal {
    id: SignalId,
}

impl NativeSignal {
    #[must_use]
    pub const fn new(id: SignalId) -> Self {
        Self { id }
    }

    #[must_use]
    pub const fn id(&self) -> &SignalId {
        &self.id
    }
}

impl CustomType for NativeSignal {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("NativeSignal")
            .with_get("key", |signal: &mut Self| signal.id.key.clone())
            .with_get("kind", |signal: &mut Self| {
                ImmutableString::from(signal.id.kind.as_str())
            });
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SignalDescriptor {
    initial: SignalValue,
}

impl SignalDescriptor {
    #[must_use]
    pub const fn new(initial: SignalValue) -> Self {
        Self { initial }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SignalRecord {
    value: SignalValue,
    revision: u64,
    last_writer: SignalWriter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalWriter {
    Declaration,
    Script,
    Host,
    Primitive,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignalSnapshot {
    pub id: SignalId,
    pub value: SignalValue,
    pub revision: u64,
    pub last_writer: SignalWriter,
}

#[derive(Clone, Debug, Default)]
pub struct SignalRegistry {
    active: BTreeMap<SignalId, SignalRecord>,
}

impl SignalRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.active.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    /// # Errors
    ///
    /// Returns [`SignalError::Stale`] after the owning component unmounts.
    pub fn read(&self, signal: &NativeSignal) -> Result<SignalValue, SignalError> {
        self.active
            .get(signal.id())
            .map(|record| record.value.clone())
            .ok_or_else(|| SignalError::Stale(signal.id().clone()))
    }

    /// # Errors
    ///
    /// Returns [`SignalError::Stale`] after the owning component unmounts.
    pub fn revision(&self, signal: &NativeSignal) -> Result<u64, SignalError> {
        self.active
            .get(signal.id())
            .map(|record| record.revision)
            .ok_or_else(|| SignalError::Stale(signal.id().clone()))
    }

    /// Resolve a component-local key into a non-owning handle.
    ///
    /// # Errors
    ///
    /// Returns [`SignalError::UnknownKey`] when no compatible signal is mounted.
    pub fn resolve(
        &self,
        component: &ComponentInstancePath,
        key: &str,
    ) -> Result<NativeSignal, SignalError> {
        self.active
            .keys()
            .find(|id| id.component() == component && id.key() == key)
            .cloned()
            .map(NativeSignal::new)
            .ok_or_else(|| SignalError::UnknownKey {
                component: component.clone(),
                key: key.to_owned(),
            })
    }

    /// Write a type-compatible value without invalidating a formal component.
    ///
    /// # Errors
    ///
    /// Returns stale/type errors without changing the current value.
    pub fn write(
        &mut self,
        signal: &NativeSignal,
        value: SignalValue,
    ) -> Result<bool, SignalError> {
        self.write_from(signal, value, SignalWriter::Host)
    }

    pub(crate) fn write_from(
        &mut self,
        signal: &NativeSignal,
        value: SignalValue,
        writer: SignalWriter,
    ) -> Result<bool, SignalError> {
        value.validate()?;
        if signal.id.kind != value.kind() {
            return Err(SignalError::TypeMismatch {
                expected: signal.id.kind,
                actual: value.kind(),
            });
        }
        let record = self
            .active
            .get_mut(signal.id())
            .ok_or_else(|| SignalError::Stale(signal.id().clone()))?;
        if record.value == value {
            return Ok(false);
        }
        record.value = value;
        record.revision = record.revision.saturating_add(1);
        record.last_writer = writer;
        Ok(true)
    }

    #[must_use]
    pub fn inspect(&self) -> Vec<SignalSnapshot> {
        self.active
            .iter()
            .map(|(id, record)| SignalSnapshot {
                id: id.clone(),
                value: record.value.clone(),
                revision: record.revision,
                last_writer: record.last_writer,
            })
            .collect()
    }

    pub(crate) fn reconcile(
        &mut self,
        root: &ComponentInstancePath,
        candidate: BTreeMap<SignalId, SignalDescriptor>,
    ) {
        self.active
            .retain(|id, _| !id.component.is_within(root) || candidate.contains_key(id));
        for (id, descriptor) in candidate {
            self.active.entry(id).or_insert(SignalRecord {
                value: descriptor.initial,
                revision: 0,
                last_writer: SignalWriter::Declaration,
            });
        }
    }

    pub(crate) fn remove_scope(&mut self, root: &ComponentInstancePath) {
        self.active.retain(|id, _| !id.component.is_within(root));
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum SignalError {
    #[error("signal key `{0}` must be 1-128 ASCII letters, digits, `_`, `-`, `.`, or `:`")]
    InvalidKey(String),
    #[error(
        "signal values support only bool, integer, finite float, string, and ColorValue; got {0}"
    )]
    UnsupportedValue(String),
    #[error("signal float must be finite")]
    NonFiniteFloat,
    #[error("signal type mismatch: expected {expected:?}, got {actual:?}")]
    TypeMismatch {
        expected: SignalKind,
        actual: SignalKind,
    },
    #[error("signal property `{0}` is not an approved native hot property")]
    UnsupportedProperty(String),
    #[error("signal property `{property:?}` requires {expected:?}, got {actual:?}")]
    InvalidBinding {
        property: SignalProperty,
        expected: SignalKind,
        actual: SignalKind,
    },
    #[error("native signal `{0:?}` is stale or unmounted")]
    Stale(SignalId),
    #[error("component `{component}` has no mounted native signal `{key}`")]
    UnknownKey {
        component: ComponentInstancePath,
        key: String,
    },
    #[error("signal `{key}` is declared more than once by component `{component}`")]
    Duplicate {
        component: ComponentInstancePath,
        key: String,
    },
}

pub(crate) fn signal_runtime_error(error: &dyn std::fmt::Display) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(error.to_string().into(), rhai::Position::NONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconcile_preserves_value_and_revision_for_compatible_identity() {
        let component = ComponentInstancePath::root("Meter", "main");
        let id = SignalId::new(component.clone(), "progress", SignalKind::Float).unwrap();
        let signal = NativeSignal::new(id.clone());
        let mut registry = SignalRegistry::new();
        registry.reconcile(
            &component,
            BTreeMap::from([(id.clone(), SignalDescriptor::new(SignalValue::Float(0.0)))]),
        );
        assert!(registry.write(&signal, SignalValue::Float(0.5)).unwrap());
        registry.reconcile(
            &component,
            BTreeMap::from([(id, SignalDescriptor::new(SignalValue::Float(0.0)))]),
        );
        assert_eq!(registry.read(&signal).unwrap(), SignalValue::Float(0.5));
        assert_eq!(registry.revision(&signal).unwrap(), 1);
        assert_eq!(registry.inspect()[0].last_writer, SignalWriter::Host);
    }

    #[test]
    fn stale_and_type_mismatched_writes_are_atomic() {
        let component = ComponentInstancePath::root("Meter", "main");
        let signal = NativeSignal::new(
            SignalId::new(component.clone(), "progress", SignalKind::Float).unwrap(),
        );
        let mut registry = SignalRegistry::new();
        assert!(matches!(
            registry.write(&signal, SignalValue::Float(1.0)),
            Err(SignalError::Stale(_))
        ));
        registry.reconcile(
            &component,
            BTreeMap::from([(
                signal.id().clone(),
                SignalDescriptor::new(SignalValue::Float(0.0)),
            )]),
        );
        assert!(matches!(
            registry.write(&signal, SignalValue::Integer(1)),
            Err(SignalError::TypeMismatch { .. })
        ));
        assert_eq!(registry.read(&signal).unwrap(), SignalValue::Float(0.0));
        assert_eq!(
            registry.write(&signal, SignalValue::Float(f64::NAN)),
            Err(SignalError::NonFiniteFloat)
        );
        assert_eq!(registry.read(&signal).unwrap(), SignalValue::Float(0.0));
    }

    #[test]
    fn optional_float_signal_supports_an_inactive_width_override() {
        let component = ComponentInstancePath::root("Table", "users");
        let id = SignalId::new(
            component.clone(),
            "column-width-0",
            SignalKind::OptionalFloat,
        )
        .unwrap();
        let signal = NativeSignal::new(id.clone());
        let mut registry = SignalRegistry::new();
        registry.reconcile(
            &component,
            BTreeMap::from([(id, SignalDescriptor::new(SignalValue::OptionalFloat(None)))]),
        );
        assert!(registry.read(&signal).unwrap().into_dynamic().is_unit());
        assert!(
            registry
                .write_from(
                    &signal,
                    SignalValue::from_dynamic_for_kind(
                        Dynamic::from_float(144.0),
                        SignalKind::OptionalFloat,
                    )
                    .unwrap(),
                    SignalWriter::Primitive,
                )
                .unwrap()
        );
        assert_eq!(
            registry.read(&signal).unwrap(),
            SignalValue::OptionalFloat(Some(144.0))
        );
        assert_eq!(registry.inspect()[0].last_writer, SignalWriter::Primitive);
        assert_eq!(
            SignalValue::from_dynamic_for_kind(Dynamic::UNIT, SignalKind::OptionalFloat).unwrap(),
            SignalValue::OptionalFloat(None)
        );
    }
}
