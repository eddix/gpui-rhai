use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use gpui::{
    AlignSelf as GpuiAlignSelf, AnyElement, App, Background, Bounds, BoxShadow, ClickEvent,
    ContentMask, Context, CursorStyle, Div, Element, ElementId, FocusHandle, FontFallbacks,
    FontFeatures, FontStyle, FontWeight, GlobalElementId, HighlightStyle, Img, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, Modifiers, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, ScrollHandle,
    ScrollWheelEvent, SharedString, Stateful, StatefulInteractiveElement, Styled, StyledText,
    TextAlign, Window, auto, div, img, linear_color_stop, linear_gradient, point, px, relative,
    rems, rgba,
};

use crate::overlay_element::{ScriptLayerElement, ScriptOverlayElement, WindowOverlayCoordinator};
use crate::slot_runtime::NodeSlotRuntime;
use crate::virtual_list_element::VirtualListEntityElement;
use crate::{
    Align, AssetRegistry, ColorValue, CursorKind, DisplayMode, EventPropagation, EventResponse,
    FlexDirection, FlexWrapMode, FontSlant, HitTestBehavior, ImageSourceSpec, InteractionState,
    Justify, LayoutLength, Length, MotionKey, MotionProperty, NodeId, OverflowMode,
    OverlayNodeSpec, PositionMode, PrimitiveRegistry, PseudoState, RetainedUiTree, Rgba8,
    ScriptCallback, SignedLength, Style, StyleProperties, TextAlignMode, TextDirection,
    UiEventHandler, UiNode, UiNodeKind, UiValue, WhiteSpaceMode,
};

/// A script callback for one UI event: the callback, the event name, the
/// payload and the target's bounds.
type DispatchFn = dyn Fn(
    ScriptCallback,
    &str,
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
type SignalWriteFn = dyn Fn(
    Vec<(crate::NativeSignal, crate::SignalValue)>,
    &mut App,
) -> Result<bool, crate::SignalError>;
type SignalReadFn =
    dyn Fn(&crate::NativeSignal, &App) -> Result<crate::SignalValue, crate::SignalError>;
type ElementBoundsFn = dyn Fn(&crate::ElementRef, &App) -> Option<crate::GeometryBounds>;
type CanvasLocalPointFn = dyn Fn(&crate::ElementRef, (f64, f64), &App) -> Option<(f64, f64)>;
type ElementHitboxFn = dyn Fn(&crate::ElementRef, &App) -> Option<gpui::HitboxId>;

#[derive(Clone)]
pub struct NodeEventDispatcher {
    script: Rc<DispatchFn>,
    native: Rc<NativeDispatchFn>,
    signal_write: Rc<SignalWriteFn>,
    signal_read: Rc<SignalReadFn>,
    element_bounds: Rc<ElementBoundsFn>,
    element_hitbox: Rc<ElementHitboxFn>,
    canvas_bounds: Rc<ElementBoundsFn>,
    canvas_local_point: Rc<CanvasLocalPointFn>,
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
        Self::with_event_names(move |callback, _, payload, target, window, app| {
            dispatch(callback, payload, target, window, app)
        })
    }

    /// A dispatcher whose script callback also receives the UI event name.
    #[must_use]
    pub(crate) fn with_event_names<R>(
        dispatch: impl Fn(
            ScriptCallback,
            &str,
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
            script: Rc::new(move |callback, event, payload, target, window, app| {
                dispatch(callback, event, payload, target, window, app).into()
            }),
            native: Rc::new(|_, _, _, _, _, _| EventResponse::new().stop()),
            signal_write: Rc::new(|updates, _| {
                updates.first().map_or(Ok(false), |(signal, _)| {
                    Err(crate::SignalError::Stale(signal.id().clone()))
                })
            }),
            signal_read: Rc::new(|signal, _| Err(crate::SignalError::Stale(signal.id().clone()))),
            element_bounds: Rc::new(|_, _| None),
            element_hitbox: Rc::new(|_, _| None),
            canvas_bounds: Rc::new(|_, _| None),
            canvas_local_point: Rc::new(|_, _, _| None),
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

    pub(crate) fn with_signal_write(
        mut self,
        write: impl Fn(
            Vec<(crate::NativeSignal, crate::SignalValue)>,
            &mut App,
        ) -> Result<bool, crate::SignalError>
        + 'static,
    ) -> Self {
        self.signal_write = Rc::new(write);
        self
    }

    pub(crate) fn with_signal_read(
        mut self,
        read: impl Fn(&crate::NativeSignal, &App) -> Result<crate::SignalValue, crate::SignalError>
        + 'static,
    ) -> Self {
        self.signal_read = Rc::new(read);
        self
    }

    pub(crate) fn with_element_bounds(
        mut self,
        read: impl Fn(&crate::ElementRef, &App) -> Option<crate::GeometryBounds> + 'static,
    ) -> Self {
        self.element_bounds = Rc::new(read);
        self
    }

    pub(crate) fn with_element_hitbox(
        mut self,
        read: impl Fn(&crate::ElementRef, &App) -> Option<gpui::HitboxId> + 'static,
    ) -> Self {
        self.element_hitbox = Rc::new(read);
        self
    }

    pub(crate) fn with_canvas_local_point(
        mut self,
        read: impl Fn(&crate::ElementRef, (f64, f64), &App) -> Option<(f64, f64)> + 'static,
    ) -> Self {
        self.canvas_local_point = Rc::new(read);
        self
    }

    pub(crate) fn with_canvas_bounds(
        mut self,
        read: impl Fn(&crate::ElementRef, &App) -> Option<crate::GeometryBounds> + 'static,
    ) -> Self {
        self.canvas_bounds = Rc::new(read);
        self
    }

    pub(crate) fn canvas_bounds(
        &self,
        reference: &crate::ElementRef,
        app: &App,
    ) -> Option<crate::GeometryBounds> {
        (self.canvas_bounds)(reference, app)
    }

    pub(crate) fn dispatch(
        &self,
        callback: ScriptCallback,
        event: &str,
        payload: UiValue,
        target: Option<crate::GeometryBounds>,
        window: &mut Window,
        cx: &mut App,
    ) -> EventResponse {
        (self.script)(callback, event, payload, target, window, cx)
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

    pub(crate) fn write_signal(
        &self,
        signal: crate::NativeSignal,
        value: crate::SignalValue,
        app: &mut App,
    ) -> Result<bool, crate::SignalError> {
        self.write_signals(vec![(signal, value)], app)
    }

    pub(crate) fn write_signals(
        &self,
        updates: Vec<(crate::NativeSignal, crate::SignalValue)>,
        app: &mut App,
    ) -> Result<bool, crate::SignalError> {
        if updates.is_empty() {
            return Ok(false);
        }
        (self.signal_write)(updates, app)
    }

    pub(crate) fn read_signal(
        &self,
        signal: &crate::NativeSignal,
        app: &App,
    ) -> Result<crate::SignalValue, crate::SignalError> {
        (self.signal_read)(signal, app)
    }

    pub(crate) fn element_bounds(
        &self,
        reference: &crate::ElementRef,
        app: &App,
    ) -> Option<crate::GeometryBounds> {
        (self.element_bounds)(reference, app)
    }

    pub(crate) fn element_hitbox(
        &self,
        reference: &crate::ElementRef,
        app: &App,
    ) -> Option<gpui::HitboxId> {
        (self.element_hitbox)(reference, app)
    }

    pub(crate) fn canvas_local_point(
        &self,
        reference: &crate::ElementRef,
        point: (f64, f64),
        app: &App,
    ) -> Option<(f64, f64)> {
        (self.canvas_local_point)(reference, point, app)
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
            |dispatcher| dispatcher.dispatch(callback.clone(), event, payload, target, window, app),
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

/// A handler declared with its own value receives that value; the others
/// receive the event's payload.
fn dispatch_ui_handler_phases(
    bindings: &[crate::UiEventBinding],
    event: &str,
    phases: &[crate::EventPhase],
    payload: &UiValue,
    route: EventRoute<'_>,
    window: &mut Window,
    app: &mut App,
) -> EventResponse {
    dispatch_ui_handler_phases_with(
        bindings,
        event,
        phases,
        |binding| binding.value().unwrap_or(payload).clone(),
        route,
        window,
        app,
    )
}

fn dispatch_ui_handler_phases_with(
    bindings: &[crate::UiEventBinding],
    event: &str,
    phases: &[crate::EventPhase],
    payload: impl Fn(&crate::UiEventBinding) -> UiValue,
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
                payload(binding),
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

/// The handler a key press reaches, in every phase: a modifier-qualified name
/// (`shift+f6`) wins, and a plain handler still fires whatever modifiers are
/// held.
fn key_handler_for<'a, V>(
    handlers: &'a BTreeMap<String, V>,
    event: &gpui::KeyDownEvent,
    direction: TextDirection,
) -> Option<&'a V> {
    let key = logical_keyboard_key(event.keystroke.key.as_str(), direction);
    let modifiers = &event.keystroke.modifiers;
    let qualified = crate::node::canonical_key_name(
        [
            modifiers.control,
            modifiers.alt,
            modifiers.shift,
            modifiers.platform,
        ],
        key,
    );
    handlers
        .get(qualified.as_str())
        .or_else(|| handlers.get(key))
}

fn key_handler_bindings(
    node: &UiNode,
    disabled: bool,
) -> BTreeMap<String, (Vec<crate::UiEventBinding>, UiValue)> {
    if disabled {
        return BTreeMap::new();
    }
    node.handlers()
        .iter()
        .filter_map(|(event, bindings)| {
            event.strip_prefix("key:").map(|key| {
                (
                    key.to_owned(),
                    (
                        bindings.clone(),
                        node.node_payload(event).cloned().unwrap_or(UiValue::Null),
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
                    .and_then(|scene| {
                        let geometry = self
                            .node
                            .and_then(|node| self.geometry.canvas_drawable(node))
                            .or(geometry)?;
                        let (window_x, window_y) = window?;
                        let transform = self
                            .node
                            .map(|node| self.geometry.canvas_transform(node))
                            .unwrap_or_default();
                        scene.hit_test_presented(
                            window_x - geometry.visual.x,
                            window_y - geometry.visual.y,
                            geometry.layout.width,
                            geometry.layout.height,
                            transform,
                        )
                    })
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
    let scrolls_x = matches!(node.style().base.overflow_x, Some(OverflowMode::Scroll));
    let scrolls_y = matches!(node.style().base.overflow_y, Some(OverflowMode::Scroll));
    if scrolls_x {
        element = element.overflow_x_scroll();
    }
    if scrolls_y {
        element = element.overflow_y_scroll();
    }
    // GPUI translates an unsupported wheel axis onto the one scrollable axis
    // by default. Ordinary one-axis UI containers promise a stricter contract;
    // two-axis canvases retain GPUI's native gesture handling, and a node can ask
    // for the translation (`.translate_wheel()`, a horizontal tab strip).
    let translate = node.attributes().get("translate_wheel") == Some(&UiValue::Bool(true));
    if scrolls_x ^ scrolls_y && !translate {
        element = element.restrict_scroll_to_axis();
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
            // A handler with a value, its own or the node's, receives
            // `#{ hovered, value }`; a plain handler receives the bool.
            let payload = |binding: &crate::UiEventBinding| {
                binding.value().or(value.as_ref()).map_or_else(
                    || UiValue::Bool(*hovered),
                    |value| {
                        UiValue::Map(BTreeMap::from([
                            ("hovered".to_owned(), UiValue::Bool(*hovered)),
                            ("value".to_owned(), value.clone()),
                        ]))
                    },
                )
            };
            let response = dispatch_ui_handler_phases_with(
                bindings,
                "hover_change",
                &[crate::EventPhase::Target],
                payload,
                EventRoute::new(target.snapshot(), dispatcher.as_ref()),
                window,
                cx,
            );
            apply_event_response(response, window, cx);
        }
    })
}

fn apply_motion_trigger_handlers(
    mut element: Stateful<Div>,
    node: Option<NodeId>,
    bindings: &[crate::MotionProgressBinding],
    geometry: &crate::GeometryRegistry,
) -> Stateful<Div> {
    let Some(node) = node else {
        return element;
    };
    if bindings
        .iter()
        .any(|binding| binding.driver == crate::MotionProgressDriver::Hover)
    {
        let geometry = geometry.clone();
        element = element.on_hover(move |hovered, window, _| {
            geometry.set_motion_trigger(node, crate::MotionProgressDriver::Hover, *hovered);
            window.request_animation_frame();
        });
    }
    if bindings
        .iter()
        .any(|binding| binding.driver == crate::MotionProgressDriver::Press)
    {
        let down_geometry = geometry.clone();
        element = element.on_any_mouse_down(move |_, window, _| {
            down_geometry.set_motion_trigger(node, crate::MotionProgressDriver::Press, true);
            window.request_animation_frame();
        });
        for button in MouseButton::all() {
            let up_geometry = geometry.clone();
            element = element.on_mouse_up(button, move |_, window, _| {
                up_geometry.set_motion_trigger(node, crate::MotionProgressDriver::Press, false);
                window.request_animation_frame();
            });
        }
    }
    element
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
        let last_pointer = Rc::new(RefCell::new(None::<(crate::LogicalPoint, f64)>));
        element = element.on_mouse_move(move |event, window, app| {
            let payload = pointer_motion_payload(
                mouse_move_payload(event),
                logical_point(event.position),
                &last_pointer,
            );
            let payload = payload_context.enrich(payload);
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

fn pointer_motion_payload(
    payload: UiValue,
    position: crate::LogicalPoint,
    last: &Rc<RefCell<Option<(crate::LogicalPoint, f64)>>>,
) -> UiValue {
    let UiValue::Map(mut payload) = payload else {
        return payload;
    };
    let timestamp = event_timestamp_ms();
    let previous = last.borrow_mut().replace((position, timestamp));
    let (movement, velocity) = previous.map_or_else(
        || {
            (
                crate::LogicalPoint::default(),
                crate::LogicalPoint::default(),
            )
        },
        |(previous, previous_timestamp)| {
            let movement = crate::LogicalPoint {
                x: position.x - previous.x,
                y: position.y - previous.y,
            };
            let elapsed = (timestamp - previous_timestamp).max(0.001) / 1_000.0;
            (
                movement,
                crate::LogicalPoint {
                    x: movement.x / elapsed,
                    y: movement.y / elapsed,
                },
            )
        },
    );
    payload.insert(
        "movement".to_owned(),
        logical_point_value(movement.x, movement.y),
    );
    payload.insert(
        "velocity".to_owned(),
        logical_point_value(velocity.x, velocity.y),
    );
    payload.insert("timestamp_ms".to_owned(), UiValue::Float(timestamp));
    UiValue::Map(payload)
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
        velocity: crate::LogicalPoint::default(),
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn pointer_capture_router_element(
    child: AnyElement,
    view_id: &str,
    tree: &RetainedUiTree,
    dispatcher: &NodeEventDispatcher,
    captures: &crate::PointerCaptureRegistry,
    geometry: &crate::GeometryRegistry,
    scroll_handles: &BTreeMap<NodeId, ScrollHandle>,
    interactions: crate::interaction::WindowInteractionCoordinator,
) -> AnyElement {
    PointerCaptureRouterElement {
        child: Some(child),
        interactions,
        routes: Some(PointerCaptureRoutes {
            view_id: view_id.to_owned(),
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
    view_id: String,
    move_handlers: BTreeMap<NodeId, Vec<crate::UiEventBinding>>,
    up_handlers: BTreeMap<NodeId, Vec<crate::UiEventBinding>>,
    dispatcher: NodeEventDispatcher,
    captures: crate::PointerCaptureRegistry,
    payload_contexts: BTreeMap<NodeId, PointerPayloadContext>,
}

impl PointerCaptureRoutes {
    fn register(self, interactions: &crate::interaction::WindowInteractionCoordinator) {
        let Self {
            view_id,
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
        interactions.set_pointer_routes(
            view_id,
            move |event: &MouseMoveEvent, window, app| {
                let Some(node) = move_captures.captured(0) else {
                    return false;
                };
                let Some(bindings) = move_handlers.get(&node) else {
                    return false;
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
                true
            },
            move |event: &MouseUpEvent, window, app| {
                let Some(node) = up_captures.captured(0) else {
                    return false;
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
                true
            },
        );
    }
}

struct PointerCaptureRouterElement {
    child: Option<AnyElement>,
    routes: Option<PointerCaptureRoutes>,
    interactions: crate::interaction::WindowInteractionCoordinator,
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
            .register(&self.interactions);
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

/// Resolves symbolic style values against a theme.
///
/// Implementors provide token leaves; expressions, environment variants and
/// scaled lengths are handled by the provided methods.
pub trait ColorResolver {
    /// Look up one color token by name or `namespace.name` path.
    fn resolve_token(&self, token: &str) -> Option<Rgba8>;

    /// Resolve a color value, evaluating expressions over token leaves.
    fn resolve(&self, color: &ColorValue) -> Option<Rgba8> {
        color.resolve_with(&mut |token| self.resolve_token(token))
    }

    /// Every color token this resolver knows, for snapshot capture.
    fn color_snapshot(&self) -> BTreeMap<String, Rgba8> {
        BTreeMap::new()
    }

    /// The environment this resolver is bound to.
    fn environment(&self) -> crate::Environment {
        crate::Environment::EMPTY
    }

    /// Resolve a length token against an explicit environment.
    fn resolve_length_in(
        &self,
        length: Length,
        _environment: &crate::Environment,
    ) -> Option<Length> {
        (!length.is_theme_token()).then_some(length)
    }

    /// Resolve a length token against the bound environment.
    fn resolve_length(&self, length: Length) -> Option<Length> {
        self.resolve_length_in(length, &self.environment())
    }

    /// Resolve a typography role against an explicit environment.
    fn resolve_typography_in(
        &self,
        _role: &str,
        _environment: &crate::Environment,
    ) -> Option<crate::ResolvedTypography> {
        None
    }

    /// Resolve a typography role against the bound environment.
    fn resolve_typography(&self, role: &str) -> Option<crate::ResolvedTypography> {
        self.resolve_typography_in(role, &self.environment())
    }

    /// The complete token set, when the resolver is backed by one. Snapshots
    /// keep it so nested environments can still resolve variants.
    fn token_set(&self) -> Option<std::sync::Arc<crate::ThemeTokens>> {
        None
    }

    fn resolve_motion(&self) -> crate::ThemeMotion {
        crate::ThemeMotion::default()
    }
}

/// A resolver bound to the environment inherited by one rendered subtree.
#[derive(Clone, Copy)]
pub(crate) struct EnvironmentScoped<'a, C: ?Sized> {
    inner: &'a C,
    environment: crate::Environment,
}

impl<'a, C: ColorResolver + ?Sized> EnvironmentScoped<'a, C> {
    pub(crate) const fn new(inner: &'a C, environment: crate::Environment) -> Self {
        Self { inner, environment }
    }
}

impl<C: ColorResolver + ?Sized> ColorResolver for EnvironmentScoped<'_, C> {
    fn resolve_token(&self, token: &str) -> Option<Rgba8> {
        self.inner.resolve_token(token)
    }

    fn color_snapshot(&self) -> BTreeMap<String, Rgba8> {
        self.inner.color_snapshot()
    }

    fn environment(&self) -> crate::Environment {
        self.environment
    }

    fn resolve_length_in(
        &self,
        length: Length,
        environment: &crate::Environment,
    ) -> Option<Length> {
        self.inner.resolve_length_in(length, environment)
    }

    fn resolve_typography_in(
        &self,
        role: &str,
        environment: &crate::Environment,
    ) -> Option<crate::ResolvedTypography> {
        self.inner.resolve_typography_in(role, environment)
    }

    fn token_set(&self) -> Option<std::sync::Arc<crate::ThemeTokens>> {
        self.inner.token_set()
    }

    fn resolve_motion(&self) -> crate::ThemeMotion {
        self.inner.resolve_motion()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LiteralColorResolver;

impl ColorResolver for LiteralColorResolver {
    fn resolve_token(&self, _token: &str) -> Option<Rgba8> {
        None
    }
}

/// An owned theme capture for deferred rendering boundaries (slot runtimes,
/// canvas, overlay backdrops). It keeps the full token set when available so
/// environment variants still resolve inside the boundary.
#[derive(Clone, Debug, Default)]
pub(crate) struct OwnedColorResolver {
    tokens: Option<std::sync::Arc<crate::ThemeTokens>>,
    colors: BTreeMap<String, Rgba8>,
    environment: crate::Environment,
    motion: crate::ThemeMotion,
}

impl OwnedColorResolver {
    pub(crate) fn capture(colors: &(impl ColorResolver + ?Sized)) -> Self {
        let tokens = colors.token_set();
        Self {
            colors: if tokens.is_some() {
                BTreeMap::new()
            } else {
                colors.color_snapshot()
            },
            tokens,
            environment: colors.environment(),
            motion: colors.resolve_motion(),
        }
    }
}

impl OwnedColorResolver {
    /// Whether both captures share the same token set.
    pub(crate) fn same_tokens(&self, other: &Self) -> bool {
        match (&self.tokens, &other.tokens) {
            (Some(left), Some(right)) => std::sync::Arc::ptr_eq(left, right),
            (None, None) => self.colors == other.colors,
            _ => false,
        }
    }
}

impl ColorResolver for OwnedColorResolver {
    fn resolve_token(&self, token: &str) -> Option<Rgba8> {
        match &self.tokens {
            Some(tokens) => tokens.color(token),
            None => self.colors.get(token).copied(),
        }
    }

    fn color_snapshot(&self) -> BTreeMap<String, Rgba8> {
        match &self.tokens {
            Some(tokens) => tokens.color_snapshot(),
            None => self.colors.clone(),
        }
    }

    fn environment(&self) -> crate::Environment {
        self.environment
    }

    fn resolve_length_in(
        &self,
        length: Length,
        environment: &crate::Environment,
    ) -> Option<Length> {
        match &self.tokens {
            Some(tokens) => tokens.resolve_length(length, environment),
            None => (!length.is_theme_token()).then_some(length),
        }
    }

    fn resolve_typography_in(
        &self,
        role: &str,
        environment: &crate::Environment,
    ) -> Option<crate::ResolvedTypography> {
        self.tokens
            .as_ref()
            .and_then(|tokens| tokens.resolve_typography(role, environment))
    }

    fn token_set(&self) -> Option<std::sync::Arc<crate::ThemeTokens>> {
        self.tokens.clone()
    }

    fn resolve_motion(&self) -> crate::ThemeMotion {
        self.motion.clone()
    }
}

/// Converts stable runtime nodes into short-lived GPUI elements.
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuiNodeRenderer;

#[derive(Clone, Copy, Default)]
struct NodeFocus {
    /// The nearest focus owner ancestor-or-self holds focus.
    owner: bool,
    /// This node or a descendant holds focus.
    within: bool,
}

#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
struct RenderEnvironment<'a, C> {
    now: Instant,
    clock: &'a crate::RuntimeClock,
    motion_preference: crate::MotionPreference,
    motion_quality: crate::MotionQuality,
    colors: &'a C,
    interaction: &'a InteractionState,
    primitives: &'a PrimitiveRegistry,
    dispatcher: Option<&'a NodeEventDispatcher>,
    assets: Option<&'a AssetRegistry>,
    overlays: &'a WindowOverlayCoordinator,
    interactions: &'a crate::interaction::WindowInteractionCoordinator,
    motions: &'a BTreeMap<MotionKey, f64>,
    signals: &'a crate::SignalRegistry,
    geometry: &'a crate::GeometryRegistry,
    pointer_capture: &'a crate::PointerCaptureRegistry,
    focus_handles: &'a BTreeMap<NodeId, FocusHandle>,
    /// For slot content rendered without its retained tree: the focus owner of
    /// each node (its nearest focus-styled ancestor), consulted only by native
    /// controls that share their owner's focus. Other nodes track only their
    /// own handle.
    focus_owners: &'a BTreeMap<NodeId, FocusHandle>,
    scroll_handles: &'a BTreeMap<NodeId, ScrollHandle>,
    scroll_anchors: &'a BTreeMap<NodeId, gpui::ScrollAnchor>,
    virtual_requests: &'a crate::VirtualRequestRegistry,
    text_selection: &'a TextSelectionRegistry,
    host_focus: Option<&'a FocusHandle>,
    direction: TextDirection,
    locale: &'a str,
    number: Option<&'a crate::NumberMetadata>,
    ambient_text_color: Option<Rgba8>,
    /// Environment values inherited from ancestors (`.env(...)`).
    environment: crate::Environment,
    /// Whether an ancestor is disabled.
    inherited_disabled: bool,
    /// The retained node whose focus handle has keyboard focus this frame,
    /// followed by its ancestors.
    focus_path: &'a [NodeId],
    /// Focus state seen by `group_focus` and `focus_within` styles.
    focus: NodeFocus,
    view_id: &'a str,
    retained: Option<&'a RetainedUiTree>,
    retained_links: Option<&'a BTreeMap<NodeId, Vec<crate::RetainedChildLink>>>,
    semantics: Option<&'a crate::CommittedSemanticFrame>,
    a11y_active: bool,
    /// The Host lets window drag areas in this view move the window.
    window_drag: bool,
    /// The parent stacks its children vertically and stretches them: a child
    /// without a width of its own is as wide as the parent, and in an RTL view a
    /// child with a definite width sits on the start (right) edge.
    stretch_parent: bool,
    /// This node stretches its own children, by its resolved style (interaction
    /// states included), so the hint follows what is actually laid out.
    stretch_children: bool,
    /// The overlays whose content this node is in, innermost first, so a
    /// `parent` key names the nearest enclosing overlay with that key.
    overlay_scope: Option<&'a OverlayScope<'a>>,
    /// The nearest ancestor declaring a `hover` style: its GPUI group drives
    /// `group_hover` paint below it.
    hover_group: Option<NodeId>,
}

/// An overlay whose content is being rendered: its script key and its
/// window-wide id, linked to the overlay around it.
struct OverlayScope<'a> {
    local: &'a crate::OverlayId,
    id: crate::OverlayId,
    outer: Option<&'a OverlayScope<'a>>,
}

impl OverlayScope<'_> {
    fn find(&self, local: &crate::OverlayId) -> Option<&crate::OverlayId> {
        let mut cursor = Some(self);
        while let Some(scope) = cursor {
            if scope.local == local {
                return Some(&scope.id);
            }
            cursor = scope.outer;
        }
        None
    }
}

impl<'a, C: ColorResolver> RenderEnvironment<'a, C> {
    /// The environment of a node's own content: its resolved text color, and
    /// no stretch hint (a Box sets that again for its own children).
    fn below(&self, style: &StyleProperties) -> Self {
        Self {
            stretch_parent: false,
            stretch_children: style.direction != Some(FlexDirection::Row)
                && matches!(style.align, None | Some(Align::Stretch)),
            ..self.with_resolved_text_color(style)
        }
    }

    /// The theme resolver bound to this subtree's inherited environment.
    fn resolver(&self) -> EnvironmentScoped<'a, C> {
        EnvironmentScoped::new(self.colors, self.environment)
    }

    /// Apply a node's own environment overrides and disabled state; the
    /// result is the scope of the node itself and of its descendants.
    fn with_node_scope(&self, node: &UiNode) -> Self {
        let mut environment = self.environment;
        if let Some(UiValue::Map(values)) = node.attributes().get("environment") {
            for (name, value) in values {
                if let UiValue::String(value) = value {
                    environment = environment
                        .with(crate::Symbol::intern(name), crate::Symbol::intern(value))
                        .unwrap_or(environment);
                }
            }
        }
        Self {
            environment,
            inherited_disabled: self.inherited_disabled || is_disabled(node),
            ..*self
        }
    }

    /// A focusable node that declares a focus style owns the focus state seen
    /// by `group_focus` styles in its subtree; `tab_stop(false)` children of a
    /// roving group leave ownership with the group. `focus_within` is per node.
    fn with_focus_scope(mut self, node: &UiNode, retained_id: Option<NodeId>) -> Self {
        self.focus.within = retained_id.is_some_and(|id| self.focus_path.contains(&id));
        if let Some(id) = retained_id
            && node.style().focus.is_some()
            && node_tab_stop(node)
            && self.focus_handles.contains_key(&id)
        {
            self.focus.owner = self.focus_path.first() == Some(&id);
        }
        self
    }

    fn with_resolved_text_color(&self, style: &StyleProperties) -> Self {
        Self {
            ambient_text_color: resolve_ambient_text_color(
                style,
                &self.resolver(),
                self.ambient_text_color,
            ),
            ..*self
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
pub(crate) struct WindowRenderResources<'a> {
    pub now: Instant,
    pub clock: &'a crate::RuntimeClock,
    pub motion_preference: crate::MotionPreference,
    pub motion_quality: crate::MotionQuality,
    pub assets: &'a AssetRegistry,
    pub dispatcher: &'a NodeEventDispatcher,
    pub overlays: &'a WindowOverlayCoordinator,
    pub interactions: &'a crate::interaction::WindowInteractionCoordinator,
    pub motions: &'a BTreeMap<MotionKey, f64>,
    pub signals: &'a crate::SignalRegistry,
    pub geometry: &'a crate::GeometryRegistry,
    pub pointer_capture: &'a crate::PointerCaptureRegistry,
    pub focus_handles: &'a BTreeMap<NodeId, FocusHandle>,
    pub focus_owners: &'a BTreeMap<NodeId, FocusHandle>,
    pub scroll_handles: &'a BTreeMap<NodeId, ScrollHandle>,
    pub scroll_anchors: &'a BTreeMap<NodeId, gpui::ScrollAnchor>,
    pub virtual_requests: &'a crate::VirtualRequestRegistry,
    pub text_selection: &'a TextSelectionRegistry,
    pub host_focus: Option<&'a FocusHandle>,
    pub direction: TextDirection,
    pub locale: &'a str,
    pub number: Option<&'a crate::NumberMetadata>,
    pub ambient_text_color: Option<Rgba8>,
    pub environment: crate::Environment,
    pub inherited_disabled: bool,
    pub focus_path: &'a [NodeId],
    pub owner_focused: bool,
    pub root_path: &'a str,
    pub view_id: &'a str,
    pub semantics: &'a crate::CommittedSemanticFrame,
    pub a11y_active: bool,
    /// Whether `window_drag_area()` nodes move the window
    /// (`ScriptViewConfig::window_drag_areas`).
    pub window_drag: bool,
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
        let interactions = crate::interaction::WindowInteractionCoordinator::default();
        let motions = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        let pointer_capture = crate::PointerCaptureRegistry::new();
        let focus_handles = BTreeMap::new();
        let scroll_handles = BTreeMap::new();
        let scroll_anchors = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let text_selection = TextSelectionRegistry::default();
        let environment = RenderEnvironment {
            now: Instant::now(),
            clock: &crate::RuntimeClock::default(),
            motion_preference: crate::MotionPreference::Normal,
            motion_quality: crate::MotionQuality::High,
            colors,
            interaction,
            primitives,
            dispatcher: None,
            assets: None,
            overlays: &overlays,
            interactions: &interactions,
            motions: &motions,
            signals: &signals,
            geometry: &geometry,
            pointer_capture: &pointer_capture,
            focus_handles: &focus_handles,
            focus_owners: &focus_handles,
            scroll_handles: &scroll_handles,
            scroll_anchors: &scroll_anchors,
            virtual_requests: &virtual_requests,
            text_selection: &text_selection,
            host_focus: None,
            direction: TextDirection::LeftToRight,
            locale: "en",
            number: None,
            ambient_text_color: None,
            environment: crate::Environment::EMPTY,
            inherited_disabled: false,
            focus_path: &[],
            focus: NodeFocus::default(),
            view_id: "standalone",
            retained: None,
            retained_links: None,
            semantics: None,
            a11y_active: false,
            window_drag: false,
            stretch_parent: false,
            stretch_children: false,
            overlay_scope: None,
            hover_group: None,
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
        match crate::CommittedSemanticFrame::from_retained(tree) {
            Ok(semantics) => Self::render_retained_with_committed_semantics(
                tree,
                &semantics,
                false,
                colors,
                interaction,
                primitives,
            ),
            Err(error) => div()
                .child(format!("Invalid accessibility semantics: {error}"))
                .into_any_element(),
        }
    }

    fn render_retained_with_committed_semantics(
        tree: &RetainedUiTree,
        semantics: &crate::CommittedSemanticFrame,
        a11y_active: bool,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
    ) -> AnyElement {
        let overlays = WindowOverlayCoordinator::default();
        let interactions = crate::interaction::WindowInteractionCoordinator::default();
        let motions = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        let pointer_capture = crate::PointerCaptureRegistry::new();
        let focus_handles = BTreeMap::new();
        let scroll_handles = BTreeMap::new();
        let scroll_anchors = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let text_selection = TextSelectionRegistry::default();
        let environment = RenderEnvironment {
            now: Instant::now(),
            clock: &crate::RuntimeClock::default(),
            motion_preference: crate::MotionPreference::Normal,
            motion_quality: crate::MotionQuality::High,
            colors,
            interaction,
            primitives,
            dispatcher: None,
            assets: None,
            overlays: &overlays,
            interactions: &interactions,
            motions: &motions,
            signals: &signals,
            geometry: &geometry,
            pointer_capture: &pointer_capture,
            focus_handles: &focus_handles,
            focus_owners: &focus_handles,
            scroll_handles: &scroll_handles,
            scroll_anchors: &scroll_anchors,
            virtual_requests: &virtual_requests,
            text_selection: &text_selection,
            host_focus: None,
            direction: TextDirection::LeftToRight,
            locale: "en",
            number: None,
            ambient_text_color: None,
            environment: crate::Environment::EMPTY,
            inherited_disabled: false,
            focus_path: &[],
            focus: NodeFocus::default(),
            view_id: "standalone",
            retained: Some(tree),
            retained_links: None,
            semantics: Some(semantics),
            a11y_active,
            window_drag: false,
            stretch_parent: false,
            stretch_children: false,
            overlay_scope: None,
            hover_group: None,
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
        let semantics = match crate::CommittedSemanticFrame::from_retained(tree) {
            Ok(semantics) => semantics,
            Err(error) => {
                return div()
                    .child(format!("Invalid accessibility semantics: {error}"))
                    .into_any_element();
            }
        };
        let overlays = WindowOverlayCoordinator::default();
        let interactions = crate::interaction::WindowInteractionCoordinator::default();
        let motions = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        let pointer_capture = crate::PointerCaptureRegistry::new();
        let focus_handles = BTreeMap::new();
        let scroll_handles = BTreeMap::new();
        let scroll_anchors = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let text_selection = TextSelectionRegistry::default();
        let environment = RenderEnvironment {
            now: Instant::now(),
            clock: &crate::RuntimeClock::default(),
            motion_preference: crate::MotionPreference::Normal,
            motion_quality: crate::MotionQuality::High,
            colors,
            interaction,
            primitives,
            dispatcher: Some(dispatcher),
            assets: None,
            overlays: &overlays,
            interactions: &interactions,
            motions: &motions,
            signals: &signals,
            geometry: &geometry,
            pointer_capture: &pointer_capture,
            focus_handles: &focus_handles,
            focus_owners: &focus_handles,
            scroll_handles: &scroll_handles,
            scroll_anchors: &scroll_anchors,
            virtual_requests: &virtual_requests,
            text_selection: &text_selection,
            host_focus: None,
            direction: TextDirection::LeftToRight,
            locale: "en",
            number: None,
            ambient_text_color: None,
            environment: crate::Environment::EMPTY,
            inherited_disabled: false,
            focus_path: &[],
            focus: NodeFocus::default(),
            view_id: "standalone",
            retained: Some(tree),
            retained_links: None,
            semantics: Some(&semantics),
            a11y_active: false,
            window_drag: false,
            stretch_parent: false,
            stretch_children: false,
            overlay_scope: None,
            hover_group: None,
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
        let interactions = crate::interaction::WindowInteractionCoordinator::default();
        let motions = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        let pointer_capture = crate::PointerCaptureRegistry::new();
        let focus_handles = BTreeMap::new();
        let scroll_handles = BTreeMap::new();
        let scroll_anchors = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let text_selection = TextSelectionRegistry::default();
        let environment = RenderEnvironment {
            now: Instant::now(),
            clock: &crate::RuntimeClock::default(),
            motion_preference: crate::MotionPreference::Normal,
            motion_quality: crate::MotionQuality::High,
            colors,
            interaction,
            primitives,
            dispatcher: Some(dispatcher),
            assets: None,
            overlays: &overlays,
            interactions: &interactions,
            motions: &motions,
            signals: &signals,
            geometry: &geometry,
            pointer_capture: &pointer_capture,
            focus_handles: &focus_handles,
            focus_owners: &focus_handles,
            scroll_handles: &scroll_handles,
            scroll_anchors: &scroll_anchors,
            virtual_requests: &virtual_requests,
            text_selection: &text_selection,
            host_focus: None,
            direction: TextDirection::LeftToRight,
            locale: "en",
            number: None,
            ambient_text_color: None,
            environment: crate::Environment::EMPTY,
            inherited_disabled: false,
            focus_path: &[],
            focus: NodeFocus::default(),
            view_id: "standalone",
            retained: None,
            retained_links: None,
            semantics: None,
            a11y_active: false,
            window_drag: false,
            stretch_parent: false,
            stretch_children: false,
            overlay_scope: None,
            hover_group: None,
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
        let interactions = crate::interaction::WindowInteractionCoordinator::default();
        let motions = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        let pointer_capture = crate::PointerCaptureRegistry::new();
        let focus_handles = BTreeMap::new();
        let scroll_handles = BTreeMap::new();
        let scroll_anchors = BTreeMap::new();
        let virtual_requests = crate::VirtualRequestRegistry::new();
        let text_selection = TextSelectionRegistry::default();
        let resources = WindowRenderResources {
            now: Instant::now(),
            clock: &crate::RuntimeClock::default(),
            motion_preference: crate::MotionPreference::Normal,
            motion_quality: crate::MotionQuality::High,
            assets,
            dispatcher,
            overlays: &overlays,
            interactions: &interactions,
            motions: &motions,
            signals: &signals,
            geometry: &geometry,
            pointer_capture: &pointer_capture,
            focus_handles: &focus_handles,
            focus_owners: &focus_handles,
            scroll_handles: &scroll_handles,
            scroll_anchors: &scroll_anchors,
            virtual_requests: &virtual_requests,
            text_selection: &text_selection,
            host_focus: None,
            direction: TextDirection::LeftToRight,
            locale: "en",
            number: None,
            ambient_text_color: None,
            environment: crate::Environment::EMPTY,
            inherited_disabled: false,
            focus_path: &[],
            owner_focused: false,
            root_path: "root",
            view_id: "standalone",
            semantics: &crate::CommittedSemanticFrame::default(),
            a11y_active: false,
            window_drag: false,
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
            now: resources.now,
            clock: resources.clock,
            motion_preference: resources.motion_preference,
            motion_quality: resources.motion_quality,
            colors,
            interaction,
            primitives,
            dispatcher: Some(resources.dispatcher),
            assets: Some(resources.assets),
            overlays: resources.overlays,
            interactions: resources.interactions,
            motions: resources.motions,
            signals: resources.signals,
            geometry: resources.geometry,
            pointer_capture: resources.pointer_capture,
            focus_handles: resources.focus_handles,
            focus_owners: resources.focus_owners,
            scroll_handles: resources.scroll_handles,
            scroll_anchors: resources.scroll_anchors,
            virtual_requests: resources.virtual_requests,
            text_selection: resources.text_selection,
            host_focus: resources.host_focus,
            direction: resources.direction,
            locale: resources.locale,
            number: resources.number,
            ambient_text_color: resources.ambient_text_color,
            environment: resources.environment,
            inherited_disabled: resources.inherited_disabled,
            focus_path: resources.focus_path,
            focus: NodeFocus {
                owner: resources.owner_focused,
                within: false,
            },
            view_id: resources.view_id,
            retained: Some(tree),
            retained_links: None,
            semantics: Some(resources.semantics),
            a11y_active: resources.a11y_active,
            window_drag: resources.window_drag,
            stretch_parent: false,
            stretch_children: false,
            overlay_scope: None,
            hover_group: None,
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
            now: resources.now,
            clock: resources.clock,
            motion_preference: resources.motion_preference,
            motion_quality: resources.motion_quality,
            colors,
            interaction,
            primitives,
            dispatcher: Some(resources.dispatcher),
            assets: Some(resources.assets),
            overlays: resources.overlays,
            interactions: resources.interactions,
            motions: resources.motions,
            signals: resources.signals,
            geometry: resources.geometry,
            pointer_capture: resources.pointer_capture,
            focus_handles: resources.focus_handles,
            focus_owners: resources.focus_owners,
            scroll_handles: resources.scroll_handles,
            scroll_anchors: resources.scroll_anchors,
            virtual_requests: resources.virtual_requests,
            text_selection: resources.text_selection,
            host_focus: resources.host_focus,
            direction: resources.direction,
            locale: resources.locale,
            number: resources.number,
            ambient_text_color: resources.ambient_text_color,
            environment: resources.environment,
            inherited_disabled: resources.inherited_disabled,
            focus_path: resources.focus_path,
            focus: NodeFocus {
                owner: resources.owner_focused,
                within: false,
            },
            view_id: resources.view_id,
            retained: None,
            retained_links: None,
            semantics: Some(resources.semantics),
            a11y_active: resources.a11y_active,
            window_drag: resources.window_drag,
            stretch_parent: false,
            stretch_children: false,
            overlay_scope: None,
            hover_group: None,
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
            now: resources.now,
            clock: resources.clock,
            motion_preference: resources.motion_preference,
            motion_quality: resources.motion_quality,
            colors,
            interaction,
            primitives,
            dispatcher: Some(resources.dispatcher),
            assets: Some(resources.assets),
            overlays: resources.overlays,
            interactions: resources.interactions,
            motions: resources.motions,
            signals: resources.signals,
            geometry: resources.geometry,
            pointer_capture: resources.pointer_capture,
            focus_handles: resources.focus_handles,
            focus_owners: resources.focus_owners,
            scroll_handles: resources.scroll_handles,
            scroll_anchors: resources.scroll_anchors,
            virtual_requests: resources.virtual_requests,
            text_selection: resources.text_selection,
            host_focus: resources.host_focus,
            direction: resources.direction,
            locale: resources.locale,
            number: resources.number,
            ambient_text_color: resources.ambient_text_color,
            environment: resources.environment,
            inherited_disabled: resources.inherited_disabled,
            focus_path: resources.focus_path,
            focus: NodeFocus {
                owner: resources.owner_focused,
                within: false,
            },
            view_id: resources.view_id,
            retained: None,
            retained_links: Some(retained.links),
            semantics: Some(resources.semantics),
            a11y_active: resources.a11y_active,
            window_drag: resources.window_drag,
            stretch_parent: false,
            stretch_children: false,
            overlay_scope: None,
            hover_group: None,
        };
        Self::render_internal(node, &environment, None, path, retained.root)
    }

    #[allow(clippy::too_many_lines)]
    fn render_internal<C: ColorResolver>(
        node: &UiNode,
        environment: &RenderEnvironment<'_, C>,
        boundary_fallback: Option<&UiNode>,
        path: &str,
        retained_id: Option<NodeId>,
    ) -> AnyElement {
        let node_environment = environment
            .with_node_scope(node)
            .with_focus_scope(node, retained_id);
        let environment = &node_environment;
        if let Some(table) = render_table_layout(node, environment, path, retained_id) {
            return table;
        }
        let local_interaction = scoped_interaction(environment);
        let motion_path = retained_id.map_or_else(
            || path.to_owned(),
            |node| crate::motion::retained_node_path(path, node),
        );
        let mut animation = node_motion(environment.motions, &motion_path);
        if let Some(retained_id) = retained_id {
            for binding in node.progress_motions() {
                if let Some(value) = environment
                    .geometry
                    .motion_progress(retained_id, binding.property())
                {
                    animation.set(binding.property(), value);
                }
            }
        }
        let signals = node_signals(environment.signals, node);
        if matches!(node.kind(), UiNodeKind::Canvas { .. }) {
            animation = apply_canvas_signal_transform(animation, &signals);
            if let Some(retained_id) = retained_id {
                environment
                    .geometry
                    .update_canvas_transform(retained_id, animation.canvas_transform());
            }
        }
        // Every layer that can set a width, margin, alignment or position is
        // merged before the stretch rules read the style.
        let mut resolved_style = node.style().resolve(&local_interaction);
        apply_motion_dimensions(&mut resolved_style, animation);
        apply_signal_style(&mut resolved_style, &signals);
        apply_signal_state_style(&mut resolved_style, environment.signals, node);
        let mut resolved_style = definite_stretch(
            rtl_start_edge(
                resolved_style,
                environment.stretch_parent && environment.direction == TextDirection::RightToLeft,
            ),
            environment.stretch_parent,
        );
        normalize_text_content_layout(node, &mut resolved_style);
        let mut local_environment = environment.below(&resolved_style);
        if node.style().hover.is_some() && retained_id.is_some() {
            local_environment.hover_group = retained_id;
        }
        let mut element = apply_style(
            div(),
            &resolved_style,
            &environment.resolver(),
            environment.direction,
        );
        if matches!(
            node.kind(),
            UiNodeKind::VirtualCollection { spec } if spec.height.is_none()
        ) {
            element = element.flex_1().min_h(px(0.0));
        }
        // An overlay's handle is its panel's, tracked by the overlay element.
        let focus_handle = retained_id
            .filter(|_| !matches!(node.kind(), UiNodeKind::Overlay { .. }))
            .and_then(|node| environment.focus_handles.get(&node).cloned());
        element =
            apply_node_focus_tracking(element, node, focus_handle.as_ref(), environment.primitives);
        if let Some(opacity) = signals.opacity.or(animation.opacity) {
            element = element.opacity(f64_to_f32(opacity.clamp(0.0, 1.0)));
        }
        if animation.clip_height.is_some() || resolved_style.clip == Some(true) {
            element = element.overflow_hidden();
        }
        let populated = Self::populate_with_interactions(
            element,
            node,
            &local_environment,
            boundary_fallback,
            path,
            retained_id,
            animation,
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
        let layout_motion = layout_motion_spec(node, environment.now);
        let progress_motions = node.progress_motions().to_vec();
        match retained_id {
            Some(retained) => GeometryTrackedElement {
                child: Some(element),
                node: retained,
                hit_area: node.element_ref().is_some(),
                registry: environment.geometry.clone(),
                translate_x: translate_x.unwrap_or(0.0),
                translate_y: translate_y.unwrap_or(0.0),
                layout_motion,
                progress_motions,
                scroll_handles: scroll_handles_for_node(
                    environment.retained,
                    retained_id,
                    environment.scroll_handles,
                ),
                focus_handle,
                now: environment.now,
                motion_preference: environment.motion_preference,
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
        motion: NodeMotionValues,
    ) -> AnyElement {
        // The interaction wrapper is built in its own frame, which is gone
        // before the children render: the recursion carries only this frame.
        match Self::wrap_interactions(element, node, environment, path, retained_id) {
            Wrapped::Interactive(element) => Self::populate(
                element,
                node,
                environment,
                boundary_fallback,
                path,
                retained_id,
                motion,
            ),
            Wrapped::Plain(element) => Self::populate(
                element,
                node,
                environment,
                boundary_fallback,
                path,
                retained_id,
                motion,
            ),
        }
    }

    /// Attach the node's handlers, focus and semantics, or return the element
    /// unchanged when the node needs no interaction wrapper.
    #[inline(never)]
    #[allow(clippy::too_many_lines)]
    fn wrap_interactions<C: ColorResolver>(
        element: Div,
        node: &UiNode,
        environment: &RenderEnvironment<'_, C>,
        path: &str,
        retained_id: Option<NodeId>,
    ) -> Wrapped {
        let disabled = environment.inherited_disabled;
        let click = (!disabled && !node.event_handlers("click").is_empty()).then(|| {
            (
                node.event_handlers("click").to_vec(),
                node.node_payload("click").cloned().unwrap_or(UiValue::Null),
            )
        });
        let hover = (!disabled && !node.event_handlers("hover_change").is_empty()).then(|| {
            (
                node.event_handlers("hover_change").to_vec(),
                node.node_payload("hover_change").cloned(),
            )
        });
        let key_handlers = key_handler_bindings(node, disabled);
        let hit_test = resolved_hit_test(node, environment);
        let semantic = environment
            .a11y_active
            .then(|| {
                retained_id.and_then(|id| environment.semantics.and_then(|frame| frame.node(id)))
            })
            .flatten()
            .filter(|semantic| {
                crate::accessibility::native_role(&semantic.role)
                    .ok()
                    .flatten()
                    .is_some()
            })
            .cloned();
        let needs = u8::from(click.is_some())
            | u8::from(hover.is_some()) << 1
            | u8::from(!key_handlers.is_empty()) << 2
            | u8::from(hit_test.is_some()) << 3
            | u8::from(node.progress_motions().iter().any(|binding| {
                matches!(
                    binding.driver,
                    crate::MotionProgressDriver::Hover
                        | crate::MotionProgressDriver::Press
                        | crate::MotionProgressDriver::Focus
                )
            })) << 4
            | u8::from(semantic.is_some()) << 5;
        if !node_needs_interaction_wrapper(node, needs, disabled) {
            return Wrapped::Plain(element);
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
        let element = element
            .id(SharedString::from(stable_id))
            .debug_selector(move || debug_path.clone());
        // A disabled node keeps its resolved disabled paint; pointer states
        // must not repaint it.
        let element = if disabled {
            element
        } else {
            apply_pseudo_backgrounds(element, node.style(), &environment.resolver())
        };
        let element = apply_hover_group(element, node, retained_id, environment, disabled);
        let element = apply_native_semantics(element, semantic.as_ref());
        let element = apply_primitive_accessibility_actions(
            element,
            node,
            retained_id,
            semantic.as_ref(),
            environment,
        );
        let element = apply_hit_test(element, hit_test);
        let element = if environment.window_drag {
            apply_window_drag_area(element, node)
        } else {
            element
        };
        let ancestor_disabled = disabled && !is_disabled(node);
        let persistent_focus = retained_id
            .and_then(|id| environment.focus_handles.get(&id))
            .filter(|_| {
                !matches!(node.kind(), UiNodeKind::Overlay { .. })
                    && !matches!(node.kind(), UiNodeKind::Custom { primitive }
                    if environment.primitives.uses_primary_focus(&primitive.primitive))
            });
        // Key handlers on a container that holds focusable children route keys
        // bubbling from those children (roving groups, overlay roots); only a
        // click target or a key target without focusable content is a stop.
        let implicit_tab_stop =
            click.is_some() || !key_handlers.is_empty() && !children_take_focus(node, disabled);
        let element = apply_tab_behavior(
            element,
            node,
            implicit_tab_stop,
            ancestor_disabled,
            persistent_focus,
        );
        let element = apply_environment_scroll(element, node, retained_id, environment);
        let element = apply_motion_trigger_handlers(
            element,
            retained_id,
            node.progress_motions(),
            environment.geometry,
        );
        let element = if let Some((bindings, payload)) = click {
            element.on_click(move |event, window, cx| {
                if !matches!(event, ClickEvent::Mouse(_)) {
                    return;
                }
                let response = dispatch_ui_handlers(
                    &bindings,
                    "click",
                    &payload,
                    click_target.snapshot(),
                    window,
                    cx,
                    click_dispatcher.as_ref(),
                );
                apply_event_response(response, window, cx);
            })
        } else {
            element
        };
        let element = apply_hover_handler(element, hover, hover_dispatcher, hover_target);
        let element = if key_handlers.values().any(|(bindings, _)| {
            bindings
                .iter()
                .any(|binding| binding.phase() == crate::EventPhase::Capture)
        }) {
            let handlers = key_handlers.clone();
            let dispatcher = keyboard_dispatcher.clone();
            let target = keyboard_target.clone();
            element.capture_key_down(move |event, window, cx| {
                if let Some((bindings, payload)) = key_handler_for(&handlers, event, text_direction)
                {
                    let response = dispatch_ui_handler_phases(
                        bindings,
                        "key",
                        &[crate::EventPhase::Capture],
                        payload,
                        EventRoute::new(target.snapshot(), dispatcher.as_ref()),
                        window,
                        cx,
                    );
                    apply_event_response(response, window, cx);
                }
            })
        } else {
            element
        };
        let element = element.on_key_down(move |event, window, cx| {
            let explicit = key_handler_for(&key_handlers, event, text_direction);
            let semantic = explicit.or_else(|| {
                matches!(event.keystroke.key.as_str(), "enter" | "space")
                    .then_some(())
                    .and(keyboard_click.as_ref())
            });
            if let Some((bindings, payload)) = semantic {
                let phases = if explicit.is_some() {
                    &[crate::EventPhase::Target, crate::EventPhase::Bubble][..]
                } else {
                    // Preserve the existing click fallback, rather than also
                    // firing a click or changing mouse-click phase policy.
                    &[crate::EventPhase::Target][..]
                };
                let response = dispatch_ui_handler_phases(
                    bindings,
                    "key",
                    phases,
                    payload,
                    EventRoute::new(keyboard_target.snapshot(), keyboard_dispatcher.as_ref()),
                    window,
                    cx,
                );
                apply_event_response(response, window, cx);
            }
        });
        let element = if disabled {
            element
        } else {
            apply_environment_raw_pointer(element, node, retained_id, environment)
        };
        Wrapped::Interactive(element)
    }

    #[allow(clippy::too_many_lines)]
    fn populate<C: ColorResolver>(
        element: impl ParentElement + IntoElement,
        node: &UiNode,
        environment: &RenderEnvironment<'_, C>,
        boundary_fallback: Option<&UiNode>,
        path: &str,
        retained_id: Option<NodeId>,
        motion: NodeMotionValues,
    ) -> AnyElement {
        match node.kind() {
            UiNodeKind::Text { text } => {
                render_text_node(element, node, text.as_str(), environment, path, retained_id)
            }
            UiNodeKind::RichText { text, spans } => element
                .child(styled_text(
                    text.as_str(),
                    spans,
                    &environment.resolver(),
                    environment.motions,
                    &retained_id.map_or_else(
                        || path.to_owned(),
                        |node| crate::motion::retained_node_path(path, node),
                    ),
                ))
                .into_any_element(),
            UiNodeKind::Canvas { scene } => render_canvas(
                element,
                scene,
                &environment.resolver(),
                motion,
                retained_id,
                environment.geometry,
            ),
            UiNodeKind::Svg { source } => render_inline_svg(element, node, source, environment),
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                let child_environment = RenderEnvironment {
                    stretch_parent: environment.stretch_children,
                    ..*environment
                };
                let children = render_flattened_children(
                    children,
                    &child_environment,
                    boundary_fallback,
                    path,
                    retained_id,
                );
                let element = if let Some(crate::table_layout::TableLayout::Resolved { extent }) =
                    node.table_layout()
                {
                    let mut track = div()
                        .flex()
                        .flex_col()
                        .w(px(f64_to_f32(*extent)))
                        .min_w(px(f64_to_f32(*extent)))
                        .flex_shrink_0();
                    if node.style().base.flex_grow == Some(true) {
                        track = track.flex_1().min_h(px(0.0));
                    }
                    element.child(track.children(children))
                } else {
                    element.children(children)
                };
                decorate_scrollbars(element, node, environment, path, retained_id)
                    .into_any_element()
            }
            UiNodeKind::Custom { primitive } => element
                .child(
                    environment.primitives.element(
                        inherit_primitive_disabled(primitive, environment.inherited_disabled),
                        retained_id.zip(node.key().map(|key| key.as_str().to_owned())),
                        environment
                            .primitives
                            .uses_primary_focus(&primitive.primitive)
                            .then(|| {
                                primitive_focus_owner(
                                    environment.retained,
                                    retained_id,
                                    environment.focus_handles,
                                )
                                .or_else(|| {
                                    retained_id
                                        .and_then(|id| environment.focus_owners.get(&id).cloned())
                                })
                            })
                            .flatten(),
                        boundary_fallback.cloned(),
                        crate::primitive::PrimitiveWindowContext::new(
                            environment.dispatcher.cloned(),
                            environment.interactions.clone(),
                            scroll_handles_for_node(
                                environment.retained,
                                retained_id,
                                environment.scroll_handles,
                            ),
                            environment.view_id,
                        ),
                        crate::PrimitiveTheme::capture_with_environment(
                            &environment.resolver(),
                            environment.direction,
                            environment.locale,
                            environment.number,
                            environment.clock.clone(),
                            environment.motion_preference,
                            environment.motion_quality,
                        )
                        .with_inherited_disabled(environment.inherited_disabled),
                    ),
                )
                .into_any_element(),
            UiNodeKind::Image { source } => render_image(element, source, environment),
            UiNodeKind::DirectionalImage {
                left_to_right,
                right_to_left,
            } => {
                let source = match environment.direction {
                    TextDirection::LeftToRight => left_to_right,
                    TextDirection::RightToLeft => right_to_left,
                };
                render_image(element, source, environment)
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

fn apply_node_focus_tracking(
    element: Div,
    node: &UiNode,
    handle: Option<&FocusHandle>,
    primitives: &PrimitiveRegistry,
) -> Div {
    let Some(handle) = handle else {
        return element;
    };
    if matches!(node.kind(), UiNodeKind::Custom { primitive }
        if primitives.uses_primary_focus(&primitive.primitive))
    {
        // The wrapper observes the native control's identity for focus paint,
        // but only the native control belongs in the tab sequence. The
        // primitive refreshes the shared handle's real tab-stop policy below.
        element.track_focus(&handle.clone().tab_stop(false))
    } else {
        element.track_focus(handle)
    }
}

fn primitive_focus_owner(
    retained: Option<&RetainedUiTree>,
    retained_id: Option<NodeId>,
    focus_handles: &BTreeMap<NodeId, FocusHandle>,
) -> Option<FocusHandle> {
    let retained_id = retained_id?;
    if let Some(handle) = focus_handles.get(&retained_id) {
        return Some(handle.clone());
    }
    let retained = retained?;
    let mut cursor = Some(retained_id);
    while let Some(node_id) = cursor {
        let node = retained.node(node_id)?;
        if node.focus_styled()
            && let Some(handle) = focus_handles.get(&node_id)
        {
            return Some(handle.clone());
        }
        cursor = node.parent();
    }
    None
}

pub(crate) fn render_motion_ghost(
    ghost: &crate::motion::MotionGhost,
    colors: &impl ColorResolver,
    primitives: &PrimitiveRegistry,
    resources: &WindowRenderResources<'_>,
) -> AnyElement {
    let child = GpuiNodeRenderer::render_subtree_with_window_runtime(
        &ghost.node,
        colors,
        &InteractionState::default(),
        primitives,
        resources,
        &ghost.path,
    );
    div()
        .absolute()
        .left(px(f64_to_f32(ghost.bounds.x)))
        .top(px(f64_to_f32(ghost.bounds.y)))
        .w(px(f64_to_f32(ghost.bounds.width)))
        .h(px(f64_to_f32(ghost.bounds.height)))
        .child(child)
        .into_any_element()
}

fn resolve_ambient_text_color<C: ColorResolver>(
    style: &StyleProperties,
    colors: &C,
    inherited: Option<Rgba8>,
) -> Option<Rgba8> {
    style
        .text_color
        .as_ref()
        .and_then(|color| colors.resolve(color))
        .or(inherited)
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
                    .and_then(|color| environment.resolver().resolve(color))
            })
            .unwrap_or_else(|| semantic_color(&environment.resolver(), token, fallback))
    };
    element.child(crate::scrollbar::ThemedScrollbar::new(
        format!("{path}/scrollbars"),
        crate::interaction::InteractionOwner::new(environment.view_id, format!("scrollbar:{path}")),
        environment.interactions.clone(),
        handle.clone(),
        spec,
        environment.direction,
        (
            part_color("scrollbar_track", "surface_raised", 0x0027_272aff),
            part_color("scrollbar_thumb", "text_muted", 0x0071_717aff),
            part_color("scrollbar_thumb_hover", "accent", 0x003b_82f6ff),
        ),
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

/// A node's element with or without its interaction wrapper.
enum Wrapped {
    Interactive(Stateful<Div>),
    Plain(Div),
}

fn node_needs_interaction_wrapper(node: &UiNode, needs: u8, disabled: bool) -> bool {
    needs & 0b10_1000 != 0
        || is_window_drag_area(node)
        || !disabled
            && (needs & 0b0111 != 0
                || node_has_focus_declaration(node)
                || node.style().hover.is_some()
                || node.style().group_hover.is_some()
                || node.style().active.is_some()
                || node.style().focus.is_some()
                || node_has_raw_pointer_handlers(node)
                || node_scrollable(node))
}

fn is_window_drag_area(node: &UiNode) -> bool {
    node.attributes().get("window_drag_area") == Some(&UiValue::Bool(true))
}

/// A window drag area: a press that no focusable control inside it took (they
/// prevent default when they take focus) moves the window, and a double press
/// runs the platform title-bar action (zoom or minimize on macOS). Window moves
/// are platform drags on macOS and Linux.
fn apply_window_drag_area(element: Stateful<Div>, node: &UiNode) -> Stateful<Div> {
    if !is_window_drag_area(node) {
        return element;
    }
    element.on_mouse_down(MouseButton::Left, |event, window, _| {
        if window.default_prevented() {
            return;
        }
        if event.click_count == 2 {
            window.titlebar_double_click();
        } else {
            window.start_window_move();
        }
    })
}

fn apply_native_semantics(
    mut element: Stateful<Div>,
    semantic: Option<&crate::AccessibilityNode>,
) -> Stateful<Div> {
    let Some(semantic) = semantic else {
        return element;
    };
    let Some(role) = crate::accessibility::native_role(&semantic.role)
        .expect("committed semantic roles are validated")
    else {
        return element;
    };
    element = element.role(role);
    if let Some(id) = &semantic.semantic_id {
        element = element.accessibility_id(id.clone());
    }
    if !semantic.name.is_empty() {
        if semantic.role == "text" {
            // A retained plain-text wrapper owns the native text value. It
            // must not masquerade as a value-less AccessKit TextRun; macOS
            // consumers traverse TextRun values without an Option fallback.
            element = element.aria_value(semantic.name.clone());
        } else {
            element = element.aria_label(semantic.name.clone());
        }
    }
    if !semantic.description.is_empty() {
        element = element.aria_description(semantic.description.clone());
    }
    if let Some(selected) = semantic
        .selected
        .or_else(|| selected_from_checked(role, semantic.checked.as_ref()))
    {
        element = element.aria_selected(selected);
    }
    if let Some(expanded) = semantic
        .expanded
        .or_else(|| expanded_from_checked(role, semantic.checked.as_ref()))
    {
        element = element.aria_expanded(expanded);
    }
    if let Some(toggled) = toggled_from_semantics(role, semantic) {
        element = element.aria_toggled(toggled);
    }
    if let Some(value) = semantic.value.as_ref().and_then(accessibility_value_text) {
        element = element.aria_value(value);
    }
    if let Some(placeholder) = &semantic.placeholder {
        element = element.aria_placeholder(placeholder.clone());
    }
    if let Some(key_shortcuts) = &semantic.key_shortcuts {
        element = element.aria_keyshortcuts(key_shortcuts.clone());
    }
    if let Some(value) = semantic.value.as_ref().and_then(accessibility_number_value) {
        element = element.aria_numeric_value(value);
    }
    if let Some(value) = semantic.value_min {
        element = element.aria_min_numeric_value(value);
    }
    if let Some(value) = semantic.value_max {
        element = element.aria_max_numeric_value(value);
    }
    if let Some(orientation) = semantic.orientation.as_deref() {
        element = element.aria_orientation(match orientation {
            "horizontal" => gpui::Orientation::Horizontal,
            "vertical" => gpui::Orientation::Vertical,
            _ => unreachable!("committed accessibility orientation is validated"),
        });
    }
    if let Some(level) = semantic.level {
        element = element.aria_level(level);
    }
    if let Some(position) = semantic.position_in_set {
        element = element.aria_position_in_set(position);
    }
    if let Some(size) = semantic.size_of_set {
        element = element.aria_size_of_set(size);
    }
    if let Some(index) = semantic.row_index {
        element = element.aria_row_index(index);
    }
    if let Some(index) = semantic.column_index {
        element = element.aria_column_index(index);
    }
    if let Some(count) = semantic.row_count {
        element = element.aria_row_count(count);
    }
    if let Some(count) = semantic.column_count {
        element = element.aria_column_count(count);
    }
    apply_native_semantic_flags(element, semantic)
}

fn apply_native_semantic_flags(
    mut element: Stateful<Div>,
    semantic: &crate::AccessibilityNode,
) -> Stateful<Div> {
    let required = semantic.required;
    let disabled = semantic.disabled;
    let read_only = semantic.read_only;
    let invalid = semantic.invalid;
    let current = semantic.current.clone();
    if !(required || disabled || read_only || invalid || current.is_some()) {
        return element;
    }
    element = element.a11y_synthetic_children(move |builder| {
        let node = builder.parent_node();
        if required {
            node.set_required();
        }
        if disabled {
            node.set_disabled();
        }
        if read_only {
            node.set_read_only();
        }
        if invalid {
            node.set_invalid(gpui::accesskit::Invalid::True);
        }
        if let Some(current) = current.as_deref() {
            node.set_aria_current(match current {
                "page" => gpui::accesskit::AriaCurrent::Page,
                "step" => gpui::accesskit::AriaCurrent::Step,
                "location" => gpui::accesskit::AriaCurrent::Location,
                "date" => gpui::accesskit::AriaCurrent::Date,
                "time" => gpui::accesskit::AriaCurrent::Time,
                "true" => gpui::accesskit::AriaCurrent::True,
                _ => unreachable!("committed aria-current values are validated"),
            });
        }
    });
    element
}

fn apply_primitive_accessibility_actions<C: ColorResolver>(
    mut element: Stateful<Div>,
    node: &UiNode,
    retained_id: Option<NodeId>,
    semantic: Option<&crate::AccessibilityNode>,
    environment: &RenderEnvironment<'_, C>,
) -> Stateful<Div> {
    let Some(semantic) = semantic else {
        return element;
    };
    if semantic.disabled {
        return element;
    }
    let (UiNodeKind::Custom { primitive }, Some(node), Some(key)) =
        (node.kind(), retained_id, node.key())
    else {
        return element;
    };
    let instance =
        crate::PrimitiveInstanceId::new(primitive.primitive.clone(), key.as_str().to_owned(), node);
    for action in environment
        .primitives
        .accessibility_actions(&instance)
        .into_iter()
        .filter(|action| !(semantic.read_only && *action == gpui::AccessibleAction::SetValue))
    {
        let registry = environment.primitives.clone();
        let instance = instance.clone();
        element = element.on_a11y_action(action, move |data, window, cx| {
            // Stale and disabled native instances intentionally reject the
            // request without falling through to a second input path.
            let _ = registry.perform_accessibility_action(&instance, action, data, window, cx);
        });
    }
    element
}

fn selected_from_checked(role: gpui::Role, checked: Option<&UiValue>) -> Option<bool> {
    matches!(
        role,
        gpui::Role::Tab
            | gpui::Role::ListBoxOption
            | gpui::Role::Row
            | gpui::Role::GridCell
            | gpui::Role::MenuItem
    )
    .then(|| checked.and_then(accessibility_bool_value))
    .flatten()
}

fn expanded_from_checked(role: gpui::Role, checked: Option<&UiValue>) -> Option<bool> {
    matches!(role, gpui::Role::ComboBox)
        .then(|| checked.and_then(accessibility_bool_value))
        .flatten()
}

fn toggled_from_semantics(
    role: gpui::Role,
    semantic: &crate::AccessibilityNode,
) -> Option<gpui::Toggled> {
    if let Some(pressed) = semantic.pressed {
        return Some(pressed.into());
    }
    if matches!(
        role,
        gpui::Role::CheckBox
            | gpui::Role::RadioButton
            | gpui::Role::Switch
            | gpui::Role::MenuItemCheckBox
            | gpui::Role::MenuItemRadio
    ) {
        return match semantic.checked.as_ref() {
            Some(UiValue::Bool(value)) => Some((*value).into()),
            Some(UiValue::String(value)) if value == "mixed" => Some(gpui::Toggled::Mixed),
            _ => None,
        };
    }
    None
}

fn accessibility_bool_value(value: &UiValue) -> Option<bool> {
    match value {
        UiValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn accessibility_number_value(value: &UiValue) -> Option<f64> {
    match value {
        UiValue::Float(value) => Some(*value),
        UiValue::Integer(value) => value.to_string().parse().ok(),
        _ => None,
    }
}

fn accessibility_value_text(value: &UiValue) -> Option<String> {
    match value {
        UiValue::Null | UiValue::Handle(_) => None,
        UiValue::Bool(value) => Some(value.to_string()),
        UiValue::Integer(value) => Some(value.to_string()),
        UiValue::Float(value) => Some(value.to_string()),
        UiValue::String(value) => Some(value.clone()),
        UiValue::Array(_) | UiValue::Map(_) => serde_json::to_string(value).ok(),
    }
}

/// The interaction state of a node scope, with `Disabled` applied when the
/// node or an ancestor is disabled.
fn scoped_interaction<C>(environment: &RenderEnvironment<'_, C>) -> InteractionState {
    let mut interaction = environment.interaction.clone();
    if environment.focus.owner {
        interaction = interaction.with(PseudoState::GroupFocused);
    }
    if environment.focus.within {
        interaction = interaction.with(PseudoState::FocusWithin);
    }
    if environment.inherited_disabled {
        interaction.with(PseudoState::Disabled)
    } else {
        interaction
    }
}

fn resolved_hit_test<C: ColorResolver>(
    node: &UiNode,
    environment: &RenderEnvironment<'_, C>,
) -> Option<HitTestBehavior> {
    node.style()
        .resolve(&scoped_interaction(environment))
        .hit_test
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
        || node.attributes().contains_key("tab_stop")
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

fn apply_tab_behavior(
    mut element: Stateful<Div>,
    node: &UiNode,
    implicit_tab_stop: bool,
    ancestor_disabled: bool,
    persistent: Option<&FocusHandle>,
) -> Stateful<Div> {
    // A node's own explicit focus declaration wins over its own `disabled`
    // (focusable disabled menu items), but a disabled ancestor removes every
    // descendant from the tab order.
    let tab_stop = if ancestor_disabled {
        false
    } else if node_has_focus_declaration(node) {
        node_tab_stop(node)
    } else {
        implicit_tab_stop
    };
    element = element.tab_index(node_tab_index(node)).tab_stop(tab_stop);
    // GPUI applies the element tab policy only to handles it creates itself; a
    // tracked persistent handle carries its own policy.
    if let Some(handle) = persistent {
        element = element.track_focus(
            &handle
                .clone()
                .tab_stop(tab_stop)
                .tab_index(node_tab_index(node)),
        );
    }
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

fn styled_text(
    text: &str,
    spans: &[crate::Span],
    colors: &impl ColorResolver,
    motions: &BTreeMap<MotionKey, f64>,
    path: &str,
) -> StyledText {
    let mut offset = 0usize;
    let mut families = Vec::new();
    let highlights = spans.iter().filter_map(|span| {
        let start = offset;
        offset = offset.saturating_add(span.text().len());
        // A span role changes the face only: size and line height stay the paragraph's.
        let role = span
            .typography_role()
            .and_then(|role| colors.resolve_typography(role));
        if let Some(family) = role.as_ref().and_then(|role| role.family.clone()) {
            families.push((start..offset, SharedString::from(family)));
        }
        let style = HighlightStyle {
            color: span
                .color_value()
                .and_then(|color| colors.resolve(color))
                .map(|color| rgba(color.as_rgba_hex()).into()),
            background_color: span
                .background_value()
                .and_then(|color| colors.resolve(color))
                .map(|color| rgba(color.as_rgba_hex()).into()),
            font_weight: if span.is_bold() {
                Some(FontWeight::BOLD)
            } else {
                role.as_ref().map(|role| FontWeight(f32::from(role.weight)))
            },
            font_style: span.is_italic().then_some(FontStyle::Italic),
            fade_out: span.key().and_then(|key| {
                motions
                    .get(&MotionKey::for_node(
                        &crate::motion::span_motion_path(path, key),
                        MotionProperty::Opacity,
                    ))
                    .map(|opacity| f64_to_f32(1.0 - opacity.clamp(0.0, 1.0)))
            }),
            ..HighlightStyle::default()
        };
        (style != HighlightStyle::default()).then_some((start..offset, style))
    });
    let highlights = highlights.collect::<Vec<_>>();
    let styled = StyledText::new(text.to_owned()).with_highlights(highlights);
    if families.is_empty() {
        styled
    } else {
        styled.with_font_family_overrides(families)
    }
}

fn render_canvas(
    element: impl ParentElement + IntoElement,
    scene: &crate::CanvasScene,
    colors: &impl ColorResolver,
    motion: NodeMotionValues,
    retained_id: Option<NodeId>,
    geometry: &crate::GeometryRegistry,
) -> AnyElement {
    let scene = scene.clone();
    let colors = OwnedColorResolver::capture(colors);
    let geometry = geometry.clone();
    let canvas = gpui::canvas(
        move |bounds, window, _| {
            if let Some(node) = retained_id
                && let Ok(drawable) = crate::GeometryBounds::new(
                    f64::from(bounds.origin.x),
                    f64::from(bounds.origin.y),
                    f64::from(bounds.size.width),
                    f64::from(bounds.size.height),
                )
            {
                let offset = window.pixel_snap_point(window.element_offset());
                geometry.update_canvas_drawable(
                    node,
                    drawable,
                    (f64::from(offset.x), f64::from(offset.y)),
                );
            }
        },
        move |bounds, (), window, _| {
            paint_canvas_scene(bounds, &scene, &colors, motion, window);
        },
    )
    .size_full();
    element.child(canvas).into_any_element()
}

fn paint_canvas_scene(
    bounds: Bounds<Pixels>,
    scene: &crate::CanvasScene,
    colors: &impl ColorResolver,
    motion: NodeMotionValues,
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
                    paint_canvas_polygon(
                        [
                            (*x, *y),
                            (x + width, *y),
                            (x + width, y + height),
                            (*x, y + height),
                        ],
                        bounds,
                        motion,
                        rgba(color.as_rgba_hex()),
                        window,
                    );
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
                    let points = (0..48).map(|index| {
                        let angle = std::f64::consts::TAU * f64::from(index) / 48.0;
                        (
                            center_x + radius * angle.cos(),
                            center_y + radius * angle.sin(),
                        )
                    });
                    paint_canvas_polygon(points, bounds, motion, rgba(color.as_rgba_hex()), window);
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
                    path.move_to(node_canvas_point(bounds, *from_x, *from_y, motion));
                    path.line_to(node_canvas_point(bounds, *to_x, *to_y, motion));
                    if let Ok(path) = path.build() {
                        window.paint_path(path, rgba(color.as_rgba_hex()));
                    }
                }
            }
            crate::CanvasCommand::Path { .. } | crate::CanvasCommand::MorphPath { .. } => {
                paint_canvas_path(bounds, command, colors, motion, window);
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
fn paint_canvas_path(
    bounds: Bounds<Pixels>,
    command: &crate::CanvasCommand,
    colors: &impl ColorResolver,
    motion: NodeMotionValues,
    window: &mut Window,
) {
    let interpolated;
    let (segments, fill, stroke, transform, clip, is_morph) = match command {
        crate::CanvasCommand::Path {
            segments,
            fill,
            stroke,
            transform,
            clip,
            ..
        } => (
            segments.as_slice(),
            fill.as_ref(),
            stroke.as_ref(),
            transform,
            clip,
            false,
        ),
        crate::CanvasCommand::MorphPath {
            from,
            to,
            stroke,
            transform,
            clip,
            ..
        } => {
            interpolated =
                crate::canvas::interpolate_path(from, to, motion.path_progress.unwrap_or(0.0))
                    .unwrap_or_else(|| from.clone());
            (
                interpolated.as_slice(),
                None,
                Some(stroke),
                transform,
                clip,
                true,
            )
        }
        _ => return,
    };
    let mut builder = stroke.map_or_else(gpui::PathBuilder::fill, |(_, width)| {
        gpui::PathBuilder::stroke(px(f64_to_f32(*width)))
    });
    if !is_morph && stroke.is_some() && motion.path_progress.is_some_and(|progress| progress < 1.0)
    {
        let paths = crate::canvas::trimmed_canvas_paths(
            crate::canvas::flatten_path(segments, *transform),
            motion.path_progress.unwrap_or(1.0).clamp(0.0, 1.0),
        );
        for path in paths {
            if let Some((first, rest)) = path.split_first() {
                builder.move_to(node_canvas_point(bounds, first.0, first.1, motion));
                for point in rest {
                    builder.line_to(node_canvas_point(bounds, point.0, point.1, motion));
                }
            }
        }
    } else {
        for segment in segments {
            match segment {
                crate::CanvasPathSegment::Move { x, y } => {
                    builder.move_to(canvas_path_point(bounds, *transform, *x, *y, motion));
                }
                crate::CanvasPathSegment::Line { x, y } => {
                    builder.line_to(canvas_path_point(bounds, *transform, *x, *y, motion));
                }
                crate::CanvasPathSegment::Quadratic {
                    x,
                    y,
                    control_x,
                    control_y,
                } => builder.curve_to(
                    canvas_path_point(bounds, *transform, *x, *y, motion),
                    canvas_path_point(bounds, *transform, *control_x, *control_y, motion),
                ),
                crate::CanvasPathSegment::Cubic {
                    x,
                    y,
                    control_a_x,
                    control_a_y,
                    control_b_x,
                    control_b_y,
                } => builder.cubic_bezier_to(
                    canvas_path_point(bounds, *transform, *x, *y, motion),
                    canvas_path_point(bounds, *transform, *control_a_x, *control_a_y, motion),
                    canvas_path_point(bounds, *transform, *control_b_x, *control_b_y, motion),
                ),
                crate::CanvasPathSegment::Close => builder.close(),
            }
        }
    }
    let Ok(path) = builder.build() else {
        return;
    };
    let paint = stroke
        .and_then(|(color, _)| {
            colors
                .resolve(color)
                .map(|color| Background::from(rgba(color.as_rgba_hex())))
        })
        .or_else(|| fill.and_then(|fill| canvas_fill(fill, colors)));
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

fn paint_canvas_polygon(
    points: impl IntoIterator<Item = (f64, f64)>,
    bounds: Bounds<Pixels>,
    motion: NodeMotionValues,
    paint: impl Into<Background>,
    window: &mut Window,
) {
    let mut points = points.into_iter();
    let Some((x, y)) = points.next() else {
        return;
    };
    let mut path = gpui::PathBuilder::fill();
    path.move_to(node_canvas_point(bounds, x, y, motion));
    for (x, y) in points {
        path.line_to(node_canvas_point(bounds, x, y, motion));
    }
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, paint.into());
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
    motion: NodeMotionValues,
) -> Point<Pixels> {
    let (transformed_x, transformed_y) = crate::canvas::canvas_transform_point(transform, x, y);
    node_canvas_point(bounds, transformed_x, transformed_y, motion)
}

fn node_canvas_point(
    bounds: Bounds<Pixels>,
    x: f64,
    y: f64,
    motion: NodeMotionValues,
) -> Point<Pixels> {
    let (transformed_x, transformed_y) = crate::canvas::canvas_motion_point(
        f64::from(bounds.size.width),
        f64::from(bounds.size.height),
        x,
        y,
        motion.canvas_transform(),
    );
    point(
        bounds.origin.x + px(f64_to_f32(transformed_x)),
        bounds.origin.y + px(f64_to_f32(transformed_y)),
    )
}

fn render_image<C: ColorResolver>(
    element: impl ParentElement + IntoElement,
    source: &ImageSourceSpec,
    environment: &RenderEnvironment<'_, C>,
) -> AnyElement {
    let tint = environment.ambient_text_color;
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
    let resolved_style = node.style().resolve(&scoped_interaction(environment));
    let color = environment.ambient_text_color;
    let fills_styled_box = resolved_style.width.is_some() || resolved_style.height.is_some();
    let image = environment.assets.map_or_else(
        || AssetRegistry::new().inline_svg_source(source.shared_source(), color),
        |assets| assets.inline_svg_source(source.shared_source(), color),
    );
    element
        .child(inline_svg_image(image, fills_styled_box))
        .into_any_element()
}

fn inline_svg_image(source: impl Into<gpui::ImageSource>, fills_styled_box: bool) -> Img {
    let image = img(source);
    if fills_styled_box {
        image.size_full()
    } else {
        image
    }
}

/// The rendered spec: window-wide ids and physical edges. The overlay is
/// scoped by the instance that declared it; a `parent` key names the nearest
/// enclosing overlay with that key, else one the same instance declared.
fn scoped_overlay_spec(
    spec: &OverlayNodeSpec,
    view_id: &str,
    direction: TextDirection,
    enclosing: Option<&OverlayScope<'_>>,
) -> OverlayNodeSpec {
    let mut rendered = spec.clone();
    let owner = spec.owner.as_ref();
    rendered.id = WindowOverlayCoordinator::scoped_id(view_id, owner, &rendered.id);
    rendered.parent = rendered.parent.as_ref().map(|parent| {
        enclosing
            .and_then(|scope| scope.find(parent))
            .cloned()
            .unwrap_or_else(|| WindowOverlayCoordinator::scoped_id(view_id, owner, parent))
    });
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
    // Logical alignment along a horizontal cross axis follows the text direction.
    let horizontal_cross = matches!(
        rendered.placement,
        crate::OverlayPlacement::Top | crate::OverlayPlacement::Bottom
    );
    if horizontal_cross && direction == TextDirection::RightToLeft {
        rendered.align = match rendered.align {
            crate::OverlayAlign::Start => crate::OverlayAlign::End,
            crate::OverlayAlign::End => crate::OverlayAlign::Start,
            crate::OverlayAlign::Center => crate::OverlayAlign::Center,
        };
    }
    rendered
}

/// The overlay's `open_change` handler as the native element calls it.
fn overlay_open_change<C: ColorResolver>(
    node: &UiNode,
    environment: &RenderEnvironment<'_, C>,
    target: EventTargetContext,
) -> Option<crate::overlay_element::OpenChangeHandler> {
    node.handler("open_change").map(|handler| {
        let handler = handler.clone();
        let dispatcher = environment.dispatcher.cloned();
        Rc::new(move |open, window: &mut Window, cx: &mut App| {
            dispatch_ui_event(
                &handler,
                "open_change",
                UiValue::Bool(open),
                target.snapshot(),
                window,
                cx,
                dispatcher.as_ref(),
            );
        }) as crate::overlay_element::OpenChangeHandler
    })
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
    let trigger_focusable = subtree_takes_focus(trigger, environment.inherited_disabled);
    let mut rendered_spec = scoped_overlay_spec(
        spec,
        environment.view_id,
        environment.direction,
        environment.overlay_scope,
    );
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
    // The panel holding focus itself is seen by `group_focus` styles in its
    // content (a panel's focus frame), as a focusable node's is in its subtree.
    let panel_focus = retained_id.and_then(|id| environment.focus_handles.get(&id).cloned());
    let scope = OverlayScope {
        local: &spec.id,
        id: rendered_spec.id.clone(),
        outer: environment.overlay_scope,
    };
    let content_environment = RenderEnvironment {
        focus: NodeFocus {
            owner: retained_id.is_some() && environment.focus_path.first() == retained_id.as_ref(),
            ..environment.focus
        },
        overlay_scope: Some(&scope),
        ..*environment
    };
    let content = GpuiNodeRenderer::render_internal(
        content,
        &content_environment,
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
    let open_change = overlay_open_change(node, environment, event_target.clone());
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
        let colors = OwnedColorResolver::capture(&environment.resolver());
        let direction = environment.direction;
        Rc::new(move |backdrop: Div| apply_style_override(backdrop, &style, &colors, direction))
            as crate::overlay_element::BackdropStyleHandler
    });
    overlay
        .with_backdrop_style(backdrop_style)
        .with_focus_ring(semantic_color(
            &environment.resolver(),
            "focus_ring",
            0x003b_82f6,
        ))
        .with_focus_surface(semantic_color(
            &environment.resolver(),
            "surface",
            0x0018_181b,
        ))
        .restore_focus_on_close(restore_focus_on_close)
        .with_trigger_focusable(trigger_focusable)
        .with_panel_focus(panel_focus)
        .with_identity(crate::overlay_element::OverlayIdentity {
            view_id: environment.view_id.to_owned(),
            owner: spec.owner.as_ref().map(ToString::to_string),
            local: spec.id.clone(),
        })
}

/// In an RTL stretching column, a child with a definite width belongs on the
/// start (right) edge; `apply_flex_alignment` mirrors `Start` to the right.
fn rtl_start_edge(mut style: StyleProperties, rtl_stretch_parent: bool) -> StyleProperties {
    if rtl_stretch_parent && style.align_self.is_none() && style.width.is_some() {
        style.align_self = Some(Align::Start);
    }
    style
}

/// A stretched child takes its stretched width as a definite width.
///
/// Stretching is a property of the parent; without a definite width Taffy
/// first measures such a child at its content width and then lays it out again
/// at the stretched width, and the two passes compound at every nesting level
/// (a 28 ms frame for a `ListDetail` inside an `AppShell` became 0.4 ms).
///
/// Only where the two are the same box: a horizontal margin (auto or not)
/// takes its share out of the stretched width, which `width: 100%` would not.
fn definite_stretch(mut style: StyleProperties, stretch_parent: bool) -> StyleProperties {
    let margin = &style.margin;
    let horizontal_margin = [margin.left, margin.right, margin.start, margin.end]
        .into_iter()
        .flatten()
        .any(|value| !is_zero_length(value));
    if stretch_parent
        && !horizontal_margin
        && style.width.is_none()
        && style.align_self.is_none()
        && style.position != Some(PositionMode::Absolute)
    {
        style.width = Some(Length::Relative(1.0).into());
    }
    style
}

fn is_zero_length(value: crate::LayoutLength) -> bool {
    match value {
        crate::LayoutLength::Definite(Length::Pixels(pixels) | Length::Rems(pixels))
        | crate::LayoutLength::Signed(
            crate::SignedLength::Pixels(pixels) | crate::SignedLength::Rems(pixels),
        ) => pixels.abs() <= f64::EPSILON,
        _ => false,
    }
}

fn children_take_focus(node: &UiNode, disabled: bool) -> bool {
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => children
            .iter()
            .any(|child| subtree_takes_focus(child, disabled)),
        UiNodeKind::Overlay { trigger, .. } => subtree_takes_focus(trigger, disabled),
        UiNodeKind::ErrorBoundary { child, .. } => subtree_takes_focus(child, disabled),
        _ => false,
    }
}

/// Whether rendering `node` puts a tab stop anywhere in its subtree, by the same
/// policy as `apply_tab_behavior`: an explicit declaration wins, otherwise a click
/// or key handler makes a node a stop, and native primitives own their focus.
fn subtree_takes_focus(node: &UiNode, disabled: bool) -> bool {
    let disabled = disabled || is_disabled(node);
    if disabled {
        return false;
    }
    if matches!(node.kind(), UiNodeKind::Custom { .. }) {
        return true;
    }
    if node_has_focus_declaration(node) && node_tab_stop(node) {
        return true;
    }
    if !node_has_focus_declaration(node) && !node.event_handlers("click").is_empty() {
        return true;
    }
    let children = children_take_focus(node, disabled);
    children || !node_has_focus_declaration(node) && !key_handler_bindings(node, false).is_empty()
}

fn overlay_panel_key(
    node: &UiNode,
    dispatcher: Option<&NodeEventDispatcher>,
    direction: TextDirection,
    target: EventTargetContext,
) -> Option<crate::overlay_element::PanelKeyHandler> {
    let handlers = node
        .handlers()
        .iter()
        .filter_map(|(event, bindings)| {
            let key = event.strip_prefix("key:")?;
            let binding = bindings
                .iter()
                .find(|binding| binding.phase() == crate::EventPhase::Target)?;
            let payload = binding
                .value()
                .or_else(|| node.node_payload(event))
                .cloned()
                .unwrap_or(UiValue::Null);
            Some((key.to_owned(), (binding.handler().clone(), payload)))
        })
        .collect::<BTreeMap<_, _>>();
    (!handlers.is_empty()).then(|| {
        let dispatcher = dispatcher.cloned();
        Rc::new(
            move |event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App| {
                let key = logical_keyboard_key(event.keystroke.key.as_str(), direction);
                let Some((callback, payload)) = handlers.get(key) else {
                    return false;
                };
                dispatch_ui_event(
                    callback,
                    "key",
                    payload.clone(),
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
    let retained_roots: BTreeMap<String, NodeId> = retained_id
        .map(|node| {
            retained_children(environment.retained, environment.retained_links, node)
                .into_iter()
                .filter(|child| child.group() == "items")
                .zip(spec.realized.keys())
                .filter_map(|(child, index)| {
                    crate::virtual_list_element::collection_item_key(spec, *index)
                        .map(|key| (format!("item:{key}"), child.node()))
                })
                .collect()
        })
        .unwrap_or_default();
    let runtime = owned_slot_runtime(environment, path, retained_roots);
    VirtualListEntityElement::new_collection(path, spec.clone(), runtime, retained_id)
}

fn owned_slot_runtime<C: ColorResolver>(
    environment: &RenderEnvironment<'_, C>,
    path: &str,
    retained_roots: BTreeMap<String, NodeId>,
) -> NodeSlotRuntime {
    let retained_links = retained_link_subtrees(
        environment.retained,
        environment.retained_links,
        retained_roots.values().copied(),
    );
    // Owners go to their own map: in `focus_handles` every slot node would
    // track its owner's handle, and the owner's focus would resolve to the
    // last of them (a Table row took the table's keys).
    let focus_handles = environment.focus_handles.clone();
    let mut focus_owners = environment.focus_owners.clone();
    if let Some(tree) = environment.retained {
        let nodes = retained_links
            .iter()
            .flat_map(|(parent, children)| {
                std::iter::once(*parent).chain(children.iter().map(crate::RetainedChildLink::node))
            })
            .collect::<BTreeSet<_>>();
        for node in nodes {
            if let Some(focus) =
                primitive_focus_owner(Some(tree), Some(node), environment.focus_handles)
            {
                focus_owners.insert(node, focus);
            }
        }
    }
    NodeSlotRuntime {
        now: environment.now,
        clock: environment.clock.clone(),
        colors: OwnedColorResolver::capture(&environment.resolver()),
        primitives: environment.primitives.clone(),
        assets: environment.assets.cloned().unwrap_or_default(),
        dispatcher: environment
            .dispatcher
            .cloned()
            .unwrap_or_else(|| NodeEventDispatcher::new(|_, _, _, _, _| EventPropagation::Handled)),
        overlays: environment.overlays.clone(),
        interactions: environment.interactions.clone(),
        motions: environment.motions.clone(),
        motion_preference: environment.motion_preference,
        motion_quality: environment.motion_quality,
        signals: environment.signals.clone(),
        geometry: environment.geometry.clone(),
        pointer_capture: environment.pointer_capture.clone(),
        focus_handles,
        focus_owners,
        scroll_handles: environment.scroll_handles.clone(),
        scroll_anchors: environment.scroll_anchors.clone(),
        virtual_requests: environment.virtual_requests.clone(),
        text_selection: environment.text_selection.clone(),
        host_focus: environment.host_focus.cloned(),
        direction: environment.direction,
        locale: environment.locale.to_owned(),
        number: environment.number.cloned(),
        ambient_text_color: environment.ambient_text_color,
        environment: environment.environment,
        inherited_disabled: environment.inherited_disabled,
        focus_path: environment.focus_path.to_vec(),
        owner_focused: environment.focus.owner,
        base_path: path.to_owned(),
        view_id: environment.view_id.to_owned(),
        semantics: environment.semantics.cloned().unwrap_or_default(),
        a11y_active: environment.a11y_active,
        window_drag: environment.window_drag,
        retained_roots,
        retained_links,
    }
}

fn render_table_layout<C: ColorResolver>(
    node: &UiNode,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
    retained_id: Option<NodeId>,
) -> Option<AnyElement> {
    let crate::table_layout::TableLayout::Columns(columns) = node.table_layout()? else {
        return None;
    };
    let mut columns = columns.clone();
    collect_table_column_minima(node, &mut columns, environment, true);
    let mut style = node.style().resolve(environment.interaction);
    resolve_style_lengths(&mut style, &environment.resolver());
    let roots = retained_id
        .into_iter()
        .map(|root| ("table".to_owned(), root))
        .collect();
    let runtime = owned_slot_runtime(environment, path, roots);
    let table = node.clone();
    let root_path = path.to_owned();
    Some(
        crate::table_layout::TableViewportElement::new(
            &interaction_element_id(retained_id, path),
            columns,
            environment.signals.clone(),
            environment.direction,
            style,
            retained_id.and_then(|root| environment.scroll_handles.get(&root).cloned()),
            move |plan, border| {
                let mut table = table;
                table.resolve_table_layout(&plan, border);
                runtime.render_at(&table, &root_path, retained_id)
            },
        )
        .into_any_element(),
    )
}

fn collect_table_column_minima<C: ColorResolver>(
    node: &UiNode,
    columns: &mut [crate::table_layout::TableColumn],
    environment: &RenderEnvironment<'_, C>,
    root: bool,
) {
    if !root && node.table_layout().is_some() {
        return;
    }
    if let Some(column) = node.table_column().and_then(|index| columns.get_mut(index)) {
        let mut style = node.style().resolve(environment.interaction);
        resolve_style_lengths(&mut style, &environment.resolver());
        column.include_minimum_style(&style, environment.direction);
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for child in children {
                collect_table_column_minima(child, columns, environment, false);
            }
        }
        UiNodeKind::VirtualCollection { spec } => {
            for child in spec.realized.values() {
                collect_table_column_minima(child, columns, environment, false);
            }
        }
        _ => {}
    }
}

fn retained_link_subtrees(
    tree: Option<&RetainedUiTree>,
    links: Option<&BTreeMap<NodeId, Vec<crate::RetainedChildLink>>>,
    roots: impl IntoIterator<Item = NodeId>,
) -> BTreeMap<NodeId, Vec<crate::RetainedChildLink>> {
    let mut result = BTreeMap::new();
    let mut pending = roots.into_iter().collect::<Vec<_>>();
    while let Some(node) = pending.pop() {
        let children = retained_children(tree, links, node);
        pending.extend(children.iter().map(crate::RetainedChildLink::node));
        result.insert(node, children);
    }
    result
}

fn retained_children(
    tree: Option<&RetainedUiTree>,
    links: Option<&BTreeMap<NodeId, Vec<crate::RetainedChildLink>>>,
    node: NodeId,
) -> Vec<crate::RetainedChildLink> {
    tree.and_then(|tree| tree.node(node))
        .map(|node| node.children().cloned().collect())
        .or_else(|| links.and_then(|links| links.get(&node)).cloned())
        .unwrap_or_default()
}

#[derive(Clone, Copy, Default)]
struct NodeMotionValues {
    opacity: Option<f64>,
    translate_x: Option<f64>,
    translate_y: Option<f64>,
    rotate: Option<f64>,
    scale_x: Option<f64>,
    scale_y: Option<f64>,
    skew_x: Option<f64>,
    skew_y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    clip_height: Option<f64>,
    path_progress: Option<f64>,
}

impl NodeMotionValues {
    fn set(&mut self, property: MotionProperty, value: f64) {
        let target = match property {
            MotionProperty::Opacity => &mut self.opacity,
            MotionProperty::TranslateX => &mut self.translate_x,
            MotionProperty::TranslateY => &mut self.translate_y,
            MotionProperty::Rotate => &mut self.rotate,
            MotionProperty::ScaleX => &mut self.scale_x,
            MotionProperty::ScaleY => &mut self.scale_y,
            MotionProperty::SkewX => &mut self.skew_x,
            MotionProperty::SkewY => &mut self.skew_y,
            MotionProperty::Width => &mut self.width,
            MotionProperty::Height => &mut self.height,
            MotionProperty::ClipHeight => &mut self.clip_height,
            MotionProperty::PathProgress => &mut self.path_progress,
        };
        *target = Some(value);
    }

    fn canvas_transform(self) -> crate::geometry::CanvasMotionTransform {
        crate::geometry::CanvasMotionTransform {
            rotate: self.rotate.unwrap_or(0.0),
            scale_x: self.scale_x.unwrap_or(1.0),
            scale_y: self.scale_y.unwrap_or(1.0),
            skew_x: self.skew_x.unwrap_or(0.0),
            skew_y: self.skew_y.unwrap_or(0.0),
            path_progress: self.path_progress,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct NodeSignalValues {
    opacity: Option<f64>,
    translate_x: Option<f64>,
    translate_y: Option<f64>,
    rotate: Option<f64>,
    scale_x: Option<f64>,
    scale_y: Option<f64>,
    width: Option<f64>,
    width_override: Option<f64>,
    height: Option<f64>,
    height_override: Option<f64>,
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
            (
                crate::SignalProperty::TranslateXOverride,
                crate::SignalValue::OptionalFloat(value),
            ) => {
                values.translate_x = value;
            }
            (crate::SignalProperty::TranslateY, crate::SignalValue::Float(value)) => {
                values.translate_y = Some(value);
            }
            (
                crate::SignalProperty::TranslateYOverride,
                crate::SignalValue::OptionalFloat(value),
            ) => {
                values.translate_y = value;
            }
            (crate::SignalProperty::Rotate, crate::SignalValue::Float(value)) => {
                values.rotate = Some(value);
            }
            (crate::SignalProperty::RotateOverride, crate::SignalValue::OptionalFloat(value)) => {
                values.rotate = value;
            }
            (crate::SignalProperty::ScaleX, crate::SignalValue::Float(value)) => {
                values.scale_x = Some(value);
            }
            (crate::SignalProperty::ScaleXOverride, crate::SignalValue::OptionalFloat(value)) => {
                values.scale_x = value;
            }
            (crate::SignalProperty::ScaleY, crate::SignalValue::Float(value)) => {
                values.scale_y = Some(value);
            }
            (crate::SignalProperty::ScaleYOverride, crate::SignalValue::OptionalFloat(value)) => {
                values.scale_y = value;
            }
            (crate::SignalProperty::Width, crate::SignalValue::Float(value)) => {
                values.width = Some(value);
            }
            (crate::SignalProperty::WidthOverride, crate::SignalValue::OptionalFloat(value)) => {
                values.width_override = value;
            }
            (crate::SignalProperty::Height, crate::SignalValue::Float(value)) => {
                values.height = Some(value);
            }
            (crate::SignalProperty::HeightOverride, crate::SignalValue::OptionalFloat(value)) => {
                values.height_override = value;
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

fn apply_canvas_signal_transform(
    mut motion: NodeMotionValues,
    signals: &NodeSignalValues,
) -> NodeMotionValues {
    motion.rotate = signals.rotate.or(motion.rotate);
    motion.scale_x = signals.scale_x.or(motion.scale_x);
    motion.scale_y = signals.scale_y.or(motion.scale_y);
    motion
}

/// Merge the variant a node's string signal currently selects.
fn apply_signal_state_style(
    style: &mut StyleProperties,
    registry: &crate::SignalRegistry,
    node: &UiNode,
) {
    let Some(signal_style) = node.signal_style() else {
        return;
    };
    if let Ok(crate::SignalValue::String(state)) = registry.read(&signal_style.signal)
        && let Some(variant) = signal_style.states.get(&state)
    {
        style.merge(&variant.base);
    }
}

fn apply_signal_style(style: &mut StyleProperties, values: &NodeSignalValues) {
    if let Some(width) = values.width {
        style.width = Some(Length::Pixels(width.max(0.0)).into());
    }
    if let Some(width) = values.width_override {
        style.width = Some(Length::Pixels(width.max(0.0)).into());
        style.flex_basis = None;
        style.flex_grow = Some(false);
        style.flex_grow_weight = None;
        style.flex_shrink = Some(false);
    }
    if let Some(height) = values.height {
        style.height = Some(Length::Pixels(height.max(0.0)).into());
    }
    if let Some(height) = values.height_override {
        style.height = Some(Length::Pixels(height.max(0.0)).into());
        style.flex_basis = None;
        style.flex_grow = Some(false);
        style.flex_grow_weight = None;
        style.flex_shrink = Some(false);
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

fn node_motion(values: &BTreeMap<MotionKey, f64>, path: &str) -> NodeMotionValues {
    let value = |property| values.get(&MotionKey::for_node(path, property)).copied();
    NodeMotionValues {
        opacity: value(MotionProperty::Opacity),
        translate_x: value(MotionProperty::TranslateX),
        translate_y: value(MotionProperty::TranslateY),
        rotate: value(MotionProperty::Rotate),
        scale_x: value(MotionProperty::ScaleX),
        scale_y: value(MotionProperty::ScaleY),
        skew_x: value(MotionProperty::SkewX),
        skew_y: value(MotionProperty::SkewY),
        width: value(MotionProperty::Width),
        height: value(MotionProperty::Height),
        clip_height: value(MotionProperty::ClipHeight),
        path_progress: value(MotionProperty::PathProgress),
    }
}

fn apply_motion_dimensions(style: &mut StyleProperties, values: NodeMotionValues) {
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

pub(crate) fn f64_to_f32(value: f64) -> f32 {
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

    #[allow(clippy::too_many_lines)]
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
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase.bubble() && event.button == MouseButton::Left && hitbox.is_hovered(window)
                {
                    let index = clamp(layout.index_for_position(event.position));
                    anchor.set(Some(index));
                    selection.begin(owner.clone(), node, layout.text(), hitbox.bounds);
                    if let Some(focus) = &host_focus {
                        focus.focus(window, cx);
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

fn layout_motion_spec(node: &UiNode, now: Instant) -> Option<LayoutMotionRenderSpec> {
    let duration_ms = match node.attributes().get("layout_motion_duration_ms") {
        Some(UiValue::Integer(value)) => u64::try_from(*value).ok()?,
        _ => return None,
    };
    let easing = match node.attributes().get("layout_motion_easing") {
        Some(UiValue::String(value)) => crate::MotionEasing::parse(value).ok()?,
        _ => crate::MotionEasing::EaseOut,
    };
    let shared = match (
        crate::motion::scoped_shared_layout_group(node),
        node.attributes().get("shared_layout_id"),
    ) {
        (Some(group), Some(UiValue::String(id))) => Some((group, id.clone())),
        _ => None,
    };
    Some(LayoutMotionRenderSpec {
        now,
        duration: std::time::Duration::from_millis(duration_ms),
        easing,
        shared,
    })
}

struct GeometryTrackedElement {
    child: Option<AnyElement>,
    node: NodeId,
    /// The node has an element ref: it records a hitbox over its visual
    /// bounds, under its ancestors' clip and in its place in paint order.
    hit_area: bool,
    registry: crate::GeometryRegistry,
    translate_x: f64,
    translate_y: f64,
    layout_motion: Option<LayoutMotionRenderSpec>,
    progress_motions: Vec<crate::MotionProgressBinding>,
    scroll_handles: Vec<ScrollHandle>,
    focus_handle: Option<FocusHandle>,
    now: Instant,
    motion_preference: crate::MotionPreference,
}

#[derive(Clone, Debug)]
struct LayoutMotionRenderSpec {
    now: Instant,
    duration: std::time::Duration,
    easing: crate::MotionEasing,
    shared: Option<(String, String)>,
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

    #[allow(clippy::too_many_lines)]
    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let base_offset = window.pixel_snap_point(window.element_offset());
        self.registry.record_element_offset(
            self.node,
            (f64::from(base_offset.x), f64::from(base_offset.y)),
        );
        if let Ok(layout) = crate::GeometryBounds::new(
            f64::from(bounds.origin.x),
            f64::from(bounds.origin.y),
            f64::from(bounds.size.width),
            f64::from(bounds.size.height),
        ) {
            let viewport = window.viewport_size();
            for binding in &self.progress_motions {
                if binding.driver == crate::MotionProgressDriver::Focus {
                    self.registry.set_motion_trigger(
                        self.node,
                        crate::MotionProgressDriver::Focus,
                        self.focus_handle
                            .as_ref()
                            .is_some_and(|handle| handle.contains_focused(window, cx)),
                    );
                }
                if matches!(
                    binding.driver,
                    crate::MotionProgressDriver::Hover
                        | crate::MotionProgressDriver::Press
                        | crate::MotionProgressDriver::Focus
                ) {
                    let sample = self.registry.sample_motion_trigger(
                        self.node,
                        binding,
                        self.motion_preference,
                        self.now,
                    );
                    let changed = self.registry.update_motion_progress(
                        self.node,
                        binding.property(),
                        sample.value,
                    );
                    if changed || sample.active {
                        window.request_animation_frame();
                    }
                    continue;
                }
                let progress = match binding.driver {
                    crate::MotionProgressDriver::InView => {
                        let visible_width = (layout.x + layout.width)
                            .min(f64::from(viewport.width))
                            .max(0.0)
                            - layout.x.max(0.0);
                        let visible_height = (layout.y + layout.height)
                            .min(f64::from(viewport.height))
                            .max(0.0)
                            - layout.y.max(0.0);
                        if layout.width <= f64::EPSILON || layout.height <= f64::EPSILON {
                            0.0
                        } else {
                            (visible_width.max(0.0) * visible_height.max(0.0)
                                / (layout.width * layout.height))
                                .clamp(0.0, 1.0)
                        }
                    }
                    crate::MotionProgressDriver::Viewport => ((f64::from(viewport.height)
                        - layout.y)
                        / (f64::from(viewport.height) + layout.height).max(1.0))
                    .clamp(0.0, 1.0),
                    crate::MotionProgressDriver::ScrollX => {
                        self.scroll_handles.first().map_or(0.0, |handle| {
                            let maximum = f64::from(handle.max_offset().x).abs();
                            if maximum <= f64::EPSILON {
                                0.0
                            } else {
                                (-f64::from(handle.offset().x) / maximum).clamp(0.0, 1.0)
                            }
                        })
                    }
                    crate::MotionProgressDriver::ScrollY => {
                        self.scroll_handles.first().map_or(0.0, |handle| {
                            let maximum = f64::from(handle.max_offset().y).abs();
                            if maximum <= f64::EPSILON {
                                0.0
                            } else {
                                (-f64::from(handle.offset().y) / maximum).clamp(0.0, 1.0)
                            }
                        })
                    }
                    crate::MotionProgressDriver::Hover
                    | crate::MotionProgressDriver::Press
                    | crate::MotionProgressDriver::Focus => unreachable!(),
                };
                let value = crate::motion::sample_progress_source(&binding.source, progress);
                if self
                    .registry
                    .update_motion_progress(self.node, binding.property(), value)
                {
                    window.request_animation_frame();
                }
            }
            let sample = self.layout_motion.as_ref().map_or_else(
                crate::geometry::LayoutMotionSample::default,
                |spec| {
                    self.registry.sample_layout_motion(
                        self.node,
                        layout,
                        crate::geometry::LayoutMotionRequest {
                            shared: spec
                                .shared
                                .as_ref()
                                .map(|(group, id)| (group.as_str(), id.as_str())),
                            duration: spec.duration,
                            easing: spec.easing,
                            preference: self.motion_preference,
                            now: spec.now,
                        },
                    )
                },
            );
            if sample.active {
                window.request_animation_frame();
            }
            let visual = crate::GeometryBounds {
                x: layout.x + self.translate_x + sample.offset_x,
                y: layout.y + self.translate_y + sample.offset_y,
                width: layout.width * sample.scale_x,
                height: layout.height * sample.scale_y,
            };
            self.registry.update(
                self.node,
                crate::ElementGeometry {
                    layout,
                    visual,
                    clip: None,
                },
            );
            let offset = point(
                px(f64_to_f32(sample.offset_x)),
                px(f64_to_f32(sample.offset_y)),
            );
            if offset == Point::default() {
                child.prepaint(window, cx);
            } else {
                window.with_element_offset(offset, |window| child.prepaint(window, cx));
            }
            // After the content, so the node's own children do not cover it.
            if self.hit_area {
                let hitbox = window.insert_hitbox(
                    Bounds::new(
                        point(px(f64_to_f32(visual.x)), px(f64_to_f32(visual.y))),
                        gpui::size(px(f64_to_f32(visual.width)), px(f64_to_f32(visual.height))),
                    ),
                    gpui::HitboxBehavior::Normal,
                );
                self.registry.record_hitbox(self.node, hitbox.id);
            }
            return;
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

/// A node with a `hover` style names a GPUI group; a `group_hover` style below it paints
/// while that group is hovered.
fn apply_hover_group<C: ColorResolver>(
    mut element: Stateful<Div>,
    node: &UiNode,
    retained_id: Option<NodeId>,
    environment: &RenderEnvironment<'_, C>,
    disabled: bool,
) -> Stateful<Div> {
    if let Some(id) = retained_id.filter(|_| node.style().hover.is_some()) {
        element = element.group(hover_group_name(id));
    }
    if !disabled
        && let (Some(group), Some(style)) = (environment.hover_group, &node.style().group_hover)
    {
        let paint = pseudo_paint(Some(style), &environment.resolver());
        element = element.group_hover(hover_group_name(group), move |style| {
            apply_pseudo_paint(style, paint)
        });
    }
    element
}

fn hover_group_name(id: NodeId) -> SharedString {
    SharedString::from(format!("gpui-rhai-hover-{}", id.get()))
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

/// A primitive under a disabled ancestor receives `disabled: true` when its
/// descriptor declares a `disabled` prop (present after defaulting).
fn inherit_primitive_disabled(
    primitive: &crate::PrimitiveNode,
    inherited_disabled: bool,
) -> crate::PrimitiveNode {
    let mut primitive = primitive.clone();
    if inherited_disabled && primitive.props.get("disabled").is_some() {
        primitive.props = primitive
            .props
            .with("disabled", crate::PrimitiveValue::Data(UiValue::Bool(true)));
    }
    primitive
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
        &mut style.inset_start,
        &mut style.inset_end,
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
        &mut style.radii.start,
        &mut style.radii.end,
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
    apply_style_override_in(
        element,
        style,
        &InteractionState::default(),
        colors,
        direction,
    )
}

/// Apply a primitive part style resolved for an explicit interaction state,
/// such as a focused slider thumb.
pub(crate) fn apply_style_override_in(
    element: Div,
    style: &Style,
    state: &InteractionState,
    colors: &impl ColorResolver,
    direction: TextDirection,
) -> Div {
    apply_style(element, &style.resolve(state), colors, direction)
}

/// Center a part on an anchor point: a zero-size flex box whose overflowing
/// child stays centered, so the part may have any size.
pub(crate) fn centered_on_anchor(anchor: Div, child: impl IntoElement) -> Div {
    anchor
        .w(px(0.0))
        .h(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .child(child)
}

/// The interaction state of a focusable primitive part.
pub(crate) fn part_interaction(focused: bool, disabled: bool) -> InteractionState {
    let mut state = InteractionState::default();
    if focused {
        state = state
            .with(PseudoState::Focused)
            .with(PseudoState::GroupFocused);
    }
    if disabled {
        state = state.with(PseudoState::Disabled);
    }
    state
}

fn apply_layout(element: Div, style: &StyleProperties, text_direction: TextDirection) -> Div {
    let element = apply_display_and_position(element, style, text_direction);
    let element = apply_flex_alignment(element, style, text_direction);
    apply_layout_dimensions(element, style)
}

fn apply_display_and_position(
    mut element: Div,
    style: &StyleProperties,
    direction: TextDirection,
) -> Div {
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
    let (left, right) = logical_horizontal_edges(
        style.left,
        style.right,
        style.inset_start,
        style.inset_end,
        direction,
    );
    if let Some(value) = right {
        element = inset_right(element, value);
    }
    if let Some(value) = style.bottom {
        element = inset_bottom(element, value);
    }
    if let Some(value) = left {
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
            // Start and end follow the flex direction, which RTL rows reverse.
            // GPUI's `justify_start`/`justify_end` are the writing-mode `start`
            // and `end`, which stay physical left/right under `row-reverse`.
            Justify::Start => {
                element.style().justify_content = Some(gpui::AlignContent::FlexStart);
                element
            }
            Justify::Center => element.justify_center(),
            Justify::End => {
                element.style().justify_content = Some(gpui::AlignContent::FlexEnd);
                element
            }
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
    if let Some(weight) = style.flex_grow_weight {
        element.style().flex_grow = Some(to_f32(weight));
    } else if style.flex_grow == Some(true) {
        element = element.flex_grow_1();
    }
    if let Some(shrink) = style.flex_shrink {
        element = if shrink {
            element.flex_shrink_1()
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

/// Physical corner radii, with the logical start and end corners mirrored in RTL.
fn apply_corner_radii(
    mut element: Div,
    radii: &crate::CornerLengths,
    direction: TextDirection,
) -> Div {
    let (top_left, top_right) = logical_horizontal_edges(
        radii.top_left,
        radii.top_right,
        radii.start,
        radii.end,
        direction,
    );
    let (bottom_left, bottom_right) = logical_horizontal_edges(
        radii.bottom_left,
        radii.bottom_right,
        radii.start,
        radii.end,
        direction,
    );
    if let Some(value) = top_left {
        element = radius_top_left(element, value);
    }
    if let Some(value) = top_right {
        element = radius_top_right(element, value);
    }
    if let Some(value) = bottom_right {
        element = radius_bottom_right(element, value);
    }
    if let Some(value) = bottom_left {
        element = radius_bottom_left(element, value);
    }
    element
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
    element = apply_corner_radii(element, &style.radii, direction);
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
        element.text_style().font_fallbacks = Some(FontFallbacks::from_fonts(fallbacks.clone()));
    }
    if let Some(features) = &style.font_features {
        element.text_style().font_features = Some(FontFeatures(Arc::new(
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
                        inset: false,
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
                Length::Token(_) => element,
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
                LayoutLength::Definite(Length::Token(_)) => element,
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
                Length::Relative(_) | Length::Token(_) => element,
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
        Length::Relative(_) | Length::Token(_) => element,
    }
}

fn line_height(element: Div, value: Length) -> Div {
    match value {
        Length::Pixels(value) => element.line_height(px(to_f32(value))),
        Length::Rems(value) => element.line_height(rems(to_f32(value))),
        Length::Relative(_) | Length::Token(_) => element,
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
    semantics: crate::CommittedSemanticFrame,
    primitives: PrimitiveRegistry,
}

#[derive(Debug, thiserror::Error)]
pub enum StaticUiViewError {
    #[error(transparent)]
    Reconcile(#[from] crate::ReconcileError),
    #[error(transparent)]
    Accessibility(#[from] crate::AccessibilityError),
}

impl StaticUiView {
    /// Create a retained view from one accepted root snapshot.
    ///
    /// # Errors
    ///
    /// Returns structural reconciliation errors such as duplicate sibling keys.
    pub fn new(root: UiNode) -> Result<Self, StaticUiViewError> {
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
    ) -> Result<Self, StaticUiViewError> {
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(root)?;
        let semantics = crate::CommittedSemanticFrame::from_retained(&tree)?;
        Ok(Self {
            tree,
            semantics,
            primitives,
        })
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
    ) -> Result<crate::ReconcileReport, StaticUiViewError> {
        let mut tree = self.tree.clone();
        let report = tree.reconcile(root)?;
        let semantics = crate::CommittedSemanticFrame::from_retained(&tree)?;
        self.tree = tree;
        self.semantics = semantics;
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
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
                GpuiNodeRenderer::render_retained_with_committed_semantics(
                    &self.tree,
                    &self.semantics,
                    window.is_a11y_active(),
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
        fn resolve_token(&self, _token: &str) -> Option<Rgba8> {
            None
        }

        fn resolve_typography_in(
            &self,
            role: &str,
            _environment: &crate::Environment,
        ) -> Option<crate::ResolvedTypography> {
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
    fn owned_color_snapshot_matches_the_native_primitive_theme_surface() {
        let engine = crate::RuntimeEngine::new();
        let mut theme = crate::load_theme_with_layers(
            engine.engine(),
            Some(
                &crate::load_token_base(
                    engine.engine(),
                    "tokens.rhai",
                    include_str!("../../../registry/tokens.rhai"),
                )
                .unwrap(),
            ),
            "default_light.rhai",
            include_str!("../../../registry/themes/default_light.rhai"),
            &crate::ThemeTokenOverrides::default(),
        )
        .unwrap();
        let custom = Rgba8::from_rgba_hex(0x55aa_ccff);
        std::sync::Arc::make_mut(&mut theme.tokens)
            .namespaces
            .insert(
                "brand".to_owned(),
                BTreeMap::from([("tint".to_owned(), crate::ThemeTokenValue::Color(custom))]),
            );
        let snapshot = OwnedColorResolver::capture(&theme);
        assert_eq!(
            snapshot.resolve(&ColorValue::Token("selection".to_owned())),
            theme.tokens.color("selection")
        );
        assert_eq!(
            snapshot.resolve(&ColorValue::Token("table.selection".to_owned())),
            theme.tokens.color("table.selection")
        );
        assert_eq!(
            snapshot.resolve_length(Length::theme_spacing("xxs").unwrap()),
            Some(Length::Pixels(2.0))
        );
        assert_eq!(
            snapshot.resolve(&ColorValue::Token("brand.tint".to_owned())),
            Some(custom)
        );
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
    fn styled_inline_svg_fills_its_node_box_without_losing_intrinsic_default() {
        let bytes = b"<svg width='24' height='24' viewBox='0 0 24 24'></svg>".to_vec();
        let image = crate::asset::svg_image(&bytes, None).unwrap();
        let mut sized = inline_svg_image(Arc::clone(&image), true);
        assert_eq!(sized.style().size.width, Some(relative(1.0).into()));
        assert_eq!(sized.style().size.height, Some(relative(1.0).into()));

        let mut intrinsic = inline_svg_image(image, false);
        assert_eq!(intrinsic.style().size.width, None);
        assert_eq!(intrinsic.style().size.height, None);
    }

    #[test]
    fn ambient_text_color_inherits_and_a_local_value_wins() {
        let inherited = Rgba8::from_rgb_hex(0x0012_34ab);
        assert_eq!(
            resolve_ambient_text_color(&Style::new().base, &LiteralColorResolver, Some(inherited)),
            Some(inherited)
        );

        let local = Rgba8::from_rgb_hex(0x00ab_cd12);
        assert_eq!(
            resolve_ambient_text_color(
                &Style::new().text_color(ColorValue::Literal(local)).base,
                &LiteralColorResolver,
                Some(inherited)
            ),
            Some(local)
        );
    }

    #[test]
    fn asset_svg_uses_an_ancestor_text_color() {
        let assets = AssetRegistry::new();
        assets
            .register(
                "app",
                crate::InMemoryAssetProvider::new(BTreeMap::from([(
                    "icon".to_owned(),
                    crate::AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="currentColor"/></svg>"#.to_vec(),
                    },
                )])),
            )
            .unwrap();
        let handle = assets
            .load_image(&crate::AssetId::parse("app/icon").unwrap())
            .unwrap();
        let color = Rgba8::from_rgb_hex(0x0012_34ab);
        let root = UiNode::row(vec![UiNode::image(handle.opaque().clone())])
            .with_style(&Style::new().text_color(ColorValue::Literal(color)));
        let dispatcher = NodeEventDispatcher::new(|_, _, _, _, _| EventPropagation::Handled);

        let _element = GpuiNodeRenderer::render_with_runtime(
            &root,
            &LiteralColorResolver,
            &InteractionState::default(),
            &PrimitiveRegistry::new(),
            &assets,
            &dispatcher,
        );

        assert!(matches!(
            assets
                .image_source_tinted(handle.opaque(), Some(color))
                .unwrap(),
            gpui::ImageSource::Custom(_)
        ));
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
    fn sampled_motion_values_override_dimensions_and_transform_without_rhai() {
        let values = BTreeMap::from([
            (
                MotionKey::for_node("root/card", MotionProperty::Width),
                180.0,
            ),
            (
                MotionKey::for_node("root/card", MotionProperty::ClipHeight),
                64.0,
            ),
            (
                MotionKey::for_node("root/card", MotionProperty::TranslateX),
                12.0,
            ),
        ]);
        let sampled = node_motion(&values, "root/card");
        let mut style = StyleProperties::default();
        apply_motion_dimensions(&mut style, sampled);
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
    fn canvas_signal_transform_is_shared_by_paint_and_hit_testing() {
        let component = crate::ComponentInstancePath::root("PanZoom", "viewport");
        let scale_ids = (
            crate::SignalId::new(component.clone(), "scale-x", crate::SignalKind::Float).unwrap(),
            crate::SignalId::new(component.clone(), "scale-y", crate::SignalKind::Float).unwrap(),
        );
        let rotate_id =
            crate::SignalId::new(component.clone(), "rotate", crate::SignalKind::Float).unwrap();
        let scale_x = crate::NativeSignal::new(scale_ids.0.clone());
        let scale_y = crate::NativeSignal::new(scale_ids.1.clone());
        let rotate = crate::NativeSignal::new(rotate_id.clone());
        let scene = crate::CanvasScene::new(vec![crate::CanvasCommand::Rect {
            key: "target".to_owned(),
            x: 20.0,
            y: 25.0,
            width: 20.0,
            height: 20.0,
            fill: ColorValue::Token("accent".to_owned()),
        }])
        .unwrap();
        let node = UiNode::canvas(scene.clone())
            .with_signal_binding(crate::SignalProperty::ScaleX, scale_x)
            .unwrap()
            .with_signal_binding(crate::SignalProperty::ScaleY, scale_y)
            .unwrap()
            .with_signal_binding(crate::SignalProperty::Rotate, rotate)
            .unwrap();
        let mut registry = crate::SignalRegistry::new();
        registry.reconcile(
            &component,
            BTreeMap::from([
                (
                    scale_ids.0,
                    crate::signal::SignalDescriptor::new(crate::SignalValue::Float(1.6)),
                ),
                (
                    scale_ids.1,
                    crate::signal::SignalDescriptor::new(crate::SignalValue::Float(0.8)),
                ),
                (
                    rotate_id,
                    crate::signal::SignalDescriptor::new(crate::SignalValue::Float(19.0)),
                ),
            ]),
        );
        let motion = apply_canvas_signal_transform(
            NodeMotionValues::default(),
            &node_signals(&registry, &node),
        );
        let mut tree = RetainedUiTree::new();
        tree.reconcile(node).unwrap();
        let retained = tree.root_id().unwrap();
        let geometry = crate::GeometryRegistry::new();
        let bounds = crate::GeometryBounds::new(100.0, 50.0, 120.0, 90.0).unwrap();
        geometry.update(
            retained,
            crate::ElementGeometry {
                layout: bounds,
                visual: bounds,
                clip: None,
            },
        );
        geometry.update_canvas_transform(retained, motion.canvas_transform());
        let paint_bounds = Bounds::new(point(px(100.0), px(50.0)), gpui::size(px(120.0), px(90.0)));
        let painted = node_canvas_point(paint_bounds, 30.0, 35.0, motion);
        let context = PointerPayloadContext::retained(retained, geometry, Some(scene), Vec::new());
        let payload = context.enrich(pointer_payload(
            painted,
            Some(MouseButton::Left),
            vec![MouseButton::Left],
            Modifiers::default(),
            1,
            false,
        ));
        let UiValue::Map(payload) = payload else {
            unreachable!()
        };
        assert_eq!(payload["canvas_key"], UiValue::String("target".to_owned()));
    }

    #[test]
    fn optional_width_override_activates_fixed_layout_only_when_present() {
        let mut base = StyleProperties {
            flex_basis: Some(Length::Relative(0.0).into()),
            flex_grow_weight: Some(2.0),
            ..StyleProperties::default()
        };
        apply_signal_style(&mut base, &NodeSignalValues::default());
        assert_eq!(base.flex_grow_weight, Some(2.0));
        let mut overridden = base;
        apply_signal_style(
            &mut overridden,
            &NodeSignalValues {
                width_override: Some(180.0),
                ..NodeSignalValues::default()
            },
        );
        assert_eq!(overridden.width, Some(Length::Pixels(180.0).into()));
        assert_eq!(overridden.flex_basis, None);
        assert_eq!(overridden.flex_grow, Some(false));
        assert_eq!(overridden.flex_grow_weight, None);
        assert_eq!(overridden.flex_shrink, Some(false));
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

        let mut weighted = apply_layout_dimensions(
            div(),
            &StyleProperties {
                flex_grow_weight: Some(2.5),
                flex_basis: Some(Length::Relative(0.0).into()),
                ..StyleProperties::default()
            },
        );
        assert_eq!(weighted.style().flex_grow, Some(2.5));
        assert_eq!(weighted.style().flex_basis, Some(relative(0.0).into()));
    }

    #[test]
    fn committed_semantics_write_native_role_label_state_and_range() {
        let source = UiNode::text("ignored")
            .with_attribute("role", UiValue::String("slider".to_owned()))
            .with_attribute("label", UiValue::String("Volume".to_owned()))
            .with_attribute("value", UiValue::Float(25.0))
            .with_attribute("value_min", UiValue::Float(0.0))
            .with_attribute("value_max", UiValue::Float(100.0))
            .with_attribute("orientation", UiValue::String("horizontal".to_owned()));
        let mut retained = RetainedUiTree::new();
        retained.reconcile(source).unwrap();
        let frame = crate::CommittedSemanticFrame::from_retained(&retained).unwrap();
        let semantic = frame.node(retained.root_id().unwrap()).unwrap();
        let element = apply_native_semantics(div().id("volume"), Some(semantic));
        assert_eq!(element.a11y_role(), Some(gpui::Role::Slider));
        let mut node = gpui::accesskit::Node::new(gpui::Role::Slider);
        element.write_a11y_info(&mut node);
        assert_eq!(node.label(), Some("Volume"));
        assert_eq!(node.value(), Some("25"));
        assert_eq!(node.numeric_value(), Some(25.0));
        assert_eq!(node.min_numeric_value(), Some(0.0));
        assert_eq!(node.max_numeric_value(), Some(100.0));
        assert_eq!(node.orientation(), Some(gpui::Orientation::Horizontal));
    }

    #[test]
    fn plain_text_semantics_use_a_valued_native_label() {
        let mut retained = RetainedUiTree::new();
        retained.reconcile(UiNode::text("Ready")).unwrap();
        let frame = crate::CommittedSemanticFrame::from_retained(&retained).unwrap();
        let semantic = frame.node(retained.root_id().unwrap()).unwrap();
        let element = apply_native_semantics(div().id("status-text"), Some(semantic));

        assert_eq!(semantic.role, "text");
        assert_eq!(element.a11y_role(), Some(gpui::Role::Label));
        let mut node = gpui::accesskit::Node::new(gpui::Role::Label);
        element.write_a11y_info(&mut node);
        assert_eq!(node.label(), None);
        assert_eq!(node.value(), Some("Ready"));
        assert!(node.character_lengths().is_empty());
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
    fn logical_corners_round_the_inline_start_and_mirror_in_rtl() {
        let style = Style::new()
            .radius(Length::Pixels(0.0))
            .radius_start(Length::Pixels(6.0));
        let corners = |direction| {
            let mut element = apply_paint(div(), &style.base, &TypographyResolver, direction);
            let radii = element.style().corner_radii.clone();
            [
                radii.top_left,
                radii.top_right,
                radii.bottom_right,
                radii.bottom_left,
            ]
            .map(|corner| corner.map(|value| format!("{value:?}")))
        };
        let six = Some(format!("{:?}", gpui::AbsoluteLength::Pixels(px(6.0))));
        let zero = Some(format!("{:?}", gpui::AbsoluteLength::Pixels(px(0.0))));
        assert_eq!(
            corners(TextDirection::LeftToRight),
            [six.clone(), zero.clone(), zero.clone(), six.clone()]
        );
        assert_eq!(
            corners(TextDirection::RightToLeft),
            [zero.clone(), six.clone(), six, zero]
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
            owner: None,
            parent: None,
            kind: crate::OverlayKind::Sheet,
            placement,
            align: crate::OverlayAlign::Center,
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
            width_policy: crate::OverlayWidthPolicy::Content,
        };
        assert_eq!(
            scoped_overlay_spec(
                &spec(crate::OverlayPlacement::Start),
                "view",
                TextDirection::RightToLeft,
                None,
            )
            .placement,
            crate::OverlayPlacement::Right
        );
        assert_eq!(
            scoped_overlay_spec(
                &spec(crate::OverlayPlacement::End),
                "view",
                TextDirection::RightToLeft,
                None,
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
        assert!(node_has_focus_declaration(&node));
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

    #[test]
    fn canvas_motion_uses_one_affine_transform_for_paint_and_hit_testing() {
        let scene = crate::CanvasScene::new(vec![crate::CanvasCommand::Rect {
            key: "target".to_owned(),
            x: 20.0,
            y: 25.0,
            width: 20.0,
            height: 20.0,
            fill: ColorValue::Token("accent".to_owned()),
        }])
        .unwrap();
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(UiNode::canvas(scene.clone()).with_key("canvas"))
            .unwrap();
        let node = tree.root_id().unwrap();
        let geometry = crate::GeometryRegistry::new();
        let bounds = crate::GeometryBounds::new(100.0, 50.0, 120.0, 90.0).unwrap();
        geometry.update(
            node,
            crate::ElementGeometry {
                layout: bounds,
                visual: bounds,
                clip: None,
            },
        );
        let transform = crate::geometry::CanvasMotionTransform {
            rotate: 31.0,
            scale_x: 1.4,
            scale_y: 0.7,
            skew_x: 12.0,
            skew_y: -8.0,
            path_progress: None,
        };
        geometry.update_canvas_transform(node, transform);
        let motion = NodeMotionValues {
            rotate: Some(transform.rotate),
            scale_x: Some(transform.scale_x),
            scale_y: Some(transform.scale_y),
            skew_x: Some(transform.skew_x),
            skew_y: Some(transform.skew_y),
            ..NodeMotionValues::default()
        };
        let paint_bounds = Bounds::new(point(px(100.0), px(50.0)), gpui::size(px(120.0), px(90.0)));
        let painted = node_canvas_point(paint_bounds, 30.0, 35.0, motion);
        let context = PointerPayloadContext::retained(node, geometry, Some(scene), Vec::new());
        let payload = context.enrich(pointer_payload(
            painted,
            Some(MouseButton::Left),
            vec![MouseButton::Left],
            Modifiers::default(),
            1,
            false,
        ));
        let UiValue::Map(payload) = payload else {
            unreachable!()
        };
        assert_eq!(payload["canvas_key"], UiValue::String("target".to_owned()));
    }
}
