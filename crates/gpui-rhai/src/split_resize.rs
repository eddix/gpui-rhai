//! Native hot-lane resize handle shared by the public `SplitPane` component.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, CursorStyle, DispatchPhase, Element, ElementId, GlobalElementId,
    Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    Pixels, Point, Style, Window, fill, point, px, relative, rgba, size,
};

use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveContext, PrimitiveDescriptor,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveProps, PrimitiveTheme, Rgba8,
    SignalKind, SignalValue, TextDirection, UiValue, ValueSchema,
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
    /// Paint the native line; off when a handle node draws its own.
    line: bool,
    /// How far the line stops short of each end of the handle.
    line_inset: f64,
    /// A string signal that receives `idle`, `hover`, `drag`, `focus` or
    /// `disabled`, for a handle node's `signal_style`.
    state_signal: Option<crate::NativeSignal>,
    /// A handle node whose bounds (an overhang past the handle included) also
    /// start a drag.
    handle_ref: Option<crate::ElementRef>,
}

#[derive(Clone)]
struct SplitResizeState(Rc<RefCell<SplitResizeStateInner>>);

#[derive(Clone)]
struct SplitResizeStateInner {
    source_ratio: f64,
    signal: crate::SignalId,
    constraint_preview: Option<f64>,
    /// The state last written to the state signal.
    written_state: Option<&'static str>,
}

impl SplitResizeState {
    fn new(config: &SplitResizeConfig) -> Self {
        Self(Rc::new(RefCell::new(SplitResizeStateInner {
            source_ratio: config.source_ratio,
            signal: config.signal.id().clone(),
            constraint_preview: None,
            written_state: None,
        })))
    }
}

struct SplitResizePrepaint {
    hitbox: Hitbox,
    state: SplitResizeState,
}

struct SplitResizeHandle {
    config: SplitResizeConfig,
    events: PrimitiveContext,
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
        let reset = {
            let mut inner = state.0.borrow_mut();
            let changed = (inner.source_ratio - self.config.source_ratio).abs() > f64::EPSILON
                || inner.signal != *self.config.signal.id();
            if changed {
                inner.source_ratio = self.config.source_ratio;
                inner.signal = self.config.signal.id().clone();
                inner.constraint_preview = None;
            }
            changed
        };
        let owner = self.events.interaction_owner(&self.config.id);
        if reset {
            self.events.cancel_interaction(&owner, window, cx);
        }
        let dragging = self.events.interaction_is_active(&owner);
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
        cx: &mut App,
    ) {
        let owner = self.events.interaction_owner(&self.config.id);
        self.events.present_interaction(owner.clone());
        let dragged = self.events.interaction_is_active(&owner);
        let handle = self
            .config
            .handle_ref
            .as_ref()
            .and_then(|reference| self.events.element_hitbox(reference, cx));
        let hovered = !self.config.disabled
            && crate::handle_state::over_handle(&prepaint.hitbox, handle, window);
        if self.config.line {
            let color = if dragged || hovered {
                self.config.active_color
            } else {
                self.config.idle_color
            };
            paint_line(bounds, &self.config, dragged, color, window);
        }
        if let Some(signal) = &self.config.state_signal {
            let state = crate::handle_state::handle_state(
                self.config.disabled,
                dragged,
                hovered,
                self.events.is_focused(window),
            );
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
            window.set_cursor_style(
                match self.config.orientation {
                    SplitOrientation::Horizontal => CursorStyle::ResizeColumn,
                    SplitOrientation::Vertical => CursorStyle::ResizeRow,
                },
                &prepaint.hitbox,
            );
        }
        register_pointer_listeners(prepaint, bounds, &self.config, &self.events, window);
    }
}

fn paint_line(
    bounds: Bounds<Pixels>,
    config: &SplitResizeConfig,
    dragged: bool,
    color: Rgba8,
    window: &mut Window,
) {
    let thickness = if dragged { px(2.0) } else { px(1.0) };
    #[allow(clippy::cast_possible_truncation)]
    let inset = px(config.line_inset as f32);
    let line = match config.orientation {
        SplitOrientation::Horizontal => Bounds::new(
            point(
                bounds.origin.x + (bounds.size.width - thickness) / 2.0,
                bounds.origin.y + inset,
            ),
            size(thickness, (bounds.size.height - inset * 2.0).max(px(1.0))),
        ),
        SplitOrientation::Vertical => Bounds::new(
            point(
                bounds.origin.x + inset,
                bounds.origin.y + (bounds.size.height - thickness) / 2.0,
            ),
            size((bounds.size.width - inset * 2.0).max(px(1.0)), thickness),
        ),
    };
    window.paint_quad(fill(line, rgba(color.as_rgba_hex())));
}

#[allow(clippy::too_many_lines)]
fn register_pointer_listeners(
    prepaint: &SplitResizePrepaint,
    handle_bounds: Bounds<Pixels>,
    config: &SplitResizeConfig,
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
        let handle = down_config
            .handle_ref
            .as_ref()
            .and_then(|reference| down_events.element_hitbox(reference, cx));
        if !crate::handle_state::over_handle(&hitbox, handle, window) {
            return;
        }
        let Some(group) = down_events.element_bounds(&down_config.group_ref, cx) else {
            return;
        };
        let Some(start) = down_events.element_bounds(&down_config.start_ref, cx) else {
            return;
        };
        let start_size = axis_size(start, down_config.orientation);
        let group_size = axis_size(group, down_config.orientation);
        let handle_size = match down_config.orientation {
            SplitOrientation::Horizontal => f64::from(handle_bounds.size.width),
            SplitOrientation::Vertical => f64::from(handle_bounds.size.height),
        };
        let update_config = down_config.clone();
        let update_events = down_events.clone();
        let update =
            move |gesture: crate::interaction::GestureUpdate, window: &mut Window, cx: &mut App| {
                if !gesture.moved() {
                    return crate::interaction::InteractionFlow::Continue;
                }
                let delta = axis_delta(
                    gesture.start(),
                    gesture.current(),
                    update_config.orientation,
                    update_config.direction,
                );
                let size =
                    clamp_start_size(start_size + delta, group_size, handle_size, &update_config);
                write_preview(
                    &update_events,
                    &update_config.signal,
                    Some(size),
                    window,
                    cx,
                );
                crate::interaction::InteractionFlow::Continue
            };
        let finish_config = down_config.clone();
        let finish_events = down_events.clone();
        let finish =
            move |gesture: crate::interaction::GestureUpdate, window: &mut Window, cx: &mut App| {
                clear_preview(&finish_events, &finish_config.signal, window, cx);
                if !gesture.moved() || group_size <= 0.0 {
                    return;
                }
                let delta = axis_delta(
                    gesture.start(),
                    gesture.current(),
                    finish_config.orientation,
                    finish_config.direction,
                );
                let size =
                    clamp_start_size(start_size + delta, group_size, handle_size, &finish_config);
                finish_events.propose(
                    "resize",
                    UiValue::Float((size / group_size).clamp(0.0, 1.0)),
                    window,
                    cx,
                );
            };
        let cancel_signal = down_config.signal.clone();
        let cancel_events = down_events.clone();
        let cancel = move |window: &mut Window, cx: &mut App| {
            clear_preview(&cancel_events, &cancel_signal, window, cx);
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
    signal: &crate::NativeSignal,
    window: &mut Window,
    cx: &mut App,
) {
    write_preview(events, signal, None, window, cx);
}

fn write_preview(
    events: &PrimitiveContext,
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
        events: &PrimitiveContext,
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
    let orientation = match props.string("orientation") {
        Some("horizontal") => SplitOrientation::Horizontal,
        Some("vertical") => SplitOrientation::Vertical,
        _ => return Err("split resize orientation must be horizontal or vertical".to_owned()),
    };
    let source_ratio = props
        .number("source_ratio")
        .ok_or_else(|| "split resize requires source_ratio".to_owned())?;
    let min_start = props.number("min_start").unwrap_or(0.0);
    let min_end = props.number("min_end").unwrap_or(0.0);
    let max_start = props.number("max_start").unwrap_or(MAX_PANEL_SIZE);
    let disabled = props.boolean("disabled").unwrap_or(false);
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
    let signal = props
        .signal("signal")
        .cloned()
        .ok_or_else(|| "split resize requires signal".to_owned())?;
    if signal.id().kind() != SignalKind::OptionalFloat {
        return Err("split resize signal must be optional_float".to_owned());
    }
    let group_ref = props
        .element_ref("group_ref")
        .cloned()
        .ok_or_else(|| "split resize requires group_ref".to_owned())?;
    let start_ref = props
        .element_ref("start_ref")
        .cloned()
        .ok_or_else(|| "split resize requires start_ref".to_owned())?;
    let state_signal = props.signal("state_signal").cloned();
    if state_signal
        .as_ref()
        .is_some_and(|signal| signal.id().kind() != SignalKind::String)
    {
        return Err("split resize state_signal must be a string signal".to_owned());
    }
    let line_inset = props.number("line_inset").unwrap_or(4.0);
    if !line_inset.is_finite() || line_inset < 0.0 {
        return Err("split resize line_inset must be finite and non-negative".to_owned());
    }
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
        line: props.boolean("line").unwrap_or(true),
        line_inset,
        state_signal,
        handle_ref: props.element_ref("handle_ref").cloned(),
    })
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
        props: {
            let mut props = BTreeMap::from([
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
            ]);
            props.extend(crate::handle_state::decoration_props());
            props
        },
        events: BTreeMap::from([(
            "resize".to_owned(),
            EventSchema {
                doc: None,
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
