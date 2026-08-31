use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::{Rc, Weak};

use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, Window, div};
use rhai::{Array, Dynamic, FnPtr, Map};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    AssetId, ColorResolver, ColorValue, ComponentStateSchema, EventSchema, Length,
    NodeEventDispatcher, ObjectField, RadiusToken, Rgba8, SchemaDefinitionError,
    SchemaValidationError, ScriptCallback, ScriptGeneration, SpacingToken, Style, UiEventHandler,
    UiNode, UiValue, UiValueError, ValueSchema,
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PrimitiveId(String);

impl PrimitiveId {
    /// Parse a namespaced primitive ID such as `my_app.code_editor`.
    ///
    /// # Errors
    ///
    /// Returns [`PrimitiveError::InvalidId`] for invalid identifiers.
    pub fn parse(value: impl Into<String>) -> Result<Self, PrimitiveError> {
        let value = value.into();
        if value
            .split_once('.')
            .is_some_and(|(namespace, name)| is_identifier(namespace) && is_identifier(name))
        {
            Ok(Self(value))
        } else {
            Err(PrimitiveError::InvalidId(value))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn namespace(&self) -> &str {
        self.0
            .split_once('.')
            .map_or("", |(namespace, _)| namespace)
    }
}

impl TryFrom<String> for PrimitiveId {
    type Error = PrimitiveError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<PrimitiveId> for String {
    fn from(value: PrimitiveId) -> Self {
        value.0
    }
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn is_pascal_case(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrimitiveDescriptor {
    pub id: PrimitiveId,
    pub export: String,
    #[serde(default)]
    pub props: BTreeMap<String, ObjectField>,
    #[serde(default)]
    pub events: BTreeMap<String, EventSchema>,
    #[serde(default)]
    pub state: ComponentStateSchema,
    #[serde(default)]
    pub lifecycle: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrimitiveValue {
    Data(UiValue),
    Node(Box<UiNode>),
    Nodes(Vec<UiNode>),
    Callback(UiEventHandler),
    Style(Box<Style>),
    Length(Length),
    Asset(AssetId),
}

/// Read-only semantic theme values captured for one native primitive render.
///
/// Primitive handlers use this snapshot to resolve component-owned paint parts
/// without receiving a mutable application/theme manager or coupling to a
/// concrete theme family.
#[derive(Clone, Debug, Default)]
pub struct PrimitiveTheme {
    colors: BTreeMap<String, Rgba8>,
    spacing: BTreeMap<SpacingToken, Length>,
    radii: BTreeMap<RadiusToken, Length>,
}

impl PrimitiveTheme {
    pub(crate) fn capture(colors: &impl ColorResolver) -> Self {
        const TOKENS: &[&str] = &[
            "surface",
            "surface_raised",
            "surface_hover",
            "text_primary",
            "text_muted",
            "accent",
            "accent_hover",
            "on_accent",
            "danger",
            "on_danger",
            "warning",
            "on_warning",
            "success",
            "on_success",
            "border",
            "focus_ring",
            "disabled",
        ];
        Self {
            colors: TOKENS
                .iter()
                .filter_map(|token| {
                    colors
                        .resolve(&ColorValue::Token((*token).to_owned()))
                        .map(|value| ((*token).to_owned(), value))
                })
                .collect(),
            spacing: [
                SpacingToken::Xs,
                SpacingToken::Sm,
                SpacingToken::Md,
                SpacingToken::Lg,
            ]
            .into_iter()
            .filter_map(|token| {
                colors
                    .resolve_length(Length::ThemeSpacing(token))
                    .map(|value| (token, value))
            })
            .collect(),
            radii: [RadiusToken::Sm, RadiusToken::Md, RadiusToken::Lg]
                .into_iter()
                .filter_map(|token| {
                    colors
                        .resolve_length(Length::ThemeRadius(token))
                        .map(|value| (token, value))
                })
                .collect(),
        }
    }

    #[must_use]
    pub fn color(&self, token: &str) -> Option<Rgba8> {
        self.colors.get(token).copied()
    }

    #[must_use]
    pub fn resolve_color(&self, value: &ColorValue) -> Option<Rgba8> {
        match value {
            ColorValue::Literal(value) => Some(*value),
            ColorValue::Token(token) => self.color(token),
        }
    }

    #[must_use]
    pub fn resolve_length(&self, value: Length) -> Option<Length> {
        match value {
            Length::ThemeSpacing(token) => self.spacing.get(&token).copied(),
            Length::ThemeRadius(token) => self.radii.get(&token).copied(),
            Length::Pixels(_) | Length::Rems(_) | Length::Relative(_) => Some(value),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrimitiveProps(BTreeMap<String, PrimitiveValue>);

impl PrimitiveProps {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&PrimitiveValue> {
        self.0.get(name)
    }

    pub fn insert(
        &mut self,
        name: impl Into<String>,
        value: PrimitiveValue,
    ) -> Option<PrimitiveValue> {
        self.0.insert(name.into(), value)
    }

    #[must_use]
    pub fn with(mut self, name: impl Into<String>, value: PrimitiveValue) -> Self {
        self.insert(name, value);
        self
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &PrimitiveValue)> {
        self.0.iter().map(|(name, value)| (name.as_str(), value))
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut PrimitiveValue)> {
        self.0
            .iter_mut()
            .map(|(name, value)| (name.as_str(), value))
    }

    pub(crate) fn bind_component_scope(
        &mut self,
        component: &crate::ComponentInstancePath,
        events: &BTreeMap<String, EventSchema>,
        native_context: Option<&crate::invocation::ScriptInvocationContext>,
    ) {
        for value in self.0.values_mut() {
            match value {
                PrimitiveValue::Callback(callback) => {
                    if let Some(callback) = callback.as_script_mut() {
                        callback.bind_component_if_unset(component.clone(), events.clone());
                        if let Some(context) = native_context {
                            callback.bind_native_context_if_unset(context.clone());
                        }
                    }
                }
                PrimitiveValue::Node(node) => {
                    node.bind_component_scope(component, events, native_context);
                }
                PrimitiveValue::Nodes(nodes) => {
                    for node in nodes {
                        node.bind_component_scope(component, events, native_context);
                    }
                }
                PrimitiveValue::Data(_)
                | PrimitiveValue::Style(_)
                | PrimitiveValue::Length(_)
                | PrimitiveValue::Asset(_) => {}
            }
        }
    }

    pub(crate) fn bind_callback_scope_by_name(
        &mut self,
        names: &BTreeSet<String>,
        component: &crate::ComponentInstancePath,
        events: &BTreeMap<String, EventSchema>,
        native_context: Option<&crate::invocation::ScriptInvocationContext>,
    ) {
        for value in self.0.values_mut() {
            match value {
                PrimitiveValue::Callback(handler)
                    if handler
                        .as_script()
                        .is_some_and(|callback| names.contains(callback.name())) =>
                {
                    let callback = handler
                        .as_script_mut()
                        .expect("matched script callback handler");
                    callback.bind_component_if_unset(component.clone(), events.clone());
                    if let (Some(context), None) = (native_context, callback.native_context()) {
                        callback.bind_native_context_if_unset(context.clone());
                    }
                }
                PrimitiveValue::Node(node) => {
                    node.bind_callback_scope_by_name(names, component, events, native_context);
                }
                PrimitiveValue::Nodes(nodes) => {
                    for node in nodes {
                        node.bind_callback_scope_by_name(names, component, events, native_context);
                    }
                }
                PrimitiveValue::Data(_)
                | PrimitiveValue::Style(_)
                | PrimitiveValue::Length(_)
                | PrimitiveValue::Asset(_)
                | PrimitiveValue::Callback(_) => {}
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PrimitiveInstanceId {
    primitive: PrimitiveId,
    key: String,
    node: crate::NodeId,
}

impl PrimitiveInstanceId {
    #[must_use]
    pub const fn primitive(&self) -> &PrimitiveId {
        &self.primitive
    }

    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    #[must_use]
    pub const fn node(&self) -> crate::NodeId {
        self.node
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveNode {
    pub primitive: PrimitiveId,
    pub key: Option<String>,
    pub props: PrimitiveProps,
}

#[derive(Clone, Debug)]
pub struct PrimitiveInstance {
    pub id: Option<PrimitiveInstanceId>,
    pub node: PrimitiveNode,
    resources: Option<PrimitiveResourceScope>,
}

impl PrimitiveInstance {
    /// Return the runtime-owned cancellation scope for a retained instance.
    /// Ephemeral primitives have no resource scope and must not start durable
    /// work from render.
    #[must_use]
    pub const fn resources(&self) -> Option<&PrimitiveResourceScope> {
        self.resources.as_ref()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PrimitiveResourceHandle(u64);

struct PrimitiveResourceEntry {
    label: String,
    cleanup: Option<Box<dyn FnOnce()>>,
}

#[derive(Default)]
struct PrimitiveResourceState {
    next_id: u64,
    entries: BTreeMap<u64, PrimitiveResourceEntry>,
}

impl Drop for PrimitiveResourceState {
    fn drop(&mut self) {
        let entries = std::mem::take(&mut self.entries);
        for entry in entries.into_values().rev() {
            let _ = run_resource_cleanup(entry);
        }
    }
}

#[derive(Clone, Default)]
pub struct PrimitiveResourceScope {
    inner: Rc<RefCell<PrimitiveResourceState>>,
}

impl fmt::Debug for PrimitiveResourceScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner.try_borrow() {
            Ok(state) => formatter
                .debug_struct("PrimitiveResourceScope")
                .field(
                    "active",
                    &state
                        .entries
                        .values()
                        .map(|entry| entry.label.as_str())
                        .collect::<Vec<_>>(),
                )
                .finish(),
            Err(_) => formatter.write_str("PrimitiveResourceScope(<borrowed>)"),
        }
    }
}

impl PrimitiveResourceScope {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Own one native task/subscription/resource cancellation callback.
    ///
    /// # Errors
    ///
    /// Returns [`PrimitiveResourceError`] for an unsafe label or conflicting
    /// scope borrow.
    pub fn own(
        &self,
        label: impl Into<String>,
        cleanup: impl FnOnce() + 'static,
    ) -> Result<PrimitiveResourceHandle, PrimitiveResourceError> {
        let label = label.into();
        if label.is_empty()
            || label.len() > 128
            || !label.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
            })
        {
            return Err(PrimitiveResourceError::InvalidLabel(label));
        }
        let mut state = self
            .inner
            .try_borrow_mut()
            .map_err(|_| PrimitiveResourceError::Borrowed)?;
        let id = state.next_id.max(1);
        state.next_id = id
            .checked_add(1)
            .ok_or(PrimitiveResourceError::IdExhausted)?;
        state.entries.insert(
            id,
            PrimitiveResourceEntry {
                label,
                cleanup: Some(Box::new(cleanup)),
            },
        );
        Ok(PrimitiveResourceHandle(id))
    }

    /// Cancel one owned resource early.
    ///
    /// # Errors
    ///
    /// Returns a borrow or cleanup-panic diagnostic.
    pub fn cancel(&self, handle: &PrimitiveResourceHandle) -> Result<bool, PrimitiveResourceError> {
        let entry = self
            .inner
            .try_borrow_mut()
            .map_err(|_| PrimitiveResourceError::Borrowed)?
            .entries
            .remove(&handle.0);
        let Some(entry) = entry else {
            return Ok(false);
        };
        run_resource_cleanup(entry)?;
        Ok(true)
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.inner.borrow().entries.len()
    }

    fn checkpoint(&self) -> Result<u64, PrimitiveResourceError> {
        self.inner
            .try_borrow()
            .map(|state| state.next_id.max(1))
            .map_err(|_| PrimitiveResourceError::Borrowed)
    }

    fn rollback(&self, checkpoint: u64) -> Result<(), PrimitiveResourceError> {
        self.cleanup_where(|id| id >= checkpoint)
    }

    fn close(&self) -> Result<(), PrimitiveResourceError> {
        self.cleanup_where(|_| true)
    }

    fn cleanup_where(&self, predicate: impl Fn(u64) -> bool) -> Result<(), PrimitiveResourceError> {
        let mut entries = {
            let mut state = self
                .inner
                .try_borrow_mut()
                .map_err(|_| PrimitiveResourceError::Borrowed)?;
            let ids = state
                .entries
                .keys()
                .copied()
                .filter(|id| predicate(*id))
                .collect::<Vec<_>>();
            ids.into_iter()
                .rev()
                .filter_map(|id| state.entries.remove(&id))
                .collect::<Vec<_>>()
        };
        let mut first_error = None;
        for entry in entries.drain(..) {
            if let Err(error) = run_resource_cleanup(entry)
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

fn run_resource_cleanup(mut entry: PrimitiveResourceEntry) -> Result<(), PrimitiveResourceError> {
    let label = entry.label;
    let Some(cleanup) = entry.cleanup.take() else {
        return Ok(());
    };
    catch_unwind(AssertUnwindSafe(cleanup))
        .map_err(|_| PrimitiveResourceError::CleanupPanic { label })
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PrimitiveResourceError {
    #[error("primitive resource label `{0}` must be 1-128 safe ASCII characters")]
    InvalidLabel(String),
    #[error("primitive resource scope is already borrowed")]
    Borrowed,
    #[error("primitive resource scope exhausted its handle identity space")]
    IdExhausted,
    #[error("primitive resource cleanup `{label}` panicked")]
    CleanupPanic { label: String },
}

#[derive(Clone)]
pub struct PrimitiveEventEmitter {
    registry: Weak<RefCell<PrimitiveRegistryInner>>,
    primitive: PrimitiveId,
    callbacks: BTreeMap<String, UiEventHandler>,
    dispatcher: Option<NodeEventDispatcher>,
}

impl PrimitiveEventEmitter {
    /// Normalize and dispatch a declared native primitive event.
    ///
    /// # Errors
    ///
    /// Returns schema and event declaration errors. Missing callback props are
    /// treated as an intentionally unobserved event.
    pub fn emit(
        &self,
        event: &str,
        payload: UiValue,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<(), PrimitiveError> {
        let registry = PrimitiveRegistry {
            inner: self
                .registry
                .upgrade()
                .ok_or(PrimitiveError::RegistryReleased)?,
        };
        let payload = registry.normalize_event(&self.primitive, event, payload)?;
        if let Some(handler) = self.callbacks.get(event) {
            match handler {
                UiEventHandler::Script(callback) => {
                    if let Some(dispatcher) = self.dispatcher.as_ref() {
                        dispatcher.dispatch(callback.clone(), payload, window, cx);
                    }
                }
                UiEventHandler::Host(callback) => {
                    callback.invoke(payload, window, cx);
                }
                UiEventHandler::Native(reference) => {
                    if let Some(dispatcher) = self.dispatcher.as_ref() {
                        dispatcher.dispatch_native(
                            reference.clone(),
                            event.to_owned(),
                            payload,
                            window,
                            cx,
                        );
                    }
                }
            }
        }
        Ok(())
    }
}

pub trait PrimitiveHandler {
    /// Called once before the first render of a keyed lifecycle primitive.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic message when native setup fails.
    fn mount(&mut self, _instance: &PrimitiveInstance) -> Result<(), String> {
        Ok(())
    }

    /// Apply a validated prop/style/event snapshot to an existing keyed instance.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic message when the retained native update fails.
    fn update(
        &mut self,
        _previous: &PrimitiveInstance,
        _next: &PrimitiveInstance,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Render the primitive into a native GPUI element.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic message when native rendering fails.
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, String>;

    /// Called when a previously mounted keyed primitive is no longer reachable.
    fn unmount(&mut self, _instance: &PrimitiveInstanceId) {}
}

struct PrimitiveEntry {
    descriptor: PrimitiveDescriptor,
    handler: Box<dyn PrimitiveHandler>,
}

#[derive(Default)]
struct PrimitiveRegistryInner {
    entries: BTreeMap<PrimitiveId, PrimitiveEntry>,
    mounted: BTreeMap<PrimitiveInstanceId, PrimitiveInstance>,
}

#[derive(Clone, Default)]
pub struct PrimitiveRegistry {
    inner: Rc<RefCell<PrimitiveRegistryInner>>,
}

impl fmt::Debug for PrimitiveRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner.try_borrow() {
            Ok(inner) => formatter
                .debug_struct("PrimitiveRegistry")
                .field("registered", &inner.entries.keys().collect::<Vec<_>>())
                .field("mounted", &inner.mounted.keys().collect::<Vec<_>>())
                .finish(),
            Err(_) => formatter.write_str("PrimitiveRegistry(<borrowed>)"),
        }
    }
}

impl PrimitiveRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a validated primitive descriptor and native handler.
    ///
    /// # Errors
    ///
    /// Returns [`PrimitiveError`] for duplicate IDs, invalid exports/defaults,
    /// or missing callback props for declared events.
    pub fn register(
        &self,
        descriptor: PrimitiveDescriptor,
        handler: impl PrimitiveHandler + 'static,
    ) -> Result<(), PrimitiveError> {
        validate_descriptor(&descriptor)?;
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| PrimitiveError::Borrowed)?;
        if inner.entries.contains_key(&descriptor.id) {
            return Err(PrimitiveError::Duplicate(descriptor.id));
        }
        if inner.entries.values().any(|entry| {
            entry.descriptor.id.namespace() == descriptor.id.namespace()
                && entry.descriptor.export == descriptor.export
        }) {
            return Err(PrimitiveError::DuplicateExport {
                namespace: descriptor.id.namespace().to_owned(),
                export: descriptor.export,
            });
        }
        inner.entries.insert(
            descriptor.id.clone(),
            PrimitiveEntry {
                descriptor,
                handler: Box::new(handler),
            },
        );
        Ok(())
    }

    /// Convert validated Rhai props into a custom `UiNode`.
    ///
    /// # Errors
    ///
    /// Returns [`PrimitiveError`] for unknown primitives, invalid props, or a
    /// missing key on stateful/lifecycle instances.
    pub fn create_node(
        &self,
        id: &PrimitiveId,
        key: Option<String>,
        props: &Map,
        generation: ScriptGeneration,
    ) -> Result<UiNode, PrimitiveError> {
        let inner = self
            .inner
            .try_borrow()
            .map_err(|_| PrimitiveError::Borrowed)?;
        let descriptor = &inner
            .entries
            .get(id)
            .ok_or_else(|| PrimitiveError::Unknown(id.clone()))?
            .descriptor;
        if (descriptor.lifecycle || !descriptor.state.is_empty()) && key.is_none() {
            return Err(PrimitiveError::MissingKey(id.clone()));
        }
        let schema = ValueSchema::object(descriptor.props.clone());
        schema
            .validate(&Dynamic::from_map(props.clone()))
            .map_err(|source| PrimitiveError::InvalidProps {
                primitive: id.clone(),
                source,
            })?;
        let props = convert_props(&descriptor.props, props, generation)?;
        Ok(UiNode::custom(PrimitiveNode {
            primitive: id.clone(),
            key,
            props,
        }))
    }

    /// Validate an event emitted by a native primitive adapter.
    ///
    /// # Errors
    ///
    /// Returns [`PrimitiveError`] for unknown events or invalid payloads.
    pub fn normalize_event(
        &self,
        id: &PrimitiveId,
        event: &str,
        payload: UiValue,
    ) -> Result<UiValue, PrimitiveError> {
        let inner = self
            .inner
            .try_borrow()
            .map_err(|_| PrimitiveError::Borrowed)?;
        let descriptor = &inner
            .entries
            .get(id)
            .ok_or_else(|| PrimitiveError::Unknown(id.clone()))?
            .descriptor;
        let schema = descriptor
            .events
            .get(event)
            .ok_or_else(|| PrimitiveError::UnknownEvent {
                primitive: id.clone(),
                event: event.to_owned(),
            })?;
        schema
            .payload
            .validate(&payload.clone().into_dynamic())
            .map_err(|source| PrimitiveError::InvalidEvent {
                primitive: id.clone(),
                event: event.to_owned(),
                source,
            })?;
        Ok(payload)
    }

    /// Unmount keyed lifecycle instances absent from the successful node tree.
    ///
    /// # Errors
    ///
    /// Returns [`PrimitiveError::Borrowed`] during a conflicting render borrow.
    pub fn retain_mounted(
        &self,
        active: &BTreeSet<PrimitiveInstanceId>,
    ) -> Result<(), PrimitiveError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| PrimitiveError::Borrowed)?;
        let removed = inner
            .mounted
            .keys()
            .filter(|instance| !active.contains(*instance))
            .cloned()
            .collect::<Vec<_>>();
        let mut first_error = None;
        for instance in &removed {
            let resources = inner
                .mounted
                .get(instance)
                .and_then(|mounted| mounted.resources.clone());
            if let Some(entry) = inner.entries.get_mut(&instance.primitive)
                && let Err(error) = guard_primitive_panic(&instance.primitive, "unmount", || {
                    entry.handler.unmount(instance);
                })
                && first_error.is_none()
            {
                first_error = Some(error);
            }
            if let Some(resources) = resources
                && let Err(error) = resources.close()
                && first_error.is_none()
            {
                first_error = Some(PrimitiveError::Resource(error));
            }
            inner.mounted.remove(instance);
        }
        first_error.map_or(Ok(()), Err)
    }

    /// Unmount keyed primitive instances absent from the current successful tree.
    ///
    /// # Errors
    ///
    /// Returns borrow or panic-boundary errors from native unmount handlers.
    pub fn retain_tree(&self, tree: &crate::RetainedUiTree) -> Result<(), PrimitiveError> {
        let active = collect_primitive_instances(tree);
        self.retain_mounted(&active)
    }

    pub(crate) fn element(
        &self,
        node: PrimitiveNode,
        retained_id: Option<crate::NodeId>,
        fallback: Option<UiNode>,
        dispatcher: Option<NodeEventDispatcher>,
        theme: PrimitiveTheme,
    ) -> AnyElement {
        RegisteredPrimitiveElement {
            registry: self.clone(),
            node,
            retained_id,
            fallback,
            dispatcher,
            theme,
        }
        .into_any_element()
    }

    fn render_instance(
        &self,
        node: PrimitiveNode,
        retained_id: Option<crate::NodeId>,
        events: &PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, PrimitiveError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| PrimitiveError::Borrowed)?;
        let retained_instance = primitive_is_retained(&inner, &node.primitive)?;
        if retained_instance && retained_id.is_none() {
            return Err(PrimitiveError::MissingRetainedIdentity(node.primitive));
        }
        let instance_id = retained_instance.then(|| PrimitiveInstanceId {
            primitive: node.primitive.clone(),
            key: node
                .key
                .clone()
                .expect("retained primitive descriptors require a key"),
            node: retained_id.expect("retained primitive renderer supplies NodeId"),
        });
        let previous = instance_id
            .as_ref()
            .and_then(|id| inner.mounted.get(id))
            .cloned();
        let resources = if retained_instance {
            Some(
                previous
                    .as_ref()
                    .and_then(|instance| instance.resources.clone())
                    .unwrap_or_default(),
            )
        } else {
            None
        };
        let checkpoint = resources
            .as_ref()
            .map(PrimitiveResourceScope::checkpoint)
            .transpose()?;
        let instance = PrimitiveInstance {
            id: instance_id.clone(),
            node,
            resources: resources.clone(),
        };
        let needs_mount = instance_id.is_some() && previous.is_none();
        let entry = inner
            .entries
            .get_mut(&instance.node.primitive)
            .ok_or_else(|| PrimitiveError::Unknown(instance.node.primitive.clone()))?;
        let operation = (|| {
            if needs_mount {
                guard_primitive_panic(&instance.node.primitive, "mount", || {
                    entry.handler.mount(&instance)
                })?
                .map_err(|message| PrimitiveError::Handler {
                    primitive: instance.node.primitive.clone(),
                    message,
                })?;
            }
            if let Some(previous) = &previous
                && previous.node != instance.node
            {
                guard_primitive_panic(&instance.node.primitive, "update", || {
                    entry.handler.update(previous, &instance)
                })?
                .map_err(|message| PrimitiveError::Handler {
                    primitive: instance.node.primitive.clone(),
                    message,
                })?;
            }
            guard_primitive_panic(&instance.node.primitive, "render", || {
                entry.handler.render(&instance, events, theme, window, cx)
            })?
            .map_err(|message| PrimitiveError::Handler {
                primitive: instance.node.primitive.clone(),
                message,
            })
        })();
        let element = match operation {
            Ok(element) => element,
            Err(error) => {
                return Err(rollback_failed_primitive_operation(
                    entry,
                    &instance,
                    needs_mount,
                    resources.as_ref(),
                    checkpoint,
                    error,
                ));
            }
        };
        if let Some(id) = instance_id {
            inner.mounted.insert(id, instance);
        }
        Ok(element)
    }
}

fn rollback_failed_primitive_operation(
    entry: &mut PrimitiveEntry,
    instance: &PrimitiveInstance,
    needs_unmount: bool,
    resources: Option<&PrimitiveResourceScope>,
    checkpoint: Option<u64>,
    original: PrimitiveError,
) -> PrimitiveError {
    let mut rollback_error = None;
    if needs_unmount
        && let Some(instance_id) = instance.id.as_ref()
        && let Err(error) =
            guard_primitive_panic(&instance.node.primitive, "failed-mount unmount", || {
                entry.handler.unmount(instance_id);
            })
    {
        rollback_error = Some(error);
    }
    if let (Some(resources), Some(checkpoint)) = (resources, checkpoint)
        && let Err(error) = resources.rollback(checkpoint)
        && rollback_error.is_none()
    {
        rollback_error = Some(PrimitiveError::Resource(error));
    }
    rollback_error.unwrap_or(original)
}

fn primitive_is_retained(
    inner: &PrimitiveRegistryInner,
    primitive: &PrimitiveId,
) -> Result<bool, PrimitiveError> {
    let descriptor = &inner
        .entries
        .get(primitive)
        .ok_or_else(|| PrimitiveError::Unknown(primitive.clone()))?
        .descriptor;
    Ok(descriptor.lifecycle || !descriptor.state.is_empty())
}

fn collect_primitive_instances(tree: &crate::RetainedUiTree) -> BTreeSet<PrimitiveInstanceId> {
    tree.nodes()
        .filter_map(|node| {
            Some(PrimitiveInstanceId {
                primitive: node.primitive()?.clone(),
                key: node.key()?.to_owned(),
                node: node.id(),
            })
        })
        .collect()
}

fn guard_primitive_panic<T>(
    primitive: &PrimitiveId,
    phase: &'static str,
    operation: impl FnOnce() -> T,
) -> Result<T, PrimitiveError> {
    catch_unwind(AssertUnwindSafe(operation)).map_err(|_| PrimitiveError::Panic {
        primitive: primitive.clone(),
        phase,
    })
}

#[derive(gpui::IntoElement)]
struct RegisteredPrimitiveElement {
    registry: PrimitiveRegistry,
    node: PrimitiveNode,
    retained_id: Option<crate::NodeId>,
    fallback: Option<UiNode>,
    dispatcher: Option<NodeEventDispatcher>,
    theme: PrimitiveTheme,
}

impl RenderOnce for RegisteredPrimitiveElement {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let registry = self.registry;
        let callbacks = self
            .node
            .props
            .iter()
            .filter_map(|(name, value)| {
                name.strip_prefix("on_").and_then(|event| match value {
                    PrimitiveValue::Callback(callback) => {
                        Some((event.to_owned(), callback.clone()))
                    }
                    _ => None,
                })
            })
            .collect();
        let events = PrimitiveEventEmitter {
            registry: Rc::downgrade(&registry.inner),
            primitive: self.node.primitive.clone(),
            callbacks,
            dispatcher: self.dispatcher,
        };
        match registry.render_instance(
            self.node,
            self.retained_id,
            &events,
            &self.theme,
            window,
            cx,
        ) {
            Ok(element) => element,
            Err(error) => self.fallback.map_or_else(
                || {
                    div()
                        .child(format!("Custom primitive error: {error}"))
                        .into_any_element()
                },
                |fallback| {
                    crate::GpuiNodeRenderer::render_with_primitives(
                        &fallback,
                        &crate::LiteralColorResolver,
                        &crate::InteractionState::default(),
                        &registry,
                    )
                },
            ),
        }
    }
}

fn validate_descriptor(descriptor: &PrimitiveDescriptor) -> Result<(), PrimitiveError> {
    if !is_pascal_case(&descriptor.export) {
        return Err(PrimitiveError::InvalidExport(descriptor.export.clone()));
    }
    for (name, field) in &descriptor.props {
        if !is_identifier(name) {
            return Err(PrimitiveError::InvalidPropName(name.clone()));
        }
        field
            .schema
            .validate_definition()
            .map_err(|source| PrimitiveError::InvalidSchema {
                location: format!("prop `{name}`"),
                source,
            })?;
        if let Some(default) = &field.default {
            field
                .schema
                .validate(&default.clone().into_dynamic())
                .map_err(|source| PrimitiveError::InvalidDefault {
                    prop: name.clone(),
                    source,
                })?;
        }
    }
    for (name, event) in &descriptor.events {
        event
            .payload
            .validate_definition()
            .map_err(|source| PrimitiveError::InvalidSchema {
                location: format!("event `{name}`"),
                source,
            })?;
        let callback = format!("on_{name}");
        if !descriptor
            .props
            .get(&callback)
            .is_some_and(|field| schema_accepts_callback(&field.schema))
        {
            return Err(PrimitiveError::MissingEventCallback {
                event: name.clone(),
                prop: callback,
            });
        }
    }
    ComponentStateSchema::new(descriptor.state.fields().clone())
        .map_err(|source| PrimitiveError::InvalidStateSchema(source.to_string()))?;
    Ok(())
}

fn schema_accepts_callback(schema: &ValueSchema) -> bool {
    matches!(schema, ValueSchema::Callback)
        || matches!(schema, ValueSchema::Optional { value } if schema_accepts_callback(value))
        || matches!(schema, ValueSchema::OneOf { variants } if variants.iter().any(schema_accepts_callback))
}

fn convert_props(
    schema: &BTreeMap<String, ObjectField>,
    values: &Map,
    generation: ScriptGeneration,
) -> Result<PrimitiveProps, PrimitiveError> {
    let mut converted = BTreeMap::new();
    for (name, field) in schema {
        let value = values
            .get(name.as_str())
            .cloned()
            .or_else(|| field.default.clone().map(UiValue::into_dynamic));
        if let Some(value) = value {
            converted.insert(
                name.clone(),
                convert_prop(&field.schema, value, generation).map_err(|source| {
                    PrimitiveError::PropConversion {
                        prop: name.clone(),
                        source,
                    }
                })?,
            );
        }
    }
    Ok(PrimitiveProps(converted))
}

fn convert_prop(
    schema: &ValueSchema,
    value: Dynamic,
    generation: ScriptGeneration,
) -> Result<PrimitiveValue, PrimitivePropConversionError> {
    match schema {
        ValueSchema::Optional { value: inner } if value.is_unit() => {
            Ok(PrimitiveValue::Data(UiValue::Null))
        }
        ValueSchema::Optional { value: inner } => convert_prop(inner, value, generation),
        ValueSchema::OneOf { variants } => {
            let branch = variants
                .iter()
                .find(|variant| variant.validate(&value).is_ok())
                .expect("validated primitive one_of prop matches one branch");
            convert_prop(branch, value, generation)
        }
        ValueSchema::Node => Ok(PrimitiveValue::Node(Box::new(value.cast::<UiNode>()))),
        ValueSchema::Callback => Ok(PrimitiveValue::Callback(UiEventHandler::Script(
            ScriptCallback::try_from_fn_ptr(value.cast::<FnPtr>(), generation)?,
        ))),
        ValueSchema::Array { items, .. } if matches!(items.as_ref(), ValueSchema::Node) => {
            Ok(PrimitiveValue::Nodes(
                value
                    .cast::<Array>()
                    .into_iter()
                    .map(Dynamic::cast::<UiNode>)
                    .collect(),
            ))
        }
        ValueSchema::Style => Ok(PrimitiveValue::Style(Box::new(value.cast::<Style>()))),
        ValueSchema::Length => Ok(PrimitiveValue::Length(value.cast::<Length>())),
        ValueSchema::Asset => Ok(PrimitiveValue::Asset(value.cast::<AssetId>())),
        _ => UiValue::from_dynamic(value)
            .map(PrimitiveValue::Data)
            .map_err(Into::into),
    }
}

#[derive(Debug, Error)]
pub enum PrimitivePropConversionError {
    #[error(transparent)]
    Value(#[from] UiValueError),
    #[error(transparent)]
    Callback(#[from] crate::ScriptCallbackDefinitionError),
}

#[derive(Debug, Error)]
pub enum PrimitiveError {
    #[error("primitive ID `{0}` must be `namespace.snake_case_name`")]
    InvalidId(String),
    #[error("primitive export `{0}` must be PascalCase")]
    InvalidExport(String),
    #[error("primitive prop `{0}` must be `snake_case`")]
    InvalidPropName(String),
    #[error("invalid schema definition for primitive {location}: {source}")]
    InvalidSchema {
        location: String,
        source: SchemaDefinitionError,
    },
    #[error("invalid primitive state schema: {0}")]
    InvalidStateSchema(String),
    #[error("primitive registry is already borrowed during rendering")]
    Borrowed,
    #[error("primitive `{0:?}` is already registered")]
    Duplicate(PrimitiveId),
    #[error("primitive export `{namespace}::{export}` is already registered")]
    DuplicateExport { namespace: String, export: String },
    #[error("primitive `{0:?}` is not registered")]
    Unknown(PrimitiveId),
    #[error("primitive `{0:?}` requires a stable key")]
    MissingKey(PrimitiveId),
    #[error("primitive `{0:?}` requires a retained NodeId renderer")]
    MissingRetainedIdentity(PrimitiveId),
    #[error("primitive event emitter outlived its registry")]
    RegistryReleased,
    #[error("props for primitive `{primitive:?}` are invalid: {source}")]
    InvalidProps {
        primitive: PrimitiveId,
        source: SchemaValidationError,
    },
    #[error("default for primitive prop `{prop}` is invalid: {source}")]
    InvalidDefault {
        prop: String,
        source: SchemaValidationError,
    },
    #[error("primitive prop `{prop}` cannot cross the runtime boundary: {source}")]
    PropConversion {
        prop: String,
        source: PrimitivePropConversionError,
    },
    #[error("primitive event `{event}` requires callback prop `{prop}`")]
    MissingEventCallback { event: String, prop: String },
    #[error("primitive `{primitive:?}` does not declare event `{event}`")]
    UnknownEvent {
        primitive: PrimitiveId,
        event: String,
    },
    #[error("primitive `{primitive:?}` event `{event}` is invalid: {source}")]
    InvalidEvent {
        primitive: PrimitiveId,
        event: String,
        source: SchemaValidationError,
    },
    #[error("primitive `{primitive:?}` handler failed: {message}")]
    Handler {
        primitive: PrimitiveId,
        message: String,
    },
    #[error("primitive `{primitive:?}` panicked during {phase}")]
    Panic {
        primitive: PrimitiveId,
        phase: &'static str,
    },
    #[error(transparent)]
    Resource(#[from] PrimitiveResourceError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ObjectField, StateField};
    use std::cell::Cell;

    struct TestHandler;

    struct TestTheme;

    impl ColorResolver for TestTheme {
        fn resolve(&self, color: &ColorValue) -> Option<Rgba8> {
            matches!(color, ColorValue::Token(token) if token == "accent")
                .then(|| Rgba8::from_rgba_hex(0x1234_56ff))
        }

        fn resolve_length(&self, length: Length) -> Option<Length> {
            (length == Length::ThemeSpacing(SpacingToken::Sm)).then_some(Length::Pixels(6.0))
        }
    }

    impl PrimitiveHandler for TestHandler {
        fn render(
            &mut self,
            _: &PrimitiveInstance,
            _: &PrimitiveEventEmitter,
            _: &PrimitiveTheme,
            _: &mut Window,
            _: &mut App,
        ) -> Result<AnyElement, String> {
            Ok(div().into_any_element())
        }
    }

    #[test]
    fn primitive_theme_exposes_only_resolved_semantic_snapshot() {
        let theme = PrimitiveTheme::capture(&TestTheme);
        assert_eq!(
            theme.color("accent"),
            Some(Rgba8::from_rgba_hex(0x1234_56ff))
        );
        assert_eq!(theme.color("unknown"), None);
        assert_eq!(
            theme.resolve_color(&ColorValue::Literal(Rgba8::from_rgba_hex(0xaabb_ccdd))),
            Some(Rgba8::from_rgba_hex(0xaabb_ccdd))
        );
        assert_eq!(
            theme.resolve_length(Length::ThemeSpacing(SpacingToken::Sm)),
            Some(Length::Pixels(6.0))
        );
    }

    fn descriptor() -> PrimitiveDescriptor {
        PrimitiveDescriptor {
            id: PrimitiveId::parse("my_app.code_editor").unwrap(),
            export: "CodeEditor".to_owned(),
            props: BTreeMap::from([
                (
                    "value".to_owned(),
                    ObjectField::required(ValueSchema::string()),
                ),
                (
                    "on_change".to_owned(),
                    ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
                ),
            ]),
            events: BTreeMap::from([(
                "change".to_owned(),
                EventSchema {
                    payload: ValueSchema::string(),
                },
            )]),
            state: ComponentStateSchema::new(BTreeMap::from([(
                "selection".to_owned(),
                StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
            )]))
            .unwrap(),
            lifecycle: true,
        }
    }

    #[test]
    fn custom_primitive_props_and_keys_are_validated() {
        let registry = PrimitiveRegistry::new();
        let descriptor = descriptor();
        let id = descriptor.id.clone();
        registry.register(descriptor, TestHandler).unwrap();
        assert!(matches!(
            registry.create_node(
                &id,
                None,
                &Map::from_iter([("value".into(), Dynamic::from("source"))]),
                ScriptGeneration::initial(),
            ),
            Err(PrimitiveError::MissingKey(_))
        ));
        registry
            .create_node(
                &id,
                Some("editor".to_owned()),
                &Map::from_iter([("value".into(), Dynamic::from("source"))]),
                ScriptGeneration::initial(),
            )
            .unwrap();
    }

    #[test]
    fn custom_primitive_events_are_normalized() {
        let registry = PrimitiveRegistry::new();
        let descriptor = descriptor();
        let id = descriptor.id.clone();
        registry.register(descriptor, TestHandler).unwrap();
        assert_eq!(
            registry
                .normalize_event(&id, "change", UiValue::String("new".to_owned()))
                .unwrap(),
            UiValue::String("new".to_owned())
        );
        assert!(matches!(
            registry.normalize_event(&id, "change", UiValue::Bool(true)),
            Err(PrimitiveError::InvalidEvent { .. })
        ));
    }

    #[test]
    fn native_panics_are_converted_to_primitive_errors() {
        let id = PrimitiveId::parse("my_app.crash").unwrap();
        assert!(matches!(
            guard_primitive_panic(&id, "render", || panic!("boom")),
            Err(PrimitiveError::Panic {
                phase: "render",
                ..
            })
        ));
    }

    #[test]
    fn primitive_resource_scope_rolls_back_and_continues_after_cleanup_panic() {
        let scope = PrimitiveResourceScope::new();
        let retained = Rc::new(Cell::new(0));
        let retained_cleanup = Rc::clone(&retained);
        scope
            .own("retained", move || retained_cleanup.set(1))
            .unwrap();
        let checkpoint = scope.checkpoint().unwrap();
        let order = Rc::new(RefCell::new(Vec::new()));
        let first = Rc::clone(&order);
        scope
            .own("first", move || first.borrow_mut().push(1))
            .unwrap();
        scope.own("panic", || panic!("cleanup failed")).unwrap();
        let last = Rc::clone(&order);
        scope
            .own("last", move || last.borrow_mut().push(3))
            .unwrap();

        assert!(matches!(
            scope.rollback(checkpoint),
            Err(PrimitiveResourceError::CleanupPanic { ref label }) if label == "panic"
        ));
        assert_eq!(*order.borrow(), vec![3, 1]);
        assert_eq!(scope.active_count(), 1);
        scope.close().unwrap();
        assert_eq!(retained.get(), 1);
    }

    #[test]
    fn successful_tree_cleanup_unmounts_removed_keyed_instances() {
        struct UnmountCounter(Rc<Cell<usize>>);
        impl PrimitiveHandler for UnmountCounter {
            fn render(
                &mut self,
                _: &PrimitiveInstance,
                _: &PrimitiveEventEmitter,
                _: &PrimitiveTheme,
                _: &mut Window,
                _: &mut App,
            ) -> Result<AnyElement, String> {
                Ok(div().into_any_element())
            }

            fn unmount(&mut self, _: &PrimitiveInstanceId) {
                self.0.set(self.0.get() + 1);
            }
        }

        let registry = PrimitiveRegistry::new();
        let descriptor = descriptor();
        let node = PrimitiveNode {
            primitive: descriptor.id.clone(),
            key: Some("editor".to_owned()),
            props: PrimitiveProps::new(),
        };
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(UiNode::custom(node.clone())).unwrap();
        let instance = collect_primitive_instances(&tree)
            .into_iter()
            .next()
            .unwrap();
        let unmounted = Rc::new(Cell::new(0));
        let cleaned = Rc::new(Cell::new(0));
        let resources = PrimitiveResourceScope::new();
        let cleanup = Rc::clone(&cleaned);
        resources
            .own("watcher", move || cleanup.set(cleanup.get() + 1))
            .unwrap();
        registry
            .register(descriptor, UnmountCounter(Rc::clone(&unmounted)))
            .unwrap();
        registry.inner.borrow_mut().mounted.insert(
            instance.clone(),
            PrimitiveInstance {
                id: Some(instance.clone()),
                node,
                resources: Some(resources),
            },
        );
        tree.reconcile(UiNode::text("removed")).unwrap();
        registry.retain_tree(&tree).unwrap();
        assert_eq!(unmounted.get(), 1);
        assert_eq!(cleaned.get(), 1);
    }

    #[test]
    fn primitive_identity_uses_retained_node_not_component_local_key() {
        let primitive = PrimitiveId::parse("my_app.editor").unwrap();
        let branch = |branch: &str| {
            UiNode::box_node(vec![UiNode::custom(PrimitiveNode {
                primitive: primitive.clone(),
                key: Some("editor".to_owned()),
                props: PrimitiveProps::new().with(
                    "branch",
                    PrimitiveValue::Data(UiValue::String(branch.to_owned())),
                ),
            })])
            .with_key(branch)
        };
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(UiNode::box_node(vec![branch("left"), branch("right")]))
            .unwrap();
        let before = collect_primitive_instances(&tree);
        assert_eq!(before.len(), 2);
        assert!(before.iter().all(|instance| instance.key() == "editor"));
        assert_eq!(
            before
                .iter()
                .map(PrimitiveInstanceId::node)
                .collect::<BTreeSet<_>>()
                .len(),
            2
        );

        tree.reconcile(UiNode::box_node(vec![branch("right"), branch("left")]))
            .unwrap();
        assert_eq!(collect_primitive_instances(&tree), before);
    }

    #[test]
    fn primitive_event_emitter_holds_only_a_weak_registry_reference() {
        let registry = PrimitiveRegistry::new();
        let weak = Rc::downgrade(&registry.inner);
        let emitter = PrimitiveEventEmitter {
            registry: Rc::downgrade(&registry.inner),
            primitive: PrimitiveId::parse("my_app.editor").unwrap(),
            callbacks: BTreeMap::new(),
            dispatcher: None,
        };
        assert_eq!(Rc::strong_count(&registry.inner), 1);
        drop(registry);
        assert!(weak.upgrade().is_none());
        drop(emitter);
    }
}
