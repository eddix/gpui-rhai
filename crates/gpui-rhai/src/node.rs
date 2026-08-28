use std::collections::{BTreeMap, BTreeSet};

use rhai::{
    Array, CustomType, Dynamic, EvalAltResult, FLOAT, FnPtr, INT, ImmutableString, Map,
    NativeCallContext, Position, TypeBuilder,
};

use crate::{
    AnimationSpec, DropdownMode, DropdownNodeSpec, DropdownOption, DropdownState, OpaqueHandle,
    OverlayId, OverlayKind, OverlayPlacement, PrimitiveNode, ScriptCallback, ScriptGeneration,
    Style, ToastHostSpec, ToastItemSpec, ToastRegion, ToastVariant, UiValue, VirtualListItem,
    VirtualListNodeSpec,
};

#[derive(Clone, Debug, PartialEq)]
pub struct OverlayNodeSpec {
    pub id: OverlayId,
    pub parent: Option<OverlayId>,
    pub kind: OverlayKind,
    pub placement: OverlayPlacement,
    pub open: bool,
    pub gap: f64,
    pub modal: bool,
    pub dismiss: OverlayDismissPolicy,
    pub tooltip_delays: Option<TooltipDelays>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TooltipDelays {
    pub show_ms: u64,
    pub hide_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayDismissPolicy {
    pub escape: bool,
    pub outside: bool,
}

/// Stable identity for a node among its siblings.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodeKey(ImmutableString);

impl NodeKey {
    #[must_use]
    pub fn new(value: impl Into<ImmutableString>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    pub module: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiNodeKind {
    Text {
        text: ImmutableString,
    },
    Container {
        children: Vec<UiNode>,
    },
    Custom {
        primitive: PrimitiveNode,
    },
    Image {
        handle: OpaqueHandle,
    },
    DirectionalImage {
        left_to_right: OpaqueHandle,
        right_to_left: OpaqueHandle,
    },
    Overlay {
        trigger: Box<UiNode>,
        content: Box<UiNode>,
        spec: OverlayNodeSpec,
    },
    Dropdown {
        spec: DropdownNodeSpec,
    },
    ToastHost {
        spec: ToastHostSpec,
    },
    VirtualList {
        spec: VirtualListNodeSpec,
    },
    ErrorBoundary {
        child: Box<UiNode>,
        fallback: Box<UiNode>,
    },
}

/// A stable declarative UI node containing no GPUI values or lifetimes.
#[derive(Clone, Debug, PartialEq)]
pub struct UiNode {
    kind: UiNodeKind,
    key: Option<NodeKey>,
    style: Style,
    part_styles: BTreeMap<String, Style>,
    source: Option<SourceLocation>,
    attributes: BTreeMap<String, UiValue>,
    handlers: BTreeMap<String, ScriptCallback>,
    handler_payloads: BTreeMap<String, UiValue>,
    animations: Vec<AnimationSpec>,
}

impl UiNode {
    #[must_use]
    pub fn text(text: impl Into<ImmutableString>) -> Self {
        Self {
            kind: UiNodeKind::Text { text: text.into() },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
        }
    }

    #[must_use]
    pub fn container(children: Vec<Self>) -> Self {
        Self {
            kind: UiNodeKind::Container { children },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
        }
    }

    #[must_use]
    pub fn column(children: Vec<Self>) -> Self {
        Self::container(children).with_style(&Style::new().flex_col())
    }

    #[must_use]
    pub fn row(children: Vec<Self>) -> Self {
        Self::container(children).with_style(&Style::new().flex_row())
    }

    #[must_use]
    pub fn custom(primitive: PrimitiveNode) -> Self {
        Self {
            kind: UiNodeKind::Custom { primitive },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
        }
    }

    #[must_use]
    pub fn error_boundary(child: Self, fallback: Self) -> Self {
        Self {
            kind: UiNodeKind::ErrorBoundary {
                child: Box::new(child),
                fallback: Box::new(fallback),
            },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
        }
    }

    #[must_use]
    pub fn image(handle: OpaqueHandle) -> Self {
        Self {
            kind: UiNodeKind::Image { handle },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
        }
    }

    #[must_use]
    pub fn directional_image(left_to_right: OpaqueHandle, right_to_left: OpaqueHandle) -> Self {
        Self {
            kind: UiNodeKind::DirectionalImage {
                left_to_right,
                right_to_left,
            },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
        }
    }

    #[must_use]
    pub fn overlay(trigger: Self, content: Self, spec: OverlayNodeSpec) -> Self {
        Self {
            kind: UiNodeKind::Overlay {
                trigger: Box::new(trigger),
                content: Box::new(content),
                spec,
            },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
        }
    }

    #[must_use]
    pub fn dropdown(spec: DropdownNodeSpec) -> Self {
        let key = spec.id.clone();
        Self {
            kind: UiNodeKind::Dropdown { spec },
            key: Some(NodeKey::new(key)),
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
        }
    }

    #[must_use]
    pub fn toast_host(spec: ToastHostSpec) -> Self {
        let key = spec.key.clone();
        Self {
            kind: UiNodeKind::ToastHost { spec },
            key: Some(NodeKey::new(key)),
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
        }
    }

    #[must_use]
    pub fn virtual_list(spec: VirtualListNodeSpec) -> Self {
        let key = spec.key.clone();
        Self {
            kind: UiNodeKind::VirtualList { spec },
            key: Some(NodeKey::new(key)),
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_key(mut self, key: impl Into<ImmutableString>) -> Self {
        self.key = Some(NodeKey::new(key));
        self
    }

    #[must_use]
    pub fn with_style(mut self, style: &Style) -> Self {
        self.style = self.style.merged(style);
        self
    }

    #[must_use]
    pub fn with_part_styles(mut self, styles: BTreeMap<String, Style>) -> Self {
        self.part_styles.extend(styles);
        self
    }

    #[must_use]
    pub fn with_source(mut self, source: SourceLocation) -> Self {
        self.source = Some(source);
        self
    }

    #[must_use]
    pub fn with_attribute(mut self, name: impl Into<String>, value: UiValue) -> Self {
        self.attributes.insert(name.into(), value);
        self
    }

    #[must_use]
    pub fn with_handler(mut self, event: impl Into<String>, callback: ScriptCallback) -> Self {
        self.handlers.insert(event.into(), callback);
        self
    }

    #[must_use]
    pub fn with_handler_payload(mut self, event: impl Into<String>, payload: UiValue) -> Self {
        self.handler_payloads.insert(event.into(), payload);
        self
    }

    #[must_use]
    pub fn with_animation(mut self, animation: AnimationSpec) -> Self {
        self.animations
            .retain(|existing| existing.property() != animation.property());
        self.animations.push(animation);
        self
    }

    #[must_use]
    pub fn handlers(&self) -> &BTreeMap<String, ScriptCallback> {
        &self.handlers
    }

    #[must_use]
    pub fn handler_payload(&self, event: &str) -> Option<&UiValue> {
        self.handler_payloads.get(event)
    }

    #[must_use]
    pub fn animations(&self) -> &[AnimationSpec] {
        &self.animations
    }

    pub(crate) fn bind_generation(&mut self, generation: ScriptGeneration) {
        for handler in self.handlers.values_mut() {
            handler.bind_generation(generation);
        }
        match &mut self.kind {
            UiNodeKind::Container { children } => {
                for child in children {
                    child.bind_generation(generation);
                }
            }
            UiNodeKind::ErrorBoundary { child, fallback } => {
                child.bind_generation(generation);
                fallback.bind_generation(generation);
            }
            UiNodeKind::Overlay {
                trigger, content, ..
            } => {
                trigger.bind_generation(generation);
                content.bind_generation(generation);
            }
            UiNodeKind::Text { .. }
            | UiNodeKind::Custom { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. }
            | UiNodeKind::ToastHost { .. } => {}
            UiNodeKind::VirtualList { spec } => {
                for item in &mut spec.items {
                    item.node.bind_generation(generation);
                }
            }
            UiNodeKind::Dropdown { spec } => {
                for slot in [
                    &mut spec.trigger_slot,
                    &mut spec.header_slot,
                    &mut spec.footer_slot,
                    &mut spec.empty_slot,
                ] {
                    if let Some(slot) = slot.as_mut() {
                        slot.bind_generation(generation);
                    }
                }
            }
        }
    }

    pub(crate) fn bind_component_scope(
        &mut self,
        component: &crate::ComponentInstancePath,
        events: &BTreeMap<String, crate::EventSchema>,
        native_context: Option<&crate::engine::ScriptNativeContext>,
    ) {
        for handler in self.handlers.values_mut() {
            handler.bind_component_if_unset(component.clone(), events.clone());
            if let Some(context) = native_context {
                handler.bind_native_context_if_unset(std::rc::Rc::clone(context));
            }
        }
        match &mut self.kind {
            UiNodeKind::Container { children } => {
                for child in children {
                    child.bind_component_scope(component, events, native_context);
                }
            }
            UiNodeKind::Custom { primitive } => {
                primitive
                    .props
                    .bind_component_scope(component, events, native_context);
            }
            UiNodeKind::ErrorBoundary { child, fallback } => {
                child.bind_component_scope(component, events, native_context);
                fallback.bind_component_scope(component, events, native_context);
            }
            UiNodeKind::Overlay {
                trigger, content, ..
            } => {
                trigger.bind_component_scope(component, events, native_context);
                content.bind_component_scope(component, events, native_context);
            }
            UiNodeKind::Dropdown { spec } => {
                for slot in [
                    &mut spec.trigger_slot,
                    &mut spec.header_slot,
                    &mut spec.footer_slot,
                    &mut spec.empty_slot,
                ]
                .into_iter()
                .flatten()
                {
                    slot.bind_component_scope(component, events, native_context);
                }
            }
            UiNodeKind::VirtualList { spec } => {
                for item in &mut spec.items {
                    item.node
                        .bind_component_scope(component, events, native_context);
                }
            }
            UiNodeKind::Text { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. }
            | UiNodeKind::ToastHost { .. } => {}
        }
    }

    pub(crate) fn bind_callback_scope_by_name(
        &mut self,
        names: &BTreeSet<String>,
        component: &crate::ComponentInstancePath,
        events: &BTreeMap<String, crate::EventSchema>,
        native_context: Option<&crate::engine::ScriptNativeContext>,
    ) {
        for handler in self.handlers.values_mut() {
            if names.contains(handler.name()) {
                handler.bind_component_if_unset(component.clone(), events.clone());
                if let (Some(context), None) = (native_context, handler.native_context()) {
                    handler.bind_native_context_if_unset(std::rc::Rc::clone(context));
                }
            }
        }
        match &mut self.kind {
            UiNodeKind::Container { children } => {
                for child in children {
                    child.bind_callback_scope_by_name(names, component, events, native_context);
                }
            }
            UiNodeKind::Custom { primitive } => primitive.props.bind_callback_scope_by_name(
                names,
                component,
                events,
                native_context,
            ),
            UiNodeKind::ErrorBoundary { child, fallback } => {
                child.bind_callback_scope_by_name(names, component, events, native_context);
                fallback.bind_callback_scope_by_name(names, component, events, native_context);
            }
            UiNodeKind::Overlay {
                trigger, content, ..
            } => {
                trigger.bind_callback_scope_by_name(names, component, events, native_context);
                content.bind_callback_scope_by_name(names, component, events, native_context);
            }
            UiNodeKind::Dropdown { spec } => {
                for slot in [
                    &mut spec.trigger_slot,
                    &mut spec.header_slot,
                    &mut spec.footer_slot,
                    &mut spec.empty_slot,
                ]
                .into_iter()
                .flatten()
                {
                    slot.bind_callback_scope_by_name(names, component, events, native_context);
                }
            }
            UiNodeKind::VirtualList { spec } => {
                for item in &mut spec.items {
                    item.node
                        .bind_callback_scope_by_name(names, component, events, native_context);
                }
            }
            UiNodeKind::Text { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. }
            | UiNodeKind::ToastHost { .. } => {}
        }
    }

    #[must_use]
    pub fn kind(&self) -> &UiNodeKind {
        &self.kind
    }

    #[must_use]
    pub fn key(&self) -> Option<&NodeKey> {
        self.key.as_ref()
    }

    #[must_use]
    pub fn style(&self) -> &Style {
        &self.style
    }

    #[must_use]
    pub fn part_style(&self, part: &str) -> Option<&Style> {
        self.part_styles.get(part)
    }

    pub fn part_styles(&self) -> impl Iterator<Item = (&str, &Style)> {
        self.part_styles
            .iter()
            .map(|(name, style)| (name.as_str(), style))
    }

    #[must_use]
    pub fn source(&self) -> Option<&SourceLocation> {
        self.source.as_ref()
    }

    #[must_use]
    pub fn attributes(&self) -> &BTreeMap<String, UiValue> {
        &self.attributes
    }
}

impl CustomType for UiNode {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("UiNode")
            .with_fn("with_key", |node: &mut Self, key: ImmutableString| {
                node.clone().with_key(key)
            })
            .with_fn("with_style", |node: &mut Self, style: Style| {
                node.clone().with_style(&style)
            })
            .with_fn("on_click", |node: &mut Self, callback: FnPtr| {
                node.clone().with_handler(
                    "click",
                    ScriptCallback::from_fn_ptr(callback, ScriptGeneration::default()),
                )
            })
            .with_fn(
                "on_click_value",
                |node: &mut Self,
                 callback: FnPtr,
                 payload: Dynamic|
                 -> Result<Self, Box<EvalAltResult>> {
                    let payload = UiValue::from_dynamic(payload).map_err(|error| {
                        Box::new(EvalAltResult::ErrorRuntime(
                            error.to_string().into(),
                            Position::NONE,
                        ))
                    })?;
                    Ok(node
                        .clone()
                        .with_handler(
                            "click",
                            ScriptCallback::from_fn_ptr(callback, ScriptGeneration::default()),
                        )
                        .with_handler_payload("click", payload))
                },
            );
        register_semantic_event_methods(&mut builder);
        builder
            .with_fn("animate", |node: &mut Self, animation: AnimationSpec| {
                node.clone().with_animation(animation)
            })
            .with_fn("disabled", |node: &mut Self, disabled: bool| {
                node.clone()
                    .with_attribute("disabled", UiValue::Bool(disabled))
            })
            .with_fn("tab_stop", |node: &mut Self, tab_stop: bool| {
                node.clone()
                    .with_attribute("tab_stop", UiValue::Bool(tab_stop))
            })
            .with_fn(
                "on_key_value",
                |node: &mut Self,
                 key: ImmutableString,
                 callback: FnPtr,
                 payload: Dynamic|
                 -> Result<Self, Box<EvalAltResult>> {
                    let key = key.trim().to_ascii_lowercase();
                    if key.is_empty()
                        || !key
                            .chars()
                            .all(|character| character.is_ascii_alphanumeric() || character == '_')
                    {
                        return Err(Box::new(EvalAltResult::ErrorRuntime(
                            "key handler name must be a non-empty key identifier".into(),
                            Position::NONE,
                        )));
                    }
                    let payload = UiValue::from_dynamic(payload).map_err(|error| {
                        Box::new(EvalAltResult::ErrorRuntime(
                            error.to_string().into(),
                            Position::NONE,
                        ))
                    })?;
                    let event = format!("key:{key}");
                    Ok(node
                        .clone()
                        .with_handler(
                            event.clone(),
                            ScriptCallback::from_fn_ptr(callback, ScriptGeneration::default()),
                        )
                        .with_handler_payload(event, payload))
                },
            );
        register_accessibility_methods(&mut builder);
    }
}

fn register_semantic_event_methods(builder: &mut TypeBuilder<UiNode>) {
    for (method, event) in [
        ("on_open_change", "open_change"),
        ("on_change", "change"),
        ("on_query_change", "query_change"),
        ("on_dismiss", "dismiss"),
    ] {
        let event = event.to_owned();
        builder.with_fn(method, move |node: &mut UiNode, callback: FnPtr| {
            node.clone().with_handler(
                event.clone(),
                ScriptCallback::from_fn_ptr(callback, ScriptGeneration::default()),
            )
        });
    }
}

fn register_accessibility_methods(builder: &mut TypeBuilder<UiNode>) {
    builder
        .with_fn(
            "accessibility_role",
            |node: &mut UiNode, role: ImmutableString| {
                node.clone()
                    .with_attribute("role", UiValue::String(role.to_string()))
            },
        )
        .with_fn(
            "accessibility_label",
            |node: &mut UiNode, label: ImmutableString| {
                node.clone()
                    .with_attribute("label", UiValue::String(label.to_string()))
            },
        )
        .with_fn(
            "accessibility_checked",
            |node: &mut UiNode, value: Dynamic| -> Result<UiNode, Box<EvalAltResult>> {
                let value = dynamic_ui_value(value)?;
                Ok(node.clone().with_attribute("checked", value))
            },
        )
        .with_fn(
            "accessibility_value",
            |node: &mut UiNode, value: Dynamic| -> Result<UiNode, Box<EvalAltResult>> {
                let value = dynamic_ui_value(value)?;
                Ok(node.clone().with_attribute("value", value))
            },
        )
        .with_fn(
            "accessibility_invalid",
            |node: &mut UiNode, invalid: bool| {
                node.clone()
                    .with_attribute("invalid", UiValue::Bool(invalid))
            },
        )
        .with_fn(
            "accessibility_id",
            |node: &mut UiNode, id: ImmutableString| {
                node.clone()
                    .with_attribute("semantic_id", UiValue::String(id.to_string()))
            },
        )
        .with_fn(
            "accessibility_labelled_by",
            |node: &mut UiNode, id: ImmutableString| {
                node.clone()
                    .with_attribute("labelled_by", UiValue::String(id.to_string()))
            },
        )
        .with_fn(
            "accessibility_described_by",
            |node: &mut UiNode, ids: ImmutableString| {
                node.clone()
                    .with_attribute("described_by", UiValue::String(ids.to_string()))
            },
        )
        .with_fn(
            "accessibility_required",
            |node: &mut UiNode, required: bool| {
                node.clone()
                    .with_attribute("required", UiValue::Bool(required))
            },
        );
}

fn dynamic_ui_value(value: Dynamic) -> Result<UiValue, Box<EvalAltResult>> {
    UiValue::from_dynamic(value).map_err(|error| {
        Box::new(EvalAltResult::ErrorRuntime(
            error.to_string().into(),
            Position::NONE,
        ))
    })
}

pub(crate) fn text_node(call: NativeCallContext<'_>, text: ImmutableString) -> UiNode {
    with_call_source(UiNode::text(text), call)
}

pub(crate) fn column_node(
    call: NativeCallContext<'_>,
    children: Array,
) -> Result<UiNode, Box<rhai::EvalAltResult>> {
    collect_children(children, "column")
        .map(|children| with_call_source(UiNode::column(children), call))
}

pub(crate) fn row_node(
    call: NativeCallContext<'_>,
    children: Array,
) -> Result<UiNode, Box<rhai::EvalAltResult>> {
    collect_children(children, "row").map(|children| with_call_source(UiNode::row(children), call))
}

pub(crate) fn error_boundary_node(
    call: NativeCallContext<'_>,
    child: UiNode,
    fallback: UiNode,
) -> UiNode {
    with_call_source(UiNode::error_boundary(child, fallback), call)
}

pub(crate) fn lazy_error_boundary_node(
    call: NativeCallContext<'_>,
    child: FnPtr,
    fallback: FnPtr,
) -> Result<UiNode, Box<EvalAltResult>> {
    let fallback_node = fallback.call_within_context::<UiNode>(&call, ())?;
    let child_result = child.call_within_context::<UiNode>(&call, ());
    drop(child);
    drop(fallback);
    match child_result {
        Ok(child) => Ok(with_call_source(
            UiNode::error_boundary(child, fallback_node),
            call,
        )),
        Err(error) => Ok(with_call_source(
            fallback_node.with_attribute("boundary_error", UiValue::String(error.to_string())),
            call,
        )),
    }
}

pub(crate) fn image_node(call: NativeCallContext<'_>, handle: OpaqueHandle) -> UiNode {
    with_call_source(UiNode::image(handle), call)
}

pub(crate) fn directional_image_node(
    call: NativeCallContext<'_>,
    left_to_right: OpaqueHandle,
    right_to_left: OpaqueHandle,
) -> UiNode {
    with_call_source(
        UiNode::directional_image(left_to_right, right_to_left),
        call,
    )
}

pub(crate) fn overlay_node(
    call: NativeCallContext<'_>,
    trigger: UiNode,
    overlay_content: UiNode,
    mut config: Map,
) -> Result<UiNode, Box<EvalAltResult>> {
    let id = required_string(&mut config, "id")?;
    let parent = optional_string(&mut config, "parent")?;
    let kind = match optional_string(&mut config, "kind")?.as_deref() {
        None | Some("popover") => OverlayKind::Popover,
        Some("dropdown") => OverlayKind::Dropdown,
        Some("tooltip") => OverlayKind::Tooltip,
        Some("dialog") => OverlayKind::Dialog,
        Some("menu") => OverlayKind::Menu,
        Some("toast") => OverlayKind::Toast,
        Some(other) => return overlay_config_error(format!("unknown overlay kind `{other}`")),
    };
    let placement = match optional_string(&mut config, "placement")?.as_deref() {
        None | Some("bottom") => OverlayPlacement::Bottom,
        Some("top") => OverlayPlacement::Top,
        Some("left") => OverlayPlacement::Left,
        Some("right") => OverlayPlacement::Right,
        Some("center") => OverlayPlacement::Center,
        Some(other) => {
            return overlay_config_error(format!("unknown overlay placement `{other}`"));
        }
    };
    let open = optional_bool(&mut config, "open")?.unwrap_or(false);
    let gap = optional_number(&mut config, "gap")?.unwrap_or(8.0);
    if !gap.is_finite() || gap < 0.0 {
        return overlay_config_error("overlay gap must be finite and non-negative");
    }
    let modal = optional_bool(&mut config, "modal")?.unwrap_or(kind == OverlayKind::Dialog);
    let dismiss_on_escape = optional_bool(&mut config, "dismiss_on_escape")?.unwrap_or(true);
    let dismiss_on_outside = optional_bool(&mut config, "dismiss_on_outside")?.unwrap_or(true);
    let show_delay_ms = optional_usize(&mut config, "show_delay_ms")?;
    let hide_delay_ms = optional_usize(&mut config, "hide_delay_ms")?;
    let tooltip_delays = (kind == OverlayKind::Tooltip).then(|| TooltipDelays {
        show_ms: u64::try_from(show_delay_ms.unwrap_or(500)).unwrap_or(u64::MAX),
        hide_ms: u64::try_from(hide_delay_ms.unwrap_or(100)).unwrap_or(u64::MAX),
    });
    if let Some((unknown, _)) = config.into_iter().next() {
        return overlay_config_error(format!("unknown overlay config field `{unknown}`"));
    }
    Ok(with_call_source(
        UiNode::overlay(
            trigger,
            overlay_content,
            OverlayNodeSpec {
                id: OverlayId::new(id),
                parent: parent.map(OverlayId::new),
                kind,
                placement,
                open,
                gap,
                modal,
                dismiss: OverlayDismissPolicy {
                    escape: dismiss_on_escape,
                    outside: dismiss_on_outside,
                },
                tooltip_delays,
            },
        ),
        call,
    ))
}

pub(crate) fn dropdown_node(
    call: NativeCallContext<'_>,
    mut config: Map,
) -> Result<UiNode, Box<EvalAltResult>> {
    let id = required_string(&mut config, "id")?;
    let parent_overlay = optional_string(&mut config, "parent_overlay")?;
    let options = parse_dropdown_options(&mut config)?;
    let mode = match optional_string(&mut config, "mode")?.as_deref() {
        None | Some("single") => DropdownMode::Single,
        Some("multiple") => DropdownMode::Multiple,
        Some(other) => return overlay_config_error(format!("unknown dropdown mode `{other}`")),
    };
    let selected = optional_string_array(&mut config, "selected")?;
    let open = optional_bool(&mut config, "open")?;
    let searchable = optional_bool(&mut config, "searchable")?.unwrap_or(false);
    let query = optional_string(&mut config, "query")?;
    let placeholder = optional_string(&mut config, "placeholder")?.unwrap_or_default();
    let search_placeholder =
        optional_string(&mut config, "search_placeholder")?.unwrap_or_default();
    let empty_text = optional_string(&mut config, "empty_text")?.unwrap_or_default();
    let trigger_slot = optional_node(&mut config, "trigger")?;
    let header_slot = optional_node(&mut config, "header")?;
    let footer_slot = optional_node(&mut config, "footer")?;
    let empty_slot = optional_node(&mut config, "empty")?;
    let disabled = optional_bool(&mut config, "disabled")?.unwrap_or(false);
    let max_visible = optional_usize(&mut config, "max_visible")?.unwrap_or(8);
    if !(1..=32).contains(&max_visible) {
        return overlay_config_error("dropdown max_visible must be between 1 and 32");
    }
    let placement = match optional_string(&mut config, "placement")?.as_deref() {
        None | Some("bottom") => OverlayPlacement::Bottom,
        Some("top") => OverlayPlacement::Top,
        Some("left") => OverlayPlacement::Left,
        Some("right") => OverlayPlacement::Right,
        Some(other) => {
            return overlay_config_error(format!("unsupported dropdown placement `{other}`"));
        }
    };
    if let Some((unknown, _)) = config.into_iter().next() {
        return overlay_config_error(format!("unknown dropdown config field `{unknown}`"));
    }
    let spec = DropdownNodeSpec {
        id,
        parent_overlay,
        options,
        mode,
        selected,
        open,
        searchable,
        query,
        placeholder,
        search_placeholder,
        empty_text,
        trigger_slot,
        header_slot,
        footer_slot,
        empty_slot,
        disabled,
        max_visible,
        placement,
    };
    DropdownState::new(
        spec.options.clone(),
        spec.mode,
        spec.selected.clone().unwrap_or_default(),
    )
    .map_err(|error| {
        Box::new(EvalAltResult::ErrorRuntime(
            error.to_string().into(),
            Position::NONE,
        ))
    })?;
    Ok(with_call_source(UiNode::dropdown(spec), call))
}

pub(crate) fn toast_host_node(
    call: NativeCallContext<'_>,
    mut config: Map,
) -> Result<UiNode, Box<EvalAltResult>> {
    let key = required_string(&mut config, "key")?;
    let max_visible = optional_usize(&mut config, "max_visible")?.unwrap_or(3);
    if !(1..=10).contains(&max_visible) {
        return overlay_config_error("toast max_visible must be between 1 and 10");
    }
    let raw_items = config.remove("items").ok_or_else(|| {
        Box::new(EvalAltResult::ErrorRuntime(
            "toast config field `items` is required".into(),
            Position::NONE,
        ))
    })?;
    let raw_items = raw_items
        .try_cast::<Array>()
        .ok_or_else(|| Box::new(overlay_type_error("items", "an array of toast maps")))?;
    if raw_items.len() > 100 {
        return overlay_config_error("toast host cannot contain more than 100 items");
    }
    let mut ids = BTreeSet::new();
    let mut items = Vec::with_capacity(raw_items.len());
    for (index, item) in raw_items.into_iter().enumerate() {
        let mut item = item.try_cast::<Map>().ok_or_else(|| {
            Box::new(EvalAltResult::ErrorRuntime(
                format!("toast item at index {index} must be a map").into(),
                Position::NONE,
            ))
        })?;
        let id = required_string(&mut item, "id")?;
        if !ids.insert(id.clone()) {
            return overlay_config_error(format!("toast ID `{id}` is duplicated"));
        }
        let title = required_string(&mut item, "title")?;
        let message = optional_string(&mut item, "message")?.unwrap_or_default();
        let variant = match optional_string(&mut item, "variant")?.as_deref() {
            None | Some("neutral") => ToastVariant::Neutral,
            Some("success") => ToastVariant::Success,
            Some("warning") => ToastVariant::Warning,
            Some("danger") => ToastVariant::Danger,
            Some(value) => return overlay_config_error(format!("unknown toast variant `{value}`")),
        };
        let region = match optional_string(&mut item, "region")?.as_deref() {
            None | Some("top_right") => ToastRegion::TopRight,
            Some("top_left") => ToastRegion::TopLeft,
            Some("bottom_left") => ToastRegion::BottomLeft,
            Some("bottom_right") => ToastRegion::BottomRight,
            Some(value) => return overlay_config_error(format!("unknown toast region `{value}`")),
        };
        let duration_ms = optional_usize(&mut item, "duration_ms")?.unwrap_or(5_000);
        if duration_ms == 0 {
            return overlay_config_error("toast duration must be greater than zero");
        }
        let paused = optional_bool(&mut item, "paused")?.unwrap_or(false);
        let dismissible = optional_bool(&mut item, "dismissible")?.unwrap_or(true);
        if let Some((unknown, _)) = item.into_iter().next() {
            return overlay_config_error(format!("unknown field `{unknown}` in toast `{id}`"));
        }
        items.push(ToastItemSpec {
            id,
            title,
            message,
            variant,
            region,
            duration_ms: u64::try_from(duration_ms).unwrap_or(u64::MAX),
            paused,
            dismissible,
        });
    }
    if let Some((unknown, _)) = config.into_iter().next() {
        return overlay_config_error(format!("unknown toast host config field `{unknown}`"));
    }
    Ok(with_call_source(
        UiNode::toast_host(ToastHostSpec {
            key,
            items,
            max_visible,
        }),
        call,
    ))
}

pub(crate) fn virtual_list_node(
    call: NativeCallContext<'_>,
    mut config: Map,
) -> Result<UiNode, Box<EvalAltResult>> {
    let key = required_string(&mut config, "key")?;
    let label = optional_string(&mut config, "label")?.unwrap_or_default();
    let row_height = optional_number(&mut config, "row_height")?
        .ok_or_else(|| Box::new(overlay_type_error("row_height", "a positive number")))?;
    let height = optional_number(&mut config, "height")?
        .ok_or_else(|| Box::new(overlay_type_error("height", "a positive number")))?;
    if !row_height.is_finite() || row_height <= 0.0 || !height.is_finite() || height <= 0.0 {
        return overlay_config_error("virtual list row_height and height must be positive");
    }
    let overscan = optional_usize(&mut config, "overscan")?.unwrap_or(2);
    if overscan > 100 {
        return overlay_config_error("virtual list overscan cannot exceed 100");
    }
    let raw_items = config.remove("items").ok_or_else(|| {
        Box::new(EvalAltResult::ErrorRuntime(
            "virtual list config field `items` is required".into(),
            Position::NONE,
        ))
    })?;
    let raw_items = raw_items
        .try_cast::<Array>()
        .ok_or_else(|| Box::new(overlay_type_error("items", "an array of item maps")))?;
    if raw_items.len() > 10_000 {
        return overlay_config_error("virtual list cannot exceed 10,000 items");
    }
    let mut keys = BTreeSet::new();
    let mut items = Vec::with_capacity(raw_items.len());
    for (index, item) in raw_items.into_iter().enumerate() {
        let mut item = item.try_cast::<Map>().ok_or_else(|| {
            Box::new(EvalAltResult::ErrorRuntime(
                format!("virtual list item at index {index} must be a map").into(),
                Position::NONE,
            ))
        })?;
        let item_key = required_string(&mut item, "key")?;
        if !keys.insert(item_key.clone()) {
            return overlay_config_error(format!("virtual list key `{item_key}` is duplicated"));
        }
        let node = item
            .remove("node")
            .and_then(Dynamic::try_cast::<UiNode>)
            .ok_or_else(|| Box::new(overlay_type_error("node", "a UiNode")))?;
        if let Some((unknown, _)) = item.into_iter().next() {
            return overlay_config_error(format!(
                "unknown field `{unknown}` in virtual list item `{item_key}`"
            ));
        }
        items.push(VirtualListItem {
            key: item_key,
            node,
        });
    }
    if let Some((unknown, _)) = config.into_iter().next() {
        return overlay_config_error(format!("unknown virtual list config field `{unknown}`"));
    }
    let node = UiNode::virtual_list(VirtualListNodeSpec {
        key,
        label: label.clone(),
        items,
        row_height,
        height,
        overscan,
    })
    .with_attribute("role", UiValue::String("list".to_owned()))
    .with_attribute("label", UiValue::String(label));
    Ok(with_call_source(node, call))
}

fn with_call_source(node: UiNode, call: NativeCallContext<'_>) -> UiNode {
    let position = call.call_position();
    let module = call
        .call_source()
        .or_else(|| call.fn_source())
        .map(ToOwned::to_owned);
    let _ = std::hint::black_box(call);
    let Some(module) = module else {
        return node;
    };
    let (Some(line), Some(column)) = (position.line(), position.position()) else {
        return node;
    };
    let (Ok(line), Ok(column)) = (u32::try_from(line), u32::try_from(column)) else {
        return node;
    };
    node.with_source(SourceLocation {
        module,
        line,
        column,
    })
}

fn parse_dropdown_options(config: &mut Map) -> Result<Vec<DropdownOption>, Box<EvalAltResult>> {
    let raw = config.remove("options").ok_or_else(|| {
        Box::new(EvalAltResult::ErrorRuntime(
            "dropdown config field `options` is required".into(),
            Position::NONE,
        ))
    })?;
    let options = raw
        .try_cast::<Array>()
        .ok_or_else(|| Box::new(overlay_type_error("options", "an array of option maps")))?;
    if options.len() > 10_000 {
        return overlay_config_error("dropdown options cannot exceed 10,000 items");
    }
    options
        .into_iter()
        .enumerate()
        .map(|(index, option)| {
            let mut option = option.try_cast::<Map>().ok_or_else(|| {
                Box::new(EvalAltResult::ErrorRuntime(
                    format!("dropdown option at index {index} must be a map").into(),
                    Position::NONE,
                ))
            })?;
            let value = required_string(&mut option, "value")?;
            let label = required_string(&mut option, "label")?;
            let keywords = optional_string_array(&mut option, "keywords")?.unwrap_or_default();
            let disabled = optional_bool(&mut option, "disabled")?.unwrap_or(false);
            if let Some((unknown, _)) = option.into_iter().next() {
                return overlay_config_error(format!(
                    "unknown field `{unknown}` in dropdown option `{value}`"
                ));
            }
            Ok(DropdownOption {
                value,
                label,
                keywords,
                disabled,
            })
        })
        .collect()
}

fn required_string(config: &mut Map, name: &str) -> Result<String, Box<EvalAltResult>> {
    optional_string(config, name)?.ok_or_else(|| {
        Box::new(EvalAltResult::ErrorRuntime(
            format!("overlay config field `{name}` is required").into(),
            Position::NONE,
        ))
    })
}

fn optional_string(config: &mut Map, name: &str) -> Result<Option<String>, Box<EvalAltResult>> {
    let Some(value) = config.remove(name) else {
        return Ok(None);
    };
    value
        .try_cast::<ImmutableString>()
        .map(|value| Some(value.to_string()))
        .ok_or_else(|| Box::new(overlay_type_error(name, "string")))
}

fn optional_bool(config: &mut Map, name: &str) -> Result<Option<bool>, Box<EvalAltResult>> {
    let Some(value) = config.remove(name) else {
        return Ok(None);
    };
    value
        .try_cast::<bool>()
        .map(Some)
        .ok_or_else(|| Box::new(overlay_type_error(name, "bool")))
}

fn optional_number(config: &mut Map, name: &str) -> Result<Option<f64>, Box<EvalAltResult>> {
    let Some(value) = config.remove(name) else {
        return Ok(None);
    };
    if value.is::<INT>() {
        value
            .cast::<INT>()
            .to_string()
            .parse::<f64>()
            .map(Some)
            .map_err(|_| Box::new(overlay_type_error(name, "number")))
    } else if value.is::<FLOAT>() {
        Ok(Some(value.cast::<FLOAT>()))
    } else {
        Err(Box::new(overlay_type_error(name, "number")))
    }
}

fn optional_usize(config: &mut Map, name: &str) -> Result<Option<usize>, Box<EvalAltResult>> {
    let Some(value) = config.remove(name) else {
        return Ok(None);
    };
    value
        .try_cast::<INT>()
        .and_then(|value| usize::try_from(value).ok())
        .map(Some)
        .ok_or_else(|| Box::new(overlay_type_error(name, "a non-negative integer")))
}

fn optional_string_array(
    config: &mut Map,
    name: &str,
) -> Result<Option<Vec<String>>, Box<EvalAltResult>> {
    let Some(value) = config.remove(name) else {
        return Ok(None);
    };
    let values = value
        .try_cast::<Array>()
        .ok_or_else(|| Box::new(overlay_type_error(name, "an array of strings")))?;
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .try_cast::<ImmutableString>()
                .map(|value| value.to_string())
                .ok_or_else(|| {
                    Box::new(EvalAltResult::ErrorRuntime(
                        format!("dropdown field `{name}` item {index} must be a string").into(),
                        Position::NONE,
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn optional_node(config: &mut Map, name: &str) -> Result<Option<Box<UiNode>>, Box<EvalAltResult>> {
    let Some(value) = config.remove(name) else {
        return Ok(None);
    };
    value
        .try_cast::<UiNode>()
        .map(|node| Some(Box::new(node)))
        .ok_or_else(|| Box::new(overlay_type_error(name, "a UiNode")))
}

fn overlay_type_error(name: &str, expected: &str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(
        format!("overlay config field `{name}` must be {expected}").into(),
        Position::NONE,
    )
}

fn overlay_config_error<T>(message: impl Into<String>) -> Result<T, Box<EvalAltResult>> {
    Err(Box::new(EvalAltResult::ErrorRuntime(
        message.into().into(),
        Position::NONE,
    )))
}

fn collect_children(
    children: Array,
    constructor: &str,
) -> Result<Vec<UiNode>, Box<rhai::EvalAltResult>> {
    let mut nodes = Vec::with_capacity(children.len());
    for (index, child) in children.into_iter().enumerate() {
        match child.try_cast::<UiNode>() {
            Some(node) => nodes.push(node),
            None => {
                return Err(format!("{constructor} child at index {index} is not a UiNode").into());
            }
        }
    }
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorValue, Length, Rgba8};

    #[test]
    fn node_identity_style_source_and_attributes_are_runtime_owned() {
        let node = UiNode::text("hello")
            .with_key("greeting")
            .with_style(
                &Style::new()
                    .width(Length::pixels(100.0).unwrap())
                    .text_color(ColorValue::Literal(Rgba8::from_rgb_hex(0x00ff_ffff))),
            )
            .with_source(SourceLocation {
                module: "ui/main.rhai".to_owned(),
                line: 4,
                column: 9,
            })
            .with_attribute("role", UiValue::String("label".to_owned()));

        assert_eq!(node.key().map(NodeKey::as_str), Some("greeting"));
        assert_eq!(node.source().unwrap().line, 4);
        assert_eq!(
            node.attributes().get("role"),
            Some(&UiValue::String("label".to_owned()))
        );
    }
}
