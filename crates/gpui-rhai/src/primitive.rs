use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, Window, div};
use rhai::{Array, Dynamic, FnPtr, Map};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    AssetId, ColorResolver, ColorValue, ComponentStateSchema, EventSchema, Length,
    NodeEventDispatcher, ObjectField, RadiusToken, Rgba8, SchemaDefinitionError,
    SchemaValidationError, ScriptCallback, ScriptGeneration, SpacingToken, Style, UiEventHandler,
    UiNode, UiNodeKind, UiValue, UiValueError, ValueSchema,
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
    pub primitive: PrimitiveId,
    pub key: String,
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
}

#[derive(Clone)]
pub struct PrimitiveEventEmitter {
    registry: PrimitiveRegistry,
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
        let payload = self
            .registry
            .normalize_event(&self.primitive, event, payload)?;
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
        for instance in &removed {
            if let Some(entry) = inner.entries.get_mut(&instance.primitive) {
                guard_primitive_panic(&instance.primitive, "unmount", || {
                    entry.handler.unmount(instance);
                })?;
            }
        }
        inner
            .mounted
            .retain(|instance, _| active.contains(instance));
        Ok(())
    }

    /// Unmount keyed primitive instances absent from the current successful tree.
    ///
    /// # Errors
    ///
    /// Returns borrow or panic-boundary errors from native unmount handlers.
    pub fn retain_tree(&self, root: &UiNode) -> Result<(), PrimitiveError> {
        let mut active = BTreeSet::new();
        collect_primitive_instances(root, &mut active);
        self.retain_mounted(&active)
    }

    pub(crate) fn element(
        &self,
        node: PrimitiveNode,
        fallback: Option<UiNode>,
        dispatcher: Option<NodeEventDispatcher>,
        theme: PrimitiveTheme,
    ) -> AnyElement {
        RegisteredPrimitiveElement {
            registry: self.clone(),
            node,
            fallback,
            dispatcher,
            theme,
        }
        .into_any_element()
    }

    fn render_instance(
        &self,
        node: PrimitiveNode,
        events: &PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, PrimitiveError> {
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| PrimitiveError::Borrowed)?;
        let instance_id = node.key.as_ref().map(|key| PrimitiveInstanceId {
            primitive: node.primitive.clone(),
            key: key.clone(),
        });
        let instance = PrimitiveInstance {
            id: instance_id.clone(),
            node,
        };
        let previous = instance_id
            .as_ref()
            .and_then(|id| inner.mounted.get(id))
            .cloned();
        let needs_mount = instance_id.is_some() && previous.is_none();
        let entry = inner
            .entries
            .get_mut(&instance.node.primitive)
            .ok_or_else(|| PrimitiveError::Unknown(instance.node.primitive.clone()))?;
        if needs_mount && entry.descriptor.lifecycle {
            guard_primitive_panic(&instance.node.primitive, "mount", || {
                entry.handler.mount(&instance)
            })?
            .map_err(|message| PrimitiveError::Handler {
                primitive: instance.node.primitive.clone(),
                message,
            })?;
        }
        if entry.descriptor.lifecycle
            && let Some(previous) = &previous
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
        let element = guard_primitive_panic(&instance.node.primitive, "render", || {
            entry.handler.render(&instance, events, theme, window, cx)
        })?
        .map_err(|message| PrimitiveError::Handler {
            primitive: instance.node.primitive.clone(),
            message,
        })?;
        if let Some(id) = instance_id {
            inner.mounted.insert(id, instance);
        }
        Ok(element)
    }
}

fn collect_primitive_instances(node: &UiNode, active: &mut BTreeSet<PrimitiveInstanceId>) {
    match node.kind() {
        UiNodeKind::Custom { primitive } => {
            if let Some(key) = &primitive.key {
                active.insert(PrimitiveInstanceId {
                    primitive: primitive.primitive.clone(),
                    key: key.clone(),
                });
            }
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for child in children {
                collect_primitive_instances(child, active);
            }
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            collect_primitive_instances(trigger, active);
            collect_primitive_instances(content, active);
        }
        UiNodeKind::ErrorBoundary { child, fallback } => {
            collect_primitive_instances(child, active);
            collect_primitive_instances(fallback, active);
        }
        UiNodeKind::Dropdown { spec } => {
            for slot in [
                spec.trigger_slot.as_deref(),
                spec.header_slot.as_deref(),
                spec.footer_slot.as_deref(),
                spec.empty_slot.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                collect_primitive_instances(slot, active);
            }
        }
        UiNodeKind::VirtualList { spec } => {
            for item in &spec.items {
                collect_primitive_instances(&item.node, active);
            }
        }
        UiNodeKind::VirtualCollection { spec } => {
            for item in spec.realized.values() {
                collect_primitive_instances(item, active);
            }
        }
        UiNodeKind::Table { spec } => {
            for column in &spec.columns {
                for cell in column.custom_cells.iter().flatten() {
                    collect_primitive_instances(cell, active);
                }
            }
            for slot in [spec.loading_slot.as_deref(), spec.empty_slot.as_deref()]
                .into_iter()
                .flatten()
            {
                collect_primitive_instances(slot, active);
            }
            for row in &spec.loading_rows {
                collect_primitive_instances(row, active);
            }
        }
        UiNodeKind::Text { .. }
        | UiNodeKind::RichText { .. }
        | UiNodeKind::Canvas { .. }
        | UiNodeKind::Image { .. }
        | UiNodeKind::DirectionalImage { .. }
        | UiNodeKind::Select { .. }
        | UiNodeKind::DatePicker { .. }
        | UiNodeKind::ToastHost { .. } => {}
    }
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
            registry: registry.clone(),
            primitive: self.node.primitive.clone(),
            callbacks,
            dispatcher: self.dispatcher,
        };
        match registry.render_instance(self.node, &events, &self.theme, window, cx) {
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
        let instance = PrimitiveInstanceId {
            primitive: descriptor.id.clone(),
            key: "editor".to_owned(),
        };
        let unmounted = Rc::new(Cell::new(0));
        registry
            .register(descriptor, UnmountCounter(Rc::clone(&unmounted)))
            .unwrap();
        registry.inner.borrow_mut().mounted.insert(
            instance.clone(),
            PrimitiveInstance {
                id: Some(instance.clone()),
                node: PrimitiveNode {
                    primitive: instance.primitive.clone(),
                    key: Some(instance.key.clone()),
                    props: PrimitiveProps::new(),
                },
            },
        );
        registry.retain_tree(&UiNode::text("removed")).unwrap();
        assert_eq!(unmounted.get(), 1);
    }
}
