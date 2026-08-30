use std::collections::{BTreeMap, BTreeSet};

use rhai::{
    Array, CustomType, Dynamic, EvalAltResult, FLOAT, FnPtr, INT, ImmutableString, Map,
    NativeCallContext, Position, TypeBuilder,
};

use crate::{
    AnimationSpec, AssetId, ComponentInstancePath, HostCallback, OpaqueHandle, OverlayId,
    OverlayKind, OverlayPlacement, PrimitiveNode, ScriptCallback, ScriptGeneration, Style,
    ToastHostSpec, ToastItemSpec, ToastRegion, ToastVariant, UiEventBinding, UiEventHandler,
    UiValue,
};

#[derive(Clone, Debug, PartialEq)]
pub enum ImageSourceSpec {
    Handle(OpaqueHandle),
    Asset(AssetId),
}

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
pub struct Span {
    text: ImmutableString,
    color: Option<crate::ColorValue>,
    bold: bool,
    italic: bool,
}

impl Span {
    #[must_use]
    pub fn new(text: impl Into<ImmutableString>) -> Self {
        Self {
            text: text.into(),
            color: None,
            bold: false,
            italic: false,
        }
    }

    #[must_use]
    pub fn color(mut self, color: crate::ColorValue) -> Self {
        self.color = Some(color);
        self
    }

    #[must_use]
    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    #[must_use]
    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    #[must_use]
    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    #[must_use]
    pub const fn color_value(&self) -> Option<&crate::ColorValue> {
        self.color.as_ref()
    }

    #[must_use]
    pub const fn is_bold(&self) -> bool {
        self.bold
    }

    #[must_use]
    pub const fn is_italic(&self) -> bool {
        self.italic
    }
}

impl CustomType for Span {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("Span")
            .with_fn("color", |span: &mut Self, color: crate::ColorValue| {
                span.clone().color(color)
            })
            .with_fn("bold", |span: &mut Self| span.clone().bold())
            .with_fn("italic", |span: &mut Self| span.clone().italic());
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiNodeKind {
    Text {
        text: ImmutableString,
    },
    RichText {
        text: ImmutableString,
        spans: Vec<Span>,
    },
    Canvas {
        scene: crate::CanvasScene,
    },
    Box {
        children: Vec<UiNode>,
    },
    Fragment {
        children: Vec<UiNode>,
    },
    Custom {
        primitive: PrimitiveNode,
    },
    Image {
        source: ImageSourceSpec,
    },
    DirectionalImage {
        left_to_right: ImageSourceSpec,
        right_to_left: ImageSourceSpec,
    },
    Overlay {
        trigger: Box<UiNode>,
        content: Box<UiNode>,
        spec: OverlayNodeSpec,
    },
    ToastHost {
        spec: ToastHostSpec,
    },
    VirtualCollection {
        spec: crate::VirtualCollectionNodeSpec,
    },
    ErrorBoundary {
        child: Box<UiNode>,
        fallback: Box<UiNode>,
    },
}

/// Stable discriminant used by retained reconciliation without exposing GPUI
/// element types or borrowing a complete [`UiNodeKind`] payload.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UiNodeKindTag {
    Text,
    Canvas,
    Box,
    Fragment,
    Custom,
    Image,
    DirectionalImage,
    Overlay,
    ToastHost,
    VirtualCollection,
    ErrorBoundary,
}

/// A stable declarative UI node. Script-produced nodes contain no GPUI values
/// or lifetimes; trusted Rust Hosts may attach opaque foreground callbacks.
#[derive(Clone, Debug, PartialEq)]
pub struct UiNode {
    kind: UiNodeKind,
    key: Option<NodeKey>,
    style: Style,
    part_styles: BTreeMap<String, Style>,
    source: Option<SourceLocation>,
    component_root: Option<ComponentInstancePath>,
    attributes: BTreeMap<String, UiValue>,
    handlers: BTreeMap<String, Vec<UiEventBinding>>,
    handler_payloads: BTreeMap<String, UiValue>,
    animations: Vec<AnimationSpec>,
    signal_bindings: BTreeMap<crate::SignalProperty, crate::NativeSignal>,
    element_ref: Option<crate::ElementRef>,
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
            component_root: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
        }
    }

    #[must_use]
    pub fn rich_text(spans: Vec<Span>) -> Self {
        let text = spans.iter().map(Span::text).collect::<String>().into();
        Self {
            kind: UiNodeKind::RichText { text, spans },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            component_root: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
        }
    }

    #[must_use]
    pub fn canvas(scene: crate::CanvasScene) -> Self {
        let mut node = Self::text("");
        node.kind = UiNodeKind::Canvas { scene };
        node
    }

    #[must_use]
    pub fn box_node(children: Vec<Self>) -> Self {
        Self {
            kind: UiNodeKind::Box { children },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            component_root: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
        }
    }

    #[must_use]
    pub fn fragment(children: Vec<Self>) -> Self {
        let mut node = Self::box_node(children);
        node.kind = match node.kind {
            UiNodeKind::Box { children } => UiNodeKind::Fragment { children },
            _ => unreachable!(),
        };
        node
    }

    #[must_use]
    pub fn column(children: Vec<Self>) -> Self {
        Self::box_node(children).with_style(&Style::new().flex_col())
    }

    #[must_use]
    pub fn row(children: Vec<Self>) -> Self {
        Self::box_node(children).with_style(&Style::new().flex_row())
    }

    #[must_use]
    pub fn custom(primitive: PrimitiveNode) -> Self {
        Self {
            kind: UiNodeKind::Custom { primitive },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            component_root: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
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
            component_root: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
        }
    }

    #[must_use]
    pub fn image(handle: OpaqueHandle) -> Self {
        Self::image_source(ImageSourceSpec::Handle(handle))
    }

    #[must_use]
    pub fn asset_image(asset: AssetId) -> Self {
        Self::image_source(ImageSourceSpec::Asset(asset))
    }

    fn image_source(source: ImageSourceSpec) -> Self {
        Self {
            kind: UiNodeKind::Image { source },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            component_root: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
        }
    }

    #[must_use]
    pub fn directional_image(left_to_right: OpaqueHandle, right_to_left: OpaqueHandle) -> Self {
        Self::directional_image_sources(
            ImageSourceSpec::Handle(left_to_right),
            ImageSourceSpec::Handle(right_to_left),
        )
    }

    #[must_use]
    pub fn directional_asset_image(left_to_right: AssetId, right_to_left: AssetId) -> Self {
        Self::directional_image_sources(
            ImageSourceSpec::Asset(left_to_right),
            ImageSourceSpec::Asset(right_to_left),
        )
    }

    fn directional_image_sources(
        left_to_right: ImageSourceSpec,
        right_to_left: ImageSourceSpec,
    ) -> Self {
        Self {
            kind: UiNodeKind::DirectionalImage {
                left_to_right,
                right_to_left,
            },
            key: None,
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            component_root: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
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
            component_root: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
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
            component_root: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
        }
    }

    #[must_use]
    pub fn virtual_collection(spec: crate::VirtualCollectionNodeSpec) -> Self {
        let key = spec.id.key.clone();
        Self {
            kind: UiNodeKind::VirtualCollection { spec },
            key: Some(NodeKey::new(key)),
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            component_root: None,
            attributes: BTreeMap::new(),
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
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

    /// Bind one approved hot property to a component-scoped native signal.
    ///
    /// # Errors
    ///
    /// Returns a property/type mismatch without changing the node.
    pub fn with_signal_binding(
        mut self,
        property: crate::SignalProperty,
        signal: crate::NativeSignal,
    ) -> Result<Self, crate::SignalError> {
        let expected = property.signal_kind();
        let actual = signal.id().kind();
        if expected != actual {
            return Err(crate::SignalError::InvalidBinding {
                property,
                expected,
                actual,
            });
        }
        self.signal_bindings.insert(property, signal);
        Ok(self)
    }

    #[must_use]
    pub fn with_element_ref(mut self, reference: crate::ElementRef) -> Self {
        self.element_ref = Some(reference);
        self
    }

    #[must_use]
    pub fn with_part_styles(mut self, styles: BTreeMap<String, Style>) -> Self {
        self.part_styles.extend(styles);
        self
    }

    #[must_use]
    pub fn with_part_style(mut self, part: impl Into<String>, style: Style) -> Self {
        self.part_styles.insert(part.into(), style);
        self
    }

    #[must_use]
    pub fn with_source(mut self, source: SourceLocation) -> Self {
        self.source = Some(source);
        self
    }

    pub(crate) fn with_component_root(mut self, component: ComponentInstancePath) -> Self {
        self.component_root = Some(component);
        self
    }

    #[must_use]
    pub fn component_root(&self) -> Option<&ComponentInstancePath> {
        self.component_root.as_ref()
    }

    pub(crate) fn replace_component_subtree(
        &mut self,
        component: &ComponentInstancePath,
        replacement: Self,
    ) -> bool {
        if self.component_root.as_ref() == Some(component) {
            *self = replacement;
            return true;
        }
        match &mut self.kind {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                replace_in_nodes(children.iter_mut(), component, &replacement)
            }
            UiNodeKind::Custom { primitive } => {
                for (_, value) in primitive.props.iter_mut() {
                    match value {
                        crate::PrimitiveValue::Node(node) => {
                            if node.replace_component_subtree(component, replacement.clone()) {
                                return true;
                            }
                        }
                        crate::PrimitiveValue::Nodes(nodes) => {
                            if replace_in_nodes(nodes.iter_mut(), component, &replacement) {
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
                false
            }
            UiNodeKind::Overlay {
                trigger, content, ..
            }
            | UiNodeKind::ErrorBoundary {
                child: trigger,
                fallback: content,
            } => {
                trigger.replace_component_subtree(component, replacement.clone())
                    || content.replace_component_subtree(component, replacement)
            }
            UiNodeKind::VirtualCollection { spec } => {
                replace_in_nodes(spec.realized.values_mut(), component, &replacement)
            }
            UiNodeKind::Text { .. }
            | UiNodeKind::RichText { .. }
            | UiNodeKind::Canvas { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. }
            | UiNodeKind::ToastHost { .. } => false,
        }
    }

    pub(crate) fn virtual_collection_items(
        &self,
        id: &crate::VirtualCollectionId,
    ) -> Option<&BTreeMap<usize, UiNode>> {
        match &self.kind {
            UiNodeKind::VirtualCollection { spec } if &spec.id == id => Some(&spec.realized),
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => children
                .iter()
                .find_map(|child| child.virtual_collection_items(id)),
            UiNodeKind::Overlay {
                trigger, content, ..
            }
            | UiNodeKind::ErrorBoundary {
                child: trigger,
                fallback: content,
            } => trigger
                .virtual_collection_items(id)
                .or_else(|| content.virtual_collection_items(id)),
            _ => None,
        }
    }

    pub(crate) fn replace_virtual_collection_items(
        &mut self,
        id: &crate::VirtualCollectionId,
        items: BTreeMap<usize, UiNode>,
    ) -> bool {
        match &mut self.kind {
            UiNodeKind::VirtualCollection { spec } if &spec.id == id => {
                spec.realized = items;
                true
            }
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => children
                .iter_mut()
                .any(|child| child.replace_virtual_collection_items(id, items.clone())),
            UiNodeKind::Overlay {
                trigger, content, ..
            }
            | UiNodeKind::ErrorBoundary {
                child: trigger,
                fallback: content,
            } => {
                trigger.replace_virtual_collection_items(id, items.clone())
                    || content.replace_virtual_collection_items(id, items)
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn with_attribute(mut self, name: impl Into<String>, value: UiValue) -> Self {
        self.attributes.insert(name.into(), value);
        self
    }

    #[must_use]
    pub fn with_handler(
        mut self,
        event: impl Into<String>,
        handler: impl Into<UiEventHandler>,
    ) -> Self {
        self.handlers
            .entry(event.into())
            .or_default()
            .push(UiEventBinding::new(crate::EventPhase::Target, handler));
        self
    }

    #[must_use]
    pub fn with_handler_phase(
        mut self,
        event: impl Into<String>,
        phase: crate::EventPhase,
        handler: impl Into<UiEventHandler>,
    ) -> Self {
        self.handlers
            .entry(event.into())
            .or_default()
            .push(UiEventBinding::new(phase, handler));
        self
    }

    #[must_use]
    pub fn with_host_handler(self, event: impl Into<String>, callback: HostCallback) -> Self {
        self.with_handler(event, UiEventHandler::Host(callback))
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
    pub fn handlers(&self) -> &BTreeMap<String, Vec<UiEventBinding>> {
        &self.handlers
    }

    #[must_use]
    pub fn handler(&self, event: &str) -> Option<&UiEventHandler> {
        self.handlers
            .get(event)?
            .iter()
            .find(|binding| binding.phase() == crate::EventPhase::Target)
            .map(UiEventBinding::handler)
    }

    #[must_use]
    pub fn event_handlers(&self, event: &str) -> &[UiEventBinding] {
        self.handlers.get(event).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn handler_payload(&self, event: &str) -> Option<&UiValue> {
        self.handler_payloads.get(event)
    }

    #[must_use]
    pub(crate) const fn handler_payloads(&self) -> &BTreeMap<String, UiValue> {
        &self.handler_payloads
    }

    #[must_use]
    pub fn animations(&self) -> &[AnimationSpec] {
        &self.animations
    }

    pub(crate) fn bind_generation(&mut self, generation: ScriptGeneration) {
        for bindings in self.handlers.values_mut() {
            for binding in bindings {
                if let Some(callback) = binding.handler_mut().as_script_mut() {
                    callback.bind_generation(generation);
                }
            }
        }
        match &mut self.kind {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
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
            | UiNodeKind::RichText { .. }
            | UiNodeKind::Canvas { .. }
            | UiNodeKind::Custom { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. }
            | UiNodeKind::ToastHost { .. } => {}
            UiNodeKind::VirtualCollection { spec } => {
                for item in spec.realized.values_mut() {
                    item.bind_generation(generation);
                }
            }
        }
    }

    pub(crate) fn bind_component_scope(
        &mut self,
        component: &crate::ComponentInstancePath,
        events: &BTreeMap<String, crate::EventSchema>,
        native_context: Option<&crate::invocation::ScriptInvocationContext>,
    ) {
        for bindings in self.handlers.values_mut() {
            for binding in bindings {
                if let Some(callback) = binding.handler_mut().as_script_mut() {
                    callback.bind_component_if_unset(component.clone(), events.clone());
                    if let Some(context) = native_context {
                        callback.bind_native_context_if_unset(context.clone());
                    }
                }
            }
        }
        match &mut self.kind {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
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
            UiNodeKind::VirtualCollection { spec } => {
                for item in spec.realized.values_mut() {
                    item.bind_component_scope(component, events, native_context);
                }
            }
            UiNodeKind::Text { .. }
            | UiNodeKind::RichText { .. }
            | UiNodeKind::Canvas { .. }
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
        native_context: Option<&crate::invocation::ScriptInvocationContext>,
    ) {
        for bindings in self.handlers.values_mut() {
            for binding in bindings {
                if let Some(callback) = binding.handler_mut().as_script_mut()
                    && names.contains(callback.name())
                {
                    callback.bind_component_if_unset(component.clone(), events.clone());
                    if let (Some(context), None) = (native_context, callback.native_context()) {
                        callback.bind_native_context_if_unset(context.clone());
                    }
                }
            }
        }
        match &mut self.kind {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
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
            UiNodeKind::VirtualCollection { spec } => {
                for item in spec.realized.values_mut() {
                    item.bind_callback_scope_by_name(names, component, events, native_context);
                }
            }
            UiNodeKind::Text { .. }
            | UiNodeKind::RichText { .. }
            | UiNodeKind::Canvas { .. }
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
    pub const fn kind_tag(&self) -> UiNodeKindTag {
        match self.kind {
            UiNodeKind::Text { .. } | UiNodeKind::RichText { .. } => UiNodeKindTag::Text,
            UiNodeKind::Canvas { .. } => UiNodeKindTag::Canvas,
            UiNodeKind::Box { .. } => UiNodeKindTag::Box,
            UiNodeKind::Fragment { .. } => UiNodeKindTag::Fragment,
            UiNodeKind::Custom { .. } => UiNodeKindTag::Custom,
            UiNodeKind::Image { .. } => UiNodeKindTag::Image,
            UiNodeKind::DirectionalImage { .. } => UiNodeKindTag::DirectionalImage,
            UiNodeKind::Overlay { .. } => UiNodeKindTag::Overlay,
            UiNodeKind::ToastHost { .. } => UiNodeKindTag::ToastHost,
            UiNodeKind::VirtualCollection { .. } => UiNodeKindTag::VirtualCollection,
            UiNodeKind::ErrorBoundary { .. } => UiNodeKindTag::ErrorBoundary,
        }
    }

    pub(crate) fn retained_child_groups(&self) -> Vec<(String, Vec<&Self>)> {
        match &self.kind {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                vec![("children".to_owned(), children.iter().collect())]
            }
            UiNodeKind::Custom { primitive } => primitive
                .props
                .iter()
                .filter_map(|(name, value)| match value {
                    crate::PrimitiveValue::Node(node) => {
                        Some((format!("prop:{name}"), vec![node.as_ref()]))
                    }
                    crate::PrimitiveValue::Nodes(nodes) => {
                        Some((format!("prop:{name}"), nodes.iter().collect::<Vec<_>>()))
                    }
                    crate::PrimitiveValue::Data(_)
                    | crate::PrimitiveValue::Callback(_)
                    | crate::PrimitiveValue::Style(_)
                    | crate::PrimitiveValue::Length(_)
                    | crate::PrimitiveValue::Asset(_) => None,
                })
                .collect(),
            UiNodeKind::Overlay {
                trigger, content, ..
            } => vec![
                ("trigger".to_owned(), vec![trigger.as_ref()]),
                ("content".to_owned(), vec![content.as_ref()]),
            ],
            UiNodeKind::VirtualCollection { spec } => {
                vec![("items".to_owned(), spec.realized.values().collect())]
            }
            UiNodeKind::ErrorBoundary { child, fallback } => vec![
                ("child".to_owned(), vec![child.as_ref()]),
                ("fallback".to_owned(), vec![fallback.as_ref()]),
            ],
            UiNodeKind::Text { .. }
            | UiNodeKind::RichText { .. }
            | UiNodeKind::Canvas { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. }
            | UiNodeKind::ToastHost { .. } => Vec::new(),
        }
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

    pub fn signal_bindings(
        &self,
    ) -> impl ExactSizeIterator<Item = (crate::SignalProperty, &crate::NativeSignal)> {
        self.signal_bindings
            .iter()
            .map(|(property, signal)| (*property, signal))
    }

    #[must_use]
    pub const fn element_ref(&self) -> Option<&crate::ElementRef> {
        self.element_ref.as_ref()
    }
}

fn replace_in_nodes<'a>(
    nodes: impl IntoIterator<Item = &'a mut UiNode>,
    component: &ComponentInstancePath,
    replacement: &UiNode,
) -> bool {
    for node in nodes {
        if node.replace_component_subtree(component, replacement.clone()) {
            return true;
        }
    }
    false
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
            .with_fn(
                "bind_signal",
                |node: &mut Self,
                 property: ImmutableString,
                 signal: crate::NativeSignal|
                 -> Result<Self, Box<EvalAltResult>> {
                    let property = crate::SignalProperty::parse(property.as_str())
                        .map_err(|error| Box::new(crate::signal::signal_runtime_error(&error)))?;
                    node.clone()
                        .with_signal_binding(property, signal)
                        .map_err(|error| Box::new(crate::signal::signal_runtime_error(&error)))
                },
            )
            .with_fn(
                "with_ref",
                |node: &mut Self, reference: crate::ElementRef| {
                    node.clone().with_element_ref(reference)
                },
            )
            .with_fn(
                "with_part_style",
                |node: &mut Self, part: ImmutableString, style: Style| {
                    node.clone().with_part_style(part.to_string(), style)
                },
            )
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
                        .with_handler("click", retained_script_callback(callback)?)
                        .with_handler_payload("click", payload))
                },
            );
        register_raw_event_methods(&mut builder);
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
                        .with_handler(event.clone(), retained_script_callback(callback)?)
                        .with_handler_payload(event, payload))
                },
            );
        register_accessibility_methods(&mut builder);
    }
}

fn register_raw_event_methods(builder: &mut TypeBuilder<UiNode>) {
    builder
        .with_fn(
            "on_click",
            |node: &mut UiNode, callback: FnPtr| -> Result<UiNode, Box<EvalAltResult>> {
                Ok(node
                    .clone()
                    .with_handler("click", retained_script_callback(callback)?))
            },
        )
        .with_fn(
            "on",
            |node: &mut UiNode,
             event: ImmutableString,
             callback: FnPtr|
             -> Result<UiNode, Box<EvalAltResult>> {
                validate_node_event_name(event.as_str())?;
                Ok(node
                    .clone()
                    .with_handler(event.to_string(), retained_script_callback(callback)?))
            },
        )
        .with_fn(
            "on_capture",
            |node: &mut UiNode,
             event: ImmutableString,
             callback: FnPtr|
             -> Result<UiNode, Box<EvalAltResult>> {
                validate_node_event_name(event.as_str())?;
                Ok(node.clone().with_handler_phase(
                    event.to_string(),
                    crate::EventPhase::Capture,
                    retained_script_callback(callback)?,
                ))
            },
        )
        .with_fn(
            "on_bubble",
            |node: &mut UiNode,
             event: ImmutableString,
             callback: FnPtr|
             -> Result<UiNode, Box<EvalAltResult>> {
                validate_node_event_name(event.as_str())?;
                Ok(node.clone().with_handler_phase(
                    event.to_string(),
                    crate::EventPhase::Bubble,
                    retained_script_callback(callback)?,
                ))
            },
        );
    register_native_event_methods(builder);
}

fn register_native_event_methods(builder: &mut TypeBuilder<UiNode>) {
    builder
        .with_fn(
            "on",
            |node: &mut UiNode,
             event: ImmutableString,
             handler: crate::NativeHandlerRef|
             -> Result<UiNode, Box<EvalAltResult>> {
                validate_node_event_name(event.as_str())?;
                handler.validate_event(event.as_str()).map_err(|error| {
                    Box::new(EvalAltResult::ErrorRuntime(
                        error.to_string().into(),
                        Position::NONE,
                    ))
                })?;
                Ok(node.clone().with_handler(event.to_string(), handler))
            },
        )
        .with_fn(
            "on_capture",
            |node: &mut UiNode,
             event: ImmutableString,
             handler: crate::NativeHandlerRef|
             -> Result<UiNode, Box<EvalAltResult>> {
                validate_node_event_name(event.as_str())?;
                handler.validate_event(event.as_str()).map_err(|error| {
                    Box::new(EvalAltResult::ErrorRuntime(
                        error.to_string().into(),
                        Position::NONE,
                    ))
                })?;
                Ok(node.clone().with_handler_phase(
                    event.to_string(),
                    crate::EventPhase::Capture,
                    handler,
                ))
            },
        )
        .with_fn(
            "on_bubble",
            |node: &mut UiNode,
             event: ImmutableString,
             handler: crate::NativeHandlerRef|
             -> Result<UiNode, Box<EvalAltResult>> {
                validate_node_event_name(event.as_str())?;
                handler.validate_event(event.as_str()).map_err(|error| {
                    Box::new(EvalAltResult::ErrorRuntime(
                        error.to_string().into(),
                        Position::NONE,
                    ))
                })?;
                Ok(node.clone().with_handler_phase(
                    event.to_string(),
                    crate::EventPhase::Bubble,
                    handler,
                ))
            },
        );
}

fn validate_node_event_name(event: &str) -> Result<(), Box<EvalAltResult>> {
    let valid = (1..=64).contains(&event.len())
        && event.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '_' | ':')
        });
    if valid {
        Ok(())
    } else {
        Err(Box::new(EvalAltResult::ErrorRuntime(
            format!("event `{event}` must be a 1-64 character snake_case or namespaced name")
                .into(),
            Position::NONE,
        )))
    }
}

fn register_semantic_event_methods(builder: &mut TypeBuilder<UiNode>) {
    for (method, event) in [
        ("on_open_change", "open_change"),
        ("on_change", "change"),
        ("on_query_change", "query_change"),
        ("on_dismiss", "dismiss"),
        ("on_sort_change", "sort_change"),
        ("on_selection_change", "selection_change"),
        ("on_row_click", "row_click"),
    ] {
        let event = event.to_owned();
        builder.with_fn(
            method,
            move |node: &mut UiNode, callback: FnPtr| -> Result<UiNode, Box<EvalAltResult>> {
                Ok(node
                    .clone()
                    .with_handler(event.clone(), retained_script_callback(callback)?))
            },
        );
    }
}

fn retained_script_callback(callback: FnPtr) -> Result<ScriptCallback, Box<EvalAltResult>> {
    ScriptCallback::try_from_fn_ptr(callback, ScriptGeneration::default()).map_err(|error| {
        Box::new(EvalAltResult::ErrorRuntime(
            error.to_string().into(),
            Position::NONE,
        ))
    })
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

pub(crate) fn span_value(text: ImmutableString) -> Span {
    Span::new(text)
}

pub(crate) fn rich_text_node(
    call: NativeCallContext<'_>,
    spans: Array,
) -> Result<UiNode, Box<EvalAltResult>> {
    let spans = spans
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            value.try_cast::<Span>().ok_or_else(|| {
                Box::new(EvalAltResult::ErrorRuntime(
                    format!("text span {index} must be Span").into(),
                    Position::NONE,
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(with_call_source(UiNode::rich_text(spans), call))
}

pub(crate) fn canvas_node(call: NativeCallContext<'_>, scene: crate::CanvasScene) -> UiNode {
    with_call_source(UiNode::canvas(scene), call)
}

pub(crate) fn box_node(
    call: NativeCallContext<'_>,
    children: Array,
) -> Result<UiNode, Box<rhai::EvalAltResult>> {
    collect_children(children, "box")
        .map(|children| with_call_source(UiNode::box_node(children), call))
}

pub(crate) fn fragment_node(
    call: NativeCallContext<'_>,
    children: Array,
) -> Result<UiNode, Box<rhai::EvalAltResult>> {
    collect_children(children, "fragment")
        .map(|children| with_call_source(UiNode::fragment(children), call))
}

pub(crate) fn stack_node(
    call: NativeCallContext<'_>,
    children: Array,
) -> Result<UiNode, Box<rhai::EvalAltResult>> {
    collect_children(children, "stack").map(|children| {
        with_call_source(
            UiNode::box_node(children).with_style(&Style::new().relative()),
            call,
        )
    })
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

pub(crate) fn asset_image_node(call: NativeCallContext<'_>, asset: AssetId) -> UiNode {
    with_call_source(UiNode::asset_image(asset), call)
}

pub(crate) fn generic_image_node(
    call: NativeCallContext<'_>,
    source: Dynamic,
) -> Result<UiNode, Box<EvalAltResult>> {
    Ok(with_call_source(
        UiNode::image_source(parse_image_source(source)?),
        call,
    ))
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

pub(crate) fn directional_asset_image_node(
    call: NativeCallContext<'_>,
    left_to_right: AssetId,
    right_to_left: AssetId,
) -> UiNode {
    with_call_source(
        UiNode::directional_asset_image(left_to_right, right_to_left),
        call,
    )
}

pub(crate) fn generic_directional_image_node(
    call: NativeCallContext<'_>,
    left_to_right: Dynamic,
    right_to_left: Dynamic,
) -> Result<UiNode, Box<EvalAltResult>> {
    Ok(with_call_source(
        UiNode::directional_image_sources(
            parse_image_source(left_to_right)?,
            parse_image_source(right_to_left)?,
        ),
        call,
    ))
}

fn parse_image_source(source: Dynamic) -> Result<ImageSourceSpec, Box<EvalAltResult>> {
    if source.is::<AssetId>() {
        Ok(ImageSourceSpec::Asset(source.cast::<AssetId>()))
    } else if source.is::<OpaqueHandle>() {
        let handle = source.cast::<OpaqueHandle>();
        if handle.kind() == "image" {
            Ok(ImageSourceSpec::Handle(handle))
        } else {
            Err(Box::new(EvalAltResult::ErrorRuntime(
                format!(
                    "image source handle must have kind `image`, got `{}`",
                    handle.kind()
                )
                .into(),
                Position::NONE,
            )))
        }
    } else {
        Err(Box::new(EvalAltResult::ErrorRuntime(
            format!(
                "image source must be an AssetId or image handle, got {}",
                source.type_name()
            )
            .into(),
            Position::NONE,
        )))
    }
}

pub(crate) fn overlay_node(
    call: NativeCallContext<'_>,
    trigger: UiNode,
    overlay_content: UiNode,
    mut config: Map,
) -> Result<UiNode, Box<EvalAltResult>> {
    let id = required_string(&mut config, "id")?;
    if id.trim().is_empty() {
        return overlay_config_error("overlay ID cannot be empty");
    }
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
