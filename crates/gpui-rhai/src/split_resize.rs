//! Native hot-lane resize handle shared by the public `SplitPane` component.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, CursorStyle, DispatchPhase, Element, ElementId, GlobalElementId,
    Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, Style, Window, fill, point, px, relative, rgba,
    size,
};

use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveDescriptor, PrimitiveEventEmitter,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveProps, PrimitiveTheme,
    PrimitiveValue, Rgba8, SignalKind, SignalValue, TextDirection, UiValue, ValueSchema,
};

const MAX_PANEL_SIZE: f64 = 16_384.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SplitOrientation {
    Horizontal,
    Vertical,
}

#[derive(Clone)]
struct SplitResizeConfig {
    id: String,
    orientation: SplitOrientation,
    source_ratio: f64,
    min_start: f64,
    min_end: f64,
    max_start: f64,
    disabled: bool,
    signal: crate::NativeSignal,
    group_ref: crate::ElementRef,
    start_ref: crate::ElementRef,
    direction: TextDirection,
    idle_color: Rgba8,
    active_color: Rgba8,
}

#[derive(Clone)]
struct SplitResizeState(Rc<RefCell<SplitResizeStateInner>>);

#[derive(Clone)]
struct SplitResizeStateInner {
    source_ratio: f64,
    signal: crate::SignalId,
    hovered: bool,
    dragging: bool,
    moved: bool,
    start_position: Point<Pixels>,
    start_size: f64,
    group_size: f64,
    handle_size: f64,
    constraint_preview: Option<f64>,
}

impl SplitResizeState {
    fn new(config: &SplitResizeConfig) -> Self {
        Self(Rc::new(RefCell::new(SplitResizeStateInner {
            source_ratio: config.source_ratio,
            signal: config.signal.id().clone(),
            hovered: false,
            dragging: false,
            moved: false,
            start_position: point(px(0.0), px(0.0)),
            start_size: 0.0,
            group_size: 0.0,
            handle_size: 0.0,
            constraint_preview: None,
        })))
    }
}

struct SplitResizePrepaint {
    hitbox: Hitbox,
    state: SplitResizeState,
}

struct SplitResizeHandle {
    config: SplitResizeConfig,
    events: PrimitiveEventEmitter,
}

impl IntoElement for SplitResizeHandle {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SplitResizeHandle {
    type RequestLayoutState = ();
    type PrepaintState = SplitResizePrepaint;

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
            .use_state(cx, |_, _| SplitResizeState::new(&self.config))
            .read(cx)
            .clone();
        let (reset, dragging) = {
            let mut inner = state.0.borrow_mut();
            let changed = (inner.source_ratio - self.config.source_ratio).abs() > f64::EPSILON
                || inner.signal != *self.config.signal.id();
            if changed {
                inner.source_ratio = self.config.source_ratio;
                inner.signal = self.config.signal.id().clone();
                inner.dragging = false;
                inner.moved = false;
                inner.constraint_preview = None;
            }
            (changed, inner.dragging)
        };
        if !dragging {
            let constrained = self
                .events
                .element_bounds(&self.config.group_ref, cx)
                .and_then(|group| {
                    let group_size = axis_size(group, self.config.orientation);
                    let handle_size = match self.config.orientation {
                        SplitOrientation::Horizontal => f64::from(bounds.size.width),
                        SplitOrientation::Vertical => f64::from(bounds.size.height),
                    };
                    let requested = self.config.source_ratio * group_size;
                    let legal = clamp_start_size(requested, group_size, handle_size, &self.config);
                    ((legal - requested).abs() > 0.5).then_some(legal)
                });
            let changed = {
                let mut inner = state.0.borrow_mut();
                let changed = !same_optional_float(inner.constraint_preview, constrained);
                inner.constraint_preview = constrained;
                changed
            };
            if reset || changed {
                write_preview(&self.events, &self.config.signal, constrained, window, cx);
            }
        }
        SplitResizePrepaint {
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
        _: &mut App,
    ) {
        let dragged = prepaint.state.0.borrow().dragging;
        let hovered = !self.config.disabled && prepaint.hitbox.is_hovered(window);
        let color = if dragged || hovered {
            self.config.active_color
        } else {
            self.config.idle_color
        };
        let thickness = if dragged { px(2.0) } else { px(1.0) };
        let line = match self.config.orientation {
            SplitOrientation::Horizontal => Bounds::new(
                point(
                    bounds.origin.x + (bounds.size.width - thickness) / 2.0,
                    bounds.origin.y + px(4.0),
                ),
                size(thickness, (bounds.size.height - px(8.0)).max(px(1.0))),
            ),
            SplitOrientation::Vertical => Bounds::new(
                point(
                    bounds.origin.x + px(4.0),
                    bounds.origin.y + (bounds.size.height - thickness) / 2.0,
                ),
                size((bounds.size.width - px(8.0)).max(px(1.0)), thickness),
            ),
        };
        window.paint_quad(fill(line, rgba(color.as_rgba_hex())));
        if !self.config.disabled {
            window.set_cursor_style(
                match self.config.orientation {
                    SplitOrientation::Horizontal => CursorStyle::ResizeColumn,
                    SplitOrientation::Vertical => CursorStyle::ResizeRow,
                },
                &prepaint.hitbox,
            );
        }
        register_pointer_listeners(
            prepaint,
            bounds,
            self.config.clone(),
            self.events.clone(),
            window,
        );
    }
}

#[allow(clippy::too_many_lines)]
fn register_pointer_listeners(
    prepaint: &SplitResizePrepaint,
    handle_bounds: Bounds<Pixels>,
    config: SplitResizeConfig,
    events: PrimitiveEventEmitter,
    window: &mut Window,
) {
    let view = window.current_view();
    let hitbox = prepaint.hitbox.clone();
    let down_state = prepaint.state.clone();
    let down_config = config.clone();
    let down_events = events.clone();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble
            || event.button != MouseButton::Left
            || down_config.disabled
            || !hitbox.is_hovered(window)
        {
            return;
        }
        let Some(group) = down_events.element_bounds(&down_config.group_ref, cx) else {
            return;
        };
        let Some(start) = down_events.element_bounds(&down_config.start_ref, cx) else {
            return;
        };
        let mut inner = down_state.0.borrow_mut();
        inner.dragging = true;
        inner.moved = false;
        inner.constraint_preview = None;
        inner.start_position = event.position;
        inner.start_size = axis_size(start, down_config.orientation);
        inner.group_size = axis_size(group, down_config.orientation);
        inner.handle_size = match down_config.orientation {
            SplitOrientation::Horizontal => f64::from(handle_bounds.size.width),
            SplitOrientation::Vertical => f64::from(handle_bounds.size.height),
        };
        cx.stop_propagation();
        cx.notify(view);
    });

    let move_state = prepaint.state.clone();
    let move_hitbox = prepaint.hitbox.clone();
    let move_config = config.clone();
    let move_events = events.clone();
    window.on_mouse_event(move |event: &MouseMoveEvent, _, window, cx| {
        let snapshot = move_state.0.borrow().clone();
        if !snapshot.dragging {
            let hovered = move_hitbox.is_hovered(window);
            let mut inner = move_state.0.borrow_mut();
            if inner.hovered != hovered {
                inner.hovered = hovered;
                cx.notify(view);
            }
            return;
        }
        if !event.dragging() {
            cancel_drag(
                &move_state,
                &move_events,
                &move_config.signal,
                window,
                cx,
                view,
            );
            return;
        }
        let delta = axis_delta(
            snapshot.start_position,
            event.position,
            move_config.orientation,
            move_config.direction,
        );
        if delta.abs() < f64::EPSILON {
            return;
        }
        move_state.0.borrow_mut().moved = true;
        let size = clamp_start_size(
            snapshot.start_size + delta,
            snapshot.group_size,
            snapshot.handle_size,
            &move_config,
        );
        let _ = move_events.write_signal(
            &move_config.signal,
            SignalValue::OptionalFloat(Some(size)),
            cx,
        );
        cx.stop_propagation();
        cx.notify(view);
    });

    let up_state = prepaint.state.clone();
    let up_config = config;
    let up_events = events;
    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
            return;
        }
        let snapshot = up_state.0.borrow().clone();
        if !snapshot.dragging {
            return;
        }
        {
            let mut inner = up_state.0.borrow_mut();
            inner.dragging = false;
            inner.moved = false;
        }
        let _ = up_events.write_signal(&up_config.signal, SignalValue::OptionalFloat(None), cx);
        if snapshot.moved && snapshot.group_size > 0.0 {
            let delta = axis_delta(
                snapshot.start_position,
                event.position,
                up_config.orientation,
                up_config.direction,
            );
            let size = clamp_start_size(
                snapshot.start_size + delta,
                snapshot.group_size,
                snapshot.handle_size,
                &up_config,
            );
            let ratio = (size / snapshot.group_size).clamp(0.0, 1.0);
            let deferred = up_events.clone();
            window.defer(cx, move |window, cx| {
                let _ = deferred.emit("resize", UiValue::Float(ratio), window, cx);
            });
        }
        cx.stop_propagation();
        cx.notify(view);
    });
}

fn cancel_drag(
    state: &SplitResizeState,
    events: &PrimitiveEventEmitter,
    signal: &crate::NativeSignal,
    window: &mut Window,
    cx: &mut App,
    view: gpui::EntityId,
) {
    {
        let mut inner = state.0.borrow_mut();
        inner.dragging = false;
        inner.moved = false;
    }
    clear_preview(events, signal, window, cx);
    cx.notify(view);
}

fn clear_preview(
    events: &PrimitiveEventEmitter,
    signal: &crate::NativeSignal,
    window: &mut Window,
    cx: &mut App,
) {
    write_preview(events, signal, None, window, cx);
}

fn write_preview(
    events: &PrimitiveEventEmitter,
    signal: &crate::NativeSignal,
    value: Option<f64>,
    window: &mut Window,
    cx: &mut App,
) {
    let events = events.clone();
    let signal = signal.clone();
    window.defer(cx, move |_, cx| {
        let _ = events.write_signal(&signal, SignalValue::OptionalFloat(value), cx);
    });
}

fn same_optional_float(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => (left - right).abs() <= 0.5,
        (None, None) => true,
        _ => false,
    }
}

fn axis_size(bounds: crate::GeometryBounds, orientation: SplitOrientation) -> f64 {
    match orientation {
        SplitOrientation::Horizontal => bounds.width,
        SplitOrientation::Vertical => bounds.height,
    }
}

fn axis_delta(
    start: Point<Pixels>,
    current: Point<Pixels>,
    orientation: SplitOrientation,
    direction: TextDirection,
) -> f64 {
    match orientation {
        SplitOrientation::Vertical => f64::from(current.y - start.y),
        SplitOrientation::Horizontal => {
            let delta = f64::from(current.x - start.x);
            match direction {
                TextDirection::LeftToRight => delta,
                TextDirection::RightToLeft => -delta,
            }
        }
    }
}

fn clamp_start_size(
    proposed: f64,
    group_size: f64,
    handle_size: f64,
    config: &SplitResizeConfig,
) -> f64 {
    clamp_start_size_values(
        proposed,
        group_size,
        handle_size,
        config.min_start,
        config.min_end,
        config.max_start,
    )
}

fn clamp_start_size_values(
    proposed: f64,
    group_size: f64,
    handle_size: f64,
    min_start: f64,
    min_end: f64,
    max_start: f64,
) -> f64 {
    let available = (group_size - handle_size).max(0.0);
    let max_from_end = (available - min_end).max(0.0);
    let max = max_start.min(max_from_end);
    let min = min_start.min(max);
    proposed.clamp(min, max)
}

#[derive(Default)]
pub struct SplitResizePrimitiveHandler;

impl PrimitiveHandler for SplitResizePrimitiveHandler {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        _: &mut Window,
        _: &mut App,
    ) -> Result<AnyElement, String> {
        Ok(SplitResizeHandle {
            config: parse_config(&instance.node.props, theme)?,
            events: events.clone(),
        }
        .into_any_element())
    }
}

fn parse_config(
    props: &PrimitiveProps,
    theme: &PrimitiveTheme,
) -> Result<SplitResizeConfig, String> {
    let orientation = match string_prop(props, "orientation").as_deref() {
        Some("horizontal") => SplitOrientation::Horizontal,
        Some("vertical") => SplitOrientation::Vertical,
        _ => return Err("split resize orientation must be horizontal or vertical".to_owned()),
    };
    let source_ratio = number_prop(props, "source_ratio")
        .ok_or_else(|| "split resize requires source_ratio".to_owned())?;
    let min_start = number_prop(props, "min_start").unwrap_or(0.0);
    let min_end = number_prop(props, "min_end").unwrap_or(0.0);
    let max_start = number_prop(props, "max_start").unwrap_or(MAX_PANEL_SIZE);
    let disabled = bool_prop(props, "disabled").unwrap_or(false);
    if !source_ratio.is_finite()
        || !(0.0..=1.0).contains(&source_ratio)
        || !min_start.is_finite()
        || !min_end.is_finite()
        || !max_start.is_finite()
        || min_start < 0.0
        || min_end < 0.0
        || max_start < min_start
        || max_start > MAX_PANEL_SIZE
    {
        return Err("split resize bounds and ratio are invalid".to_owned());
    }
    let signal =
        signal_prop(props, "signal").ok_or_else(|| "split resize requires signal".to_owned())?;
    if signal.id().kind() != SignalKind::OptionalFloat {
        return Err("split resize signal must be optional_float".to_owned());
    }
    let group_ref =
        ref_prop(props, "group_ref").ok_or_else(|| "split resize requires group_ref".to_owned())?;
    let start_ref =
        ref_prop(props, "start_ref").ok_or_else(|| "split resize requires start_ref".to_owned())?;
    Ok(SplitResizeConfig {
        id: format!(
            "gpui-rhai-split-resize:{}:{}",
            signal.id().component(),
            signal.id().key()
        ),
        orientation,
        source_ratio,
        min_start,
        min_end,
        max_start,
        disabled,
        signal,
        group_ref,
        start_ref,
        direction: theme.direction(),
        idle_color: theme
            .color("border")
            .unwrap_or(Rgba8::from_rgba_hex(0x5555_55ff)),
        active_color: theme
            .color("accent")
            .unwrap_or(Rgba8::from_rgba_hex(0x3b82_f6ff)),
    })
}

fn number_prop(props: &PrimitiveProps, name: &str) -> Option<f64> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::Float(value))) => Some(*value),
        Some(PrimitiveValue::Data(UiValue::Integer(value))) => value.to_string().parse().ok(),
        _ => None,
    }
}

fn string_prop(props: &PrimitiveProps, name: &str) -> Option<String> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::String(value))) => Some(value.clone()),
        _ => None,
    }
}

fn bool_prop(props: &PrimitiveProps, name: &str) -> Option<bool> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::Bool(value))) => Some(*value),
        _ => None,
    }
}

fn signal_prop(props: &PrimitiveProps, name: &str) -> Option<crate::NativeSignal> {
    match props.get(name) {
        Some(PrimitiveValue::Signal(signal)) => Some(signal.clone()),
        _ => None,
    }
}

fn ref_prop(props: &PrimitiveProps, name: &str) -> Option<crate::ElementRef> {
    match props.get(name) {
        Some(PrimitiveValue::Ref(reference)) => Some(reference.clone()),
        _ => None,
    }
}

fn panel_bound_schema() -> ValueSchema {
    ValueSchema::Number {
        min: Some(0.0),
        max: Some(MAX_PANEL_SIZE),
        exclusive_min: None,
        exclusive_max: None,
    }
}

/// Build the native `SplitPane` drag-handle schema.
///
/// # Panics
///
/// Panics only if the static built-in primitive ID becomes invalid.
#[must_use]
pub fn split_resize_primitive_descriptor() -> PrimitiveDescriptor {
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.split_resize").expect("static primitive ID"),
        export: "SplitResizePrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "orientation".to_owned(),
                ObjectField::required(ValueSchema::String {
                    allowed: vec!["horizontal".to_owned(), "vertical".to_owned()],
                }),
            ),
            (
                "source_ratio".to_owned(),
                ObjectField::required(ValueSchema::Number {
                    min: Some(0.0),
                    max: Some(1.0),
                    exclusive_min: None,
                    exclusive_max: None,
                }),
            ),
            (
                "min_start".to_owned(),
                ObjectField::optional(panel_bound_schema()),
            ),
            (
                "min_end".to_owned(),
                ObjectField::optional(panel_bound_schema()),
            ),
            (
                "max_start".to_owned(),
                ObjectField::optional(panel_bound_schema()),
            ),
            (
                "disabled".to_owned(),
                ObjectField::optional(ValueSchema::Bool).with_default(UiValue::Bool(false)),
            ),
            (
                "signal".to_owned(),
                ObjectField::required(ValueSchema::Signal),
            ),
            (
                "group_ref".to_owned(),
                ObjectField::required(ValueSchema::Ref),
            ),
            (
                "start_ref".to_owned(),
                ObjectField::required(ValueSchema::Ref),
            ),
            (
                "on_resize".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
            ),
        ]),
        events: BTreeMap::from([(
            "resize".to_owned(),
            EventSchema {
                payload: ValueSchema::Number {
                    min: Some(0.0),
                    max: Some(1.0),
                    exclusive_min: None,
                    exclusive_max: None,
                },
            },
        )]),
        state: ComponentStateSchema::default(),
        lifecycle: false,
        effect: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_delta_and_constraints_are_directional_and_deterministic() {
        let start = point(px(100.0), px(100.0));
        let moved = point(px(132.0), px(118.0));
        let ltr = axis_delta(
            start,
            moved,
            SplitOrientation::Horizontal,
            TextDirection::LeftToRight,
        );
        let rtl = axis_delta(
            start,
            moved,
            SplitOrientation::Horizontal,
            TextDirection::RightToLeft,
        );
        let vertical = axis_delta(
            start,
            moved,
            SplitOrientation::Vertical,
            TextDirection::RightToLeft,
        );
        assert!((ltr - 32.0).abs() < f64::EPSILON);
        assert!((rtl + 32.0).abs() < f64::EPSILON);
        assert!((vertical - 18.0).abs() < f64::EPSILON);
        let maximum = clamp_start_size_values(450.0, 500.0, 8.0, 100.0, 120.0, 360.0);
        let minimum = clamp_start_size_values(20.0, 500.0, 8.0, 100.0, 120.0, 360.0);
        let overconstrained = clamp_start_size_values(100.0, 180.0, 8.0, 120.0, 120.0, 360.0);
        assert!((maximum - 360.0).abs() < f64::EPSILON);
        assert!((minimum - 100.0).abs() < f64::EPSILON);
        assert!(
            (overconstrained - 52.0).abs() < f64::EPSILON,
            "when minima exceed available space, the end-panel minimum wins deterministically"
        );
    }
}
