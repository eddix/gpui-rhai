//! Native hot-lane edge and corner handles for the public `Resizable` component.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, CursorStyle, DispatchPhase, Element, ElementId, FocusHandle,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, InteractiveElement, IntoElement,
    KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, ParentElement, Pixels, Style, Styled,
    Window, div, fill, point, px, relative, rgba, size,
};

use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveContext, PrimitiveDescriptor,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId, PrimitiveProps,
    PrimitiveTheme, PrimitiveValue, Rgba8, SignalKind, SignalValue, UiValue, ValueSchema,
};

const MAX_RESIZE_DIMENSION: f64 = 16_384.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResizeRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl ResizeRect {
    fn right(self) -> f64 {
        self.x + self.width
    }

    fn bottom(self) -> f64 {
        self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResizeHandle {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

impl ResizeHandle {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "n" => Some(Self::North),
            "s" => Some(Self::South),
            "e" => Some(Self::East),
            "w" => Some(Self::West),
            "ne" => Some(Self::NorthEast),
            "nw" => Some(Self::NorthWest),
            "se" => Some(Self::SouthEast),
            "sw" => Some(Self::SouthWest),
            _ => None,
        }
    }

    const fn moves_west(self) -> bool {
        matches!(self, Self::West | Self::NorthWest | Self::SouthWest)
    }

    const fn moves_east(self) -> bool {
        matches!(self, Self::East | Self::NorthEast | Self::SouthEast)
    }

    const fn moves_north(self) -> bool {
        matches!(self, Self::North | Self::NorthEast | Self::NorthWest)
    }

    const fn moves_south(self) -> bool {
        matches!(self, Self::South | Self::SouthEast | Self::SouthWest)
    }

    const fn moves_horizontal(self) -> bool {
        self.moves_west() || self.moves_east()
    }

    const fn moves_vertical(self) -> bool {
        self.moves_north() || self.moves_south()
    }

    const fn cursor(self) -> CursorStyle {
        match self {
            Self::North | Self::South => CursorStyle::ResizeUpDown,
            Self::East | Self::West => CursorStyle::ResizeLeftRight,
            Self::NorthEast | Self::SouthWest => CursorStyle::ResizeUpRightDownLeft,
            Self::NorthWest | Self::SouthEast => CursorStyle::ResizeUpLeftDownRight,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResizeConstraints {
    min_width: f64,
    min_height: f64,
    max_width: f64,
    max_height: f64,
    aspect_ratio: Option<f64>,
    contain: bool,
}

#[derive(Clone)]
struct ResizableConfig {
    id: String,
    handle: ResizeHandle,
    source: ResizeRect,
    constraints: ResizeConstraints,
    boundary_ref: crate::ElementRef,
    boundary_size: Rc<Cell<Option<(f64, f64)>>>,
    x_signal: crate::NativeSignal,
    y_signal: crate::NativeSignal,
    width_signal: crate::NativeSignal,
    height_signal: crate::NativeSignal,
    keyboard_step: f64,
    disabled: bool,
    idle_color: Rgba8,
    active_color: Rgba8,
    focus: Option<FocusHandle>,
    /// Paint the native edge line or corner mark; off when a grip draws its own.
    line: bool,
    /// How far an edge line stops short of each end of the handle.
    line_inset: f64,
    /// A string signal that receives the handle's state for a grip's `signal_style`.
    state_signal: Option<crate::NativeSignal>,
    /// A grip node whose bounds also start a drag.
    handle_ref: Option<crate::ElementRef>,
}

#[derive(Clone)]
struct ResizableState(Rc<RefCell<ResizableStateInner>>);

#[derive(Clone)]
struct ResizableStateInner {
    source: ResizeRect,
    constraints: ResizeConstraints,
    signals: [crate::SignalId; 4],
    written_state: Option<&'static str>,
}

impl ResizableState {
    fn new(config: &ResizableConfig) -> Self {
        Self(Rc::new(RefCell::new(ResizableStateInner {
            source: config.source,
            constraints: config.constraints,
            signals: signal_ids(config),
            written_state: None,
        })))
    }
}

fn signal_ids(config: &ResizableConfig) -> [crate::SignalId; 4] {
    [
        config.x_signal.id().clone(),
        config.y_signal.id().clone(),
        config.width_signal.id().clone(),
        config.height_signal.id().clone(),
    ]
}

struct ResizablePrepaint {
    hitbox: Hitbox,
    state: ResizableState,
}

struct ResizableHandleElement {
    config: ResizableConfig,
    events: PrimitiveContext,
}

impl IntoElement for ResizableHandleElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ResizableHandleElement {
    type RequestLayoutState = ();
    type PrepaintState = ResizablePrepaint;

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name(self.config.id.clone().into()))
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (
            window.request_layout(
                Style {
                    size: size(relative(1.0).into(), relative(1.0).into()),
                    ..Style::default()
                },
                None,
                cx,
            ),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let state = window
            .use_state(cx, |_, _| ResizableState::new(&self.config))
            .read(cx)
            .clone();
        let reset = {
            let mut inner = state.0.borrow_mut();
            let signals = signal_ids(&self.config);
            let changed = inner.source != self.config.source
                || inner.constraints != self.config.constraints
                || inner.signals != signals;
            if changed {
                inner.source = self.config.source;
                inner.constraints = self.config.constraints;
                inner.signals = signals;
            }
            changed
        };
        let owner = self.events.interaction_owner(&self.config.id);
        self.config.boundary_size.set(
            self.events
                .element_bounds(&self.config.boundary_ref, cx)
                .map(|bounds| (bounds.width, bounds.height)),
        );
        if reset && !self.events.cancel_interaction(&owner, window, cx) {
            clear_preview(&self.events, &self.config, window, cx);
        }
        ResizablePrepaint {
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::BlockMouseExceptScroll),
            state,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let owner = self.events.interaction_owner(&self.config.id);
        self.events.present_interaction(owner.clone());
        let dragging = self.events.interaction_is_active(&owner);
        let grip = self
            .config
            .handle_ref
            .as_ref()
            .and_then(|reference| self.events.element_hitbox(reference, cx));
        let hovered = !self.config.disabled
            && crate::handle_state::over_handle(&prepaint.hitbox, grip, window);
        if self.config.line {
            let color = if dragging || hovered {
                self.config.active_color
            } else {
                self.config.idle_color
            };
            paint_handle(bounds, &self.config, color, dragging, window);
        }
        if let Some(signal) = &self.config.state_signal {
            let focused = self
                .config
                .focus
                .as_ref()
                .is_some_and(|focus| focus.is_focused(window));
            let state =
                crate::handle_state::handle_state(self.config.disabled, dragging, hovered, focused);
            let mut inner = prepaint.state.0.borrow_mut();
            crate::handle_state::publish_state(
                &self.events,
                signal,
                &mut inner.written_state,
                state,
                window,
                cx,
            );
        }
        if !self.config.disabled {
            crate::handle_state::track_hover(
                &prepaint.hitbox,
                self.config.handle_ref.as_ref(),
                &self.events,
                hovered,
                window,
            );
        }
        if !self.config.disabled {
            window.set_cursor_style(self.config.handle.cursor(), &prepaint.hitbox);
        }
        register_pointer_listeners(prepaint, &self.config, &self.events, window);
    }
}

fn paint_handle(
    bounds: Bounds<Pixels>,
    config: &ResizableConfig,
    color: Rgba8,
    dragging: bool,
    window: &mut Window,
) {
    let handle = config.handle;
    let thickness = if dragging { px(2.0) } else { px(1.0) };
    #[allow(clippy::cast_possible_truncation)]
    let inset = px(config.line_inset as f32);
    let visual = if handle.moves_horizontal() && handle.moves_vertical() {
        let side = if dragging { px(7.0) } else { px(5.0) };
        Bounds::new(
            point(
                bounds.origin.x + (bounds.size.width - side) / 2.0,
                bounds.origin.y + (bounds.size.height - side) / 2.0,
            ),
            size(side, side),
        )
    } else if handle.moves_horizontal() {
        Bounds::new(
            point(
                bounds.origin.x + (bounds.size.width - thickness) / 2.0,
                bounds.origin.y + inset,
            ),
            size(thickness, (bounds.size.height - inset * 2.0).max(px(1.0))),
        )
    } else {
        Bounds::new(
            point(
                bounds.origin.x + inset,
                bounds.origin.y + (bounds.size.height - thickness) / 2.0,
            ),
            size((bounds.size.width - inset * 2.0).max(px(1.0)), thickness),
        )
    };
    window.paint_quad(fill(visual, rgba(color.as_rgba_hex())));
}

#[allow(clippy::too_many_lines)]
fn register_pointer_listeners(
    prepaint: &ResizablePrepaint,
    config: &ResizableConfig,
    events: &PrimitiveContext,
    window: &mut Window,
) {
    let view = window.current_view();
    let hitbox = prepaint.hitbox.clone();
    let down_config = config.clone();
    let down_events = events.clone();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble
            || event.button != MouseButton::Left
            || down_config.disabled
        {
            return;
        }
        let grip = down_config
            .handle_ref
            .as_ref()
            .and_then(|reference| down_events.element_hitbox(reference, cx));
        if !crate::handle_state::over_handle(&hitbox, grip, window) {
            return;
        }
        let Some(boundary) = down_events.element_bounds(&down_config.boundary_ref, cx) else {
            return;
        };
        if down_config.constraints.contain
            && !rect_within_boundary(down_config.source, (boundary.width, boundary.height))
        {
            return;
        }
        if let Some(focus) = down_config.focus.as_ref() {
            focus.focus(window, cx);
        }
        let boundary_size = (boundary.width, boundary.height);
        let update_config = down_config.clone();
        let update_events = down_events.clone();
        let update =
            move |gesture: crate::interaction::GestureUpdate, _: &mut Window, cx: &mut App| {
                let Some(boundary) = update_events.element_bounds(&update_config.boundary_ref, cx)
                else {
                    return crate::interaction::InteractionFlow::Cancel;
                };
                if (boundary.width - boundary_size.0).abs() > 0.5
                    || (boundary.height - boundary_size.1).abs() > 0.5
                {
                    return crate::interaction::InteractionFlow::Cancel;
                }
                if gesture.moved() {
                    let (dx, dy) = gesture.delta();
                    let rect = resize_rect(
                        update_config.source,
                        update_config.handle,
                        dx,
                        dy,
                        update_config.constraints,
                        boundary_size,
                    );
                    write_preview(&update_events, &update_config, Some(rect), cx);
                }
                crate::interaction::InteractionFlow::Continue
            };
        let finish_config = down_config.clone();
        let finish_events = down_events.clone();
        let finish =
            move |gesture: crate::interaction::GestureUpdate, window: &mut Window, cx: &mut App| {
                clear_preview(&finish_events, &finish_config, window, cx);
                if !gesture.moved() {
                    return;
                }
                let boundary_unchanged = finish_events
                    .element_bounds(&finish_config.boundary_ref, cx)
                    .is_some_and(|boundary| {
                        (boundary.width - boundary_size.0).abs() <= 0.5
                            && (boundary.height - boundary_size.1).abs() <= 0.5
                    });
                if !boundary_unchanged {
                    return;
                }
                let (dx, dy) = gesture.delta();
                let rect = resize_rect(
                    finish_config.source,
                    finish_config.handle,
                    dx,
                    dy,
                    finish_config.constraints,
                    boundary_size,
                );
                finish_events.propose("resize", resize_payload(rect), window, cx);
            };
        let cancel_config = down_config.clone();
        let cancel_events = down_events.clone();
        let cancel = move |window: &mut Window, cx: &mut App| {
            clear_preview(&cancel_events, &cancel_config, window, cx);
        };
        let owner = down_events.interaction_owner(&down_config.id);
        down_events.begin_interaction(
            crate::interaction::NativeGesture::new(
                owner,
                event.position,
                view,
                update,
                finish,
                cancel,
            ),
            window,
            cx,
        );
        cx.stop_propagation();
    });
}

fn clear_preview(
    events: &PrimitiveContext,
    config: &ResizableConfig,
    window: &mut Window,
    cx: &mut App,
) {
    let events = events.clone();
    let config = config.clone();
    window.defer(cx, move |_, cx| write_preview(&events, &config, None, cx));
}

fn write_preview(
    events: &PrimitiveContext,
    config: &ResizableConfig,
    rect: Option<ResizeRect>,
    cx: &mut App,
) {
    let x = rect.map(|rect| rect.x - config.source.x);
    let y = rect.map(|rect| rect.y - config.source.y);
    let width = rect.map(|rect| rect.width);
    let height = rect.map(|rect| rect.height);
    let _ = events.write_signals(
        [
            (config.x_signal.clone(), SignalValue::OptionalFloat(x)),
            (config.y_signal.clone(), SignalValue::OptionalFloat(y)),
            (
                config.width_signal.clone(),
                SignalValue::OptionalFloat(width),
            ),
            (
                config.height_signal.clone(),
                SignalValue::OptionalFloat(height),
            ),
        ],
        cx,
    );
}

fn keyboard_delta(handle: ResizeHandle, key: &str, step: f64) -> Option<(f64, f64)> {
    match key {
        "left" if handle.moves_horizontal() => Some((-step, 0.0)),
        "right" if handle.moves_horizontal() => Some((step, 0.0)),
        "up" if handle.moves_vertical() => Some((0.0, -step)),
        "down" if handle.moves_vertical() => Some((0.0, step)),
        _ => None,
    }
}

fn resize_rect(
    source: ResizeRect,
    handle: ResizeHandle,
    dx: f64,
    dy: f64,
    constraints: ResizeConstraints,
    boundary: (f64, f64),
) -> ResizeRect {
    if let Some(aspect_ratio) = constraints.aspect_ratio {
        resize_rect_aspect(source, handle, dx, dy, constraints, boundary, aspect_ratio)
    } else {
        resize_rect_free(source, handle, dx, dy, constraints, boundary)
    }
}

fn resize_rect_free(
    source: ResizeRect,
    handle: ResizeHandle,
    dx: f64,
    dy: f64,
    constraints: ResizeConstraints,
    boundary: (f64, f64),
) -> ResizeRect {
    let mut rect = source;
    if handle.moves_west() {
        let maximum = horizontal_max(source, handle, constraints, boundary.0);
        rect.width = clamp_dimension(source.width - dx, constraints.min_width, maximum);
        rect.x = source.right() - rect.width;
    } else if handle.moves_east() {
        let maximum = horizontal_max(source, handle, constraints, boundary.0);
        rect.width = clamp_dimension(source.width + dx, constraints.min_width, maximum);
    }
    if handle.moves_north() {
        let maximum = vertical_max(source, handle, constraints, boundary.1);
        rect.height = clamp_dimension(source.height - dy, constraints.min_height, maximum);
        rect.y = source.bottom() - rect.height;
    } else if handle.moves_south() {
        let maximum = vertical_max(source, handle, constraints, boundary.1);
        rect.height = clamp_dimension(source.height + dy, constraints.min_height, maximum);
    }
    rect
}

fn resize_rect_aspect(
    source: ResizeRect,
    handle: ResizeHandle,
    dx: f64,
    dy: f64,
    constraints: ResizeConstraints,
    boundary: (f64, f64),
    aspect_ratio: f64,
) -> ResizeRect {
    let raw_width = if handle.moves_west() {
        source.width - dx
    } else if handle.moves_east() {
        source.width + dx
    } else {
        source.width
    };
    let raw_height = if handle.moves_north() {
        source.height - dy
    } else if handle.moves_south() {
        source.height + dy
    } else {
        source.height
    };
    let desired_width = if handle.moves_horizontal() && handle.moves_vertical() {
        let horizontal_intent = (raw_width - source.width).abs();
        let vertical_intent = (raw_height * aspect_ratio - source.width).abs();
        if horizontal_intent >= vertical_intent {
            raw_width
        } else {
            raw_height * aspect_ratio
        }
    } else if handle.moves_horizontal() {
        raw_width
    } else {
        raw_height * aspect_ratio
    };
    let max_width = horizontal_max(source, handle, constraints, boundary.0)
        .min(vertical_max(source, handle, constraints, boundary.1) * aspect_ratio);
    let min_width = constraints
        .min_width
        .max(constraints.min_height * aspect_ratio)
        .min(max_width);
    let width = desired_width.clamp(min_width, max_width);
    let height = width / aspect_ratio;
    let x = if handle.moves_west() {
        source.right() - width
    } else if handle.moves_east() {
        source.x
    } else {
        source.x + (source.width - width) / 2.0
    };
    let y = if handle.moves_north() {
        source.bottom() - height
    } else if handle.moves_south() {
        source.y
    } else {
        source.y + (source.height - height) / 2.0
    };
    ResizeRect {
        x,
        y,
        width,
        height,
    }
}

fn horizontal_max(
    source: ResizeRect,
    handle: ResizeHandle,
    constraints: ResizeConstraints,
    boundary_width: f64,
) -> f64 {
    if !constraints.contain {
        return constraints.max_width;
    }
    let boundary_max = if handle.moves_west() {
        source.right().max(0.0)
    } else if handle.moves_east() {
        (boundary_width - source.x).max(0.0)
    } else {
        let center = source.x + source.width / 2.0;
        (2.0 * center.min((boundary_width - center).max(0.0))).max(0.0)
    };
    constraints.max_width.min(boundary_max)
}

fn vertical_max(
    source: ResizeRect,
    handle: ResizeHandle,
    constraints: ResizeConstraints,
    boundary_height: f64,
) -> f64 {
    if !constraints.contain {
        return constraints.max_height;
    }
    let boundary_max = if handle.moves_north() {
        source.bottom().max(0.0)
    } else if handle.moves_south() {
        (boundary_height - source.y).max(0.0)
    } else {
        let center = source.y + source.height / 2.0;
        (2.0 * center.min((boundary_height - center).max(0.0))).max(0.0)
    };
    constraints.max_height.min(boundary_max)
}

fn clamp_dimension(value: f64, minimum: f64, maximum: f64) -> f64 {
    value.clamp(minimum.min(maximum), maximum)
}

fn rect_within_boundary(rect: ResizeRect, boundary: (f64, f64)) -> bool {
    rect.x >= 0.0
        && rect.y >= 0.0
        && rect.right() <= boundary.0 + 0.5
        && rect.bottom() <= boundary.1 + 0.5
}

/// The proposal is exactly the next `rect`, so a caller can store it as is.
fn resize_payload(rect: ResizeRect) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("x".to_owned(), UiValue::Float(rect.x)),
        ("y".to_owned(), UiValue::Float(rect.y)),
        ("width".to_owned(), UiValue::Float(rect.width)),
        ("height".to_owned(), UiValue::Float(rect.height)),
    ]))
}

#[derive(Default)]
pub struct ResizablePrimitiveHandler {
    controls: BTreeMap<PrimitiveInstanceId, (ResizableConfig, PrimitiveContext)>,
}

fn keyboard_resize_payload(config: &ResizableConfig, key: &str, shift: bool) -> Option<UiValue> {
    if config.disabled {
        return None;
    }
    let step = if shift {
        config.keyboard_step * 4.0
    } else {
        config.keyboard_step
    };
    let (dx, dy) = keyboard_delta(config.handle, key, step)?;
    let boundary = config.boundary_size.get()?;
    if config.constraints.contain && !rect_within_boundary(config.source, boundary) {
        return None;
    }
    let rect = resize_rect(
        config.source,
        config.handle,
        dx,
        dy,
        config.constraints,
        boundary,
    );
    Some(resize_payload(rect))
}

impl PrimitiveHandler for ResizablePrimitiveHandler {
    fn uses_primary_focus(&self) -> bool {
        true
    }

    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveContext,
        theme: &PrimitiveTheme,
        _: &mut Window,
        _: &mut App,
    ) -> Result<AnyElement, String> {
        let id = instance
            .id
            .clone()
            .ok_or_else(|| "ResizableHandlePrimitive requires a stable key".to_owned())?;
        let config = parse_config(
            &instance.node.props,
            instance.focus_handle().cloned(),
            theme,
        )?;
        let key_config = config.clone();
        let key_events = events.clone();
        self.controls.insert(id, (config.clone(), events.clone()));
        let mut root = div().size_full();
        if let Some(focus) = config.focus.as_ref() {
            root = root.track_focus(&focus.clone().tab_stop(!config.disabled));
        }
        Ok(root
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if let Some(payload) = keyboard_resize_payload(
                    &key_config,
                    event.keystroke.key.as_str(),
                    event.keystroke.modifiers.shift,
                ) {
                    key_events.propose("resize", payload, window, cx);
                    cx.stop_propagation();
                }
            })
            .child(ResizableHandleElement {
                config,
                events: events.clone(),
            })
            .into_any_element())
    }

    fn perform_key(
        &mut self,
        instance: &PrimitiveInstanceId,
        key: &str,
    ) -> Result<Option<crate::primitive::PrimitiveSemanticProposal>, String> {
        let Some((config, events)) = self.controls.get(instance).cloned() else {
            return Ok(None);
        };
        let Some(payload) = keyboard_resize_payload(&config, key, false) else {
            return Ok(None);
        };
        events
            .prepare_proposal("resize", payload)
            .map(Some)
            .map_err(|error| error.to_string())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.controls.remove(instance);
    }
}

fn parse_config(
    props: &PrimitiveProps,
    focus: Option<FocusHandle>,
    theme: &PrimitiveTheme,
) -> Result<ResizableConfig, String> {
    let handle = props
        .string("handle")
        .and_then(ResizeHandle::parse)
        .ok_or_else(|| "resizable handle must be n, s, e, w, ne, nw, se, or sw".to_owned())?;
    let source = ResizeRect {
        x: required_number(props, "x")?,
        y: required_number(props, "y")?,
        width: required_number(props, "width")?,
        height: required_number(props, "height")?,
    };
    let constraints = ResizeConstraints {
        min_width: props.number("min_width").unwrap_or(24.0),
        min_height: props.number("min_height").unwrap_or(24.0),
        max_width: props.number("max_width").unwrap_or(MAX_RESIZE_DIMENSION),
        max_height: props.number("max_height").unwrap_or(MAX_RESIZE_DIMENSION),
        aspect_ratio: optional_number_prop(props, "aspect_ratio")?,
        contain: props.boolean("contain").unwrap_or(true),
    };
    validate_geometry(source, constraints)?;
    let x_signal = required_optional_float_signal(props, "x_signal")?;
    let y_signal = required_optional_float_signal(props, "y_signal")?;
    let width_signal = required_optional_float_signal(props, "width_signal")?;
    let height_signal = required_optional_float_signal(props, "height_signal")?;
    let state_signal = props.signal("state_signal").cloned();
    if state_signal
        .as_ref()
        .is_some_and(|signal| signal.id().kind() != SignalKind::String)
    {
        return Err("resizable state_signal must be a string signal".to_owned());
    }
    let line_inset = props.number("line_inset").unwrap_or(4.0);
    if !line_inset.is_finite() || line_inset < 0.0 {
        return Err("resizable line_inset must be finite and non-negative".to_owned());
    }
    let keyboard_step = props.number("keyboard_step").unwrap_or(8.0);
    if !keyboard_step.is_finite() || !(0.0..=512.0).contains(&keyboard_step) || keyboard_step == 0.0
    {
        return Err("resizable keyboard_step must be finite and in (0, 512]".to_owned());
    }
    Ok(ResizableConfig {
        id: format!(
            "gpui-rhai-resizable:{}:{}:{handle:?}",
            width_signal.id().component(),
            width_signal.id().key()
        ),
        handle,
        source,
        constraints,
        boundary_ref: props
            .element_ref("boundary_ref")
            .cloned()
            .ok_or_else(|| "resizable requires boundary_ref".to_owned())?,
        boundary_size: Rc::new(Cell::new(None)),
        x_signal,
        y_signal,
        width_signal,
        height_signal,
        keyboard_step,
        disabled: props.boolean("disabled").unwrap_or(false),
        idle_color: theme
            .color("border")
            .unwrap_or(Rgba8::from_rgba_hex(0x5555_55ff)),
        active_color: theme
            .color("accent")
            .unwrap_or(Rgba8::from_rgba_hex(0x3b82_f6ff)),
        focus,
        line: props.boolean("line").unwrap_or(true),
        line_inset,
        state_signal,
        handle_ref: props.element_ref("handle_ref").cloned(),
    })
}

fn validate_geometry(source: ResizeRect, constraints: ResizeConstraints) -> Result<(), String> {
    let values = [
        source.x,
        source.y,
        source.width,
        source.height,
        constraints.min_width,
        constraints.min_height,
        constraints.max_width,
        constraints.max_height,
    ];
    if values.iter().any(|value| !value.is_finite())
        || source.width <= 0.0
        || source.height <= 0.0
        || constraints.min_width <= 0.0
        || constraints.min_height <= 0.0
        || constraints.max_width < constraints.min_width
        || constraints.max_height < constraints.min_height
        || constraints.max_width > MAX_RESIZE_DIMENSION
        || constraints.max_height > MAX_RESIZE_DIMENSION
        || constraints
            .aspect_ratio
            .is_some_and(|ratio| !ratio.is_finite() || ratio <= 0.0)
    {
        return Err("resizable rectangle and constraints are invalid".to_owned());
    }
    Ok(())
}

fn required_number(props: &PrimitiveProps, name: &str) -> Result<f64, String> {
    props
        .number(name)
        .ok_or_else(|| format!("resizable requires numeric {name}"))
}

fn optional_number_prop(props: &PrimitiveProps, name: &str) -> Result<Option<f64>, String> {
    match props.get(name) {
        None | Some(PrimitiveValue::Data(UiValue::Null)) => Ok(None),
        Some(_) => props
            .number(name)
            .map(Some)
            .ok_or_else(|| format!("resizable {name} must be an optional number")),
    }
}

fn required_optional_float_signal(
    props: &PrimitiveProps,
    name: &str,
) -> Result<crate::NativeSignal, String> {
    let signal = props
        .signal(name)
        .cloned()
        .ok_or_else(|| format!("resizable requires signal {name}"))?;
    if signal.id().kind() != SignalKind::OptionalFloat {
        return Err(format!("resizable {name} must be optional_float"));
    }
    Ok(signal)
}

fn rect_schema() -> ValueSchema {
    ValueSchema::object(BTreeMap::from([
        ("x".to_owned(), ObjectField::required(ValueSchema::number())),
        ("y".to_owned(), ObjectField::required(ValueSchema::number())),
        (
            "width".to_owned(),
            ObjectField::required(ValueSchema::positive_number()),
        ),
        (
            "height".to_owned(),
            ObjectField::required(ValueSchema::positive_number()),
        ),
    ]))
}

fn bounded_dimension_schema() -> ValueSchema {
    ValueSchema::Number {
        min: None,
        max: Some(MAX_RESIZE_DIMENSION),
        exclusive_min: Some(0.0),
        exclusive_max: None,
    }
}

/// Build the native `Resizable` edge/corner primitive schema.
///
/// # Panics
///
/// Panics only if the static built-in primitive ID becomes invalid.
#[must_use]
#[allow(clippy::too_many_lines)] // One declarative list of documented props and events.
pub fn resizable_primitive_descriptor() -> PrimitiveDescriptor {
    let optional_number = || ObjectField::optional(ValueSchema::optional(ValueSchema::number()));
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.resizable_handle").expect("static primitive ID"),
        export: "ResizableHandlePrimitive".to_owned(),
        props: {
            let mut props = BTreeMap::from([
                (
                    "handle".to_owned(),
                    ObjectField::required(ValueSchema::String {
                        allowed: vec![
                            "n".to_owned(),
                            "s".to_owned(),
                            "e".to_owned(),
                            "w".to_owned(),
                            "ne".to_owned(),
                            "nw".to_owned(),
                            "se".to_owned(),
                            "sw".to_owned(),
                        ],
                    })
                    .with_doc(
                        "Physical edge (`n`, `s`, `e`, `w`) or corner (`ne`, `nw`, `se`, `sw`) this handle drags; it does not flip in RTL.",
                    ),
                ),
                (
                    "x".to_owned(),
                    ObjectField::required(ValueSchema::number()).with_doc(
                        "Controlled left edge of the rectangle in `boundary_ref`'s local logical pixels.",
                    ),
                ),
                (
                    "y".to_owned(),
                    ObjectField::required(ValueSchema::number()).with_doc(
                        "Controlled top edge of the rectangle in `boundary_ref`'s local logical pixels.",
                    ),
                ),
                (
                    "width".to_owned(),
                    ObjectField::required(ValueSchema::positive_number())
                        .with_doc("Controlled width of the rectangle in logical pixels."),
                ),
                (
                    "height".to_owned(),
                    ObjectField::required(ValueSchema::positive_number())
                        .with_doc("Controlled height of the rectangle in logical pixels."),
                ),
                (
                    "min_width".to_owned(),
                    ObjectField::optional(bounded_dimension_schema()).with_doc(
                        "Smallest width a resize may propose, in logical pixels; defaults to 24.",
                    ),
                ),
                (
                    "min_height".to_owned(),
                    ObjectField::optional(bounded_dimension_schema()).with_doc(
                        "Smallest height a resize may propose, in logical pixels; defaults to 24.",
                    ),
                ),
                (
                    "max_width".to_owned(),
                    ObjectField::optional(bounded_dimension_schema()).with_doc(
                        "Largest width a resize may propose, in logical pixels; defaults to 16,384.",
                    ),
                ),
                (
                    "max_height".to_owned(),
                    ObjectField::optional(bounded_dimension_schema()).with_doc(
                        "Largest height a resize may propose, in logical pixels; defaults to 16,384.",
                    ),
                ),
                (
                    "aspect_ratio".to_owned(),
                    optional_number().with_doc(
                        "Width-to-height ratio a resize keeps, or `()` to resize each axis freely.",
                    ),
                ),
                (
                    "contain".to_owned(),
                    ObjectField::optional(ValueSchema::Bool)
                        .with_default(UiValue::Bool(true))
                        .with_doc(
                            "Keeps the rectangle inside `boundary_ref`; presses are ignored while the controlled rectangle lies outside it.",
                        ),
                ),
                (
                    "keyboard_step".to_owned(),
                    ObjectField::optional(ValueSchema::Number {
                        min: None,
                        max: Some(512.0),
                        exclusive_min: Some(0.0),
                        exclusive_max: None,
                    })
                    .with_doc(
                        "Logical pixels one arrow-key press moves the dragged edge; Shift multiplies it by four; defaults to 8.",
                    ),
                ),
                (
                    "disabled".to_owned(),
                    ObjectField::optional(ValueSchema::Bool)
                        .with_default(UiValue::Bool(false))
                        .with_doc(
                            "Ignores presses and arrow keys and removes the handle from the tab order.",
                        ),
                ),
                (
                    "boundary_ref".to_owned(),
                    ObjectField::required(ValueSchema::Ref).with_doc(
                        "Ref to the container the rectangle is local to; resizing it during a drag cancels the drag.",
                    ),
                ),
                (
                    "on_resize".to_owned(),
                    ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)).with_doc(
                        "Called with the proposed `{x, y, width, height}` when a drag ends or an arrow key is pressed.",
                    ),
                ),
            ]);
            props.extend(
                [
                    (
                        "x_signal",
                        "Optional-float signal that receives the previewed horizontal offset from `x` during a drag, or `()` when idle.",
                    ),
                    (
                        "y_signal",
                        "Optional-float signal that receives the previewed vertical offset from `y` during a drag, or `()` when idle.",
                    ),
                    (
                        "width_signal",
                        "Optional-float signal that receives the previewed width during a drag, or `()` when idle.",
                    ),
                    (
                        "height_signal",
                        "Optional-float signal that receives the previewed height during a drag, or `()` when idle.",
                    ),
                ]
                .map(|(name, doc)| {
                    (
                        name.to_owned(),
                        ObjectField::required(ValueSchema::Signal).with_doc(doc),
                    )
                }),
            );
            props.extend(crate::handle_state::decoration_props());
            props
        },
        events: BTreeMap::from([(
            "resize".to_owned(),
            EventSchema {
                doc: Some(
                    "Emitted once when a drag ends or an arrow key is pressed; the payload is the next rectangle, shaped like the controlled one."
                        .to_owned(),
                ),
                payload: rect_schema(),
            },
        )]),
        state: ComponentStateSchema::default(),
        lifecycle: true,
        effect: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constraints(aspect_ratio: Option<f64>) -> ResizeConstraints {
        ResizeConstraints {
            min_width: 40.0,
            min_height: 30.0,
            max_width: 500.0,
            max_height: 400.0,
            aspect_ratio,
            contain: true,
        }
    }

    #[test]
    fn west_and_north_handles_keep_the_opposite_corner_fixed() {
        let source = ResizeRect {
            x: 100.0,
            y: 80.0,
            width: 200.0,
            height: 120.0,
        };
        let resized = resize_rect(
            source,
            ResizeHandle::NorthWest,
            -30.0,
            -20.0,
            constraints(None),
            (600.0, 500.0),
        );
        assert!((resized.x - 70.0).abs() < f64::EPSILON);
        assert!((resized.y - 60.0).abs() < f64::EPSILON);
        assert!((resized.right() - source.right()).abs() < f64::EPSILON);
        assert!((resized.bottom() - source.bottom()).abs() < f64::EPSILON);
        let east_boundary = resize_rect(
            source,
            ResizeHandle::East,
            500.0,
            0.0,
            constraints(None),
            (350.0, 500.0),
        );
        assert!((east_boundary.width - 250.0).abs() < f64::EPSILON);
        let west_boundary = resize_rect(
            source,
            ResizeHandle::West,
            -500.0,
            0.0,
            constraints(None),
            (600.0, 500.0),
        );
        assert!((west_boundary.x - 0.0).abs() < f64::EPSILON);
        assert!((west_boundary.width - 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn containment_and_aspect_ratio_clamp_one_atomic_rect() {
        let source = ResizeRect {
            x: 120.0,
            y: 90.0,
            width: 160.0,
            height: 90.0,
        };
        let resized = resize_rect(
            source,
            ResizeHandle::SouthEast,
            500.0,
            500.0,
            constraints(Some(16.0 / 9.0)),
            (420.0, 300.0),
        );
        assert!(resized.right() <= 420.0 + f64::EPSILON);
        assert!(resized.bottom() <= 300.0 + f64::EPSILON);
        assert!((resized.width / resized.height - 16.0 / 9.0).abs() < 1e-9);
    }
}
