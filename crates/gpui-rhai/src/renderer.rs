use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use gpui::{
    AlignSelf as GpuiAlignSelf, AnyElement, App, Background, Bounds, BoxShadow, ClickEvent,
    ContentMask, Context, CursorStyle, DispatchPhase, Div, Element, ElementId, FocusHandle,
    FontFallbacks, FontFeatures, FontStyle, FontWeight, GlobalElementId, HighlightStyle, Image,
    ImageFormat, InspectorElementId, InteractiveElement, IntoElement, LayoutId, Modifiers,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point,
    Render, ScrollHandle, ScrollWheelEvent, SharedString, Stateful, StatefulInteractiveElement,
    Styled, StyledText, TextAlign, Window, auto, div, img, linear_color_stop, linear_gradient,
    point, px, relative, rems, rgba,
};

use crate::overlay_element::{ScriptLayerElement, ScriptOverlayElement, WindowOverlayCoordinator};
use crate::slot_runtime::NodeSlotRuntime;
use crate::virtual_list_element::VirtualListEntityElement;
use crate::{
    Align, AnimationKey, AnimationProperty, AssetRegistry, ColorValue, CursorKind, DisplayMode,
    EventPropagation, EventResponse, FlexDirection, FlexWrapMode, FontSlant, HitTestBehavior,
    ImageSourceSpec, InteractionState, Justify, LayoutLength, Length, NodeId, OverflowMode,
    OverlayNodeSpec, PositionMode, PrimitiveRegistry, PseudoState, RadiusToken, RetainedUiTree,
    Rgba8, ScriptCallback, SignedLength, SpacingToken, Style, StyleProperties, TextAlignMode,
    TextDirection, UiEventHandler, UiNode, UiNodeKind, UiValue, WhiteSpaceMode,
};

type DispatchFn = dyn Fn(
    ScriptCallback,
    UiValue,
    Option<crate::GeometryBounds>,
    &mut Window,
    &mut App,
) -> EventResponse;
type NativeDispatchFn = dyn Fn(
    crate::NativeHandlerRef,
    String,
    UiValue,
    Option<crate::GeometryBounds>,
    &mut Window,
    &mut App,
) -> EventResponse;

#[derive(Clone)]
pub struct NodeEventDispatcher {
    script: Rc<DispatchFn>,
    native: Rc<NativeDispatchFn>,
}

impl NodeEventDispatcher {
    #[must_use]
    pub fn new<R>(
        dispatch: impl Fn(
            ScriptCallback,
            UiValue,
            Option<crate::GeometryBounds>,
            &mut Window,
            &mut App,
        ) -> R
        + 'static,
    ) -> Self
    where
        R: Into<EventResponse>,
    {
        Self {
            script: Rc::new(move |callback, payload, target, window, app| {
                dispatch(callback, payload, target, window, app).into()
            }),
            native: Rc::new(|_, _, _, _, _, _| EventResponse::new().stop()),
        }
    }

    #[must_use]
    pub fn with_native<R>(
        mut self,
        dispatch: impl Fn(
            crate::NativeHandlerRef,
            String,
            UiValue,
            Option<crate::GeometryBounds>,
            &mut Window,
            &mut App,
        ) -> R
        + 'static,
    ) -> Self
    where
        R: Into<EventResponse>,
    {
        self.native = Rc::new(move |handler, event, payload, target, window, app| {
            dispatch(handler, event, payload, target, window, app).into()
        });
        self
    }

    pub(crate) fn dispatch(
        &self,
        callback: ScriptCallback,
        payload: UiValue,
        target: Option<crate::GeometryBounds>,
        window: &mut Window,
        cx: &mut App,
    ) -> EventResponse {
        (self.script)(callback, payload, target, window, cx)
    }

    pub(crate) fn dispatch_native(
        &self,
        handler: crate::NativeHandlerRef,
        event: String,
        payload: UiValue,
        target: Option<crate::GeometryBounds>,
        window: &mut Window,
        app: &mut App,
    ) -> EventResponse {
        (self.native)(handler, event, payload, target, window, app)
    }
}

fn dispatch_ui_event(
    handler: &UiEventHandler,
    event: &str,
    payload: UiValue,
    target: Option<crate::GeometryBounds>,
    window: &mut Window,
    app: &mut App,
    script_dispatcher: Option<&NodeEventDispatcher>,
) -> EventResponse {
    match handler {
        UiEventHandler::Script(callback) => script_dispatcher.map_or_else(
            || EventResponse::new().stop(),
            |dispatcher| dispatcher.dispatch(callback.clone(), payload, target, window, app),
        ),
        UiEventHandler::Host(callback) => callback.invoke(payload, window, app),
        UiEventHandler::Native(reference) => script_dispatcher.map_or_else(
            || EventResponse::new().stop(),
            |dispatcher| {
                dispatcher.dispatch_native(
                    reference.clone(),
                    event.to_owned(),
                    payload,
                    target,
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
    target: Option<crate::GeometryBounds>,
    window: &mut Window,
    app: &mut App,
    script_dispatcher: Option<&NodeEventDispatcher>,
) -> EventResponse {
    dispatch_ui_handler_phases(
        bindings,
        event,
        &[crate::EventPhase::Target],
        payload,
        EventRoute::new(target, script_dispatcher),
        window,
        app,
    )
}

#[derive(Clone, Copy)]
struct EventRoute<'a> {
    target: Option<crate::GeometryBounds>,
    dispatcher: Option<&'a NodeEventDispatcher>,
}

impl<'a> EventRoute<'a> {
    const fn new(
        target: Option<crate::GeometryBounds>,
        dispatcher: Option<&'a NodeEventDispatcher>,
    ) -> Self {
        Self { target, dispatcher }
    }
}

fn dispatch_ui_handler_phases(
    bindings: &[crate::UiEventBinding],
    event: &str,
    phases: &[crate::EventPhase],
    payload: &UiValue,
    route: EventRoute<'_>,
    window: &mut Window,
    app: &mut App,
) -> EventResponse {
    let mut combined = EventResponse::new();
    for phase in phases {
        let mut stop_route = false;
        for binding in bindings.iter().filter(|binding| binding.phase() == *phase) {
            let response = dispatch_ui_event(
                binding.handler(),
                event,
                payload.clone(),
                route.target,
                window,
                app,
                route.dispatcher,
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

fn node_scrollable(node: &UiNode) -> bool {
    matches!(node.style().base.overflow_x, Some(OverflowMode::Scroll))
        || matches!(node.style().base.overflow_y, Some(OverflowMode::Scroll))
}

fn scroll_handles_for_node(
    tree: Option<&RetainedUiTree>,
    node: Option<NodeId>,
    handles: &BTreeMap<NodeId, ScrollHandle>,
) -> Vec<ScrollHandle> {
    let (Some(tree), Some(mut node)) = (tree, node) else {
        return Vec::new();
    };
    let mut resolved = Vec::new();
    while let Some(retained) = tree.node(node) {
        if retained.scrollable()
            && let Some(handle) = handles.get(&node)
        {
            resolved.push(handle.clone());
        }
        let Some(parent) = retained.parent() else {
            break;
        };
        node = parent;
    }
    resolved
}

fn node_has_raw_pointer_handlers(node: &UiNode) -> bool {
    ["pointer_down", "pointer_up", "pointer_move", "wheel"]
        .into_iter()
        .any(|event| !node.event_handlers(event).is_empty())
}

#[derive(Clone)]
struct PointerPayloadContext {
    node: Option<NodeId>,
    geometry: crate::GeometryRegistry,
    canvas: Option<crate::CanvasScene>,
    scroll_handles: Vec<ScrollHandle>,
}

#[derive(Clone)]
struct EventTargetContext {
    node: Option<NodeId>,
    geometry: crate::GeometryRegistry,
}

impl EventTargetContext {
    fn new(node: Option<NodeId>, geometry: crate::GeometryRegistry) -> Self {
        Self { node, geometry }
    }

    fn snapshot(&self) -> Option<crate::GeometryBounds> {
        self.node
            .and_then(|node| self.geometry.get(node))
            .map(|geometry| geometry.visual)
    }
}

impl PointerPayloadContext {
    fn target_bounds(&self) -> Option<crate::GeometryBounds> {
        self.node
            .and_then(|node| self.geometry.get(node))
            .map(|geometry| geometry.visual)
    }

    fn new(
        node: &UiNode,
        retained_id: Option<NodeId>,
        geometry: crate::GeometryRegistry,
        scroll_handles: Vec<ScrollHandle>,
    ) -> Self {
        let canvas = match node.kind() {
            UiNodeKind::Canvas { scene } => Some(scene.clone()),
            _ => None,
        };
        Self {
            node: retained_id,
            geometry,
            canvas,
            scroll_handles,
        }
    }

    fn retained(
        node: NodeId,
        geometry: crate::GeometryRegistry,
        canvas: Option<crate::CanvasScene>,
        scroll_handles: Vec<ScrollHandle>,
    ) -> Self {
        Self {
            node: Some(node),
            geometry,
            canvas,
            scroll_handles,
        }
    }

    fn enrich(&self, payload: UiValue) -> UiValue {
        let UiValue::Map(mut payload) = payload else {
            return payload;
        };
        let geometry = self.node.and_then(|node| self.geometry.get(node));
        payload.insert(
            "target".to_owned(),
            geometry.map_or(UiValue::Null, |geometry| geometry.visual.into_value()),
        );
        let window = payload.get("window").and_then(value_point);
        let local = geometry.and_then(|geometry| {
            window.map(|(x, y)| (x - geometry.visual.x, y - geometry.visual.y))
        });
        if let Some((x, y)) = local {
            payload.insert("local".to_owned(), logical_point_value(x, y));
            let offset = self
                .scroll_handles
                .iter()
                .map(ScrollHandle::offset)
                .fold(point(px(0.0), px(0.0)), |total, offset| {
                    point(total.x + offset.x, total.y + offset.y)
                });
            payload.insert(
                "content".to_owned(),
                logical_point_value(x - f64::from(offset.x), y - f64::from(offset.y)),
            );
            payload.insert(
                "canvas_key".to_owned(),
                self.canvas
                    .as_ref()
                    .and_then(|scene| scene.hit_test(x, y))
                    .map_or(UiValue::Null, |key| UiValue::String(key.to_owned())),
            );
        }
        UiValue::Map(payload)
    }
}

fn value_point(value: &UiValue) -> Option<(f64, f64)> {
    let UiValue::Map(point) = value else {
        return None;
    };
    match (point.get("x"), point.get("y")) {
        (Some(UiValue::Float(x)), Some(UiValue::Float(y))) => Some((*x, *y)),
        _ => None,
    }
}

fn logical_point_value(x: f64, y: f64) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("x".to_owned(), UiValue::Float(x)),
        ("y".to_owned(), UiValue::Float(y)),
    ]))
}

fn apply_scroll_behavior(
    mut element: Stateful<Div>,
    node: &UiNode,
    retained_id: Option<NodeId>,
    handles: &BTreeMap<NodeId, ScrollHandle>,
    anchors: &BTreeMap<NodeId, gpui::ScrollAnchor>,
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
    if let Some(anchor) = retained_id.and_then(|node| anchors.get(&node)) {
        element = element.anchor_scroll(Some(anchor.clone()));
    }
    element
}

fn apply_raw_pointer_handlers(
    element: Stateful<Div>,
    node: &UiNode,
    dispatcher: Option<&NodeEventDispatcher>,
    retained_id: Option<NodeId>,
    captures: &crate::PointerCaptureRegistry,
    geometry: &crate::GeometryRegistry,
    scroll_handles: Vec<ScrollHandle>,
) -> Stateful<Div> {
    let payload = PointerPayloadContext::new(node, retained_id, geometry.clone(), scroll_handles);
    let element =
        apply_pointer_down_handlers(element, node, dispatcher, retained_id, captures, &payload);
    let element =
        apply_pointer_up_handlers(element, node, dispatcher, retained_id, captures, &payload);
    apply_pointer_motion_handlers(
        element,
        node,
        dispatcher.cloned(),
        retained_id,
        captures.clone(),
        &payload,
    )
}

fn apply_hover_handler(
    element: Stateful<Div>,
    hover: Option<(Vec<crate::UiEventBinding>, Option<UiValue>)>,
    dispatcher: Option<NodeEventDispatcher>,
    target: EventTargetContext,
) -> Stateful<Div> {
    element.on_hover(move |hovered, window, cx| {
        if let Some((bindings, value)) = &hover {
            let payload = value.as_ref().map_or_else(
                || UiValue::Bool(*hovered),
                |value| {
                    UiValue::Map(BTreeMap::from([
                        ("hovered".to_owned(), UiValue::Bool(*hovered)),
                        ("value".to_owned(), value.clone()),
                    ]))
                },
            );
            let response = dispatch_ui_handlers(
                bindings,
                "hover_change",
                &payload,
                target.snapshot(),
                window,
                cx,
                dispatcher.as_ref(),
            );
            apply_event_response(response, window, cx);
        }
    })
}

fn apply_pointer_down_handlers(
    mut element: Stateful<Div>,
    node: &UiNode,
    dispatcher: Option<&NodeEventDispatcher>,
    retained_id: Option<NodeId>,
    captures: &crate::PointerCaptureRegistry,
    payload_context: &PointerPayloadContext,
) -> Stateful<Div> {
    let pointer_down = node.event_handlers("pointer_down").to_vec();
    if pointer_down
        .iter()
        .any(|binding| binding.phase() == crate::EventPhase::Capture)
    {
        let bindings = pointer_down.clone();
        let dispatcher = dispatcher.cloned();
        let captures = captures.clone();
        let payload_context = (*payload_context).clone();
        element = element.capture_any_mouse_down(move |event, window, app| {
            let payload = payload_context.enrich(mouse_down_payload(event));
            let response = dispatch_ui_handler_phases(
                &bindings,
                "pointer_down",
                &[crate::EventPhase::Capture],
                &payload,
                EventRoute::new(payload_context.target_bounds(), dispatcher.as_ref()),
                window,
                app,
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
        let payload_context = (*payload_context).clone();
        element = element.on_any_mouse_down(move |event, window, app| {
            let payload = payload_context.enrich(mouse_down_payload(event));
            let response = dispatch_ui_handler_phases(
                &bindings,
                "pointer_down",
                &[crate::EventPhase::Target, crate::EventPhase::Bubble],
                &payload,
                EventRoute::new(payload_context.target_bounds(), dispatcher.as_ref()),
                window,
                app,
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
    payload_context: &PointerPayloadContext,
) -> Stateful<Div> {
    let pointer_up = node.event_handlers("pointer_up").to_vec();
    if pointer_up
        .iter()
        .any(|binding| binding.phase() == crate::EventPhase::Capture)
    {
        let bindings = pointer_up.clone();
        let dispatcher = dispatcher.cloned();
        let captures = captures.clone();
        let payload_context = (*payload_context).clone();
        element = element.capture_any_mouse_up(move |event, window, app| {
            let payload = payload_context.enrich(mouse_up_payload(event));
            let response = dispatch_ui_handler_phases(
                &bindings,
                "pointer_up",
                &[crate::EventPhase::Capture],
                &payload,
                EventRoute::new(payload_context.target_bounds(), dispatcher.as_ref()),
                window,
                app,
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
            let payload_context = (*payload_context).clone();
            element = element.on_mouse_up(button, move |event, window, app| {
                let payload = payload_context.enrich(mouse_up_payload(event));
                let response = dispatch_ui_handler_phases(
                    &bindings,
                    "pointer_up",
                    &[crate::EventPhase::Target, crate::EventPhase::Bubble],
                    &payload,
                    EventRoute::new(payload_context.target_bounds(), dispatcher.as_ref()),
                    window,
                    app,
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
    payload_context: &PointerPayloadContext,
) -> Stateful<Div> {
    let pointer_move = node.event_handlers("pointer_move").to_vec();
    if !pointer_move.is_empty() {
        let dispatcher = dispatcher.clone();
        let captures = captures.clone();
        let payload_context = (*payload_context).clone();
        element = element.on_mouse_move(move |event, window, app| {
            let payload = payload_context.enrich(mouse_move_payload(event));
            let response = dispatch_ui_handler_phases(
                &pointer_move,
                "pointer_move",
                &[crate::EventPhase::Target, crate::EventPhase::Bubble],
                &payload,
                EventRoute::new(payload_context.target_bounds(), dispatcher.as_ref()),
                window,
                app,
            );
            apply_pointer_response(response, retained_id, 0, &captures, window, app);
        });
    }

    let wheel = node.event_handlers("wheel").to_vec();
    if !wheel.is_empty() {
        let payload_context = (*payload_context).clone();
        element = element.on_scroll_wheel(move |event, window, app| {
            let payload = payload_context.enrich(wheel_payload(event));
            let response = dispatch_ui_handler_phases(
                &wheel,
                "wheel",
                &[crate::EventPhase::Target, crate::EventPhase::Bubble],
                &payload,
                EventRoute::new(payload_context.target_bounds(), dispatcher.as_ref()),
                window,
                app,
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
        target: None,
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
        target: None,
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

pub(crate) fn pointer_capture_router_element(
    child: AnyElement,
    tree: &RetainedUiTree,
    dispatcher: &NodeEventDispatcher,
    captures: &crate::PointerCaptureRegistry,
    geometry: &crate::GeometryRegistry,
    scroll_handles: &BTreeMap<NodeId, ScrollHandle>,
) -> AnyElement {
    PointerCaptureRouterElement {
        child: Some(child),
        routes: Some(PointerCaptureRoutes {
            move_handlers: retained_handlers(tree, "pointer_move"),
            up_handlers: retained_handlers(tree, "pointer_up"),
            dispatcher: dispatcher.clone(),
            captures: captures.clone(),
            payload_contexts: tree
                .nodes()
                .map(|node| {
                    (
                        node.id(),
                        PointerPayloadContext::retained(
                            node.id(),
                            geometry.clone(),
                            node.canvas_scene().cloned(),
                            scroll_handles_for_node(Some(tree), Some(node.id()), scroll_handles),
                        ),
                    )
                })
                .collect(),
        }),
    }
    .into_any_element()
}

struct PointerCaptureRoutes {
    move_handlers: BTreeMap<NodeId, Vec<crate::UiEventBinding>>,
    up_handlers: BTreeMap<NodeId, Vec<crate::UiEventBinding>>,
    dispatcher: NodeEventDispatcher,
    captures: crate::PointerCaptureRegistry,
    payload_contexts: BTreeMap<NodeId, PointerPayloadContext>,
}

impl PointerCaptureRoutes {
    fn install(self, window: &mut Window) {
        let Self {
            move_handlers,
            up_handlers,
            dispatcher,
            captures,
            payload_contexts,
        } = self;
        let move_dispatcher = dispatcher.clone();
        let up_dispatcher = dispatcher;
        let move_captures = captures.clone();
        let up_captures = captures;
        let move_payload_contexts = payload_contexts.clone();
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
            let payload = move_payload_contexts.get(&node).map_or_else(
                || mouse_move_payload_with_capture(event, true),
                |context| context.enrich(mouse_move_payload_with_capture(event, true)),
            );
            let target = move_payload_contexts
                .get(&node)
                .and_then(PointerPayloadContext::target_bounds);
            let response = dispatch_ui_handler_phases(
                bindings,
                "pointer_move",
                &[crate::EventPhase::Target, crate::EventPhase::Bubble],
                &payload,
                EventRoute::new(target, Some(&move_dispatcher)),
                window,
                app,
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
                let payload = payload_contexts.get(&node).map_or_else(
                    || mouse_up_payload_with_capture(event, true),
                    |context| context.enrich(mouse_up_payload_with_capture(event, true)),
                );
                let target = payload_contexts
                    .get(&node)
                    .and_then(PointerPayloadContext::target_bounds);
                let response = dispatch_ui_handler_phases(
                    bindings,
                    "pointer_up",
                    &[crate::EventPhase::Target, crate::EventPhase::Bubble],
                    &payload,
                    EventRoute::new(target, Some(&up_dispatcher)),
                    window,
                    app,
                );
                apply_pointer_response(response, Some(node), 0, &up_captures, window, app);
            }
            up_captures.release(0);
            app.stop_propagation();
        });
    }
}

struct PointerCaptureRouterElement {
    child: Option<AnyElement>,
    routes: Option<PointerCaptureRoutes>,
}

impl Element for PointerCaptureRouterElement {
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
        let mut child = self.child.take().expect("pointer router renders once");
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
        self.routes
            .take()
            .expect("pointer router paints once")
            .install(window);
        child.paint(window, cx);
    }
}

impl IntoElement for PointerCaptureRouterElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
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

    fn resolve_typography(&self, role: &str) -> Option<crate::ResolvedTypography> {
        default_typography(role)
    }

    fn resolve_length(&self, length: Length) -> Option<Length> {
        match length {
            Length::Pixels(_) | Length::Rems(_) | Length::Relative(_) => Some(length),
            Length::ThemeSpacing(_) | Length::ThemeRadius(_) => None,
        }
    }
}

fn default_typography(role: &str) -> Option<crate::ResolvedTypography> {
    let (size, line_height, weight) = match role {
        "caption" => (10.0, 14.0, 400),
        "body_small" => (11.0, 14.0, 400),
        "body" => (12.0, 16.0, 400),
        "subtitle" => (13.0, 18.0, 400),
        "title" => (14.0, 20.0, 700),
        "heading" => (16.0, 22.0, 700),
        "display" => (24.0, 30.0, 700),
        "display_large" => (28.0, 34.0, 700),
        _ => return None,
    };
    Some(crate::ResolvedTypography {
        family: None,
        fallbacks: Vec::new(),
        size: Length::Pixels(size),
        line_height: Length::Pixels(line_height),
        weight,
    })
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
    typography: BTreeMap<String, crate::ResolvedTypography>,
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
        let typography = crate::REQUIRED_TYPOGRAPHY
            .iter()
            .filter_map(|role| {
                colors
                    .resolve_typography(role)
                    .map(|value| ((*role).to_owned(), value))
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
            typography,
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

    fn resolve_typography(&self, role: &str) -> Option<crate::ResolvedTypography> {
        self.typography.get(role).cloned()
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
    scroll_anchors: &'a BTreeMap<NodeId, gpui::ScrollAnchor>,
    virtual_requests: &'a crate::VirtualRequestRegistry,
    text_selection: &'a TextSelectionRegistry,
    host_focus: Option<&'a FocusHandle>,
    direction: TextDirection,
    view_id: &'a str,
    retained: Option<&'a RetainedUiTree>,
    retained_links: Option<&'a BTreeMap<NodeId, Vec<crate::RetainedChildLink>>>,
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
    pub scroll_anchors: &'a BTreeMap<NodeId, gpui::ScrollAnchor>,
    pub virtual_requests: &'a crate::VirtualRequestRegistry,
    pub text_selection: &'a TextSelectionRegistry,
    pub host_focus: Option<&'a FocusHandle>,
    pub direction: TextDirection,
    pub root_path: &'a str,
    pub view_id: &'a str,
}

#[derive(Clone, Copy)]
pub(crate) struct RetainedSubtree<'a> {
    pub root: Option<NodeId>,
    pub links: &'a BTreeMap<NodeId, Vec<crate::RetainedChildLink>>,
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
        let scroll_anchors = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let text_selection = TextSelectionRegistry::default();
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
            scroll_anchors: &scroll_anchors,
            virtual_requests: &virtual_requests,
            text_selection: &text_selection,
            host_focus: None,
            direction: TextDirection::LeftToRight,
            view_id: "standalone",
            retained: None,
            retained_links: None,
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
        let scroll_anchors = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let text_selection = TextSelectionRegistry::default();
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
            scroll_anchors: &scroll_anchors,
            virtual_requests: &virtual_requests,
            text_selection: &text_selection,
            host_focus: None,
            direction: TextDirection::LeftToRight,
            view_id: "standalone",
            retained: Some(tree),
            retained_links: None,
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
    pub fn render_retained_with_dispatcher(
        tree: &RetainedUiTree,
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
        let scroll_anchors = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let text_selection = TextSelectionRegistry::default();
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
            scroll_anchors: &scroll_anchors,
            virtual_requests: &virtual_requests,
            text_selection: &text_selection,
            host_focus: None,
            direction: TextDirection::LeftToRight,
            view_id: "standalone",
            retained: Some(tree),
            retained_links: None,
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
        let scroll_anchors = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let text_selection = TextSelectionRegistry::default();
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
            scroll_anchors: &scroll_anchors,
            virtual_requests: &virtual_requests,
            text_selection: &text_selection,
            host_focus: None,
            direction: TextDirection::LeftToRight,
            view_id: "standalone",
            retained: None,
            retained_links: None,
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
        let scroll_anchors = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let text_selection = TextSelectionRegistry::default();
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
            scroll_anchors: &scroll_anchors,
            virtual_requests: &virtual_requests,
            text_selection: &text_selection,
            host_focus: None,
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
            scroll_anchors: resources.scroll_anchors,
            virtual_requests: resources.virtual_requests,
            text_selection: resources.text_selection,
            host_focus: resources.host_focus,
            direction: resources.direction,
            view_id: resources.view_id,
            retained: Some(tree),
            retained_links: None,
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
            scroll_anchors: resources.scroll_anchors,
            virtual_requests: resources.virtual_requests,
            text_selection: resources.text_selection,
            host_focus: resources.host_focus,
            direction: resources.direction,
            view_id: resources.view_id,
            retained: None,
            retained_links: None,
        };
        Self::render_internal(node, &environment, None, path, None)
    }

    pub(crate) fn render_subtree_with_window_runtime_at(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
        resources: &WindowRenderResources<'_>,
        path: &str,
        retained: RetainedSubtree<'_>,
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
            scroll_anchors: resources.scroll_anchors,
            virtual_requests: resources.virtual_requests,
            text_selection: resources.text_selection,
            host_focus: resources.host_focus,
            direction: resources.direction,
            view_id: resources.view_id,
            retained: None,
            retained_links: Some(retained.links),
        };
        Self::render_internal(node, &environment, None, path, retained.root)
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
        normalize_text_content_layout(node, &mut resolved_style);
        let mut element = apply_style(
            div(),
            &resolved_style,
            environment.colors,
            environment.direction,
        );
        if matches!(
            node.kind(),
            UiNodeKind::VirtualCollection { spec } if spec.height.is_none()
        ) {
            element = element.flex_1().min_h(px(0.0));
        }
        if let Some(handle) = retained_id.and_then(|node| environment.focus_handles.get(&node)) {
            element = element.track_focus(handle);
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
            animation.rotate,
        );
        let translate_x = signals
            .translate_x
            .or(animation.translate_x)
            .or(resolved_style.translate_x);
        let translate_y = signals
            .translate_y
            .or(animation.translate_y)
            .or(resolved_style.translate_y);
        let element = translated(populated, translate_x, translate_y);
        match retained_id {
            Some(node) => GeometryTrackedElement {
                child: Some(element),
                node,
                registry: environment.geometry.clone(),
                translate_x: translate_x.unwrap_or(0.0),
                translate_y: translate_y.unwrap_or(0.0),
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
        rotation: Option<f64>,
    ) -> AnyElement {
        let click = (!node.event_handlers("click").is_empty()).then(|| {
            (
                node.event_handlers("click").to_vec(),
                node.handler_payload("click")
                    .cloned()
                    .unwrap_or(UiValue::Null),
            )
        });
        let hover = (!node.event_handlers("hover_change").is_empty()).then(|| {
            (
                node.event_handlers("hover_change").to_vec(),
                node.handler_payload("hover_change").cloned(),
            )
        });
        let key_handlers = key_handler_bindings(node);
        let hit_test = resolved_hit_test(node, environment);
        let needs = u8::from(click.is_some())
            | u8::from(hover.is_some()) << 1
            | u8::from(!key_handlers.is_empty()) << 2
            | u8::from(hit_test.is_some()) << 3;
        if !node_needs_interaction_wrapper(node, needs) {
            return Self::populate(
                element,
                node,
                environment,
                boundary_fallback,
                path,
                retained_id,
                rotation,
            );
        }

        let click_dispatcher = environment.dispatcher.cloned();
        let hover_dispatcher = environment.dispatcher.cloned();
        let keyboard_dispatcher = environment.dispatcher.cloned();
        let event_target = EventTargetContext::new(retained_id, environment.geometry.clone());
        let click_target = event_target.clone();
        let hover_target = event_target.clone();
        let keyboard_target = event_target;
        let keyboard_click = click.clone();
        let text_direction = environment.direction;
        let stable_id = interaction_element_id(retained_id, path);
        let debug_path = path.to_owned();
        let element = apply_pseudo_backgrounds(
            element
                .id(SharedString::from(stable_id))
                .debug_selector(move || debug_path.clone()),
            node.style(),
            environment.colors,
        );
        let element = apply_hit_test(element, hit_test);
        let element = apply_tab_behavior(element, node);
        let element = apply_environment_scroll(element, node, retained_id, environment);
        let element = element.on_click(move |event, window, cx| {
            if matches!(event, ClickEvent::Mouse(_))
                && let Some((bindings, payload)) = &click
            {
                let response = dispatch_ui_handlers(
                    bindings,
                    "click",
                    payload,
                    click_target.snapshot(),
                    window,
                    cx,
                    click_dispatcher.as_ref(),
                );
                apply_event_response(response, window, cx);
            }
        });
        let element = apply_hover_handler(element, hover, hover_dispatcher, hover_target);
        let element = element.on_key_down(move |event, window, cx| {
            let semantic_key = logical_keyboard_key(event.keystroke.key.as_str(), text_direction);
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
                    keyboard_target.snapshot(),
                    window,
                    cx,
                    keyboard_dispatcher.as_ref(),
                );
                apply_event_response(response, window, cx);
            }
        });
        let element = apply_environment_raw_pointer(element, node, retained_id, environment);
        Self::populate(
            element,
            node,
            environment,
            boundary_fallback,
            path,
            retained_id,
            rotation,
        )
    }

    fn populate<C: ColorResolver>(
        element: impl ParentElement + IntoElement,
        node: &UiNode,
        environment: &RenderEnvironment<'_, C>,
        boundary_fallback: Option<&UiNode>,
        path: &str,
        retained_id: Option<NodeId>,
        rotation: Option<f64>,
    ) -> AnyElement {
        match node.kind() {
            UiNodeKind::Text { text } => {
                render_text_node(element, node, text.as_str(), environment, path, retained_id)
            }
            UiNodeKind::RichText { text, spans } => element
                .child(styled_text(text.as_str(), spans, environment.colors))
                .into_any_element(),
            UiNodeKind::Canvas { scene } => {
                render_canvas(element, scene, environment.colors, rotation)
            }
            UiNodeKind::Svg { source } => render_inline_svg(element, node, source, environment),
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                let element = element.children(render_flattened_children(
                    children,
                    environment,
                    boundary_fallback,
                    path,
                    retained_id,
                ));
                decorate_scrollbars(element, node, environment, path, retained_id)
                    .into_any_element()
            }
            UiNodeKind::Custom { primitive } => element
                .child(environment.primitives.element(
                    primitive.clone(),
                    retained_id,
                    boundary_fallback.cloned(),
                    environment.dispatcher.cloned(),
                    crate::PrimitiveTheme::capture_with_direction(
                        environment.colors,
                        environment.direction,
                    ),
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
            UiNodeKind::Layer { content, spec } => render_layer_node(
                element,
                content,
                spec,
                environment,
                boundary_fallback,
                path,
                retained_id,
            ),
            UiNodeKind::VirtualCollection { spec } => {
                render_virtual_collection(element, spec, environment, path, retained_id)
            }
            UiNodeKind::ErrorBoundary { child, fallback } => element
                .child(Self::render_internal(
                    child,
                    environment,
                    Some(fallback),
                    &format!("{path}/boundary"),
                    retained_child_id(
                        environment.retained,
                        environment.retained_links,
                        retained_id,
                        "child",
                        0,
                    ),
                ))
                .into_any_element(),
        }
    }
}

fn render_layer_node<C: ColorResolver>(
    element: impl ParentElement + IntoElement,
    content: &UiNode,
    spec: &crate::LayerNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    boundary_fallback: Option<&UiNode>,
    path: &str,
    retained_id: Option<NodeId>,
) -> AnyElement {
    let content = GpuiNodeRenderer::render_internal(
        content,
        environment,
        boundary_fallback,
        &format!("{path}/content"),
        retained_child_id(
            environment.retained,
            environment.retained_links,
            retained_id,
            "content",
            0,
        ),
    );
    element
        .child(ScriptLayerElement::new(
            path,
            content,
            spec.clone(),
            environment.overlays.clone(),
            environment.view_id,
        ))
        .into_any_element()
}

fn decorate_scrollbars<C, E>(
    element: E,
    node: &UiNode,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
    retained_id: Option<NodeId>,
) -> E
where
    C: ColorResolver,
    E: ParentElement + IntoElement,
{
    let (Some(spec), Some(handle)) = (
        crate::ScrollbarSpec::from_node(node),
        retained_id.and_then(|node| environment.scroll_handles.get(&node)),
    ) else {
        return element;
    };
    let part_color = |part, token, fallback| {
        node.part_style(part)
            .and_then(|style| {
                style
                    .resolve(&InteractionState::default())
                    .background
                    .as_ref()
                    .and_then(|color| environment.colors.resolve(color))
            })
            .unwrap_or_else(|| semantic_color(environment.colors, token, fallback))
    };
    element.child(crate::scrollbar::ThemedScrollbar::new(
        format!("{path}/scrollbars"),
        handle.clone(),
        spec,
        environment.direction,
        part_color("scrollbar_track", "surface_raised", 0x0027_272aff),
        part_color("scrollbar_thumb", "text_muted", 0x0071_717aff),
        part_color("scrollbar_thumb_hover", "accent", 0x003b_82f6ff),
    ))
}

fn normalize_text_content_layout(node: &UiNode, style: &mut StyleProperties) {
    if matches!(
        node.kind(),
        UiNodeKind::Text { .. } | UiNodeKind::RichText { .. }
    ) && style.display.is_none()
        && (style.align.is_some() || style.justify.is_some())
    {
        style.display = Some(DisplayMode::Flex);
    }
}

fn render_text_node<C: ColorResolver>(
    element: impl ParentElement + IntoElement,
    node: &UiNode,
    text: &str,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
    retained_id: Option<NodeId>,
) -> AnyElement {
    if !node_selectable(node) {
        return element.child(text.to_owned()).into_any_element();
    }
    let highlight = environment
        .colors
        .resolve(&ColorValue::Token("selection".to_owned()))
        .unwrap_or(Rgba8::from_rgba_hex(0x3b82_f655));
    let selection_id = retained_id.map_or_else(
        || path.to_owned(),
        |node| format!("{}:{node}", environment.view_id),
    );
    element
        .child(SelectableText::new(
            selection_id,
            retained_id,
            text,
            highlight,
            environment.text_selection.clone(),
            environment.host_focus.cloned(),
        ))
        .into_any_element()
}

fn render_virtual_collection<C: ColorResolver>(
    element: impl ParentElement + IntoElement,
    spec: &crate::VirtualCollectionNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
    retained_id: Option<NodeId>,
) -> AnyElement {
    element
        .child(native_virtual_collection_element(
            spec,
            environment,
            path,
            retained_id,
        ))
        .into_any_element()
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
        let child_id = retained_child_id(
            environment.retained,
            environment.retained_links,
            retained_id,
            "children",
            index,
        );
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

fn node_needs_interaction_wrapper(node: &UiNode, needs: u8) -> bool {
    needs & 0b1000 != 0
        || !is_disabled(node)
            && (needs & 0b0111 != 0
                || node_has_focus_declaration(node)
                || node_has_raw_pointer_handlers(node)
                || node_scrollable(node))
}

fn resolved_hit_test<C: ColorResolver>(
    node: &UiNode,
    environment: &RenderEnvironment<'_, C>,
) -> Option<HitTestBehavior> {
    let interaction = if is_disabled(node) {
        environment.interaction.clone().with(PseudoState::Disabled)
    } else {
        environment.interaction.clone()
    };
    node.style().resolve(&interaction).hit_test
}

fn apply_hit_test(element: Stateful<Div>, hit_test: Option<HitTestBehavior>) -> Stateful<Div> {
    match hit_test {
        Some(HitTestBehavior::Block) => element.occlude(),
        Some(HitTestBehavior::BlockExceptScroll) => element.block_mouse_except_scroll(),
        None => element,
    }
}

fn node_has_focus_declaration(node: &UiNode) -> bool {
    node.attributes().contains_key("tab_index")
        || node.attributes().get("tab_group") == Some(&UiValue::Bool(true))
        || node.attributes().get("tab_stop") == Some(&UiValue::Bool(true))
}

fn apply_environment_scroll<C: ColorResolver>(
    element: Stateful<Div>,
    node: &UiNode,
    retained_id: Option<NodeId>,
    environment: &RenderEnvironment<'_, C>,
) -> Stateful<Div> {
    apply_scroll_behavior(
        element,
        node,
        retained_id,
        environment.scroll_handles,
        environment.scroll_anchors,
    )
}

fn apply_environment_raw_pointer<C: ColorResolver>(
    element: Stateful<Div>,
    node: &UiNode,
    retained_id: Option<NodeId>,
    environment: &RenderEnvironment<'_, C>,
) -> Stateful<Div> {
    apply_raw_pointer_handlers(
        element,
        node,
        environment.dispatcher,
        retained_id,
        environment.pointer_capture,
        environment.geometry,
        scroll_handles_for_node(
            environment.retained,
            retained_id,
            environment.scroll_handles,
        ),
    )
}

fn node_tab_stop(node: &UiNode) -> bool {
    !matches!(
        node.attributes().get("tab_stop"),
        Some(UiValue::Bool(false))
    )
}

fn node_selectable(node: &UiNode) -> bool {
    matches!(
        node.attributes().get("selectable"),
        Some(UiValue::Bool(true))
    )
}

fn node_tab_index(node: &UiNode) -> isize {
    match node.attributes().get("tab_index") {
        Some(UiValue::Integer(index)) => isize::try_from(*index).unwrap_or_default(),
        _ => 0,
    }
}

fn apply_tab_behavior(mut element: Stateful<Div>, node: &UiNode) -> Stateful<Div> {
    element = element
        .tab_index(node_tab_index(node))
        .tab_stop(node_tab_stop(node));
    if node.attributes().get("tab_group") == Some(&UiValue::Bool(true)) {
        element = element.tab_group();
    }
    element
}

fn interaction_element_id(retained_id: Option<NodeId>, path: &str) -> String {
    retained_id.map_or_else(|| path.to_owned(), |id| format!("gpui-rhai-node-{id}"))
}

fn retained_child_id(
    tree: Option<&RetainedUiTree>,
    links: Option<&BTreeMap<NodeId, Vec<crate::RetainedChildLink>>>,
    parent: Option<NodeId>,
    group: &str,
    index: usize,
) -> Option<NodeId> {
    let parent = parent?;
    tree.and_then(|tree| tree.node(parent))
        .and_then(|node| {
            node.children()
                .filter(|child| child.group() == group)
                .nth(index)
        })
        .or_else(|| {
            links
                .and_then(|links| links.get(&parent))
                .and_then(|children| {
                    children
                        .iter()
                        .filter(|child| child.group() == group)
                        .nth(index)
                })
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
    rotate: Option<f64>,
) -> AnyElement {
    let scene = scene.clone();
    let colors = OwnedColorResolver::capture(colors);
    let canvas = gpui::canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            paint_canvas_scene(bounds, &scene, &colors, rotate.unwrap_or(0.0), window);
        },
    )
    .size_full();
    element.child(canvas).into_any_element()
}

fn paint_canvas_scene(
    bounds: Bounds<Pixels>,
    scene: &crate::CanvasScene,
    colors: &impl ColorResolver,
    rotate: f64,
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
                    path.move_to(rotated_canvas_point(bounds, *from_x, *from_y, rotate));
                    path.line_to(rotated_canvas_point(bounds, *to_x, *to_y, rotate));
                    if let Ok(path) = path.build() {
                        window.paint_path(path, rgba(color.as_rgba_hex()));
                    }
                }
            }
            crate::CanvasCommand::Path { .. } => {
                paint_canvas_path(bounds, command, colors, rotate, window);
            }
        }
    }
}

fn paint_canvas_path(
    bounds: Bounds<Pixels>,
    command: &crate::CanvasCommand,
    colors: &impl ColorResolver,
    rotate: f64,
    window: &mut Window,
) {
    let crate::CanvasCommand::Path {
        segments,
        fill,
        stroke,
        transform,
        clip,
        ..
    } = command
    else {
        return;
    };
    let mut builder = stroke
        .as_ref()
        .map_or_else(gpui::PathBuilder::fill, |(_, width)| {
            gpui::PathBuilder::stroke(px(f64_to_f32(*width)))
        });
    for segment in segments {
        match segment {
            crate::CanvasPathSegment::Move { x, y } => {
                builder.move_to(canvas_path_point(bounds, *transform, *x, *y, rotate));
            }
            crate::CanvasPathSegment::Line { x, y } => {
                builder.line_to(canvas_path_point(bounds, *transform, *x, *y, rotate));
            }
            crate::CanvasPathSegment::Quadratic {
                x,
                y,
                control_x,
                control_y,
            } => builder.curve_to(
                canvas_path_point(bounds, *transform, *x, *y, rotate),
                canvas_path_point(bounds, *transform, *control_x, *control_y, rotate),
            ),
            crate::CanvasPathSegment::Cubic {
                x,
                y,
                control_a_x,
                control_a_y,
                control_b_x,
                control_b_y,
            } => builder.cubic_bezier_to(
                canvas_path_point(bounds, *transform, *x, *y, rotate),
                canvas_path_point(bounds, *transform, *control_a_x, *control_a_y, rotate),
                canvas_path_point(bounds, *transform, *control_b_x, *control_b_y, rotate),
            ),
            crate::CanvasPathSegment::Close => builder.close(),
        }
    }
    let Ok(path) = builder.build() else {
        return;
    };
    let paint = stroke
        .as_ref()
        .and_then(|(color, _)| {
            colors
                .resolve(color)
                .map(|color| Background::from(rgba(color.as_rgba_hex())))
        })
        .or_else(|| fill.as_ref().and_then(|fill| canvas_fill(fill, colors)));
    let Some(paint) = paint else {
        return;
    };
    if let Some(clip) = clip {
        let mask = ContentMask {
            bounds: Bounds::new(
                point(
                    bounds.origin.x + px(f64_to_f32(clip.x)),
                    bounds.origin.y + px(f64_to_f32(clip.y)),
                ),
                gpui::size(px(f64_to_f32(clip.width)), px(f64_to_f32(clip.height))),
            ),
        };
        window.with_content_mask(Some(mask), |window| window.paint_path(path, paint));
    } else {
        window.paint_path(path, paint);
    }
}

fn canvas_fill(fill: &crate::CanvasFill, colors: &impl ColorResolver) -> Option<Background> {
    match fill {
        crate::CanvasFill::Solid(color) => colors
            .resolve(color)
            .map(|color| Background::from(rgba(color.as_rgba_hex()))),
        crate::CanvasFill::LinearGradient(gradient) => {
            let from = colors.resolve(&gradient.from)?;
            let to = colors.resolve(&gradient.to)?;
            Some(linear_gradient(
                f64_to_f32(gradient.angle_degrees),
                linear_color_stop(rgba(from.as_rgba_hex()), 0.0),
                linear_color_stop(rgba(to.as_rgba_hex()), 1.0),
            ))
        }
    }
}

fn canvas_path_point(
    bounds: Bounds<Pixels>,
    transform: crate::CanvasTransform,
    x: f64,
    y: f64,
    node_rotate: f64,
) -> Point<Pixels> {
    let radians = transform.rotate_degrees.to_radians();
    let scaled_x = x * transform.scale;
    let scaled_y = y * transform.scale;
    let rotated_x = scaled_x * radians.cos() - scaled_y * radians.sin();
    let rotated_y = scaled_x * radians.sin() + scaled_y * radians.cos();
    rotated_canvas_point(
        bounds,
        rotated_x + transform.translate_x,
        rotated_y + transform.translate_y,
        node_rotate,
    )
}

fn rotated_canvas_point(bounds: Bounds<Pixels>, x: f64, y: f64, degrees: f64) -> Point<Pixels> {
    let center_x = f64::from(bounds.size.width) / 2.0;
    let center_y = f64::from(bounds.size.height) / 2.0;
    let radians = degrees.to_radians();
    let local_x = x - center_x;
    let local_y = y - center_y;
    let rotated_x = local_x * radians.cos() - local_y * radians.sin() + center_x;
    let rotated_y = local_x * radians.sin() + local_y * radians.cos() + center_y;
    point(
        bounds.origin.x + px(f64_to_f32(rotated_x)),
        bounds.origin.y + px(f64_to_f32(rotated_y)),
    )
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
                Ok(source) => element.child(img(source).size_full()).into_any_element(),
                Err(error) => div()
                    .child(format!("Image error: {error}"))
                    .into_any_element(),
            }
        },
    )
}

fn render_inline_svg<C: ColorResolver>(
    element: impl ParentElement + IntoElement,
    node: &UiNode,
    source: &crate::InlineSvg,
    environment: &RenderEnvironment<'_, C>,
) -> AnyElement {
    let interaction = if is_disabled(node) {
        environment.interaction.clone().with(PseudoState::Disabled)
    } else {
        environment.interaction.clone()
    };
    let color = node
        .style()
        .resolve(&interaction)
        .text_color
        .as_ref()
        .and_then(|color| environment.colors.resolve(color));
    let bytes = color.map_or_else(
        || source.as_str().as_bytes().to_vec(),
        |color| {
            let rgb = color.as_rgba_hex() >> 8;
            source
                .as_str()
                .replace("currentColor", &format!("#{rgb:06x}"))
                .replace("currentcolor", &format!("#{rgb:06x}"))
                .into_bytes()
        },
    );
    element
        .child(img(Arc::new(Image::from_bytes(ImageFormat::Svg, bytes))))
        .into_any_element()
}

fn scoped_overlay_spec(
    spec: &OverlayNodeSpec,
    view_id: &str,
    direction: TextDirection,
) -> OverlayNodeSpec {
    let mut rendered = spec.clone();
    rendered.id = WindowOverlayCoordinator::scoped_id(view_id, &rendered.id);
    rendered.parent = rendered
        .parent
        .as_ref()
        .map(|parent| WindowOverlayCoordinator::scoped_id(view_id, parent));
    rendered.placement = match (rendered.placement, direction) {
        (crate::OverlayPlacement::Start, TextDirection::LeftToRight)
        | (crate::OverlayPlacement::End, TextDirection::RightToLeft) => {
            crate::OverlayPlacement::Left
        }
        (crate::OverlayPlacement::End, TextDirection::LeftToRight)
        | (crate::OverlayPlacement::Start, TextDirection::RightToLeft) => {
            crate::OverlayPlacement::Right
        }
        (placement, _) => placement,
    };
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
    let mut rendered_spec = scoped_overlay_spec(spec, environment.view_id, environment.direction);
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
        retained_child_id(
            environment.retained,
            environment.retained_links,
            retained_id,
            "trigger",
            0,
        ),
    );
    let content = GpuiNodeRenderer::render_internal(
        content,
        environment,
        boundary_fallback,
        &format!("{path}/content"),
        retained_child_id(
            environment.retained,
            environment.retained_links,
            retained_id,
            "content",
            0,
        ),
    );
    let event_target = EventTargetContext::new(retained_id, environment.geometry.clone());
    let open_target = event_target.clone();
    let open_change = node.handler("open_change").map(|handler| {
        let handler = handler.clone();
        let dispatcher = environment.dispatcher.cloned();
        Rc::new(move |open, window: &mut Window, cx: &mut App| {
            dispatch_ui_event(
                &handler,
                "open_change",
                UiValue::Bool(open),
                open_target.snapshot(),
                window,
                cx,
                dispatcher.as_ref(),
            );
        }) as crate::overlay_element::OpenChangeHandler
    });
    let panel_key = overlay_panel_key(
        node,
        environment.dispatcher,
        environment.direction,
        event_target,
    );
    let restore_focus_on_close =
        rendered_spec.modal || rendered_spec.kind == crate::OverlayKind::Menu;
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
    target: EventTargetContext,
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
                    target.snapshot(),
                    window,
                    cx,
                    dispatcher.as_ref(),
                );
                true
            },
        ) as crate::overlay_element::PanelKeyHandler
    })
}

fn native_virtual_collection_element<C: ColorResolver>(
    spec: &crate::VirtualCollectionNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
    retained_id: Option<NodeId>,
) -> VirtualListEntityElement {
    let retained_roots: BTreeMap<String, NodeId> = environment
        .retained
        .and_then(|tree| retained_id.and_then(|id| tree.node(id)))
        .map(|retained| {
            retained
                .children()
                .filter(|child| child.group() == "items")
                .zip(spec.realized.keys())
                .filter_map(|(child, index)| {
                    crate::virtual_list_element::collection_item_key(spec, *index)
                        .map(|key| (format!("item:{key}"), child.node()))
                })
                .collect()
        })
        .unwrap_or_default();
    let retained_links = environment.retained.map_or_else(BTreeMap::new, |tree| {
        retained_link_subtrees(tree, retained_roots.values().copied())
    });
    let runtime = NodeSlotRuntime {
        colors: OwnedColorResolver::capture(environment.colors),
        primitives: environment.primitives.clone(),
        assets: environment.assets.cloned().unwrap_or_default(),
        dispatcher: environment
            .dispatcher
            .cloned()
            .unwrap_or_else(|| NodeEventDispatcher::new(|_, _, _, _, _| EventPropagation::Handled)),
        overlays: environment.overlays.clone(),
        animations: environment.animations.clone(),
        signals: environment.signals.clone(),
        geometry: environment.geometry.clone(),
        pointer_capture: environment.pointer_capture.clone(),
        focus_handles: environment.focus_handles.clone(),
        scroll_handles: environment.scroll_handles.clone(),
        scroll_anchors: environment.scroll_anchors.clone(),
        virtual_requests: environment.virtual_requests.clone(),
        text_selection: environment.text_selection.clone(),
        host_focus: environment.host_focus.cloned(),
        direction: environment.direction,
        base_path: path.to_owned(),
        view_id: environment.view_id.to_owned(),
        retained_roots,
        retained_links,
    };
    VirtualListEntityElement::new_collection(path, spec.clone(), runtime)
}

fn retained_link_subtrees(
    tree: &RetainedUiTree,
    roots: impl IntoIterator<Item = NodeId>,
) -> BTreeMap<NodeId, Vec<crate::RetainedChildLink>> {
    let mut links = BTreeMap::new();
    let mut pending = roots.into_iter().collect::<Vec<_>>();
    while let Some(node) = pending.pop() {
        let Some(retained) = tree.node(node) else {
            continue;
        };
        let children = retained.children().cloned().collect::<Vec<_>>();
        pending.extend(children.iter().map(crate::RetainedChildLink::node));
        links.insert(node, children);
    }
    links
}

#[derive(Clone, Copy, Default)]
struct NodeAnimationValues {
    opacity: Option<f64>,
    translate_x: Option<f64>,
    translate_y: Option<f64>,
    rotate: Option<f64>,
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
        style.width = Some(Length::Pixels(width.max(0.0)).into());
    }
    if let Some(height) = values.height {
        style.height = Some(Length::Pixels(height.max(0.0)).into());
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
        rotate: value(AnimationProperty::Rotate),
        width: value(AnimationProperty::Width),
        height: value(AnimationProperty::Height),
        clip_height: value(AnimationProperty::ClipHeight),
    }
}

fn apply_animated_dimensions(style: &mut StyleProperties, values: NodeAnimationValues) {
    if let Some(width) = values.width {
        style.width = Some(Length::Pixels(width.max(0.0)).into());
    }
    if let Some(height) = values.height {
        style.height = Some(Length::Pixels(height.max(0.0)).into());
    }
    if let Some(height) = values.clip_height {
        style.height = Some(Length::Pixels(height.max(0.0)).into());
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

#[derive(Clone, Debug)]
struct ActiveTextSelection {
    owner: String,
    node: Option<NodeId>,
    text: String,
    range: Option<(usize, usize)>,
    bounds: Bounds<Pixels>,
}

/// One active text selection per mounted script view.
#[derive(Clone, Debug, Default)]
pub(crate) struct TextSelectionRegistry {
    active: Rc<RefCell<Option<ActiveTextSelection>>>,
}

impl TextSelectionRegistry {
    fn begin(&self, owner: String, node: Option<NodeId>, text: String, bounds: Bounds<Pixels>) {
        self.active.borrow_mut().replace(ActiveTextSelection {
            owner,
            node,
            text,
            range: None,
            bounds,
        });
    }

    fn update(&self, owner: &str, text: &str, bounds: Bounds<Pixels>, range: (usize, usize)) {
        let mut active = self.active.borrow_mut();
        let Some(selection) = active.as_mut().filter(|selection| selection.owner == owner) else {
            return;
        };
        text.clone_into(&mut selection.text);
        selection.range = (range.1 > range.0).then_some(range);
        selection.bounds = bounds;
    }

    fn range(&self, owner: &str, text: &str) -> Option<(usize, usize)> {
        let mut active = self.active.borrow_mut();
        let selection = active.as_ref()?;
        if selection.owner != owner {
            return None;
        }
        if selection.text != text {
            active.take();
            return None;
        }
        selection.range
    }

    pub(crate) fn selected_text(&self) -> Option<String> {
        let active = self.active.borrow();
        let selection = active.as_ref()?;
        let (start, end) = selection.range?;
        selection.text.get(start..end).map(ToOwned::to_owned)
    }

    pub(crate) fn clear_outside(&self, position: Point<Pixels>) -> bool {
        let mut active = self.active.borrow_mut();
        if active
            .as_ref()
            .is_some_and(|selection| !selection.bounds.contains(&position))
        {
            active.take();
            true
        } else {
            false
        }
    }

    pub(crate) fn retain(&self, tree: &RetainedUiTree) {
        let mut active = self.active.borrow_mut();
        if active
            .as_ref()
            .and_then(|selection| selection.node)
            .is_some_and(|node| tree.node(node).is_none())
        {
            active.take();
        }
    }
}

/// Drag-to-select text for a `text().selectable(true)` node. Selection is
/// single-node and retained by stable node identity. Clipboard mutation is
/// deliberately handled by the host's normal copy action, not mouse-up.
struct SelectableText {
    id: ElementId,
    owner: String,
    node: Option<NodeId>,
    text: StyledText,
    highlight: Rgba8,
    selection: TextSelectionRegistry,
    host_focus: Option<FocusHandle>,
}

#[derive(Default)]
struct SelectableTextState {
    anchor: Rc<std::cell::Cell<Option<usize>>>,
}

impl SelectableText {
    fn new(
        owner: String,
        node: Option<NodeId>,
        text: &str,
        highlight: Rgba8,
        selection: TextSelectionRegistry,
        host_focus: Option<FocusHandle>,
    ) -> Self {
        Self {
            id: ElementId::Name(SharedString::from(format!("{owner}/selectable"))),
            owner,
            node,
            text: StyledText::new(text.to_owned()),
            highlight,
            selection,
            host_focus,
        }
    }
}

pub(crate) fn selectable_text_element(
    owner: String,
    text: &str,
    highlight: Rgba8,
    selection: TextSelectionRegistry,
    host_focus: Option<FocusHandle>,
) -> AnyElement {
    SelectableText::new(owner, None, text, highlight, selection, host_focus).into_any_element()
}

/// Build one tight highlight rectangle per visual line from UTF-8 character
/// boundaries. This avoids painting trailing whitespace across wrapped lines.
fn selection_rects(layout: &gpui::TextLayout, start: usize, end: usize) -> Vec<Bounds<Pixels>> {
    let text = layout.text();
    let Some(selected) = text.get(start..end) else {
        return Vec::new();
    };
    let mut rows: Vec<(Pixels, Pixels, Pixels)> = Vec::new();
    for index in selected
        .char_indices()
        .map(|(offset, _)| start + offset)
        .chain(std::iter::once(end))
    {
        let Some(position) = layout.position_for_index(index) else {
            continue;
        };
        if let Some((_, left, right)) = rows.iter_mut().find(|(y, _, _)| *y == position.y) {
            *left = (*left).min(position.x);
            *right = (*right).max(position.x);
        } else {
            rows.push((position.y, position.x, position.x));
        }
    }
    let line_height = layout.line_height();
    rows.into_iter()
        .filter(|(_, left, right)| right > left)
        .map(|(y, left, right)| Bounds::from_corners(point(left, y), point(right, y + line_height)))
        .collect()
}

impl Element for SelectableText {
    type RequestLayoutState = ();
    type PrepaintState = gpui::Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.text.request_layout(None, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::Hitbox {
        self.text
            .prepaint(None, inspector_id, bounds, state, window, cx);
        window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(global_id) = global_id else {
            self.text
                .paint(None, inspector_id, bounds, state, &mut (), window, cx);
            return;
        };
        let layout = self.text.layout().clone();
        let rendered_text = layout.text();
        let highlight = rgba(self.highlight.as_rgba_hex());
        let anchor = window.with_element_state::<SelectableTextState, _>(global_id, |prev, _| {
            let prev = prev.unwrap_or_default();
            (prev.anchor.clone(), prev)
        });

        // Highlight under the glyphs: paint quads first, text second.
        if let Some((start, end)) = self.selection.range(&self.owner, &rendered_text) {
            for rect in selection_rects(&layout, start, end) {
                window.paint_quad(gpui::fill(rect, highlight));
            }
        }
        self.text
            .paint(None, inspector_id, bounds, state, &mut (), window, cx);
        window.set_cursor_style(CursorStyle::IBeam, hitbox);

        let clamp = |index: Result<usize, usize>| match index {
            Ok(ix) | Err(ix) => ix,
        };

        {
            let anchor = anchor.clone();
            let layout = layout.clone();
            let hitbox = hitbox.clone();
            let owner = self.owner.clone();
            let node = self.node;
            let selection = self.selection.clone();
            let host_focus = self.host_focus.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, _| {
                if phase.bubble() && event.button == MouseButton::Left && hitbox.is_hovered(window)
                {
                    let index = clamp(layout.index_for_position(event.position));
                    anchor.set(Some(index));
                    selection.begin(owner.clone(), node, layout.text(), hitbox.bounds);
                    if let Some(focus) = &host_focus {
                        focus.focus(window);
                    }
                    window.refresh();
                }
            });
        }
        {
            let anchor = anchor.clone();
            let layout = layout.clone();
            let owner = self.owner.clone();
            let selection = self.selection.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if phase.bubble()
                    && event.pressed_button == Some(MouseButton::Left)
                    && let Some(from) = anchor.get()
                {
                    let to = clamp(layout.index_for_position(event.position));
                    let range = (from.min(to), from.max(to));
                    selection.update(&owner, &layout.text(), layout.bounds(), range);
                    if range.1 > range.0 {
                        cx.stop_propagation();
                    }
                    window.refresh();
                }
            });
        }
        {
            let owner = self.owner.clone();
            let selection = self.selection.clone();
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase.bubble() && event.button == MouseButton::Left && anchor.take().is_some() {
                    if selection.range(&owner, &layout.text()).is_some() {
                        cx.stop_propagation();
                    }
                    window.refresh();
                }
            });
        }
    }
}

impl IntoElement for SelectableText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct GeometryTrackedElement {
    child: Option<AnyElement>,
    node: NodeId,
    registry: crate::GeometryRegistry,
    translate_x: f64,
    translate_y: f64,
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
        if let Ok(layout) = crate::GeometryBounds::new(
            f64::from(bounds.origin.x),
            f64::from(bounds.origin.y),
            f64::from(bounds.size.width),
            f64::from(bounds.size.height),
        ) {
            self.registry.update(
                self.node,
                crate::ElementGeometry {
                    layout,
                    visual: crate::GeometryBounds {
                        x: layout.x + self.translate_x,
                        y: layout.y + self.translate_y,
                        ..layout
                    },
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
        child.paint(window, cx);
    }
}

impl IntoElement for TranslatedElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn semantic_color(colors: &impl ColorResolver, token: &str, fallback: u32) -> Rgba8 {
    colors
        .resolve(&ColorValue::Token(token.to_owned()))
        .unwrap_or_else(|| Rgba8::from_rgb_hex(fallback))
}

#[derive(Clone, Copy, Default)]
struct PseudoPaint {
    background: Option<Rgba8>,
    border: Option<Rgba8>,
    text: Option<Rgba8>,
    opacity: Option<f32>,
}

fn pseudo_paint(properties: Option<&StyleProperties>, colors: &impl ColorResolver) -> PseudoPaint {
    let Some(properties) = properties else {
        return PseudoPaint::default();
    };
    PseudoPaint {
        background: properties
            .background
            .as_ref()
            .and_then(|color| colors.resolve(color)),
        border: properties
            .border_color
            .as_ref()
            .and_then(|color| colors.resolve(color)),
        text: properties
            .text_color
            .as_ref()
            .and_then(|color| colors.resolve(color)),
        opacity: properties
            .opacity
            .map(|opacity| f64_to_f32(opacity.clamp(0.0, 1.0))),
    }
}

fn apply_pseudo_paint(
    mut style: gpui::StyleRefinement,
    paint: PseudoPaint,
) -> gpui::StyleRefinement {
    if let Some(color) = paint.background {
        style = style.bg(rgba(color.as_rgba_hex()));
    }
    if let Some(color) = paint.border {
        style.style().border_color = Some(rgba(color.as_rgba_hex()).into());
    }
    if let Some(color) = paint.text {
        style = style.text_color(rgba(color.as_rgba_hex()));
    }
    if let Some(opacity) = paint.opacity {
        style = style.opacity(opacity);
    }
    style
}

fn apply_pseudo_backgrounds(
    mut element: Stateful<Div>,
    style: &Style,
    colors: &impl ColorResolver,
) -> Stateful<Div> {
    let hover = pseudo_paint(style.hover.as_ref(), colors);
    let active = pseudo_paint(style.active.as_ref(), colors);
    let mut focus = pseudo_paint(style.focus.as_ref(), colors);
    if focus.border.is_none() {
        focus.border = Some(semantic_color(colors, "focus_ring", 0x003b_82f6));
    }
    element = element.hover(move |style| apply_pseudo_paint(style, hover));
    element = element.active(move |style| apply_pseudo_paint(style, active));
    element = element.focus(move |style| apply_pseudo_paint(style, focus));
    element
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
    resolve_typography_role(&mut style, colors);
    resolve_style_lengths(&mut style, colors);
    let element = apply_layout(element, &style, direction);
    let element = apply_spacing(element, &style, direction);
    apply_paint_and_text(element, &style, colors, direction)
}

fn resolve_typography_role(style: &mut StyleProperties, resolver: &impl ColorResolver) {
    let Some(role) = style.typography.as_deref() else {
        return;
    };
    let Some(typography) = resolver.resolve_typography(role) else {
        return;
    };
    if style.font_family.is_none() {
        style.font_family = typography.family;
    }
    if style.font_fallbacks.is_none() && !typography.fallbacks.is_empty() {
        style.font_fallbacks = Some(typography.fallbacks);
    }
    if style.font_size.is_none() {
        style.font_size = Some(typography.size);
    }
    if style.line_height.is_none() {
        style.line_height = Some(typography.line_height);
    }
    if style.font_weight.is_none() {
        style.font_weight = Some(typography.weight);
    }
}

fn resolve_style_lengths(style: &mut StyleProperties, resolver: &impl ColorResolver) {
    let resolve_definite = |value: &mut Option<Length>| {
        *value = (*value).and_then(|value| resolver.resolve_length(value));
    };
    let resolve_layout = |value: &mut Option<LayoutLength>| {
        *value = (*value).and_then(|value| match value {
            LayoutLength::Definite(value) => {
                resolver.resolve_length(value).map(LayoutLength::Definite)
            }
            LayoutLength::Signed(_) | LayoutLength::Auto => Some(value),
        });
    };
    for value in [
        &mut style.width,
        &mut style.height,
        &mut style.min_width,
        &mut style.max_width,
        &mut style.min_height,
        &mut style.max_height,
        &mut style.flex_basis,
    ] {
        resolve_layout(value);
    }
    for value in [
        &mut style.margin.top,
        &mut style.margin.right,
        &mut style.margin.bottom,
        &mut style.margin.left,
        &mut style.margin.start,
        &mut style.margin.end,
        &mut style.top,
        &mut style.right,
        &mut style.bottom,
        &mut style.left,
    ] {
        resolve_layout(value);
    }
    for value in [
        &mut style.gap,
        &mut style.padding.top,
        &mut style.padding.right,
        &mut style.padding.bottom,
        &mut style.padding.left,
        &mut style.padding.start,
        &mut style.padding.end,
        &mut style.border_widths.top,
        &mut style.border_widths.right,
        &mut style.border_widths.bottom,
        &mut style.border_widths.left,
        &mut style.border_widths.start,
        &mut style.border_widths.end,
        &mut style.radii.top_left,
        &mut style.radii.top_right,
        &mut style.radii.bottom_right,
        &mut style.radii.bottom_left,
        &mut style.font_size,
        &mut style.line_height,
    ] {
        resolve_definite(value);
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

fn apply_layout(element: Div, style: &StyleProperties, text_direction: TextDirection) -> Div {
    let element = apply_display_and_position(element, style);
    let element = apply_flex_alignment(element, style, text_direction);
    apply_layout_dimensions(element, style)
}

fn apply_display_and_position(mut element: Div, style: &StyleProperties) -> Div {
    if let Some(display) = style.display {
        element = match display {
            DisplayMode::Block => element.block(),
            DisplayMode::Flex => element.flex(),
            DisplayMode::Grid => element.grid(),
            DisplayMode::None => element.hidden(),
        };
    }
    if let Some(position) = style.position {
        element = match position {
            PositionMode::Relative => element.relative(),
            PositionMode::Absolute => element.absolute(),
        };
    }
    if let Some(value) = style.top {
        element = inset_top(element, value);
    }
    if let Some(value) = style.right {
        element = inset_right(element, value);
    }
    if let Some(value) = style.bottom {
        element = inset_bottom(element, value);
    }
    if let Some(value) = style.left {
        element = inset_left(element, value);
    }
    if matches!(style.overflow_x, Some(OverflowMode::Hidden))
        || matches!(style.overflow_y, Some(OverflowMode::Hidden))
    {
        element = element.overflow_hidden();
    }
    element
}

fn apply_flex_alignment(
    mut element: Div,
    style: &StyleProperties,
    text_direction: TextDirection,
) -> Div {
    if let Some(direction) = style.direction {
        element = element.flex();
        element = match (direction, text_direction) {
            (FlexDirection::Row, TextDirection::LeftToRight) => element.flex_row(),
            (FlexDirection::Row, TextDirection::RightToLeft) => element.flex_row_reverse(),
            (FlexDirection::Column, _) => element.flex_col(),
        };
    }
    if let Some(wrap) = style.flex_wrap {
        element = match wrap {
            FlexWrapMode::NoWrap => element.flex_nowrap(),
            FlexWrapMode::Wrap => element.flex_wrap(),
            FlexWrapMode::WrapReverse => element.flex_wrap_reverse(),
        };
    }
    if let Some(align) = style.align_self {
        let align = if text_direction == TextDirection::RightToLeft {
            match align {
                Align::Start => Align::End,
                Align::End => Align::Start,
                other => other,
            }
        } else {
            align
        };
        element.style().align_self = Some(match align {
            Align::Start => GpuiAlignSelf::Start,
            Align::Center => GpuiAlignSelf::Center,
            Align::End => GpuiAlignSelf::End,
            Align::Stretch => GpuiAlignSelf::Stretch,
        });
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
    element
}

fn apply_layout_dimensions(mut element: Div, style: &StyleProperties) -> Div {
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
    if let Some(shrink) = style.flex_shrink {
        element = if shrink {
            element.flex_shrink()
        } else {
            element.flex_shrink_0()
        };
    }
    if let Some(value) = style.flex_basis {
        element = flex_basis(element, value);
    }
    if let Some(columns) = style.grid_columns {
        element = element.grid_cols(columns);
    }
    if let Some(rows) = style.grid_rows {
        element = element.grid_rows(rows);
    }
    if let Some(span) = style.column_span {
        element = element.col_span(span);
    }
    if let Some(span) = style.row_span {
        element = element.row_span(span);
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

fn logical_horizontal_edges<T: Copy>(
    mut left: Option<T>,
    mut right: Option<T>,
    start: Option<T>,
    end: Option<T>,
    direction: TextDirection,
) -> (Option<T>, Option<T>) {
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
    element: Div,
    style: &StyleProperties,
    colors: &impl ColorResolver,
    direction: TextDirection,
) -> Div {
    let element = apply_paint(element, style, colors, direction);
    let element = apply_typography(element, style, direction);
    apply_shadows(element, style, colors)
}

fn apply_paint(
    mut element: Div,
    style: &StyleProperties,
    colors: &impl ColorResolver,
    direction: TextDirection,
) -> Div {
    if let Some(gradient) = &style.gradient
        && let (Some(from), Some(to)) =
            (colors.resolve(&gradient.from), colors.resolve(&gradient.to))
    {
        element = element.bg(linear_gradient(
            f64_to_f32(gradient.angle_degrees),
            linear_color_stop(rgba(from.as_rgba_hex()), 0.0),
            linear_color_stop(rgba(to.as_rgba_hex()), 1.0),
        ));
    } else if let Some(color) = style
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
    if let Some(border_style) = style.border_style {
        match border_style {
            crate::BorderLineStyle::Solid => {
                element.style().border_style = Some(gpui::BorderStyle::Solid);
            }
            crate::BorderLineStyle::Dashed => {
                element = element.border_dashed();
            }
        }
    }
    if let Some(value) = style.border_widths.top {
        element = border_top(element, value);
    }
    let (border_left_value, border_right_value) = logical_horizontal_edges(
        style.border_widths.left,
        style.border_widths.right,
        style.border_widths.start,
        style.border_widths.end,
        direction,
    );
    if let Some(value) = border_right_value {
        element = border_right(element, value);
    }
    if let Some(value) = style.border_widths.bottom {
        element = border_bottom(element, value);
    }
    if let Some(value) = border_left_value {
        element = border_left(element, value);
    }
    if let Some(value) = style.radii.top_left {
        element = radius_top_left(element, value);
    }
    if let Some(value) = style.radii.top_right {
        element = radius_top_right(element, value);
    }
    if let Some(value) = style.radii.bottom_right {
        element = radius_bottom_right(element, value);
    }
    if let Some(value) = style.radii.bottom_left {
        element = radius_bottom_left(element, value);
    }
    if let Some(value) = style.font_size {
        element = font_size(element, value);
    }
    if let Some(opacity) = style.opacity {
        element = element.opacity(f64_to_f32(opacity));
    }
    if let Some(visible) = style.visible {
        element = if visible {
            element.visible()
        } else {
            element.invisible()
        };
    }
    if let Some(cursor) = style.cursor {
        element = element.cursor(gpui_cursor(cursor));
    }
    element
}

fn apply_typography(mut element: Div, style: &StyleProperties, direction: TextDirection) -> Div {
    if let Some(family) = &style.font_family {
        element = element.font_family(family.clone());
    }
    if let Some(fallbacks) = &style.font_fallbacks {
        element
            .text_style()
            .get_or_insert_with(Default::default)
            .font_fallbacks = Some(FontFallbacks::from_fonts(fallbacks.clone()));
    }
    if let Some(features) = &style.font_features {
        element
            .text_style()
            .get_or_insert_with(Default::default)
            .font_features = Some(FontFeatures(Arc::new(
            features
                .iter()
                .map(|(tag, value)| (tag.clone(), *value))
                .collect(),
        )));
    }
    if let Some(weight) = style.font_weight {
        element = element.font_weight(FontWeight(f32::from(weight)));
    }
    if let Some(slant) = style.font_slant {
        element = match slant {
            FontSlant::Normal => element.not_italic(),
            FontSlant::Italic => element.italic(),
        };
    }
    if let Some(value) = style.line_height {
        element = line_height(element, value);
    }
    if let Some(align) = style.text_align {
        let align = match (align, direction) {
            (TextAlignMode::Start, TextDirection::LeftToRight)
            | (TextAlignMode::End, TextDirection::RightToLeft) => TextAlign::Left,
            (TextAlignMode::Start, TextDirection::RightToLeft)
            | (TextAlignMode::End, TextDirection::LeftToRight) => TextAlign::Right,
            (TextAlignMode::Center, _) => TextAlign::Center,
        };
        element = element.text_align(align);
    }
    if let Some(white_space) = style.white_space {
        element = match white_space {
            WhiteSpaceMode::Normal => element.whitespace_normal(),
            WhiteSpaceMode::NoWrap => element.whitespace_nowrap(),
        };
    }
    if style.text_ellipsis == Some(true) {
        element = element.text_ellipsis();
    }
    if let Some(lines) = style.line_clamp {
        element = element.line_clamp(lines);
    }
    element
}

fn apply_shadows(mut element: Div, style: &StyleProperties, colors: &impl ColorResolver) -> Div {
    if let Some(shadows) = &style.shadows {
        element = element.shadow(
            shadows
                .iter()
                .filter_map(|shadow| {
                    colors.resolve(&shadow.color).map(|color| BoxShadow {
                        color: rgba(color.as_rgba_hex()).into(),
                        offset: point(px(f64_to_f32(shadow.x)), px(f64_to_f32(shadow.y))),
                        blur_radius: px(f64_to_f32(shadow.blur)),
                        spread_radius: px(f64_to_f32(shadow.spread)),
                    })
                })
                .collect(),
        );
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

definite_length_fn!(gap, gap);
definite_length_fn!(padding_top, pt);
definite_length_fn!(padding_right, pr);
definite_length_fn!(padding_bottom, pb);
definite_length_fn!(padding_left, pl);

macro_rules! layout_length_fn {
    ($name:ident, $method:ident) => {
        fn $name(element: Div, value: LayoutLength) -> Div {
            match value {
                LayoutLength::Definite(Length::Pixels(value)) => element.$method(px(to_f32(value))),
                LayoutLength::Signed(SignedLength::Pixels(value)) => {
                    element.$method(px(f64_to_f32(value)))
                }
                LayoutLength::Definite(Length::Rems(value)) => element.$method(rems(to_f32(value))),
                LayoutLength::Signed(SignedLength::Rems(value)) => {
                    element.$method(rems(f64_to_f32(value)))
                }
                LayoutLength::Definite(Length::Relative(value)) => {
                    element.$method(relative(to_f32(value)))
                }
                LayoutLength::Signed(SignedLength::Relative(value)) => {
                    element.$method(relative(f64_to_f32(value)))
                }
                LayoutLength::Auto => element.$method(auto()),
                LayoutLength::Definite(Length::ThemeSpacing(_) | Length::ThemeRadius(_)) => element,
            }
        }
    };
}

layout_length_fn!(width, w);
layout_length_fn!(height, h);
layout_length_fn!(min_width, min_w);
layout_length_fn!(max_width, max_w);
layout_length_fn!(min_height, min_h);
layout_length_fn!(max_height, max_h);
layout_length_fn!(margin_top, mt);
layout_length_fn!(margin_right, mr);
layout_length_fn!(margin_bottom, mb);
layout_length_fn!(margin_left, ml);
layout_length_fn!(inset_top, top);
layout_length_fn!(inset_right, right);
layout_length_fn!(inset_bottom, bottom);
layout_length_fn!(inset_left, left);
layout_length_fn!(flex_basis, flex_basis);

macro_rules! absolute_length_fn {
    ($name:ident, $method:ident) => {
        fn $name(element: Div, value: Length) -> Div {
            match value {
                Length::Pixels(value) => element.$method(px(to_f32(value))),
                Length::Rems(value) => element.$method(rems(to_f32(value))),
                Length::Relative(_) | Length::ThemeSpacing(_) | Length::ThemeRadius(_) => element,
            }
        }
    };
}

absolute_length_fn!(border_top, border_t);
absolute_length_fn!(border_right, border_r);
absolute_length_fn!(border_bottom, border_b);
absolute_length_fn!(border_left, border_l);
absolute_length_fn!(radius_top_left, rounded_tl);
absolute_length_fn!(radius_top_right, rounded_tr);
absolute_length_fn!(radius_bottom_right, rounded_br);
absolute_length_fn!(radius_bottom_left, rounded_bl);

fn font_size(element: Div, value: Length) -> Div {
    match value {
        Length::Pixels(value) => element.text_size(px(to_f32(value))),
        Length::Rems(value) => element.text_size(rems(to_f32(value))),
        Length::Relative(_) | Length::ThemeSpacing(_) | Length::ThemeRadius(_) => element,
    }
}

fn line_height(element: Div, value: Length) -> Div {
    match value {
        Length::Pixels(value) => element.line_height(px(to_f32(value))),
        Length::Rems(value) => element.line_height(rems(to_f32(value))),
        Length::Relative(_) | Length::ThemeSpacing(_) | Length::ThemeRadius(_) => element,
    }
}

const fn gpui_cursor(cursor: CursorKind) -> CursorStyle {
    match cursor {
        CursorKind::Default => CursorStyle::Arrow,
        CursorKind::Pointer => CursorStyle::PointingHand,
        CursorKind::Text => CursorStyle::IBeam,
        CursorKind::Move => CursorStyle::ClosedHand,
        CursorKind::Crosshair => CursorStyle::Crosshair,
        CursorKind::NotAllowed => CursorStyle::OperationNotAllowed,
        CursorKind::ResizeHorizontal => CursorStyle::ResizeLeftRight,
        CursorKind::ResizeVertical => CursorStyle::ResizeUpDown,
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
        if let Err(error) = self.primitives.retain_tree(&self.tree) {
            return div()
                .child(format!("Custom primitive lifecycle error: {error}"))
                .into_any_element();
        }
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

    struct TypographyResolver;

    impl ColorResolver for TypographyResolver {
        fn resolve(&self, color: &ColorValue) -> Option<Rgba8> {
            LiteralColorResolver.resolve(color)
        }

        fn resolve_typography(&self, role: &str) -> Option<crate::ResolvedTypography> {
            (role == "body").then(|| crate::ResolvedTypography {
                family: Some("JetBrains Mono".to_owned()),
                fallbacks: vec!["PingFang SC".to_owned()],
                size: Length::Pixels(12.0),
                line_height: Length::Pixels(16.0),
                weight: 400,
            })
        }
    }

    #[test]
    fn declarative_nodes_and_typed_styles_convert_without_a_gpui_context() {
        let root = UiNode::column(vec![
            UiNode::text("one"),
            UiNode::text("two"),
            UiNode::svg("<svg viewBox='0 0 1 1'><path fill='currentColor' d='M0 0L1 1'/></svg>")
                .unwrap()
                .with_style(
                    &Style::new().text_color(ColorValue::Literal(Rgba8::from_rgb_hex(0x00ff_ffff))),
                ),
        ])
        .with_style(
            &Style::new()
                .gap(Length::pixels(8.0).unwrap())
                .background(ColorValue::Literal(Rgba8::from_rgb_hex(0x0022_2222))),
        );
        let _element = GpuiNodeRenderer::render(&root);
    }

    #[test]
    fn symbolic_typography_resolves_and_explicit_fields_win() {
        let mut style = Style::new()
            .typography("body")
            .unwrap()
            .font_weight(650)
            .unwrap()
            .base;
        resolve_typography_role(&mut style, &TypographyResolver);
        assert_eq!(style.font_family.as_deref(), Some("JetBrains Mono"));
        assert_eq!(
            style.font_fallbacks.as_deref(),
            Some(["PingFang SC".to_owned()].as_slice())
        );
        assert_eq!(style.font_size, Some(Length::Pixels(12.0)));
        assert_eq!(style.line_height, Some(Length::Pixels(16.0)));
        assert_eq!(style.font_weight, Some(650));
    }

    #[test]
    fn pseudo_paint_preserves_native_border_text_and_opacity_states() {
        let hover = Style::new()
            .background(ColorValue::Literal(Rgba8::from_rgb_hex(0x0011_2233)))
            .border_color(ColorValue::Literal(Rgba8::from_rgb_hex(0x0044_5566)))
            .text_color(ColorValue::Literal(Rgba8::from_rgb_hex(0x0077_8899)))
            .opacity(0.625)
            .unwrap();
        let paint = pseudo_paint(Some(&hover.base), &LiteralColorResolver);
        assert_eq!(paint.background, Some(Rgba8::from_rgb_hex(0x0011_2233)));
        assert_eq!(paint.border, Some(Rgba8::from_rgb_hex(0x0044_5566)));
        assert_eq!(paint.text, Some(Rgba8::from_rgb_hex(0x0077_8899)));
        assert_eq!(paint.opacity, Some(0.625));
    }

    #[test]
    fn aligned_text_nodes_become_flex_content_boxes() {
        let node = UiNode::text("Centered").with_style(
            &Style::new()
                .items_center()
                .justify_center()
                .height(Length::pixels(32.0).unwrap()),
        );
        let mut resolved = node.style().resolve(&InteractionState::default());

        normalize_text_content_layout(&node, &mut resolved);

        assert_eq!(resolved.display, Some(DisplayMode::Flex));
        assert_eq!(resolved.align, Some(Align::Center));
        assert_eq!(resolved.justify, Some(Justify::Center));
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
        assert_eq!(style.width, Some(Length::Pixels(180.0).into()));
        assert_eq!(style.height, Some(Length::Pixels(64.0).into()));
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
        assert_eq!(style.width, Some(Length::Pixels(144.0).into()));
    }

    #[test]
    fn property_specific_layout_values_reach_gpui_without_loss() {
        let mut automatic = width(div(), LayoutLength::Auto);
        assert_eq!(automatic.style().size.width, Some(auto()));

        let mut signed = margin_left(div(), LayoutLength::Signed(SignedLength::Pixels(-12.0)));
        assert_eq!(
            signed.style().margin.left,
            Some(gpui::Length::from(px(-12.0)))
        );
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

    #[test]
    fn logical_overlay_edges_resolve_against_text_direction() {
        let spec = |placement| crate::OverlayNodeSpec {
            id: crate::OverlayId::new("logical-sheet"),
            parent: None,
            kind: crate::OverlayKind::Sheet,
            placement,
            anchor: None,
            open: true,
            gap: 0.0,
            modal: true,
            dismiss: crate::OverlayDismissPolicy {
                escape: true,
                outside: true,
            },
            tooltip_delays: None,
            initial_focus: crate::OverlayInitialFocus::Panel,
            activate_on_trigger: false,
        };
        assert_eq!(
            scoped_overlay_spec(
                &spec(crate::OverlayPlacement::Start),
                "view",
                TextDirection::RightToLeft,
            )
            .placement,
            crate::OverlayPlacement::Right
        );
        assert_eq!(
            scoped_overlay_spec(
                &spec(crate::OverlayPlacement::End),
                "view",
                TextDirection::RightToLeft,
            )
            .placement,
            crate::OverlayPlacement::Left
        );
    }

    #[test]
    fn retained_tab_order_reads_explicit_group_indices() {
        let node = UiNode::text("tab")
            .with_attribute("tab_index", UiValue::Integer(7))
            .with_attribute("tab_stop", UiValue::Bool(false))
            .with_attribute("tab_group", UiValue::Bool(true));
        assert_eq!(node_tab_index(&node), 7);
        assert!(!node_tab_stop(&node));
        let _element = GpuiNodeRenderer::render(&node);
    }

    #[test]
    fn pointer_payload_uses_committed_local_geometry_and_canvas_hit_key() {
        let scene = crate::CanvasScene::new(vec![crate::CanvasCommand::Rect {
            key: "clip".to_owned(),
            x: 0.0,
            y: 0.0,
            width: 40.0,
            height: 30.0,
            fill: ColorValue::Token("accent".to_owned()),
        }])
        .unwrap();
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(UiNode::canvas(scene.clone()).with_key("canvas"))
            .unwrap();
        let node = tree.root_id().unwrap();
        let geometry = crate::GeometryRegistry::new();
        geometry.update(
            node,
            crate::ElementGeometry {
                layout: crate::GeometryBounds::new(100.0, 50.0, 200.0, 120.0).unwrap(),
                visual: crate::GeometryBounds::new(105.0, 54.0, 200.0, 120.0).unwrap(),
                clip: None,
            },
        );
        let outer_scroll = ScrollHandle::new();
        outer_scroll.set_offset(point(px(-10.0), px(-20.0)));
        let inner_scroll = ScrollHandle::new();
        inner_scroll.set_offset(point(px(-3.0), px(-4.0)));
        let context = PointerPayloadContext::retained(
            node,
            geometry,
            Some(scene),
            vec![outer_scroll, inner_scroll],
        );
        let payload = context.enrich(pointer_payload(
            point(px(112.0), px(68.0)),
            Some(MouseButton::Left),
            vec![MouseButton::Left],
            Modifiers::default(),
            1,
            false,
        ));
        let UiValue::Map(payload) = payload else {
            unreachable!()
        };
        assert_eq!(payload["canvas_key"], UiValue::String("clip".to_owned()));
        assert_eq!(
            payload["target"],
            crate::GeometryBounds::new(105.0, 54.0, 200.0, 120.0)
                .unwrap()
                .into_value()
        );
        assert!(value_point(&payload["local"]).is_some_and(|(x, y)| {
            (x - 7.0).abs() < f64::EPSILON && (y - 14.0).abs() < f64::EPSILON
        }));
        assert!(value_point(&payload["content"]).is_some_and(|(x, y)| {
            (x - 20.0).abs() < f64::EPSILON && (y - 38.0).abs() < f64::EPSILON
        }));

        let scroll_style = Style::new().overflow_scroll();
        let nested_root = UiNode::box_node(vec![
            UiNode::box_node(vec![UiNode::text("target").with_key("target")])
                .with_key("inner")
                .with_style(&scroll_style),
        ])
        .with_key("outer")
        .with_style(&scroll_style);
        let mut nested = RetainedUiTree::new();
        nested.reconcile(nested_root).unwrap();
        let ids = nested
            .nodes()
            .filter_map(|node| node.key().map(|key| (key.to_owned(), node.id())))
            .collect::<BTreeMap<_, _>>();
        let handles = BTreeMap::from([
            (ids["outer"], ScrollHandle::new()),
            (ids["inner"], ScrollHandle::new()),
        ]);
        assert_eq!(
            scroll_handles_for_node(Some(&nested), Some(ids["target"]), &handles).len(),
            2
        );
    }
}
