use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use gpui::{App, Window};
use rhai::{CustomType, ImmutableString, TypeBuilder};
use thiserror::Error;

use crate::{EventResponse, UiRuntimeState, UiValue, ValueSchema};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NativeHandlerId(String);

impl NativeHandlerId {
    /// Parse a namespaced `snake_case` handler identifier.
    ///
    /// # Errors
    ///
    /// Returns [`NativeHandlerError::InvalidId`] for unsafe identifiers.
    pub fn parse(value: impl Into<String>) -> Result<Self, NativeHandlerError> {
        let value = value.into();
        let valid = value
            .split_once('.')
            .is_some_and(|(namespace, name)| valid_segment(namespace) && valid_segment(name));
        valid
            .then_some(Self(value.clone()))
            .ok_or(NativeHandlerError::InvalidId(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NativeHandlerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

fn valid_segment(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeHandlerDescriptor {
    pub id: NativeHandlerId,
    pub events: BTreeMap<String, ValueSchema>,
}

impl NativeHandlerDescriptor {
    /// Construct a schema-checked native handler descriptor.
    ///
    /// # Errors
    ///
    /// Returns for empty event sets, unsafe names, or invalid payload schemas.
    pub fn new(
        id: NativeHandlerId,
        events: BTreeMap<String, ValueSchema>,
    ) -> Result<Self, NativeHandlerError> {
        if events.is_empty() {
            return Err(NativeHandlerError::MissingEvents(id));
        }
        for (event, schema) in &events {
            if !valid_segment(event) {
                return Err(NativeHandlerError::InvalidEvent(event.clone()));
            }
            schema
                .validate_definition()
                .map_err(|source| NativeHandlerError::InvalidSchema {
                    event: event.clone(),
                    source,
                })?;
        }
        Ok(Self { id, events })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeHandlerRef {
    descriptor: NativeHandlerDescriptor,
}

impl NativeHandlerRef {
    #[must_use]
    pub const fn descriptor(&self) -> &NativeHandlerDescriptor {
        &self.descriptor
    }

    /// Validate that this ref may receive the named normalized event.
    ///
    /// # Errors
    ///
    /// Returns [`NativeHandlerError::UnsupportedEvent`] when not declared.
    pub fn validate_event(&self, event: &str) -> Result<(), NativeHandlerError> {
        self.descriptor
            .events
            .contains_key(event)
            .then_some(())
            .ok_or_else(|| NativeHandlerError::UnsupportedEvent {
                handler: self.descriptor.id.clone(),
                event: event.to_owned(),
            })
    }
}

impl CustomType for NativeHandlerRef {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("NativeHandlerRef")
            .with_get("id", |reference: &mut Self| {
                ImmutableString::from(reference.descriptor.id.as_str())
            });
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeEvent {
    pub name: String,
    pub payload: UiValue,
    /// Event-time visual bounds of the retained node owning the handler.
    pub target: Option<crate::GeometryBounds>,
}

type NativeHandlerFn = dyn FnMut(
    NativeEvent,
    &mut UiRuntimeState,
    &mut Window,
    &mut App,
) -> Result<EventResponse, String>;

#[derive(Clone)]
struct RegisteredNativeHandler {
    descriptor: NativeHandlerDescriptor,
    handler: Rc<RefCell<NativeHandlerFn>>,
}

impl fmt::Debug for RegisteredNativeHandler {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegisteredNativeHandler")
            .field("descriptor", &self.descriptor)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Default)]
pub struct NativeHandlerRegistry {
    handlers: Rc<RefCell<BTreeMap<NativeHandlerId, RegisteredNativeHandler>>>,
}

impl NativeHandlerRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one trusted foreground Rust handler.
    ///
    /// # Errors
    ///
    /// Returns [`NativeHandlerError::Duplicate`] for an existing ID.
    pub fn register(
        &self,
        descriptor: NativeHandlerDescriptor,
        handler: impl FnMut(
            NativeEvent,
            &mut UiRuntimeState,
            &mut Window,
            &mut App,
        ) -> Result<EventResponse, String>
        + 'static,
    ) -> Result<(), NativeHandlerError> {
        let id = descriptor.id.clone();
        let mut handlers = self
            .handlers
            .try_borrow_mut()
            .map_err(|_| NativeHandlerError::Borrowed)?;
        if handlers.contains_key(&id) {
            return Err(NativeHandlerError::Duplicate(id));
        }
        handlers.insert(
            id,
            RegisteredNativeHandler {
                descriptor,
                handler: Rc::new(RefCell::new(handler)),
            },
        );
        Ok(())
    }

    /// Resolve a Rhai-attachable, non-owning reference.
    ///
    /// # Errors
    ///
    /// Returns [`NativeHandlerError::Missing`] for unregistered IDs.
    pub fn resolve(&self, id: &NativeHandlerId) -> Result<NativeHandlerRef, NativeHandlerError> {
        self.handlers
            .try_borrow()
            .map_err(|_| NativeHandlerError::Borrowed)?
            .get(id)
            .map(|registered| NativeHandlerRef {
                descriptor: registered.descriptor.clone(),
            })
            .ok_or_else(|| NativeHandlerError::Missing(id.clone()))
    }

    /// Invoke one handler after validating its event payload schema.
    ///
    /// # Errors
    ///
    /// Returns schema, stale-reference, borrow, or handler errors.
    pub fn invoke(
        &self,
        reference: &NativeHandlerRef,
        event: NativeEvent,
        runtime: &mut UiRuntimeState,
        window: &mut Window,
        app: &mut App,
    ) -> Result<EventResponse, NativeHandlerError> {
        let registered = self
            .handlers
            .try_borrow()
            .map_err(|_| NativeHandlerError::Borrowed)?
            .get(&reference.descriptor.id)
            .cloned()
            .ok_or_else(|| NativeHandlerError::Missing(reference.descriptor.id.clone()))?;
        if registered.descriptor != reference.descriptor {
            return Err(NativeHandlerError::Stale(reference.descriptor.id.clone()));
        }
        let schema = registered
            .descriptor
            .events
            .get(&event.name)
            .ok_or_else(|| NativeHandlerError::UnsupportedEvent {
                handler: registered.descriptor.id.clone(),
                event: event.name.clone(),
            })?;
        schema
            .validate(&event.payload.clone().into_dynamic())
            .map_err(|source| NativeHandlerError::InvalidPayload {
                event: event.name.clone(),
                source,
            })?;
        registered
            .handler
            .try_borrow_mut()
            .map_err(|_| NativeHandlerError::Borrowed)?(event, runtime, window, app)
        .map_err(NativeHandlerError::Handler)
    }
}

#[derive(Debug, Error)]
pub enum NativeHandlerError {
    #[error("native handler ID `{0}` must be namespaced snake_case")]
    InvalidId(String),
    #[error("native handler `{0}` must declare at least one event")]
    MissingEvents(NativeHandlerId),
    #[error("native handler event `{0}` must be snake_case")]
    InvalidEvent(String),
    #[error("native handler schema for `{event}` is invalid: {source}")]
    InvalidSchema {
        event: String,
        source: crate::SchemaDefinitionError,
    },
    #[error("native handler `{handler}` does not accept event `{event}`")]
    UnsupportedEvent {
        handler: NativeHandlerId,
        event: String,
    },
    #[error("native handler `{0}` is already registered")]
    Duplicate(NativeHandlerId),
    #[error("native handler `{0}` is not registered")]
    Missing(NativeHandlerId),
    #[error("native handler ref `{0}` no longer matches its registration")]
    Stale(NativeHandlerId),
    #[error("native handler payload for `{event}` is invalid: {source}")]
    InvalidPayload {
        event: String,
        source: crate::SchemaValidationError,
    },
    #[error("native handler registry or handler is already borrowed")]
    Borrowed,
    #[error("native handler failed: {0}")]
    Handler(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_rejects_unsafe_ids_and_events() {
        assert!(NativeHandlerId::parse("drag").is_err());
        let id = NativeHandlerId::parse("timeline.drag").unwrap();
        assert!(NativeHandlerDescriptor::new(id, BTreeMap::new()).is_err());
    }
}
