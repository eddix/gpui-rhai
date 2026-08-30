use std::collections::{BTreeMap, BTreeSet};

use rhai::{
    Array, CustomType, Dynamic, EvalAltResult, FLOAT, FnPtr, INT, ImmutableString, Map,
    NativeCallContext, Position, TypeBuilder,
};

use crate::{
    AnimationSpec, AssetId, CalendarMetadata, ChoiceBehavior, ComponentInstancePath,
    DatePickerNodeSpec, DatePickerPreset, DropdownMode, DropdownNodeSpec, DropdownOption,
    DropdownState, GregorianDate, HostCallback, NumberMetadata, OpaqueHandle, OverlayId,
    OverlayKind, OverlayPlacement, PrimitiveNode, ScriptCallback, ScriptGeneration, SelectNodeSpec,
    Style, TableAlign, TableCellFormat, TableColumnSpec, TableColumnWidth, TableNodeSpec,
    TableRowSpec, TableSelectionMode, TableSort, TableSortDirection, ToastHostSpec, ToastItemSpec,
    ToastRegion, ToastVariant, UiEventBinding, UiEventHandler, UiValue, VirtualListItem,
    VirtualListNodeSpec,
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
pub enum UiNodeKind {
    Text {
        text: ImmutableString,
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
    Dropdown {
        spec: DropdownNodeSpec,
    },
    Select {
        spec: SelectNodeSpec,
    },
    DatePicker {
        spec: Box<DatePickerNodeSpec>,
    },
    Table {
        spec: Box<TableNodeSpec>,
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

/// Stable discriminant used by retained reconciliation without exposing GPUI
/// element types or borrowing a complete [`UiNodeKind`] payload.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UiNodeKindTag {
    Text,
    Box,
    Fragment,
    Custom,
    Image,
    DirectionalImage,
    Overlay,
    Dropdown,
    Select,
    DatePicker,
    Table,
    ToastHost,
    VirtualList,
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
    pub fn dropdown(spec: DropdownNodeSpec) -> Self {
        let key = spec.id.clone();
        Self {
            kind: UiNodeKind::Dropdown { spec },
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
    pub fn select(spec: SelectNodeSpec) -> Self {
        let key = spec.choice.id.clone();
        let attributes = BTreeMap::from([
            ("role".to_owned(), UiValue::String("combobox".to_owned())),
            (
                "option_count".to_owned(),
                UiValue::Integer(INT::try_from(spec.choice.options.len()).unwrap_or(INT::MAX)),
            ),
        ]);
        Self {
            kind: UiNodeKind::Select { spec },
            key: Some(NodeKey::new(key)),
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            component_root: None,
            attributes,
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
        }
    }

    #[must_use]
    pub fn date_picker(spec: DatePickerNodeSpec) -> Self {
        let key = spec.id.clone();
        let attributes = BTreeMap::from([
            ("role".to_owned(), UiValue::String("combobox".to_owned())),
            (
                "calendar_open_label".to_owned(),
                UiValue::String(spec.open_label.clone()),
            ),
            (
                "calendar_previous_label".to_owned(),
                UiValue::String(spec.previous_label.clone()),
            ),
            (
                "calendar_next_label".to_owned(),
                UiValue::String(spec.next_label.clone()),
            ),
            (
                "calendar_clear_label".to_owned(),
                UiValue::String(spec.clear_label.clone()),
            ),
            ("row_count".to_owned(), UiValue::Integer(6)),
            ("column_count".to_owned(), UiValue::Integer(7)),
        ]);
        Self {
            kind: UiNodeKind::DatePicker {
                spec: Box::new(spec),
            },
            key: Some(NodeKey::new(key)),
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            component_root: None,
            attributes,
            handlers: BTreeMap::new(),
            handler_payloads: BTreeMap::new(),
            animations: Vec::new(),
            signal_bindings: BTreeMap::new(),
            element_ref: None,
        }
    }

    #[must_use]
    pub fn table(spec: TableNodeSpec) -> Self {
        let key = spec.key.clone();
        let attributes = BTreeMap::from([
            ("role".to_owned(), UiValue::String("table".to_owned())),
            (
                "row_count".to_owned(),
                UiValue::Integer(INT::try_from(spec.rows.len()).unwrap_or(INT::MAX)),
            ),
            (
                "column_count".to_owned(),
                UiValue::Integer(INT::try_from(spec.columns.len()).unwrap_or(INT::MAX)),
            ),
        ]);
        Self {
            kind: UiNodeKind::Table {
                spec: Box::new(spec),
            },
            key: Some(NodeKey::new(key)),
            style: Style::new(),
            part_styles: BTreeMap::new(),
            source: None,
            component_root: None,
            attributes,
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
    pub fn virtual_list(spec: VirtualListNodeSpec) -> Self {
        let key = spec.key.clone();
        Self {
            kind: UiNodeKind::VirtualList { spec },
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
            UiNodeKind::Dropdown { spec } => replace_in_optional_nodes(
                [
                    &mut spec.trigger_slot,
                    &mut spec.header_slot,
                    &mut spec.footer_slot,
                    &mut spec.empty_slot,
                ],
                component,
                &replacement,
            ),
            UiNodeKind::Select { spec } => replace_in_optional_nodes(
                [
                    &mut spec.choice.trigger_slot,
                    &mut spec.choice.header_slot,
                    &mut spec.choice.footer_slot,
                    &mut spec.choice.empty_slot,
                ],
                component,
                &replacement,
            ),
            UiNodeKind::Table { spec } => {
                for column in &mut spec.columns {
                    if let Some(cells) = &mut column.custom_cells
                        && replace_in_nodes(cells.iter_mut(), component, &replacement)
                    {
                        return true;
                    }
                }
                if replace_in_optional_nodes(
                    [&mut spec.loading_slot, &mut spec.empty_slot],
                    component,
                    &replacement,
                ) {
                    return true;
                }
                replace_in_nodes(spec.loading_rows.iter_mut(), component, &replacement)
            }
            UiNodeKind::VirtualList { spec } => replace_in_nodes(
                spec.items.iter_mut().map(|item| &mut item.node),
                component,
                &replacement,
            ),
            UiNodeKind::Text { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. }
            | UiNodeKind::DatePicker { .. }
            | UiNodeKind::ToastHost { .. } => false,
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
            | UiNodeKind::Custom { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. }
            | UiNodeKind::Select { .. }
            | UiNodeKind::DatePicker { .. }
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
            UiNodeKind::Table { spec } => {
                for column in &mut spec.columns {
                    for cell in column.custom_cells.iter_mut().flatten() {
                        cell.bind_generation(generation);
                    }
                }
                for slot in [&mut spec.loading_slot, &mut spec.empty_slot]
                    .into_iter()
                    .flatten()
                {
                    slot.bind_generation(generation);
                }
                for row in &mut spec.loading_rows {
                    row.bind_generation(generation);
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
            UiNodeKind::Table { spec } => {
                for column in &mut spec.columns {
                    for cell in column.custom_cells.iter_mut().flatten() {
                        cell.bind_component_scope(component, events, native_context);
                    }
                }
                for slot in [&mut spec.loading_slot, &mut spec.empty_slot]
                    .into_iter()
                    .flatten()
                {
                    slot.bind_component_scope(component, events, native_context);
                }
                for row in &mut spec.loading_rows {
                    row.bind_component_scope(component, events, native_context);
                }
            }
            UiNodeKind::Text { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. }
            | UiNodeKind::Select { .. }
            | UiNodeKind::DatePicker { .. }
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
            UiNodeKind::Table { spec } => {
                for column in &mut spec.columns {
                    for cell in column.custom_cells.iter_mut().flatten() {
                        cell.bind_callback_scope_by_name(names, component, events, native_context);
                    }
                }
                for slot in [&mut spec.loading_slot, &mut spec.empty_slot]
                    .into_iter()
                    .flatten()
                {
                    slot.bind_callback_scope_by_name(names, component, events, native_context);
                }
                for row in &mut spec.loading_rows {
                    row.bind_callback_scope_by_name(names, component, events, native_context);
                }
            }
            UiNodeKind::Text { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. }
            | UiNodeKind::Select { .. }
            | UiNodeKind::DatePicker { .. }
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
            UiNodeKind::Text { .. } => UiNodeKindTag::Text,
            UiNodeKind::Box { .. } => UiNodeKindTag::Box,
            UiNodeKind::Fragment { .. } => UiNodeKindTag::Fragment,
            UiNodeKind::Custom { .. } => UiNodeKindTag::Custom,
            UiNodeKind::Image { .. } => UiNodeKindTag::Image,
            UiNodeKind::DirectionalImage { .. } => UiNodeKindTag::DirectionalImage,
            UiNodeKind::Overlay { .. } => UiNodeKindTag::Overlay,
            UiNodeKind::Dropdown { .. } => UiNodeKindTag::Dropdown,
            UiNodeKind::Select { .. } => UiNodeKindTag::Select,
            UiNodeKind::DatePicker { .. } => UiNodeKindTag::DatePicker,
            UiNodeKind::Table { .. } => UiNodeKindTag::Table,
            UiNodeKind::ToastHost { .. } => UiNodeKindTag::ToastHost,
            UiNodeKind::VirtualList { .. } => UiNodeKindTag::VirtualList,
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
            UiNodeKind::Dropdown { spec } => dropdown_child_groups(spec),
            UiNodeKind::Select { spec } => dropdown_child_groups(&spec.choice),
            UiNodeKind::Table { spec } => {
                let mut groups = spec
                    .columns
                    .iter()
                    .filter_map(|column| {
                        column.custom_cells.as_ref().map(|cells| {
                            (
                                format!("column:{}:cells", column.key),
                                cells.iter().collect::<Vec<_>>(),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                if let Some(slot) = spec.loading_slot.as_deref() {
                    groups.push(("loading-slot".to_owned(), vec![slot]));
                }
                if !spec.loading_rows.is_empty() {
                    groups.push((
                        "loading-rows".to_owned(),
                        spec.loading_rows.iter().collect(),
                    ));
                }
                if let Some(slot) = spec.empty_slot.as_deref() {
                    groups.push(("empty-slot".to_owned(), vec![slot]));
                }
                groups
            }
            UiNodeKind::VirtualList { spec } => vec![(
                "items".to_owned(),
                spec.items.iter().map(|item| &item.node).collect(),
            )],
            UiNodeKind::ErrorBoundary { child, fallback } => vec![
                ("child".to_owned(), vec![child.as_ref()]),
                ("fallback".to_owned(), vec![fallback.as_ref()]),
            ],
            UiNodeKind::Text { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. }
            | UiNodeKind::DatePicker { .. }
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

fn replace_in_optional_nodes<const N: usize>(
    nodes: [&mut Option<Box<UiNode>>; N],
    component: &ComponentInstancePath,
    replacement: &UiNode,
) -> bool {
    replace_in_nodes(
        nodes.into_iter().filter_map(Option::as_deref_mut),
        component,
        replacement,
    )
}

fn dropdown_child_groups(spec: &DropdownNodeSpec) -> Vec<(String, Vec<&UiNode>)> {
    [
        ("trigger-slot", spec.trigger_slot.as_deref()),
        ("header-slot", spec.header_slot.as_deref()),
        ("footer-slot", spec.footer_slot.as_deref()),
        ("empty-slot", spec.empty_slot.as_deref()),
    ]
    .into_iter()
    .filter_map(|(name, node)| node.map(|node| (name.to_owned(), vec![node])))
    .collect()
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
    collect_children(children, "stack")
        .map(|children| with_call_source(UiNode::box_node(children), call))
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

pub(crate) fn dropdown_node(
    call: NativeCallContext<'_>,
    mut config: Map,
) -> Result<UiNode, Box<EvalAltResult>> {
    let id = required_string(&mut config, "id")?;
    if id.trim().is_empty() {
        return overlay_config_error("dropdown ID cannot be empty");
    }
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
    let clearable = optional_bool(&mut config, "clearable")?.unwrap_or(false);
    let reset_query_on_close = optional_bool(&mut config, "reset_query_on_close")?.unwrap_or(false);
    let row_height = positive_config_number(&mut config, "row_height", 32.0)?;
    let trigger_height = positive_config_number(&mut config, "trigger_height", 32.0)?;
    let trigger_width = required_length(&mut config, "trigger_width")?;
    let panel_width = positive_config_number(&mut config, "panel_width", 280.0)?;
    let panel_extra_height = optional_nonnegative_number(&mut config, "panel_extra_height")?;
    let overlay_gap = optional_nonnegative_number(&mut config, "overlay_gap")?.unwrap_or(0.0);
    let clear_asset = required_asset(&mut config, "clear_asset")?;
    let indicator_asset = required_asset(&mut config, "indicator_asset")?;
    let check_asset = required_asset(&mut config, "check_asset")?;
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
        behavior: ChoiceBehavior {
            searchable,
            clearable,
            reset_query_on_close,
        },
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
        row_height,
        trigger_height,
        trigger_width,
        panel_width,
        panel_extra_height,
        overlay_gap,
        clear_asset,
        indicator_asset,
        check_asset,
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

pub(crate) fn select_node(
    call: NativeCallContext<'_>,
    mut config: Map,
) -> Result<UiNode, Box<EvalAltResult>> {
    let id = required_string(&mut config, "id")?;
    if id.trim().is_empty() {
        return overlay_config_error("select ID cannot be empty");
    }
    let parent_overlay = optional_string(&mut config, "parent_overlay")?;
    let options = parse_dropdown_options(&mut config)?;
    let value = optional_nullable_string(&mut config, "value")?;
    let searchable = optional_bool(&mut config, "searchable")?.unwrap_or(false);
    let clearable = optional_bool(&mut config, "clearable")?.unwrap_or(false);
    let placeholder = optional_string(&mut config, "placeholder")?.unwrap_or_default();
    let search_placeholder =
        optional_string(&mut config, "search_placeholder")?.unwrap_or_default();
    let empty_text = optional_string(&mut config, "empty_text")?.unwrap_or_default();
    let disabled = optional_bool(&mut config, "disabled")?.unwrap_or(false);
    let max_visible = optional_usize(&mut config, "max_visible")?.unwrap_or(8);
    if !(1..=32).contains(&max_visible) {
        return overlay_config_error("select max_visible must be between 1 and 32");
    }
    let placement = match optional_string(&mut config, "placement")?.as_deref() {
        None | Some("bottom") => OverlayPlacement::Bottom,
        Some("top") => OverlayPlacement::Top,
        Some("left") => OverlayPlacement::Left,
        Some("right") => OverlayPlacement::Right,
        Some(other) => {
            return overlay_config_error(format!("unsupported select placement `{other}`"));
        }
    };
    let row_height = positive_config_number(&mut config, "row_height", 32.0)?;
    let trigger_height = positive_config_number(&mut config, "trigger_height", 32.0)?;
    let trigger_width = required_length(&mut config, "trigger_width")?;
    let panel_width = positive_config_number(&mut config, "panel_width", 280.0)?;
    let panel_extra_height = optional_nonnegative_number(&mut config, "panel_extra_height")?;
    let overlay_gap = optional_nonnegative_number(&mut config, "overlay_gap")?.unwrap_or(0.0);
    let clear_asset = required_asset(&mut config, "clear_asset")?;
    let indicator_asset = required_asset(&mut config, "indicator_asset")?;
    let check_asset = required_asset(&mut config, "check_asset")?;
    if let Some((unknown, _)) = config.into_iter().next() {
        return overlay_config_error(format!("unknown select config field `{unknown}`"));
    }
    let choice = DropdownNodeSpec {
        id,
        parent_overlay,
        options,
        mode: DropdownMode::Single,
        selected: Some(value.into_iter().collect()),
        open: None,
        behavior: ChoiceBehavior {
            searchable,
            clearable,
            reset_query_on_close: true,
        },
        query: None,
        placeholder,
        search_placeholder,
        empty_text,
        trigger_slot: None,
        header_slot: None,
        footer_slot: None,
        empty_slot: None,
        disabled,
        max_visible,
        placement,
        row_height,
        trigger_height,
        trigger_width,
        panel_width,
        panel_extra_height,
        overlay_gap,
        clear_asset,
        indicator_asset,
        check_asset,
    };
    DropdownState::new(
        choice.options.clone(),
        DropdownMode::Single,
        choice.selected.clone().unwrap_or_default(),
    )
    .map_err(|error| {
        Box::new(EvalAltResult::ErrorRuntime(
            error.to_string().into(),
            Position::NONE,
        ))
    })?;
    Ok(with_call_source(
        UiNode::select(SelectNodeSpec { choice }),
        call,
    ))
}

pub(crate) fn date_picker_node(
    call: NativeCallContext<'_>,
    mut config: Map,
) -> Result<UiNode, Box<EvalAltResult>> {
    let id = required_string(&mut config, "id")?;
    let parent_overlay = optional_string(&mut config, "parent_overlay")?;
    let value = optional_date(&mut config, "value")?;
    let min_date = optional_date(&mut config, "min_date")?;
    let max_date = optional_date(&mut config, "max_date")?;
    let today = required_date(&mut config, "today")?;
    let display_value = optional_string(&mut config, "display_value")?.unwrap_or_default();
    let placeholder = optional_string(&mut config, "placeholder")?.unwrap_or_default();
    let open_label = required_string(&mut config, "open_label")?;
    let previous_label = required_string(&mut config, "previous_label")?;
    let next_label = required_string(&mut config, "next_label")?;
    let clear_label = required_string(&mut config, "clear_label")?;
    let calendar = required_decoded::<CalendarMetadata>(&mut config, "calendar")?;
    let number = required_decoded::<NumberMetadata>(&mut config, "number")?;
    let mut presets = parse_date_picker_presets(&mut config)?;
    let clearable = optional_bool(&mut config, "clearable")?.unwrap_or(false);
    let disabled = optional_bool(&mut config, "disabled")?.unwrap_or(false);
    let placement = match optional_string(&mut config, "placement")?.as_deref() {
        None | Some("bottom") => OverlayPlacement::Bottom,
        Some("top") => OverlayPlacement::Top,
        Some("left") => OverlayPlacement::Left,
        Some("right") => OverlayPlacement::Right,
        Some(other) => {
            return overlay_config_error(format!("unsupported DatePicker placement `{other}`"));
        }
    };
    let cell_size = positive_config_number(&mut config, "cell_size", 32.0)?;
    let trigger_height = positive_config_number(&mut config, "trigger_height", 32.0)?;
    let panel_width = positive_config_number(&mut config, "panel_width", 280.0)?;
    let overlay_gap = optional_number(&mut config, "overlay_gap")?.unwrap_or(0.0);
    let previous_asset = required_asset(&mut config, "previous_asset")?;
    let next_asset = required_asset(&mut config, "next_asset")?;
    let trigger_asset = required_asset(&mut config, "trigger_asset")?;
    let clear_asset = required_asset(&mut config, "clear_asset")?;
    if let Some((unknown, _)) = config.into_iter().next() {
        return overlay_config_error(format!("unknown DatePicker config field `{unknown}`"));
    }
    for preset in &mut presets {
        preset.disabled = preset.disabled
            || min_date.is_some_and(|min| preset.value < min)
            || max_date.is_some_and(|max| preset.value > max);
    }
    let spec = DatePickerNodeSpec {
        id,
        parent_overlay,
        value,
        min_date,
        max_date,
        today,
        display_value,
        placeholder,
        open_label,
        previous_label,
        next_label,
        clear_label,
        calendar,
        number,
        presets,
        clearable,
        disabled,
        placement,
        cell_size,
        trigger_height,
        panel_width,
        overlay_gap,
        previous_asset,
        next_asset,
        trigger_asset,
        clear_asset,
    };
    spec.validate()
        .map_err(|message| Box::new(EvalAltResult::ErrorRuntime(message.into(), Position::NONE)))?;
    Ok(with_call_source(UiNode::date_picker(spec), call))
}

pub(crate) fn table_node(
    call: NativeCallContext<'_>,
    mut config: Map,
) -> Result<UiNode, Box<EvalAltResult>> {
    let key = required_string(&mut config, "key")?;
    let label = required_string(&mut config, "label")?;
    let row_key = required_string(&mut config, "row_key")?;
    if row_key.trim().is_empty() {
        return overlay_config_error("Table row_key field name cannot be empty");
    }
    let (raw_rows, rows) = parse_table_rows(&mut config, &row_key)?;
    let columns = parse_table_columns(&call, &mut config, &raw_rows, &rows)?;
    let height = config
        .remove("height")
        .and_then(Dynamic::try_cast::<crate::Length>)
        .ok_or_else(|| Box::new(overlay_type_error("height", "a Length")))?;
    let row_height = positive_config_number(&mut config, "row_height", 32.0)?;
    let flex_min_width = positive_config_number(&mut config, "flex_min_width", 80.0)?;
    let selection_width = positive_config_number(&mut config, "selection_width", row_height)?;
    let selection_size = positive_config_number(&mut config, "selection_size", row_height / 2.0)?;
    let horizontal_scrollbar_height = positive_config_number(
        &mut config,
        "horizontal_scrollbar_height",
        row_height * 0.375,
    )?;
    let horizontal_scrollbar_thumb_min_width = positive_config_number(
        &mut config,
        "horizontal_scrollbar_thumb_min_width",
        row_height * 2.0,
    )?;
    let horizontal_scrollbar_inset = positive_config_number(
        &mut config,
        "horizontal_scrollbar_inset",
        row_height * 0.125,
    )?;
    let overscan = optional_usize(&mut config, "overscan")?.unwrap_or(2);
    if overscan > 100 {
        return overlay_config_error("Table overscan cannot exceed 100");
    }
    let loading = optional_bool(&mut config, "loading")?.unwrap_or(false);
    let loading_slot = optional_node(&mut config, "loading_slot")?;
    let loading_rows = required_node_array(&mut config, "loading_rows")?;
    let empty_slot = optional_node(&mut config, "empty_slot")?;
    let empty_text = optional_string(&mut config, "empty_text")?.unwrap_or_default();
    let striped = optional_bool(&mut config, "striped")?.unwrap_or(false);
    let selection_mode = match optional_string(&mut config, "selection_mode")?.as_deref() {
        None | Some("none") => TableSelectionMode::None,
        Some("single") => TableSelectionMode::Single,
        Some("multiple") => TableSelectionMode::Multiple,
        Some(value) => {
            return overlay_config_error(format!("unknown Table selection mode `{value}`"));
        }
    };
    let selected_keys = optional_string_array(&mut config, "selected_keys")?
        .unwrap_or_default()
        .into_iter()
        .collect();
    let sort = parse_table_sort(&mut config)?;
    let calendar = required_decoded::<CalendarMetadata>(&mut config, "calendar")?;
    let number = required_decoded::<NumberMetadata>(&mut config, "number")?;
    let check_asset = required_asset(&mut config, "check_asset")?;
    let sort_ascending_asset = required_asset(&mut config, "sort_ascending_asset")?;
    let sort_descending_asset = required_asset(&mut config, "sort_descending_asset")?;
    if let Some((unknown, _)) = config.into_iter().next() {
        return overlay_config_error(format!("unknown Table config field `{unknown}`"));
    }
    let spec = TableNodeSpec {
        key,
        label,
        columns,
        rows,
        height,
        row_height,
        flex_min_width,
        selection_width,
        selection_size,
        horizontal_scrollbar_height,
        horizontal_scrollbar_thumb_min_width,
        horizontal_scrollbar_inset,
        overscan,
        loading,
        loading_slot,
        loading_rows,
        empty_slot,
        empty_text,
        striped,
        selection_mode,
        selected_keys,
        sort,
        calendar,
        number,
        check_asset,
        sort_ascending_asset,
        sort_descending_asset,
    };
    spec.validate().map_err(|error| {
        Box::new(EvalAltResult::ErrorRuntime(
            error.to_string().into(),
            Position::NONE,
        ))
    })?;
    Ok(with_call_source(UiNode::table(spec), call))
}

fn parse_table_rows(
    config: &mut Map,
    row_key: &str,
) -> Result<(Vec<Map>, Vec<TableRowSpec>), Box<EvalAltResult>> {
    let values = config
        .remove("rows")
        .and_then(Dynamic::try_cast::<Array>)
        .ok_or_else(|| Box::new(overlay_type_error("rows", "an array of row maps")))?;
    if values.len() > 10_000 {
        return overlay_config_error("Table rows cannot exceed 10,000 items");
    }
    let mut raw_rows = Vec::with_capacity(values.len());
    let mut rows = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        let row = value.try_cast::<Map>().ok_or_else(|| {
            Box::new(EvalAltResult::ErrorRuntime(
                format!("Table row at index {index} must be a map").into(),
                Position::NONE,
            ))
        })?;
        let key = row
            .get(row_key)
            .filter(|value| value.is::<ImmutableString>())
            .map(|value| value.clone_cast::<ImmutableString>().to_string())
            .ok_or_else(|| {
                Box::new(EvalAltResult::ErrorRuntime(
                    format!("Table row at index {index} requires string field `{row_key}`").into(),
                    Position::NONE,
                ))
            })?;
        let values = UiValue::from_dynamic(Dynamic::from_map(row.clone())).map_err(|error| {
            Box::new(EvalAltResult::ErrorRuntime(
                error.to_string().into(),
                Position::NONE,
            ))
        })?;
        let UiValue::Map(values) = values else {
            unreachable!("a Rhai map converts to UiValue::Map")
        };
        raw_rows.push(row);
        rows.push(TableRowSpec { key, values });
    }
    Ok((raw_rows, rows))
}

fn parse_table_columns(
    call: &NativeCallContext<'_>,
    config: &mut Map,
    raw_rows: &[Map],
    rows: &[TableRowSpec],
) -> Result<Vec<TableColumnSpec>, Box<EvalAltResult>> {
    let values = config
        .remove("columns")
        .and_then(Dynamic::try_cast::<Array>)
        .ok_or_else(|| Box::new(overlay_type_error("columns", "an array of column maps")))?;
    if values.len() > 256 {
        return overlay_config_error("Table columns cannot exceed 256 items");
    }
    validate_table_column_budget(&values, rows.len())?;
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let mut column = value.try_cast::<Map>().ok_or_else(|| {
                Box::new(EvalAltResult::ErrorRuntime(
                    format!("Table column at index {index} must be a map").into(),
                    Position::NONE,
                ))
            })?;
            parse_table_column(call, &mut column, raw_rows, rows)
        })
        .collect()
}

fn validate_table_column_budget(
    values: &Array,
    row_count: usize,
) -> Result<(), Box<EvalAltResult>> {
    let custom_columns = values
        .iter()
        .filter_map(|value| value.clone().try_cast::<Map>())
        .filter(|column| {
            column
                .get("cell_renderer")
                .is_some_and(|renderer| !renderer.is_unit())
        })
        .count();
    if custom_columns
        .checked_mul(row_count)
        .is_none_or(|cells| cells > 10_000)
    {
        return overlay_config_error(
            "Table custom renderers cannot build more than 10,000 eager cell nodes",
        );
    }
    let mut column_keys = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let Some(column) = value.clone().try_cast::<Map>() else {
            continue;
        };
        let Some(key) = column
            .get("key")
            .and_then(|key| key.clone().try_cast::<ImmutableString>())
            .map(|key| key.to_string())
        else {
            continue;
        };
        if key.trim().is_empty() {
            return overlay_config_error(format!("Table column at index {index} has an empty key"));
        }
        if !column_keys.insert(key.clone()) {
            return overlay_config_error(format!("Table column `{key}` is duplicated"));
        }
    }
    Ok(())
}

fn parse_table_column(
    call: &NativeCallContext<'_>,
    column: &mut Map,
    raw_rows: &[Map],
    rows: &[TableRowSpec],
) -> Result<TableColumnSpec, Box<EvalAltResult>> {
    let key = required_string(column, "key")?;
    let title = required_string(column, "title")?;
    if let Some((row_index, _)) = raw_rows
        .iter()
        .enumerate()
        .find(|(_, row)| !row.contains_key(key.as_str()))
    {
        return overlay_config_error(format!(
            "Table row `{}` is missing column `{key}`",
            rows[row_index].key
        ));
    }
    let width = parse_table_width(column, &key)?;
    let format = parse_table_format(column, &key)?;
    let align = match optional_string(column, "align")?.as_deref() {
        None if matches!(
            format,
            TableCellFormat::Number(_) | TableCellFormat::Date(_)
        ) =>
        {
            TableAlign::End
        }
        None | Some("start") => TableAlign::Start,
        Some("center") => TableAlign::Center,
        Some("end") => TableAlign::End,
        Some(value) => {
            return overlay_config_error(format!("unknown Table alignment `{value}`"));
        }
    };
    let sortable = optional_bool(column, "sortable")?.unwrap_or(false);
    let renderer = optional_callback(column, "cell_renderer")?;
    if let Some((unknown, _)) = column.iter().next() {
        return overlay_config_error(format!("unknown field `{unknown}` in Table column `{key}`"));
    }
    let custom_cells = renderer
        .map(|renderer| {
            raw_rows
                .iter()
                .zip(rows)
                .enumerate()
                .map(|(row_index, (raw, row))| {
                    let context = Map::from_iter([
                        ("row".into(), Dynamic::from_map(raw.clone())),
                        (
                            "value".into(),
                            raw.get(key.as_str()).cloned().unwrap_or(Dynamic::UNIT),
                        ),
                        ("row_key".into(), Dynamic::from(row.key.clone())),
                        (
                            "row_index".into(),
                            Dynamic::from(INT::try_from(row_index).unwrap_or(INT::MAX)),
                        ),
                        ("column_key".into(), Dynamic::from(key.clone())),
                    ]);
                    renderer.call_within_context::<UiNode>(call, (context,))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    Ok(TableColumnSpec {
        key,
        title,
        width,
        align,
        format,
        sortable,
        custom_cells,
    })
}

fn parse_table_width(column: &mut Map, key: &str) -> Result<TableColumnWidth, Box<EvalAltResult>> {
    let mut width = column
        .remove("width")
        .and_then(Dynamic::try_cast::<Map>)
        .ok_or_else(|| Box::new(overlay_type_error("width", "a tagged width map")))?;
    let kind = required_string(&mut width, "kind")?;
    let value = optional_number(&mut width, "value")?.ok_or_else(|| {
        Box::new(EvalAltResult::ErrorRuntime(
            format!("Table column `{key}` width requires value").into(),
            Position::NONE,
        ))
    })?;
    if let Some((unknown, _)) = width.into_iter().next() {
        return overlay_config_error(format!("unknown Table width field `{unknown}`"));
    }
    match kind.as_str() {
        "fixed" => Ok(TableColumnWidth::Fixed(value)),
        "percent" => Ok(TableColumnWidth::Percent(value)),
        "flex" => Ok(TableColumnWidth::Flex(value)),
        _ => overlay_config_error(format!("unknown Table width kind `{kind}`")),
    }
}

fn parse_table_format(column: &mut Map, key: &str) -> Result<TableCellFormat, Box<EvalAltResult>> {
    let Some(value) = column.remove("format") else {
        return Ok(TableCellFormat::Text);
    };
    let mut format = value
        .try_cast::<Map>()
        .ok_or_else(|| Box::new(overlay_type_error("format", "a tagged format map")))?;
    let kind = required_string(&mut format, "kind")?;
    let result = match kind.as_str() {
        "text" => TableCellFormat::Text,
        "number" => {
            let min_fraction_digits =
                optional_usize(&mut format, "min_fraction_digits")?.unwrap_or(0);
            let max_fraction_digits =
                optional_usize(&mut format, "max_fraction_digits")?.unwrap_or(3);
            let grouping = optional_bool(&mut format, "grouping")?.unwrap_or(true);
            let options = crate::NumberFormatOptions {
                min_fraction_digits: u8::try_from(min_fraction_digits).unwrap_or(u8::MAX),
                max_fraction_digits: u8::try_from(max_fraction_digits).unwrap_or(u8::MAX),
                grouping,
            };
            options.validate().map_err(|error| {
                Box::new(EvalAltResult::ErrorRuntime(
                    format!("Table column `{key}` number format is invalid: {error}").into(),
                    Position::NONE,
                ))
            })?;
            TableCellFormat::Number(options)
        }
        "date" => {
            let style =
                optional_string(&mut format, "style")?.unwrap_or_else(|| "short".to_owned());
            TableCellFormat::Date(crate::DateStyle::parse(&style).map_err(|error| {
                Box::new(EvalAltResult::ErrorRuntime(
                    error.to_string().into(),
                    Position::NONE,
                ))
            })?)
        }
        _ => return overlay_config_error(format!("unknown Table format kind `{kind}`")),
    };
    if let Some((unknown, _)) = format.into_iter().next() {
        return overlay_config_error(format!("unknown Table format field `{unknown}`"));
    }
    Ok(result)
}

fn parse_table_sort(config: &mut Map) -> Result<Option<TableSort>, Box<EvalAltResult>> {
    let Some(value) = config.remove("sort") else {
        return Ok(None);
    };
    if value.is_unit() {
        return Ok(None);
    }
    let mut sort = value
        .try_cast::<Map>()
        .ok_or_else(|| Box::new(overlay_type_error("sort", "a sort map or null")))?;
    let key = required_string(&mut sort, "key")?;
    let direction = match required_string(&mut sort, "direction")?.as_str() {
        "ascending" => TableSortDirection::Ascending,
        "descending" => TableSortDirection::Descending,
        value => return overlay_config_error(format!("unknown Table sort direction `{value}`")),
    };
    if let Some((unknown, _)) = sort.into_iter().next() {
        return overlay_config_error(format!("unknown Table sort field `{unknown}`"));
    }
    Ok(Some(TableSort { key, direction }))
}

fn optional_callback(config: &mut Map, name: &str) -> Result<Option<FnPtr>, Box<EvalAltResult>> {
    let Some(value) = config.remove(name) else {
        return Ok(None);
    };
    if value.is_unit() {
        Ok(None)
    } else {
        value
            .try_cast::<FnPtr>()
            .map(Some)
            .ok_or_else(|| Box::new(overlay_type_error(name, "a callback")))
    }
}

fn required_asset(config: &mut Map, name: &str) -> Result<AssetId, Box<EvalAltResult>> {
    config
        .remove(name)
        .and_then(Dynamic::try_cast::<AssetId>)
        .ok_or_else(|| Box::new(overlay_type_error(name, "an AssetId")))
}

fn required_length(config: &mut Map, name: &str) -> Result<crate::Length, Box<EvalAltResult>> {
    config
        .remove(name)
        .and_then(Dynamic::try_cast::<crate::Length>)
        .ok_or_else(|| Box::new(overlay_type_error(name, "a Length")))
}

fn optional_nonnegative_number(
    config: &mut Map,
    name: &str,
) -> Result<Option<f64>, Box<EvalAltResult>> {
    let value = optional_number(config, name)?;
    if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
        return overlay_config_error(format!("{name} must be a finite non-negative number"));
    }
    Ok(value)
}

fn optional_date(
    config: &mut Map,
    name: &str,
) -> Result<Option<GregorianDate>, Box<EvalAltResult>> {
    optional_nullable_string(config, name)?
        .map(|value| {
            GregorianDate::parse_iso(&value).map_err(|error| {
                Box::new(EvalAltResult::ErrorRuntime(
                    format!("invalid DatePicker {name}: {error}").into(),
                    Position::NONE,
                ))
            })
        })
        .transpose()
}

fn required_date(config: &mut Map, name: &str) -> Result<GregorianDate, Box<EvalAltResult>> {
    let value = required_string(config, name)?;
    GregorianDate::parse_iso(&value).map_err(|error| {
        Box::new(EvalAltResult::ErrorRuntime(
            format!("invalid DatePicker {name}: {error}").into(),
            Position::NONE,
        ))
    })
}

fn required_decoded<T: serde::de::DeserializeOwned>(
    config: &mut Map,
    name: &str,
) -> Result<T, Box<EvalAltResult>> {
    let value = config.remove(name).ok_or_else(|| {
        Box::new(EvalAltResult::ErrorRuntime(
            format!("DatePicker config field `{name}` is required").into(),
            Position::NONE,
        ))
    })?;
    rhai::serde::from_dynamic(&value).map_err(|error| {
        Box::new(EvalAltResult::ErrorRuntime(
            format!("DatePicker {name} is invalid: {error}").into(),
            Position::NONE,
        ))
    })
}

fn parse_date_picker_presets(
    config: &mut Map,
) -> Result<Vec<DatePickerPreset>, Box<EvalAltResult>> {
    let Some(value) = config.remove("presets") else {
        return Ok(Vec::new());
    };
    let presets = value
        .try_cast::<Array>()
        .ok_or_else(|| Box::new(overlay_type_error("presets", "an array of preset maps")))?;
    if presets.len() > 32 {
        return overlay_config_error("DatePicker presets cannot exceed 32 items");
    }
    presets
        .into_iter()
        .enumerate()
        .map(|(index, preset)| {
            let mut preset = preset.try_cast::<Map>().ok_or_else(|| {
                Box::new(EvalAltResult::ErrorRuntime(
                    format!("DatePicker preset at index {index} must be a map").into(),
                    Position::NONE,
                ))
            })?;
            let label = required_string(&mut preset, "label")?;
            let value = required_date(&mut preset, "value")?;
            let disabled = optional_bool(&mut preset, "disabled")?.unwrap_or(false);
            if let Some((unknown, _)) = preset.into_iter().next() {
                return overlay_config_error(format!(
                    "unknown field `{unknown}` in DatePicker preset `{label}`"
                ));
            }
            Ok(DatePickerPreset {
                label,
                value,
                disabled,
            })
        })
        .collect()
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
            let group = optional_string(&mut option, "group")?;
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
                group,
            })
        })
        .collect()
}

fn positive_config_number(
    config: &mut Map,
    name: &str,
    default: f64,
) -> Result<f64, Box<EvalAltResult>> {
    let value = optional_number(config, name)?.unwrap_or(default);
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        overlay_config_error(format!("{name} must be finite and positive"))
    }
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

fn optional_nullable_string(
    config: &mut Map,
    name: &str,
) -> Result<Option<String>, Box<EvalAltResult>> {
    let Some(value) = config.remove(name) else {
        return Ok(None);
    };
    if value.is_unit() {
        Ok(None)
    } else {
        value
            .try_cast::<ImmutableString>()
            .map(|value| Some(value.to_string()))
            .ok_or_else(|| Box::new(overlay_type_error(name, "string or null")))
    }
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

fn required_node_array(config: &mut Map, name: &str) -> Result<Vec<UiNode>, Box<EvalAltResult>> {
    let values = config
        .remove(name)
        .and_then(Dynamic::try_cast::<Array>)
        .ok_or_else(|| Box::new(overlay_type_error(name, "an array of UiNode values")))?;
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            value.try_cast::<UiNode>().ok_or_else(|| {
                Box::new(EvalAltResult::ErrorRuntime(
                    format!("{name} item at index {index} must be a UiNode").into(),
                    Position::NONE,
                ))
            })
        })
        .collect()
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
