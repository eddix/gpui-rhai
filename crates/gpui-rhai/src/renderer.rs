use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Instant;

use gpui::{
    AnyElement, App, Bounds, BoxShadow, ClickEvent, Context, DispatchPhase, Div, Element,
    ElementId, FocusHandle, FontStyle, FontWeight, GlobalElementId, HighlightStyle,
    InspectorElementId, InteractiveElement, IntoElement, LayoutId, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render,
    ScrollHandle, ScrollWheelEvent, SharedString, Stateful, StatefulInteractiveElement, Styled,
    StyledText, Window, div, img, point, px, relative, rems, rgba,
};

use crate::date_picker_element::{
    DateChangeHandler, DatePickerCallbacks, DatePickerEntityElement, DatePickerPalette,
};
use crate::overlay_element::{ScriptOverlayElement, WindowOverlayCoordinator};
use crate::slot_runtime::NodeSlotRuntime;
use crate::toast_element::{ToastDismissHandler, ToastHostElement, ToastPalette, ToastPartStyles};
use crate::virtual_list_element::VirtualListEntityElement;
use crate::{
    Align, AnimationKey, AnimationProperty, AssetRegistry, ColorValue, DatePickerNodeSpec,
    EventPropagation, EventResponse, FlexDirection, ImageSourceSpec, InteractionState, Justify,
    Length, NodeId, OverflowMode, OverlayNodeSpec, PositionMode, PrimitiveRegistry, PseudoState,
    RadiusToken, RetainedUiTree, Rgba8, ScriptCallback, SpacingToken, Style, StyleProperties,
    TextDirection, ToastHostSpec, UiEventHandler, UiNode, UiNodeKind, UiValue,
};

type DispatchFn = dyn Fn(ScriptCallback, UiValue, &mut Window, &mut App) -> EventResponse;
type NativeDispatchFn =
    dyn Fn(crate::NativeHandlerRef, String, UiValue, &mut Window, &mut App) -> EventResponse;

#[derive(Clone)]
pub struct NodeEventDispatcher {
    script: Rc<DispatchFn>,
    native: Rc<NativeDispatchFn>,
}

impl NodeEventDispatcher {
    #[must_use]
    pub fn new<R>(
        dispatch: impl Fn(ScriptCallback, UiValue, &mut Window, &mut App) -> R + 'static,
    ) -> Self
    where
        R: Into<EventResponse>,
    {
        Self {
            script: Rc::new(move |callback, payload, window, app| {
                dispatch(callback, payload, window, app).into()
            }),
            native: Rc::new(|_, _, _, _, _| EventResponse::new().stop()),
        }
    }

    #[must_use]
    pub fn with_native<R>(
        mut self,
        dispatch: impl Fn(crate::NativeHandlerRef, String, UiValue, &mut Window, &mut App) -> R
        + 'static,
    ) -> Self
    where
        R: Into<EventResponse>,
    {
        self.native = Rc::new(move |handler, event, payload, window, app| {
            dispatch(handler, event, payload, window, app).into()
        });
        self
    }

    pub(crate) fn dispatch(
        &self,
        callback: ScriptCallback,
        payload: UiValue,
        window: &mut Window,
        cx: &mut App,
    ) -> EventResponse {
        (self.script)(callback, payload, window, cx)
    }

    pub(crate) fn dispatch_native(
        &self,
        handler: crate::NativeHandlerRef,
        event: String,
        payload: UiValue,
        window: &mut Window,
        app: &mut App,
    ) -> EventResponse {
        (self.native)(handler, event, payload, window, app)
    }
}

fn dispatch_ui_event(
    handler: &UiEventHandler,
    event: &str,
    payload: UiValue,
    window: &mut Window,
    app: &mut App,
    script_dispatcher: Option<&NodeEventDispatcher>,
) -> EventResponse {
    match handler {
        UiEventHandler::Script(callback) => script_dispatcher.map_or_else(
            || EventResponse::new().stop(),
            |dispatcher| dispatcher.dispatch(callback.clone(), payload, window, app),
        ),
        UiEventHandler::Host(callback) => callback.invoke(payload, window, app),
        UiEventHandler::Native(reference) => script_dispatcher.map_or_else(
            || EventResponse::new().stop(),
            |dispatcher| {
                dispatcher.dispatch_native(
                    reference.clone(),
                    event.to_owned(),
                    payload,
                    window,
                    app,
                )
            },
        ),
    }
}

fn dispatch_ui_handlers(
    bindings: &[crate::UiEventBinding],
    event: &str,
    payload: &UiValue,
    window: &mut Window,
    app: &mut App,
    script_dispatcher: Option<&NodeEventDispatcher>,
) -> EventResponse {
    dispatch_ui_handler_phases(
        bindings,
        event,
        &[crate::EventPhase::Target],
        payload,
        window,
        app,
        script_dispatcher,
    )
}

fn dispatch_ui_handler_phases(
    bindings: &[crate::UiEventBinding],
    event: &str,
    phases: &[crate::EventPhase],
    payload: &UiValue,
    window: &mut Window,
    app: &mut App,
    script_dispatcher: Option<&NodeEventDispatcher>,
) -> EventResponse {
    let mut combined = EventResponse::new();
    for phase in phases {
        let mut stop_route = false;
        for binding in bindings.iter().filter(|binding| binding.phase() == *phase) {
            let response = dispatch_ui_event(
                binding.handler(),
                event,
                payload.clone(),
                window,
                app,
                script_dispatcher,
            );
            combined.merge(response);
            if matches!(
                response.propagation(),
                crate::PropagationControl::StopImmediate
            ) {
                return combined;
            }
            stop_route |= matches!(response.propagation(), crate::PropagationControl::Stop);
        }
        if stop_route {
            return combined;
        }
    }
    combined
}

fn apply_event_response(response: EventResponse, window: &mut Window, app: &mut App) {
    if response.default_prevented() {
        window.prevent_default();
    }
    if response.stops_propagation() {
        app.stop_propagation();
    }
}

fn apply_pointer_response(
    response: EventResponse,
    node: Option<NodeId>,
    pointer_id: u64,
    captures: &crate::PointerCaptureRegistry,
    window: &mut Window,
    app: &mut App,
) {
    match response.pointer_capture() {
        crate::PointerCaptureDirective::Capture => {
            if let Some(node) = node {
                captures.capture(pointer_id, node);
            }
        }
        crate::PointerCaptureDirective::Release => {
            captures.release(pointer_id);
        }
        crate::PointerCaptureDirective::None => {}
    }
    apply_event_response(response, window, app);
}

fn key_handler_bindings(node: &UiNode) -> BTreeMap<String, (Vec<crate::UiEventBinding>, UiValue)> {
    node.handlers()
        .iter()
        .filter_map(|(event, bindings)| {
            event.strip_prefix("key:").map(|key| {
                (
                    key.to_owned(),
                    (
                        bindings.clone(),
                        node.handler_payload(event)
                            .cloned()
                            .unwrap_or(UiValue::Null),
                    ),
                )
            })
        })
        .collect()
}

fn owned_part_styles(node: &UiNode) -> BTreeMap<String, Style> {
    node.part_styles()
        .map(|(name, style)| (name.to_owned(), style.clone()))
        .collect()
}

fn node_scrollable(node: &UiNode) -> bool {
    matches!(node.style().base.overflow_x, Some(OverflowMode::Scroll))
        || matches!(node.style().base.overflow_y, Some(OverflowMode::Scroll))
}

fn node_has_raw_pointer_handlers(node: &UiNode) -> bool {
    ["pointer_down", "pointer_up", "pointer_move", "wheel"]
        .into_iter()
        .any(|event| !node.event_handlers(event).is_empty())
}

fn apply_scroll_behavior(
    mut element: Stateful<Div>,
    node: &UiNode,
    retained_id: Option<NodeId>,
    handles: &BTreeMap<NodeId, ScrollHandle>,
) -> Stateful<Div> {
    if matches!(node.style().base.overflow_x, Some(OverflowMode::Scroll)) {
        element = element.overflow_x_scroll();
    }
    if matches!(node.style().base.overflow_y, Some(OverflowMode::Scroll)) {
        element = element.overflow_y_scroll();
    }
    if let Some(handle) = retained_id.and_then(|node| handles.get(&node)) {
        element = element.track_scroll(handle);
    }
    element
}

fn apply_raw_pointer_handlers(
    element: Stateful<Div>,
    node: &UiNode,
    dispatcher: Option<&NodeEventDispatcher>,
    retained_id: Option<NodeId>,
    captures: &crate::PointerCaptureRegistry,
) -> Stateful<Div> {
    let element = apply_pointer_down_handlers(element, node, dispatcher, retained_id, captures);
    let element = apply_pointer_up_handlers(element, node, dispatcher, retained_id, captures);
    apply_pointer_motion_handlers(
        element,
        node,
        dispatcher.cloned(),
        retained_id,
        captures.clone(),
    )
}

fn apply_pointer_down_handlers(
    mut element: Stateful<Div>,
    node: &UiNode,
    dispatcher: Option<&NodeEventDispatcher>,
    retained_id: Option<NodeId>,
    captures: &crate::PointerCaptureRegistry,
) -> Stateful<Div> {
    let pointer_down = node.event_handlers("pointer_down").to_vec();
    if pointer_down
        .iter()
        .any(|binding| binding.phase() == crate::EventPhase::Capture)
    {
        let bindings = pointer_down.clone();
        let dispatcher = dispatcher.cloned();
        let captures = captures.clone();
        element = element.capture_any_mouse_down(move |event, window, app| {
            let response = dispatch_ui_handler_phases(
                &bindings,
                "pointer_down",
                &[crate::EventPhase::Capture],
                &mouse_down_payload(event),
                window,
                app,
                dispatcher.as_ref(),
            );
            apply_pointer_response(response, retained_id, 0, &captures, window, app);
        });
    }
    if pointer_down.iter().any(|binding| {
        matches!(
            binding.phase(),
            crate::EventPhase::Target | crate::EventPhase::Bubble
        )
    }) {
        let bindings = pointer_down;
        let dispatcher = dispatcher.cloned();
        let captures = captures.clone();
        element = element.on_any_mouse_down(move |event, window, app| {
            let response = dispatch_ui_handler_phases(
                &bindings,
                "pointer_down",
                &[crate::EventPhase::Target, crate::EventPhase::Bubble],
                &mouse_down_payload(event),
                window,
                app,
                dispatcher.as_ref(),
            );
            apply_pointer_response(response, retained_id, 0, &captures, window, app);
        });
    }
    element
}

fn apply_pointer_up_handlers(
    mut element: Stateful<Div>,
    node: &UiNode,
    dispatcher: Option<&NodeEventDispatcher>,
    retained_id: Option<NodeId>,
    captures: &crate::PointerCaptureRegistry,
) -> Stateful<Div> {
    let pointer_up = node.event_handlers("pointer_up").to_vec();
    if pointer_up
        .iter()
        .any(|binding| binding.phase() == crate::EventPhase::Capture)
    {
        let bindings = pointer_up.clone();
        let dispatcher = dispatcher.cloned();
        let captures = captures.clone();
        element = element.capture_any_mouse_up(move |event, window, app| {
            let response = dispatch_ui_handler_phases(
                &bindings,
                "pointer_up",
                &[crate::EventPhase::Capture],
                &mouse_up_payload(event),
                window,
                app,
                dispatcher.as_ref(),
            );
            apply_pointer_response(response, retained_id, 0, &captures, window, app);
            captures.release(0);
        });
    }
    if pointer_up.iter().any(|binding| {
        matches!(
            binding.phase(),
            crate::EventPhase::Target | crate::EventPhase::Bubble
        )
    }) {
        for button in MouseButton::all() {
            let bindings = pointer_up.clone();
            let dispatcher = dispatcher.cloned();
            let captures = captures.clone();
            element = element.on_mouse_up(button, move |event, window, app| {
                let response = dispatch_ui_handler_phases(
                    &bindings,
                    "pointer_up",
                    &[crate::EventPhase::Target, crate::EventPhase::Bubble],
                    &mouse_up_payload(event),
                    window,
                    app,
                    dispatcher.as_ref(),
                );
                apply_pointer_response(response, retained_id, 0, &captures, window, app);
                captures.release(0);
            });
        }
    }
    element
}

fn apply_pointer_motion_handlers(
    mut element: Stateful<Div>,
    node: &UiNode,
    dispatcher: Option<NodeEventDispatcher>,
    retained_id: Option<NodeId>,
    captures: crate::PointerCaptureRegistry,
) -> Stateful<Div> {
    let pointer_move = node.event_handlers("pointer_move").to_vec();
    if !pointer_move.is_empty() {
        let dispatcher = dispatcher.clone();
        let captures = captures.clone();
        element = element.on_mouse_move(move |event, window, app| {
            let response = dispatch_ui_handler_phases(
                &pointer_move,
                "pointer_move",
                &[crate::EventPhase::Target, crate::EventPhase::Bubble],
                &mouse_move_payload(event),
                window,
                app,
                dispatcher.as_ref(),
            );
            apply_pointer_response(response, retained_id, 0, &captures, window, app);
        });
    }

    let wheel = node.event_handlers("wheel").to_vec();
    if !wheel.is_empty() {
        element = element.on_scroll_wheel(move |event, window, app| {
            let response = dispatch_ui_handler_phases(
                &wheel,
                "wheel",
                &[crate::EventPhase::Target, crate::EventPhase::Bubble],
                &wheel_payload(event),
                window,
                app,
                dispatcher.as_ref(),
            );
            apply_pointer_response(response, retained_id, 0, &captures, window, app);
        });
    }
    element
}

fn mouse_down_payload(event: &MouseDownEvent) -> UiValue {
    pointer_payload(
        event.position,
        Some(event.button),
        vec![event.button],
        event.modifiers,
        event.click_count,
        false,
    )
}

fn mouse_up_payload(event: &MouseUpEvent) -> UiValue {
    mouse_up_payload_with_capture(event, false)
}

fn mouse_up_payload_with_capture(event: &MouseUpEvent, captured: bool) -> UiValue {
    pointer_payload(
        event.position,
        Some(event.button),
        Vec::new(),
        event.modifiers,
        event.click_count,
        captured,
    )
}

fn mouse_move_payload(event: &MouseMoveEvent) -> UiValue {
    mouse_move_payload_with_capture(event, false)
}

fn mouse_move_payload_with_capture(event: &MouseMoveEvent, captured: bool) -> UiValue {
    pointer_payload(
        event.position,
        None,
        event.pressed_button.into_iter().collect(),
        event.modifiers,
        0,
        captured,
    )
}

fn pointer_payload(
    position: Point<Pixels>,
    button: Option<MouseButton>,
    buttons: Vec<MouseButton>,
    modifiers: Modifiers,
    click_count: usize,
    captured: bool,
) -> UiValue {
    let position = logical_point(position);
    crate::PointerEventData {
        pointer_id: 0,
        pointer_type: "mouse".to_owned(),
        window: position,
        local: position,
        content: position,
        movement: crate::LogicalPoint::default(),
        button: button.map(mouse_button_name),
        buttons: buttons.into_iter().map(mouse_button_name).collect(),
        modifiers: event_modifiers(modifiers),
        click_count,
        timestamp_ms: event_timestamp_ms(),
        captured,
    }
    .into_value()
}

fn wheel_payload(event: &ScrollWheelEvent) -> UiValue {
    let position = logical_point(event.position);
    let delta = event.delta.pixel_delta(px(16.0));
    crate::WheelEventData {
        window: position,
        local: position,
        content: position,
        delta: logical_point(delta),
        precise: event.delta.precise(),
        modifiers: event_modifiers(event.modifiers),
        timestamp_ms: event_timestamp_ms(),
    }
    .into_value()
}

fn logical_point(point: Point<Pixels>) -> crate::LogicalPoint {
    crate::LogicalPoint {
        x: f64::from(point.x),
        y: f64::from(point.y),
    }
}

fn event_modifiers(modifiers: Modifiers) -> crate::EventModifiers {
    crate::EventModifiers {
        control: modifiers.control,
        alt: modifiers.alt,
        shift: modifiers.shift,
        platform: modifiers.platform,
        function: modifiers.function,
    }
}

fn mouse_button_name(button: MouseButton) -> String {
    match button {
        MouseButton::Left => "left",
        MouseButton::Right => "right",
        MouseButton::Middle => "middle",
        MouseButton::Navigate(gpui::NavigationDirection::Back) => "back",
        MouseButton::Navigate(gpui::NavigationDirection::Forward) => "forward",
    }
    .to_owned()
}

fn event_timestamp_ms() -> f64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1_000.0
}

pub(crate) fn install_pointer_capture_router(
    window: &mut Window,
    tree: &RetainedUiTree,
    dispatcher: &NodeEventDispatcher,
    captures: &crate::PointerCaptureRegistry,
) {
    let move_handlers = retained_handlers(tree, "pointer_move");
    let up_handlers = retained_handlers(tree, "pointer_up");
    let move_dispatcher = dispatcher.clone();
    let up_dispatcher = dispatcher.clone();
    let move_captures = captures.clone();
    let up_captures = captures.clone();
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, app| {
        if phase != DispatchPhase::Capture {
            return;
        }
        let Some(node) = move_captures.captured(0) else {
            return;
        };
        let Some(bindings) = move_handlers.get(&node) else {
            return;
        };
        let response = dispatch_ui_handler_phases(
            bindings,
            "pointer_move",
            &[crate::EventPhase::Target, crate::EventPhase::Bubble],
            &mouse_move_payload_with_capture(event, true),
            window,
            app,
            Some(&move_dispatcher),
        );
        apply_pointer_response(response, Some(node), 0, &move_captures, window, app);
        app.stop_propagation();
    });
    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, app| {
        if phase != DispatchPhase::Capture {
            return;
        }
        let Some(node) = up_captures.captured(0) else {
            return;
        };
        if let Some(bindings) = up_handlers.get(&node) {
            let response = dispatch_ui_handler_phases(
                bindings,
                "pointer_up",
                &[crate::EventPhase::Target, crate::EventPhase::Bubble],
                &mouse_up_payload_with_capture(event, true),
                window,
                app,
                Some(&up_dispatcher),
            );
            apply_pointer_response(response, Some(node), 0, &up_captures, window, app);
        }
        up_captures.release(0);
        app.stop_propagation();
    });
}

fn retained_handlers(
    tree: &RetainedUiTree,
    event: &str,
) -> BTreeMap<NodeId, Vec<crate::UiEventBinding>> {
    tree.nodes()
        .filter_map(|node| {
            let handlers = node.event_handlers(event);
            (!handlers.is_empty()).then(|| (node.id(), handlers.to_vec()))
        })
        .collect()
}

pub trait ColorResolver {
    fn resolve(&self, color: &ColorValue) -> Option<Rgba8>;

    fn resolve_length(&self, length: Length) -> Option<Length> {
        match length {
            Length::Pixels(_) | Length::Rems(_) | Length::Relative(_) => Some(length),
            Length::ThemeSpacing(_) | Length::ThemeRadius(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LiteralColorResolver;

impl ColorResolver for LiteralColorResolver {
    fn resolve(&self, color: &ColorValue) -> Option<Rgba8> {
        match color {
            ColorValue::Literal(color) => Some(*color),
            ColorValue::Token(_) => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct OwnedColorResolver {
    tokens: BTreeMap<String, Rgba8>,
    spacing: BTreeMap<SpacingToken, Length>,
    radii: BTreeMap<RadiusToken, Length>,
}

impl OwnedColorResolver {
    fn capture(colors: &impl ColorResolver) -> Self {
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
        let spacing = [
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
        .collect();
        let radii = [RadiusToken::Sm, RadiusToken::Md, RadiusToken::Lg]
            .into_iter()
            .filter_map(|token| {
                colors
                    .resolve_length(Length::ThemeRadius(token))
                    .map(|value| (token, value))
            })
            .collect();
        Self {
            tokens: TOKENS
                .iter()
                .filter_map(|token| {
                    colors
                        .resolve(&ColorValue::Token((*token).to_owned()))
                        .map(|value| ((*token).to_owned(), value))
                })
                .collect(),
            spacing,
            radii,
        }
    }
}

impl ColorResolver for OwnedColorResolver {
    fn resolve(&self, color: &ColorValue) -> Option<Rgba8> {
        match color {
            ColorValue::Literal(color) => Some(*color),
            ColorValue::Token(token) => self.tokens.get(token).copied(),
        }
    }

    fn resolve_length(&self, length: Length) -> Option<Length> {
        match length {
            Length::ThemeSpacing(token) => self.spacing.get(&token).copied(),
            Length::ThemeRadius(token) => self.radii.get(&token).copied(),
            Length::Pixels(_) | Length::Rems(_) | Length::Relative(_) => Some(length),
        }
    }
}

/// Converts stable runtime nodes into short-lived GPUI elements.
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuiNodeRenderer;

struct RenderEnvironment<'a, C> {
    colors: &'a C,
    interaction: &'a InteractionState,
    primitives: &'a PrimitiveRegistry,
    dispatcher: Option<&'a NodeEventDispatcher>,
    assets: Option<&'a AssetRegistry>,
    overlays: &'a WindowOverlayCoordinator,
    animations: &'a BTreeMap<AnimationKey, f64>,
    signals: &'a crate::SignalRegistry,
    geometry: &'a crate::GeometryRegistry,
    pointer_capture: &'a crate::PointerCaptureRegistry,
    focus_handles: &'a BTreeMap<NodeId, FocusHandle>,
    scroll_handles: &'a BTreeMap<NodeId, ScrollHandle>,
    virtual_requests: &'a crate::VirtualRequestRegistry,
    direction: TextDirection,
    view_id: &'a str,
    retained: Option<&'a RetainedUiTree>,
}

pub(crate) struct WindowRenderResources<'a> {
    pub assets: &'a AssetRegistry,
    pub dispatcher: &'a NodeEventDispatcher,
    pub overlays: &'a WindowOverlayCoordinator,
    pub animations: &'a BTreeMap<AnimationKey, f64>,
    pub signals: &'a crate::SignalRegistry,
    pub geometry: &'a crate::GeometryRegistry,
    pub pointer_capture: &'a crate::PointerCaptureRegistry,
    pub focus_handles: &'a BTreeMap<NodeId, FocusHandle>,
    pub scroll_handles: &'a BTreeMap<NodeId, ScrollHandle>,
    pub virtual_requests: &'a crate::VirtualRequestRegistry,
    pub direction: TextDirection,
    pub root_path: &'a str,
    pub view_id: &'a str,
}

impl GpuiNodeRenderer {
    #[must_use]
    pub fn render(node: &UiNode) -> AnyElement {
        Self::render_with_primitives(
            node,
            &LiteralColorResolver,
            &InteractionState::default(),
            &PrimitiveRegistry::new(),
        )
    }

    #[must_use]
    pub fn render_with(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
    ) -> AnyElement {
        Self::render_with_primitives(node, colors, interaction, &PrimitiveRegistry::new())
    }

    #[must_use]
    pub fn render_with_primitives(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
    ) -> AnyElement {
        let overlays = WindowOverlayCoordinator::default();
        let animations = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        let pointer_capture = crate::PointerCaptureRegistry::new();
        let focus_handles = BTreeMap::new();
        let scroll_handles = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let environment = RenderEnvironment {
            colors,
            interaction,
            primitives,
            dispatcher: None,
            assets: None,
            overlays: &overlays,
            animations: &animations,
            signals: &signals,
            geometry: &geometry,
            pointer_capture: &pointer_capture,
            focus_handles: &focus_handles,
            scroll_handles: &scroll_handles,
            virtual_requests: &virtual_requests,
            direction: TextDirection::LeftToRight,
            view_id: "standalone",
            retained: None,
        };
        Self::render_internal(node, &environment, None, "root", None)
    }

    #[must_use]
    pub fn render_retained_with_primitives(
        tree: &RetainedUiTree,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
    ) -> AnyElement {
        let overlays = WindowOverlayCoordinator::default();
        let animations = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        let pointer_capture = crate::PointerCaptureRegistry::new();
        let focus_handles = BTreeMap::new();
        let scroll_handles = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let environment = RenderEnvironment {
            colors,
            interaction,
            primitives,
            dispatcher: None,
            assets: None,
            overlays: &overlays,
            animations: &animations,
            signals: &signals,
            geometry: &geometry,
            pointer_capture: &pointer_capture,
            focus_handles: &focus_handles,
            scroll_handles: &scroll_handles,
            virtual_requests: &virtual_requests,
            direction: TextDirection::LeftToRight,
            view_id: "standalone",
            retained: Some(tree),
        };
        tree.root().map_or_else(
            || {
                div()
                    .child("Retained UI tree has no root")
                    .into_any_element()
            },
            |root| Self::render_internal(root, &environment, None, "root", tree.root_id()),
        )
    }

    #[must_use]
    pub fn render_with_dispatcher(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
        dispatcher: &NodeEventDispatcher,
    ) -> AnyElement {
        let overlays = WindowOverlayCoordinator::default();
        let animations = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        let pointer_capture = crate::PointerCaptureRegistry::new();
        let focus_handles = BTreeMap::new();
        let scroll_handles = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let environment = RenderEnvironment {
            colors,
            interaction,
            primitives,
            dispatcher: Some(dispatcher),
            assets: None,
            overlays: &overlays,
            animations: &animations,
            signals: &signals,
            geometry: &geometry,
            pointer_capture: &pointer_capture,
            focus_handles: &focus_handles,
            scroll_handles: &scroll_handles,
            virtual_requests: &virtual_requests,
            direction: TextDirection::LeftToRight,
            view_id: "standalone",
            retained: None,
        };
        Self::render_internal(node, &environment, None, "root", None)
    }

    #[must_use]
    pub fn render_with_runtime(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
        assets: &AssetRegistry,
        dispatcher: &NodeEventDispatcher,
    ) -> AnyElement {
        let overlays = WindowOverlayCoordinator::default();
        let animations = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        let pointer_capture = crate::PointerCaptureRegistry::new();
        let focus_handles = BTreeMap::new();
        let scroll_handles = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let resources = WindowRenderResources {
            assets,
            dispatcher,
            overlays: &overlays,
            animations: &animations,
            signals: &signals,
            geometry: &geometry,
            pointer_capture: &pointer_capture,
            focus_handles: &focus_handles,
            scroll_handles: &scroll_handles,
            virtual_requests: &virtual_requests,
            direction: TextDirection::LeftToRight,
            root_path: "root",
            view_id: "standalone",
        };
        Self::render_with_window_runtime(node, colors, interaction, primitives, &resources)
    }

    pub(crate) fn render_with_window_runtime(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
        resources: &WindowRenderResources<'_>,
    ) -> AnyElement {
        Self::render_subtree_with_window_runtime(
            node,
            colors,
            interaction,
            primitives,
            resources,
            resources.root_path,
        )
    }

    pub(crate) fn render_retained_with_window_runtime(
        tree: &RetainedUiTree,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
        resources: &WindowRenderResources<'_>,
    ) -> AnyElement {
        let environment = RenderEnvironment {
            colors,
            interaction,
            primitives,
            dispatcher: Some(resources.dispatcher),
            assets: Some(resources.assets),
            overlays: resources.overlays,
            animations: resources.animations,
            signals: resources.signals,
            geometry: resources.geometry,
            pointer_capture: resources.pointer_capture,
            focus_handles: resources.focus_handles,
            scroll_handles: resources.scroll_handles,
            virtual_requests: resources.virtual_requests,
            direction: resources.direction,
            view_id: resources.view_id,
            retained: Some(tree),
        };
        tree.root().map_or_else(
            || {
                div()
                    .child("Retained UI tree has no root")
                    .into_any_element()
            },
            |root| {
                Self::render_internal(
                    root,
                    &environment,
                    None,
                    resources.root_path,
                    tree.root_id(),
                )
            },
        )
    }

    pub(crate) fn render_subtree_with_window_runtime(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
        resources: &WindowRenderResources<'_>,
        path: &str,
    ) -> AnyElement {
        let environment = RenderEnvironment {
            colors,
            interaction,
            primitives,
            dispatcher: Some(resources.dispatcher),
            assets: Some(resources.assets),
            overlays: resources.overlays,
            animations: resources.animations,
            signals: resources.signals,
            geometry: resources.geometry,
            pointer_capture: resources.pointer_capture,
            focus_handles: resources.focus_handles,
            scroll_handles: resources.scroll_handles,
            virtual_requests: resources.virtual_requests,
            direction: resources.direction,
            view_id: resources.view_id,
            retained: None,
        };
        Self::render_internal(node, &environment, None, path, None)
    }

    fn render_internal<C: ColorResolver>(
        node: &UiNode,
        environment: &RenderEnvironment<'_, C>,
        boundary_fallback: Option<&UiNode>,
        path: &str,
        retained_id: Option<NodeId>,
    ) -> AnyElement {
        let local_interaction = if is_disabled(node) {
            environment.interaction.clone().with(PseudoState::Disabled)
        } else {
            environment.interaction.clone()
        };
        let animation = node_animation(environment.animations, path);
        let signals = node_signals(environment.signals, node);
        let mut resolved_style = node.style().resolve(&local_interaction);
        apply_animated_dimensions(&mut resolved_style, animation);
        apply_signal_style(&mut resolved_style, &signals);
        let mut element = apply_style(
            div(),
            &resolved_style,
            environment.colors,
            environment.direction,
        );
        if let Some(handle) = retained_id.and_then(|node| environment.focus_handles.get(&node)) {
            element = element.track_focus(handle);
        }
        if matches!(node.kind(), UiNodeKind::Custom { .. }) {
            let focus_ring = semantic_color(environment.colors, "focus_ring", 0x003b_82f6);
            let focus_surface = semantic_color(environment.colors, "surface", 0x0018_181b);
            element =
                element.in_focus(move |style| focus_ring_shadow(style, focus_ring, focus_surface));
        }
        if let Some(opacity) = signals.opacity.or(animation.opacity) {
            element = element.opacity(f64_to_f32(opacity.clamp(0.0, 1.0)));
        }
        if animation.clip_height.is_some() || resolved_style.clip == Some(true) {
            element = element.overflow_hidden();
        }
        let populated = Self::populate_with_interactions(
            element,
            node,
            environment,
            boundary_fallback,
            path,
            retained_id,
        );
        let element = translated(
            populated,
            signals.translate_x.or(animation.translate_x),
            signals.translate_y.or(animation.translate_y),
        );
        match retained_id {
            Some(node) => GeometryTrackedElement {
                child: Some(element),
                node,
                registry: environment.geometry.clone(),
            }
            .into_any_element(),
            None => element,
        }
    }

    fn populate_with_interactions<C: ColorResolver>(
        element: Div,
        node: &UiNode,
        environment: &RenderEnvironment<'_, C>,
        boundary_fallback: Option<&UiNode>,
        path: &str,
        retained_id: Option<NodeId>,
    ) -> AnyElement {
        let click = (!node.event_handlers("click").is_empty()).then(|| {
            (
                node.event_handlers("click").to_vec(),
                node.handler_payload("click")
                    .cloned()
                    .unwrap_or(UiValue::Null),
            )
        });
        let key_handlers = key_handler_bindings(node);
        let has_raw_pointer_handlers = node_has_raw_pointer_handlers(node);
        let has_scroll = node_scrollable(node);
        if is_disabled(node)
            || (click.is_none()
                && key_handlers.is_empty()
                && !has_raw_pointer_handlers
                && !has_scroll)
        {
            return Self::populate(
                element,
                node,
                environment,
                boundary_fallback,
                path,
                retained_id,
            );
        }

        let click_dispatcher = environment.dispatcher.cloned();
        let keyboard_dispatcher = environment.dispatcher.cloned();
        let keyboard_click = click.clone();
        let tab_stop = match node.attributes().get("tab_stop") {
            Some(UiValue::Bool(tab_stop)) => *tab_stop,
            _ => true,
        };
        let text_direction = environment.direction;
        let stable_id = retained_id.map_or_else(
            || path.to_owned(),
            |node_id| format!("gpui-rhai-node-{node_id}"),
        );
        let debug_path = path.to_owned();
        let element = apply_pseudo_backgrounds(
            element
                .id(SharedString::from(stable_id))
                .debug_selector(move || debug_path.clone()),
            node.style(),
            environment.colors,
        )
        .tab_index(0)
        .tab_stop(tab_stop);
        let element = apply_scroll_behavior(element, node, retained_id, environment.scroll_handles);
        let element = element
            .on_click(move |event, window, cx| {
                if matches!(event, ClickEvent::Mouse(_))
                    && let Some((bindings, payload)) = &click
                {
                    let response = dispatch_ui_handlers(
                        bindings,
                        "click",
                        payload,
                        window,
                        cx,
                        click_dispatcher.as_ref(),
                    );
                    apply_event_response(response, window, cx);
                }
            })
            .on_key_down(move |event, window, cx| {
                let semantic_key =
                    logical_keyboard_key(event.keystroke.key.as_str(), text_direction);
                let semantic = key_handlers.get(semantic_key).or_else(|| {
                    matches!(event.keystroke.key.as_str(), "enter" | "space")
                        .then_some(())
                        .and(keyboard_click.as_ref())
                });
                if let Some((bindings, payload)) = semantic {
                    let response = dispatch_ui_handlers(
                        bindings,
                        "key",
                        payload,
                        window,
                        cx,
                        keyboard_dispatcher.as_ref(),
                    );
                    apply_event_response(response, window, cx);
                }
            });
        let element = apply_raw_pointer_handlers(
            element,
            node,
            environment.dispatcher,
            retained_id,
            environment.pointer_capture,
        );
        Self::populate(
            element,
            node,
            environment,
            boundary_fallback,
            path,
            retained_id,
        )
    }

    fn populate<C: ColorResolver>(
        element: impl ParentElement + IntoElement,
        node: &UiNode,
        environment: &RenderEnvironment<'_, C>,
        boundary_fallback: Option<&UiNode>,
        path: &str,
        retained_id: Option<NodeId>,
    ) -> AnyElement {
        match node.kind() {
            UiNodeKind::Text { text } => element.child(text.as_str().to_owned()).into_any_element(),
            UiNodeKind::RichText { text, spans } => element
                .child(styled_text(text.as_str(), spans, environment.colors))
                .into_any_element(),
            UiNodeKind::Canvas { scene } => render_canvas(element, scene, environment.colors),
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => element
                .children(render_flattened_children(
                    children,
                    environment,
                    boundary_fallback,
                    path,
                    retained_id,
                ))
                .into_any_element(),
            UiNodeKind::Custom { primitive } => element
                .child(environment.primitives.element(
                    primitive.clone(),
                    boundary_fallback.cloned(),
                    environment.dispatcher.cloned(),
                    crate::PrimitiveTheme::capture(environment.colors),
                ))
                .into_any_element(),
            UiNodeKind::Image { source } => render_image(element, node, source, environment),
            UiNodeKind::DirectionalImage {
                left_to_right,
                right_to_left,
            } => {
                let source = match environment.direction {
                    TextDirection::LeftToRight => left_to_right,
                    TextDirection::RightToLeft => right_to_left,
                };
                render_image(element, node, source, environment)
            }
            UiNodeKind::Overlay {
                trigger,
                content,
                spec,
            } => element
                .child(native_overlay_element(
                    node,
                    trigger,
                    content,
                    spec,
                    environment,
                    boundary_fallback,
                    (path, retained_id),
                ))
                .into_any_element(),
            UiNodeKind::DatePicker { spec } => element
                .child(native_date_picker_element(node, spec, environment, path))
                .into_any_element(),
            UiNodeKind::ToastHost { spec } => element
                .child(native_toast_element(node, spec, environment, path))
                .into_any_element(),
            UiNodeKind::VirtualCollection { spec } => element
                .child(native_virtual_collection_element(spec, environment, path))
                .into_any_element(),
            UiNodeKind::ErrorBoundary { child, fallback } => element
                .child(Self::render_internal(
                    child,
                    environment,
                    Some(fallback),
                    &format!("{path}/boundary"),
                    retained_child_id(environment.retained, retained_id, "child", 0),
                ))
                .into_any_element(),
        }
    }
}

fn render_flattened_children<C: ColorResolver>(
    children: &[UiNode],
    environment: &RenderEnvironment<'_, C>,
    boundary_fallback: Option<&UiNode>,
    path: &str,
    retained_id: Option<NodeId>,
) -> Vec<AnyElement> {
    let mut rendered = Vec::new();
    for (index, child) in children.iter().enumerate() {
        let child_path = child.key().map_or_else(
            || format!("{path}/{index}"),
            |key| format!("{path}/{}", key.as_str()),
        );
        let child_id = retained_child_id(environment.retained, retained_id, "children", index);
        if let UiNodeKind::Fragment { children } = child.kind() {
            rendered.extend(render_flattened_children(
                children,
                environment,
                boundary_fallback,
                &child_path,
                child_id,
            ));
        } else {
            rendered.push(GpuiNodeRenderer::render_internal(
                child,
                environment,
                boundary_fallback,
                &child_path,
                child_id,
            ));
        }
    }
    rendered
}

fn retained_child_id(
    tree: Option<&RetainedUiTree>,
    parent: Option<NodeId>,
    group: &str,
    index: usize,
) -> Option<NodeId> {
    tree.and_then(|tree| tree.node(parent?))
        .and_then(|node| {
            node.children()
                .filter(|child| child.group() == group)
                .nth(index)
        })
        .map(crate::RetainedChildLink::node)
}

fn styled_text(text: &str, spans: &[crate::Span], colors: &impl ColorResolver) -> StyledText {
    let mut offset = 0usize;
    let highlights = spans.iter().filter_map(|span| {
        let start = offset;
        offset = offset.saturating_add(span.text().len());
        let style = HighlightStyle {
            color: span
                .color_value()
                .and_then(|color| colors.resolve(color))
                .map(|color| rgba(color.as_rgba_hex()).into()),
            font_weight: span.is_bold().then_some(FontWeight::BOLD),
            font_style: span.is_italic().then_some(FontStyle::Italic),
            ..HighlightStyle::default()
        };
        (style != HighlightStyle::default()).then_some((start..offset, style))
    });
    StyledText::new(text.to_owned()).with_highlights(highlights)
}

fn render_canvas(
    element: impl ParentElement + IntoElement,
    scene: &crate::CanvasScene,
    colors: &impl ColorResolver,
) -> AnyElement {
    let scene = scene.clone();
    let colors = OwnedColorResolver::capture(colors);
    let canvas = gpui::canvas(
        |_, _, _| (),
        move |bounds, (), window, _| paint_canvas_scene(bounds, &scene, &colors, window),
    )
    .size_full();
    element.child(canvas).into_any_element()
}

fn paint_canvas_scene(
    bounds: Bounds<Pixels>,
    scene: &crate::CanvasScene,
    colors: &impl ColorResolver,
    window: &mut Window,
) {
    for command in scene.commands() {
        match command {
            crate::CanvasCommand::Rect {
                x,
                y,
                width,
                height,
                fill: color,
                ..
            } => {
                if let Some(color) = colors.resolve(color) {
                    window.paint_quad(gpui::fill(
                        Bounds::new(
                            point(
                                bounds.origin.x + px(f64_to_f32(*x)),
                                bounds.origin.y + px(f64_to_f32(*y)),
                            ),
                            gpui::size(px(f64_to_f32(*width)), px(f64_to_f32(*height))),
                        ),
                        rgba(color.as_rgba_hex()),
                    ));
                }
            }
            crate::CanvasCommand::Circle {
                center_x,
                center_y,
                radius,
                fill: color,
                ..
            } => {
                if let Some(color) = colors.resolve(color) {
                    let radius = px(f64_to_f32(*radius));
                    window.paint_quad(gpui::quad(
                        Bounds::new(
                            point(
                                bounds.origin.x + px(f64_to_f32(*center_x)) - radius,
                                bounds.origin.y + px(f64_to_f32(*center_y)) - radius,
                            ),
                            gpui::size(radius * 2.0, radius * 2.0),
                        ),
                        radius,
                        rgba(color.as_rgba_hex()),
                        px(0.0),
                        gpui::transparent_black(),
                        gpui::BorderStyle::default(),
                    ));
                }
            }
            crate::CanvasCommand::Line {
                from_x,
                from_y,
                to_x,
                to_y,
                width,
                color,
                ..
            } => {
                if let Some(color) = colors.resolve(color) {
                    let mut path = gpui::PathBuilder::stroke(px(f64_to_f32(*width)));
                    path.move_to(point(
                        bounds.origin.x + px(f64_to_f32(*from_x)),
                        bounds.origin.y + px(f64_to_f32(*from_y)),
                    ));
                    path.line_to(point(
                        bounds.origin.x + px(f64_to_f32(*to_x)),
                        bounds.origin.y + px(f64_to_f32(*to_y)),
                    ));
                    if let Ok(path) = path.build() {
                        window.paint_path(path, rgba(color.as_rgba_hex()));
                    }
                }
            }
        }
    }
}

fn render_image<C: ColorResolver>(
    element: impl ParentElement + IntoElement,
    node: &UiNode,
    source: &ImageSourceSpec,
    environment: &RenderEnvironment<'_, C>,
) -> AnyElement {
    let interaction = if is_disabled(node) {
        environment.interaction.clone().with(PseudoState::Disabled)
    } else {
        environment.interaction.clone()
    };
    let tint = node
        .style()
        .resolve(&interaction)
        .text_color
        .as_ref()
        .and_then(|color| environment.colors.resolve(color));
    environment.assets.map_or_else(
        || div().child("Image registry unavailable").into_any_element(),
        |assets| {
            let handle = match source {
                ImageSourceSpec::Handle(handle) => Ok(handle.clone()),
                ImageSourceSpec::Asset(asset) => assets
                    .cached_image(asset)
                    .map(|image| image.opaque().clone()),
            };
            match handle.and_then(|handle| assets.image_source_tinted(&handle, tint)) {
                Ok(source) => element.child(img(source)).into_any_element(),
                Err(error) => div()
                    .child(format!("Image error: {error}"))
                    .into_any_element(),
            }
        },
    )
}

fn scoped_overlay_spec(spec: &OverlayNodeSpec, view_id: &str) -> OverlayNodeSpec {
    let mut rendered = spec.clone();
    rendered.id = WindowOverlayCoordinator::scoped_id(view_id, &rendered.id);
    rendered.parent = rendered
        .parent
        .as_ref()
        .map(|parent| WindowOverlayCoordinator::scoped_id(view_id, parent));
    rendered
}

fn native_overlay_element<C: ColorResolver>(
    node: &UiNode,
    trigger: &UiNode,
    content: &UiNode,
    spec: &OverlayNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    boundary_fallback: Option<&UiNode>,
    (path, retained_id): (&str, Option<NodeId>),
) -> ScriptOverlayElement {
    let mut rendered_spec = scoped_overlay_spec(spec, environment.view_id);
    if rendered_spec.kind == crate::OverlayKind::Tooltip {
        rendered_spec.open = environment
            .overlays
            .tooltip_visible(&rendered_spec.id, std::time::Instant::now());
    }
    let trigger = GpuiNodeRenderer::render_internal(
        trigger,
        environment,
        boundary_fallback,
        &format!("{path}/trigger"),
        retained_child_id(environment.retained, retained_id, "trigger", 0),
    );
    let content = GpuiNodeRenderer::render_internal(
        content,
        environment,
        boundary_fallback,
        &format!("{path}/content"),
        retained_child_id(environment.retained, retained_id, "content", 0),
    );
    let open_change = node.handler("open_change").map(|handler| {
        let handler = handler.clone();
        let dispatcher = environment.dispatcher.cloned();
        Rc::new(move |open, window: &mut Window, cx: &mut App| {
            dispatch_ui_event(
                &handler,
                "open_change",
                UiValue::Bool(open),
                window,
                cx,
                dispatcher.as_ref(),
            );
        }) as crate::overlay_element::OpenChangeHandler
    });
    let panel_key = overlay_panel_key(node, environment.dispatcher, environment.direction);
    let restore_focus_on_close = rendered_spec.kind == crate::OverlayKind::Menu;
    let overlay = ScriptOverlayElement::new(
        path,
        trigger,
        content,
        rendered_spec,
        open_change,
        panel_key,
        environment.overlays.clone(),
    );
    let backdrop_style = node.part_style("backdrop").map(|style| {
        let style = style.clone();
        let colors = OwnedColorResolver::capture(environment.colors);
        let direction = environment.direction;
        Rc::new(move |backdrop: Div| apply_style_override(backdrop, &style, &colors, direction))
            as crate::overlay_element::BackdropStyleHandler
    });
    overlay
        .with_backdrop_style(backdrop_style)
        .with_focus_ring(semantic_color(
            environment.colors,
            "focus_ring",
            0x003b_82f6,
        ))
        .with_focus_surface(semantic_color(environment.colors, "surface", 0x0018_181b))
        .restore_focus_on_close(restore_focus_on_close)
}

fn overlay_panel_key(
    node: &UiNode,
    dispatcher: Option<&NodeEventDispatcher>,
    direction: TextDirection,
) -> Option<crate::overlay_element::PanelKeyHandler> {
    let handlers = node
        .handlers()
        .keys()
        .filter_map(|event| {
            event.strip_prefix("key:").and_then(|key| {
                node.handler(event)
                    .cloned()
                    .map(|handler| (key.to_owned(), handler))
            })
        })
        .collect::<BTreeMap<_, _>>();
    (!handlers.is_empty()).then(|| {
        let dispatcher = dispatcher.cloned();
        let payloads = node
            .handlers()
            .keys()
            .filter_map(|event| {
                event
                    .strip_prefix("key:")
                    .map(|key| (key.to_owned(), node.handler_payload(event).cloned()))
            })
            .collect::<BTreeMap<_, _>>();
        Rc::new(
            move |event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App| {
                let key = logical_keyboard_key(event.keystroke.key.as_str(), direction);
                let Some(callback) = handlers.get(key) else {
                    return false;
                };
                dispatch_ui_event(
                    callback,
                    "key",
                    payloads
                        .get(key)
                        .and_then(Clone::clone)
                        .unwrap_or(UiValue::Null),
                    window,
                    cx,
                    dispatcher.as_ref(),
                );
                true
            },
        ) as crate::overlay_element::PanelKeyHandler
    })
}

fn native_toast_element<C: ColorResolver>(
    node: &UiNode,
    spec: &ToastHostSpec,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
) -> ToastHostElement {
    let dismiss = node.handler("dismiss").map_or_else(
        || Rc::new(|_: String, _: &mut Window, _: &mut App| {}) as ToastDismissHandler,
        |handler| {
            let handler = handler.clone();
            let dispatcher = environment.dispatcher.cloned();
            Rc::new(move |id: String, window: &mut Window, cx: &mut App| {
                dispatch_ui_event(
                    &handler,
                    "dismiss",
                    UiValue::String(id),
                    window,
                    cx,
                    dispatcher.as_ref(),
                );
            }) as ToastDismissHandler
        },
    );
    ToastHostElement::new(
        path,
        spec.clone(),
        ToastPalette {
            surface: semantic_color(environment.colors, "surface_raised", 0x0027_272a),
            text: semantic_color(environment.colors, "text_primary", 0x00f4_f4f5),
            muted: semantic_color(environment.colors, "text_muted", 0x00a1_a1aa),
            border: semantic_color(environment.colors, "border", 0x003f_3f46),
            success: semantic_color(environment.colors, "success", 0x0022_c55e),
            warning: semantic_color(environment.colors, "warning", 0x00f5_9e0b),
            danger: semantic_color(environment.colors, "danger", 0x00ef_4444),
        },
        environment.overlays.clone(),
        dismiss,
        ToastPartStyles {
            styles: node
                .part_styles()
                .map(|(name, style)| (name.to_owned(), style.clone()))
                .collect(),
            colors: OwnedColorResolver::capture(environment.colors),
            direction: environment.direction,
        },
        environment.view_id,
    )
}

fn native_date_picker_element<C: ColorResolver>(
    node: &UiNode,
    spec: &DatePickerNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
) -> DatePickerEntityElement {
    let callbacks = date_picker_callbacks(node, environment.dispatcher);
    let palette = DatePickerPalette {
        surface: semantic_color(environment.colors, "surface", 0x0018_181b),
        hover: semantic_color(environment.colors, "surface_hover", 0x003f_3f46),
        text: semantic_color(environment.colors, "text_primary", 0x00f4_f4f5),
        muted: semantic_color(environment.colors, "text_muted", 0x00a1_a1aa),
        accent: semantic_color(environment.colors, "accent", 0x003b_82f6),
        on_accent: semantic_color(environment.colors, "on_accent", 0x00ff_ffff),
        focus_ring: semantic_color(environment.colors, "focus_ring", 0x003b_82f6),
    };
    let runtime = NodeSlotRuntime {
        colors: OwnedColorResolver::capture(environment.colors),
        primitives: environment.primitives.clone(),
        assets: environment.assets.cloned().unwrap_or_default(),
        dispatcher: environment
            .dispatcher
            .cloned()
            .unwrap_or_else(|| NodeEventDispatcher::new(|_, _, _, _| EventPropagation::Handled)),
        overlays: environment.overlays.clone(),
        animations: environment.animations.clone(),
        signals: environment.signals.clone(),
        geometry: environment.geometry.clone(),
        pointer_capture: environment.pointer_capture.clone(),
        focus_handles: environment.focus_handles.clone(),
        scroll_handles: environment.scroll_handles.clone(),
        virtual_requests: environment.virtual_requests.clone(),
        direction: environment.direction,
        base_path: path.to_owned(),
        view_id: environment.view_id.to_owned(),
        part_styles: owned_part_styles(node),
    };
    DatePickerEntityElement::new(
        path,
        spec.clone(),
        callbacks,
        palette,
        environment.overlays.clone(),
        runtime,
    )
}

fn native_virtual_collection_element<C: ColorResolver>(
    spec: &crate::VirtualCollectionNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
) -> VirtualListEntityElement {
    let runtime = NodeSlotRuntime {
        colors: OwnedColorResolver::capture(environment.colors),
        primitives: environment.primitives.clone(),
        assets: environment.assets.cloned().unwrap_or_default(),
        dispatcher: environment
            .dispatcher
            .cloned()
            .unwrap_or_else(|| NodeEventDispatcher::new(|_, _, _, _| EventPropagation::Handled)),
        overlays: environment.overlays.clone(),
        animations: environment.animations.clone(),
        signals: environment.signals.clone(),
        geometry: environment.geometry.clone(),
        pointer_capture: environment.pointer_capture.clone(),
        focus_handles: environment.focus_handles.clone(),
        scroll_handles: environment.scroll_handles.clone(),
        virtual_requests: environment.virtual_requests.clone(),
        direction: environment.direction,
        base_path: path.to_owned(),
        view_id: environment.view_id.to_owned(),
        part_styles: BTreeMap::new(),
    };
    VirtualListEntityElement::new_collection(path, spec.clone(), runtime)
}

#[derive(Clone, Copy, Default)]
struct NodeAnimationValues {
    opacity: Option<f64>,
    translate_x: Option<f64>,
    translate_y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    clip_height: Option<f64>,
}

#[derive(Clone, Debug, Default)]
struct NodeSignalValues {
    opacity: Option<f64>,
    translate_x: Option<f64>,
    translate_y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    background: Option<ColorValue>,
    text_color: Option<ColorValue>,
    border_color: Option<ColorValue>,
}

fn node_signals(registry: &crate::SignalRegistry, node: &UiNode) -> NodeSignalValues {
    let mut values = NodeSignalValues::default();
    for (property, signal) in node.signal_bindings() {
        let Ok(value) = registry.read(signal) else {
            continue;
        };
        match (property, value) {
            (crate::SignalProperty::Opacity, crate::SignalValue::Float(value)) => {
                values.opacity = Some(value);
            }
            (crate::SignalProperty::TranslateX, crate::SignalValue::Float(value)) => {
                values.translate_x = Some(value);
            }
            (crate::SignalProperty::TranslateY, crate::SignalValue::Float(value)) => {
                values.translate_y = Some(value);
            }
            (crate::SignalProperty::Width, crate::SignalValue::Float(value)) => {
                values.width = Some(value);
            }
            (crate::SignalProperty::Height, crate::SignalValue::Float(value)) => {
                values.height = Some(value);
            }
            (crate::SignalProperty::Background, crate::SignalValue::Color(value)) => {
                values.background = Some(value);
            }
            (crate::SignalProperty::TextColor, crate::SignalValue::Color(value)) => {
                values.text_color = Some(value);
            }
            (crate::SignalProperty::BorderColor, crate::SignalValue::Color(value)) => {
                values.border_color = Some(value);
            }
            _ => debug_assert!(false, "validated signal binding changed type"),
        }
    }
    values
}

fn apply_signal_style(style: &mut StyleProperties, values: &NodeSignalValues) {
    if let Some(width) = values.width {
        style.width = Some(Length::Pixels(width.max(0.0)));
    }
    if let Some(height) = values.height {
        style.height = Some(Length::Pixels(height.max(0.0)));
    }
    if let Some(background) = &values.background {
        style.background = Some(background.clone());
    }
    if let Some(text_color) = &values.text_color {
        style.text_color = Some(text_color.clone());
    }
    if let Some(border_color) = &values.border_color {
        style.border_color = Some(border_color.clone());
    }
}

fn node_animation(values: &BTreeMap<AnimationKey, f64>, path: &str) -> NodeAnimationValues {
    let value = |property| values.get(&AnimationKey::for_node(path, property)).copied();
    NodeAnimationValues {
        opacity: value(AnimationProperty::Opacity),
        translate_x: value(AnimationProperty::TranslateX),
        translate_y: value(AnimationProperty::TranslateY),
        width: value(AnimationProperty::Width),
        height: value(AnimationProperty::Height),
        clip_height: value(AnimationProperty::ClipHeight),
    }
}

fn apply_animated_dimensions(style: &mut StyleProperties, values: NodeAnimationValues) {
    if let Some(width) = values.width {
        style.width = Some(Length::Pixels(width.max(0.0)));
    }
    if let Some(height) = values.height {
        style.height = Some(Length::Pixels(height.max(0.0)));
    }
    if let Some(height) = values.clip_height {
        style.height = Some(Length::Pixels(height.max(0.0)));
    }
}

fn translated(element: AnyElement, x: Option<f64>, y: Option<f64>) -> AnyElement {
    let offset = point(
        px(f64_to_f32(x.unwrap_or(0.0))),
        px(f64_to_f32(y.unwrap_or(0.0))),
    );
    if offset == Point::default() {
        element
    } else {
        TranslatedElement {
            child: Some(element),
            offset,
        }
        .into_any_element()
    }
}

fn f64_to_f32(value: f64) -> f32 {
    value.to_string().parse().unwrap_or_else(|_| {
        if value.is_sign_negative() {
            f32::MIN
        } else {
            f32::MAX
        }
    })
}

struct TranslatedElement {
    child: Option<AnyElement>,
    offset: Point<Pixels>,
}

struct GeometryTrackedElement {
    child: Option<AnyElement>,
    node: NodeId,
    registry: crate::GeometryRegistry,
}

impl Element for GeometryTrackedElement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child = self.child.take().expect("tracked element renders once");
        let layout = child.request_layout(window, cx);
        (layout, child)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Ok(bounds) = crate::GeometryBounds::new(
            f64::from(bounds.origin.x),
            f64::from(bounds.origin.y),
            f64::from(bounds.size.width),
            f64::from(bounds.size.height),
        ) {
            self.registry.update(
                self.node,
                crate::ElementGeometry {
                    layout: bounds,
                    visual: bounds,
                    clip: None,
                },
            );
        }
        child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}

impl IntoElement for GeometryTrackedElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TranslatedElement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child = self.child.take().expect("translated element renders once");
        let layout = child.request_layout(window, cx);
        (layout, child)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.with_element_offset(self.offset, |window| child.prepaint(window, cx));
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.with_element_offset(self.offset, |window| child.paint(window, cx));
    }
}

impl IntoElement for TranslatedElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn date_picker_callbacks(
    node: &UiNode,
    dispatcher: Option<&NodeEventDispatcher>,
) -> DatePickerCallbacks {
    let dispatcher = dispatcher.cloned();
    let change = node.handler("change").map(|handler| {
        let handler = handler.clone();
        Rc::new(
            move |value: Option<String>, window: &mut Window, cx: &mut App| {
                dispatch_ui_event(
                    &handler,
                    "change",
                    value.map_or(UiValue::Null, UiValue::String),
                    window,
                    cx,
                    dispatcher.as_ref(),
                );
            },
        ) as DateChangeHandler
    });
    DatePickerCallbacks { change }
}

fn semantic_color(colors: &impl ColorResolver, token: &str, fallback: u32) -> Rgba8 {
    colors
        .resolve(&ColorValue::Token(token.to_owned()))
        .unwrap_or_else(|| Rgba8::from_rgb_hex(fallback))
}

fn apply_pseudo_backgrounds(
    mut element: Stateful<Div>,
    style: &Style,
    colors: &impl ColorResolver,
) -> Stateful<Div> {
    if let Some(color) = style
        .hover
        .as_ref()
        .and_then(|properties| properties.background.as_ref())
        .and_then(|color| colors.resolve(color))
    {
        element = element.hover(move |style| style.bg(rgba(color.as_rgba_hex())));
    }
    if let Some(color) = style
        .active
        .as_ref()
        .and_then(|properties| properties.background.as_ref())
        .and_then(|color| colors.resolve(color))
    {
        element = element.active(move |style| style.bg(rgba(color.as_rgba_hex())));
    }
    let focus_background = style
        .focus
        .as_ref()
        .and_then(|properties| properties.background.as_ref())
        .and_then(|color| colors.resolve(color));
    let focus_ring = semantic_color(colors, "focus_ring", 0x003b_82f6);
    let focus_surface = semantic_color(colors, "surface", 0x0018_181b);
    element = element.focus(move |style| {
        let style = match focus_background {
            Some(color) => style.bg(rgba(color.as_rgba_hex())),
            None => style,
        };
        focus_ring_shadow(style, focus_ring, focus_surface)
    });
    element
}

fn focus_ring_shadow(
    style: gpui::StyleRefinement,
    color: Rgba8,
    surface: Rgba8,
) -> gpui::StyleRefinement {
    style.shadow(vec![
        BoxShadow {
            color: rgba(color.as_rgba_hex()).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(4.0),
        },
        BoxShadow {
            color: rgba(surface.as_rgba_hex()).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(2.0),
        },
    ])
}

fn is_disabled(node: &UiNode) -> bool {
    node.attributes().get("disabled") == Some(&UiValue::Bool(true))
}

fn apply_style(
    element: Div,
    style: &StyleProperties,
    colors: &impl ColorResolver,
    direction: TextDirection,
) -> Div {
    let mut style = style.clone();
    resolve_style_lengths(&mut style, colors);
    let element = apply_layout(element, &style, direction);
    let element = apply_spacing(element, &style, direction);
    apply_paint_and_text(element, &style, colors)
}

fn resolve_style_lengths(style: &mut StyleProperties, resolver: &impl ColorResolver) {
    let resolve = |value: &mut Option<Length>| {
        *value = (*value).and_then(|value| resolver.resolve_length(value));
    };
    for value in [
        &mut style.width,
        &mut style.height,
        &mut style.min_width,
        &mut style.max_width,
        &mut style.min_height,
        &mut style.max_height,
        &mut style.gap,
    ] {
        resolve(value);
    }
    for value in [
        &mut style.padding.top,
        &mut style.padding.right,
        &mut style.padding.bottom,
        &mut style.padding.left,
        &mut style.padding.start,
        &mut style.padding.end,
        &mut style.margin.top,
        &mut style.margin.right,
        &mut style.margin.bottom,
        &mut style.margin.left,
        &mut style.margin.start,
        &mut style.margin.end,
        &mut style.border_width,
        &mut style.radius,
        &mut style.font_size,
        &mut style.top,
        &mut style.left,
    ] {
        resolve(value);
    }
}

pub(crate) fn apply_style_override(
    element: Div,
    style: &Style,
    colors: &impl ColorResolver,
    direction: TextDirection,
) -> Div {
    apply_style(
        element,
        &style.resolve(&InteractionState::default()),
        colors,
        direction,
    )
}

fn apply_layout(mut element: Div, style: &StyleProperties, text_direction: TextDirection) -> Div {
    if let Some(position) = style.position {
        element = match position {
            PositionMode::Relative => element.relative(),
            PositionMode::Absolute => element.absolute(),
        };
    }
    if let Some(value) = style.top {
        element = inset_top(element, value);
    }
    if let Some(value) = style.left {
        element = inset_left(element, value);
    }
    if matches!(style.overflow_x, Some(OverflowMode::Hidden))
        || matches!(style.overflow_y, Some(OverflowMode::Hidden))
    {
        element = element.overflow_hidden();
    }
    if let Some(direction) = style.direction {
        element = element.flex();
        element = match (direction, text_direction) {
            (FlexDirection::Row, TextDirection::LeftToRight) => element.flex_row(),
            (FlexDirection::Row, TextDirection::RightToLeft) => element.flex_row_reverse(),
            (FlexDirection::Column, _) => element.flex_col(),
        };
    }
    if let Some(align) = style.align {
        let align = if style.direction == Some(FlexDirection::Column)
            && text_direction == TextDirection::RightToLeft
        {
            match align {
                Align::Start => Align::End,
                Align::End => Align::Start,
                other => other,
            }
        } else {
            align
        };
        element = match align {
            Align::Start => element.items_start(),
            Align::Center => element.items_center(),
            Align::End => element.items_end(),
            Align::Stretch => element,
        };
    }
    if let Some(justify) = style.justify {
        element = match justify {
            Justify::Start => element.justify_start(),
            Justify::Center => element.justify_center(),
            Justify::End => element.justify_end(),
            Justify::Between => element.justify_between(),
            Justify::Around => element.justify_around(),
        };
    }
    if let Some(value) = style.width {
        element = width(element, value);
    }
    if let Some(value) = style.height {
        element = height(element, value);
    }
    if let Some(value) = style.min_width {
        element = min_width(element, value);
    }
    if let Some(value) = style.max_width {
        element = max_width(element, value);
    }
    if let Some(value) = style.min_height {
        element = min_height(element, value);
    }
    if let Some(value) = style.max_height {
        element = max_height(element, value);
    }
    if let Some(value) = style.gap {
        element = gap(element, value);
    }
    if style.flex_grow == Some(true) {
        element = element.flex_grow();
    }
    element
}

fn apply_spacing(mut element: Div, style: &StyleProperties, direction: TextDirection) -> Div {
    if let Some(value) = style.padding.top {
        element = padding_top(element, value);
    }
    let (padding_left_value, padding_right_value) = logical_horizontal_edges(
        style.padding.left,
        style.padding.right,
        style.padding.start,
        style.padding.end,
        direction,
    );
    if let Some(value) = padding_right_value {
        element = padding_right(element, value);
    }
    if let Some(value) = style.padding.bottom {
        element = padding_bottom(element, value);
    }
    if let Some(value) = padding_left_value {
        element = padding_left(element, value);
    }
    if let Some(value) = style.margin.top {
        element = margin_top(element, value);
    }
    let (margin_left_value, margin_right_value) = logical_horizontal_edges(
        style.margin.left,
        style.margin.right,
        style.margin.start,
        style.margin.end,
        direction,
    );
    if let Some(value) = margin_right_value {
        element = margin_right(element, value);
    }
    if let Some(value) = style.margin.bottom {
        element = margin_bottom(element, value);
    }
    if let Some(value) = margin_left_value {
        element = margin_left(element, value);
    }
    element
}

fn logical_horizontal_edges(
    mut left: Option<Length>,
    mut right: Option<Length>,
    start: Option<Length>,
    end: Option<Length>,
    direction: TextDirection,
) -> (Option<Length>, Option<Length>) {
    match direction {
        TextDirection::LeftToRight => {
            if start.is_some() {
                left = start;
            }
            if end.is_some() {
                right = end;
            }
        }
        TextDirection::RightToLeft => {
            if start.is_some() {
                right = start;
            }
            if end.is_some() {
                left = end;
            }
        }
    }
    (left, right)
}

fn logical_keyboard_key(key: &str, direction: TextDirection) -> &str {
    match (key, direction) {
        ("left", TextDirection::RightToLeft) => "right",
        ("right", TextDirection::RightToLeft) => "left",
        _ => key,
    }
}

fn apply_paint_and_text(
    mut element: Div,
    style: &StyleProperties,
    colors: &impl ColorResolver,
) -> Div {
    if let Some(color) = style
        .background
        .as_ref()
        .and_then(|color| colors.resolve(color))
    {
        element = element.bg(rgba(color.as_rgba_hex()));
    }
    if let Some(color) = style
        .text_color
        .as_ref()
        .and_then(|color| colors.resolve(color))
    {
        element = element.text_color(rgba(color.as_rgba_hex()));
    }
    if let Some(color) = style
        .border_color
        .as_ref()
        .and_then(|color| colors.resolve(color))
    {
        element = element.border_color(rgba(color.as_rgba_hex()));
    }
    if let Some(value) = style.border_width {
        element = border(element, value);
    }
    if let Some(value) = style.radius {
        element = radius(element, value);
    }
    if let Some(value) = style.font_size {
        element = font_size(element, value);
    }
    element
}

macro_rules! definite_length_fn {
    ($name:ident, $method:ident) => {
        fn $name(element: Div, value: Length) -> Div {
            match value {
                Length::Pixels(value) => element.$method(px(to_f32(value))),
                Length::Rems(value) => element.$method(rems(to_f32(value))),
                Length::Relative(value) => element.$method(relative(to_f32(value))),
                Length::ThemeSpacing(_) | Length::ThemeRadius(_) => element,
            }
        }
    };
}

definite_length_fn!(width, w);
definite_length_fn!(height, h);
definite_length_fn!(min_width, min_w);
definite_length_fn!(max_width, max_w);
definite_length_fn!(min_height, min_h);
definite_length_fn!(max_height, max_h);
definite_length_fn!(gap, gap);
definite_length_fn!(padding_top, pt);
definite_length_fn!(padding_right, pr);
definite_length_fn!(padding_bottom, pb);
definite_length_fn!(padding_left, pl);
definite_length_fn!(margin_top, mt);
definite_length_fn!(margin_right, mr);
definite_length_fn!(margin_bottom, mb);
definite_length_fn!(margin_left, ml);
definite_length_fn!(inset_top, top);
definite_length_fn!(inset_left, left);

fn border(element: Div, value: Length) -> Div {
    match value {
        Length::Pixels(value) => element.border(px(to_f32(value))),
        Length::Rems(value) => element.border(rems(to_f32(value))),
        Length::Relative(_) | Length::ThemeSpacing(_) | Length::ThemeRadius(_) => element,
    }
}

fn radius(element: Div, value: Length) -> Div {
    match value {
        Length::Pixels(value) => element.rounded(px(to_f32(value))),
        Length::Rems(value) => element.rounded(rems(to_f32(value))),
        Length::Relative(_) | Length::ThemeSpacing(_) | Length::ThemeRadius(_) => element,
    }
}

fn font_size(element: Div, value: Length) -> Div {
    match value {
        Length::Pixels(value) => element.text_size(px(to_f32(value))),
        Length::Rems(value) => element.text_size(rems(to_f32(value))),
        Length::Relative(_) | Length::ThemeSpacing(_) | Length::ThemeRadius(_) => element,
    }
}

#[allow(clippy::cast_possible_truncation)]
fn to_f32(value: f64) -> f32 {
    debug_assert!(value.is_finite() && value >= 0.0 && value <= f64::from(f32::MAX));
    value as f32
}

/// Minimal GPUI view for a previously evaluated Rhai tree or a Host-built tree.
pub struct StaticUiView {
    tree: crate::RetainedUiTree,
    primitives: PrimitiveRegistry,
}

impl StaticUiView {
    /// Create a retained view from one accepted root snapshot.
    ///
    /// # Errors
    ///
    /// Returns structural reconciliation errors such as duplicate sibling keys.
    pub fn new(root: UiNode) -> Result<Self, crate::ReconcileError> {
        Self::with_primitives(root, PrimitiveRegistry::new())
    }

    /// Create a retained view with a custom primitive registry.
    ///
    /// # Errors
    ///
    /// Returns structural reconciliation errors such as duplicate sibling keys.
    pub fn with_primitives(
        root: UiNode,
        primitives: PrimitiveRegistry,
    ) -> Result<Self, crate::ReconcileError> {
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(root)?;
        Ok(Self { tree, primitives })
    }

    /// Reconcile and atomically accept a new Host-owned snapshot.
    ///
    /// # Errors
    ///
    /// Returns structural reconciliation errors without changing the live root.
    pub fn set_root(
        &mut self,
        root: UiNode,
        cx: &mut Context<Self>,
    ) -> Result<crate::ReconcileReport, crate::ReconcileError> {
        let report = self.tree.reconcile(root)?;
        cx.notify();
        Ok(report)
    }

    #[must_use]
    pub fn root(&self) -> Option<&UiNode> {
        self.tree.root()
    }

    #[must_use]
    pub const fn retained(&self) -> &crate::RetainedUiTree {
        &self.tree
    }
}

impl Render for StaticUiView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.root().map_or_else(
            || {
                div()
                    .child("Static UI view has no accepted root")
                    .into_any_element()
            },
            |_| {
                GpuiNodeRenderer::render_retained_with_primitives(
                    &self.tree,
                    &LiteralColorResolver,
                    &InteractionState::default(),
                    &self.primitives,
                )
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorValue, Length, Rgba8, Style};

    #[test]
    fn declarative_nodes_and_typed_styles_convert_without_a_gpui_context() {
        let root = UiNode::column(vec![UiNode::text("one"), UiNode::text("two")]).with_style(
            &Style::new()
                .gap(Length::pixels(8.0).unwrap())
                .background(ColorValue::Literal(Rgba8::from_rgb_hex(0x0022_2222))),
        );
        let _element = GpuiNodeRenderer::render(&root);
    }

    #[test]
    fn sampled_animation_values_override_dimensions_and_transform_without_rhai() {
        let values = BTreeMap::from([
            (
                AnimationKey::for_node("root/card", AnimationProperty::Width),
                180.0,
            ),
            (
                AnimationKey::for_node("root/card", AnimationProperty::ClipHeight),
                64.0,
            ),
            (
                AnimationKey::for_node("root/card", AnimationProperty::TranslateX),
                12.0,
            ),
        ]);
        let sampled = node_animation(&values, "root/card");
        let mut style = StyleProperties::default();
        apply_animated_dimensions(&mut style, sampled);
        assert_eq!(style.width, Some(Length::Pixels(180.0)));
        assert_eq!(style.height, Some(Length::Pixels(64.0)));
        assert_eq!(sampled.translate_x, Some(12.0));
    }

    #[test]
    fn native_signal_values_override_approved_properties_without_rhai() {
        let component = crate::ComponentInstancePath::root("Meter", "primary");
        let id =
            crate::SignalId::new(component.clone(), "width", crate::SignalKind::Float).unwrap();
        let signal = crate::NativeSignal::new(id.clone());
        let node = UiNode::text("meter")
            .with_signal_binding(crate::SignalProperty::Width, signal.clone())
            .unwrap();
        let mut registry = crate::SignalRegistry::new();
        registry.reconcile(
            &component,
            BTreeMap::from([(
                id,
                crate::signal::SignalDescriptor::new(crate::SignalValue::Float(80.0)),
            )]),
        );
        registry
            .write(&signal, crate::SignalValue::Float(144.0))
            .unwrap();
        let values = node_signals(&registry, &node);
        let mut style = StyleProperties::default();
        apply_signal_style(&mut style, &values);
        assert_eq!(style.width, Some(Length::Pixels(144.0)));
    }

    #[test]
    fn logical_spacing_resolves_to_physical_edges_in_both_directions() {
        let start = Length::Pixels(12.0);
        let end = Length::Pixels(4.0);
        assert_eq!(
            logical_horizontal_edges(
                None,
                None,
                Some(start),
                Some(end),
                TextDirection::LeftToRight,
            ),
            (Some(start), Some(end))
        );
        assert_eq!(
            logical_horizontal_edges(
                None,
                None,
                Some(start),
                Some(end),
                TextDirection::RightToLeft,
            ),
            (Some(end), Some(start))
        );
    }

    #[test]
    fn rtl_keyboard_navigation_maps_physical_arrows_to_logical_handlers() {
        assert_eq!(
            logical_keyboard_key("left", TextDirection::RightToLeft),
            "right"
        );
        assert_eq!(
            logical_keyboard_key("right", TextDirection::RightToLeft),
            "left"
        );
        assert_eq!(
            logical_keyboard_key("left", TextDirection::LeftToRight),
            "left"
        );
        assert_eq!(
            logical_keyboard_key("down", TextDirection::RightToLeft),
            "down"
        );
    }
}
