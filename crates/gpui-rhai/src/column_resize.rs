//! Native hot-lane column resize handle used by the Rhai Table component.

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
    PrimitiveValue, Rgba8, SignalValue, TextDirection, UiValue, ValueSchema,
};

const DEFAULT_MIN_WIDTH: f64 = 48.0;
const MAX_COLUMN_WIDTH: f64 = 16_384.0;

#[derive(Clone, Debug, PartialEq)]
struct SourceWidth {
    column_key: String,
    kind: String,
    value: f64,
    min: f64,
    max: f64,
}

#[derive(Clone)]
struct ResizeConfig {
    id: String,
    column_key: String,
    signal: crate::NativeSignal,
    reference: crate::ElementRef,
    source: SourceWidth,
    min: f64,
    max: f64,
    direction: TextDirection,
    idle_color: Rgba8,
    active_color: Rgba8,
}

#[derive(Clone)]
struct ResizeState(Rc<RefCell<ResizeStateInner>>);

#[derive(Clone)]
struct ResizeStateInner {
    source: SourceWidth,
    signal: crate::SignalId,
    hovered: bool,
    dragging: bool,
    start_position: Point<Pixels>,
    start_width: f64,
}

impl ResizeState {
    fn new(config: &ResizeConfig) -> Self {
        Self(Rc::new(RefCell::new(ResizeStateInner {
            source: config.source.clone(),
            signal: config.signal.id().clone(),
            hovered: false,
            dragging: false,
            start_position: point(px(0.0), px(0.0)),
            start_width: 0.0,
        })))
    }
}

struct ResizePrepaint {
    hitbox: Hitbox,
    state: ResizeState,
}

struct ColumnResizeHandle {
    config: ResizeConfig,
    events: PrimitiveEventEmitter,
}

impl IntoElement for ColumnResizeHandle {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ColumnResizeHandle {
    type RequestLayoutState = ();
    type PrepaintState = ResizePrepaint;

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
        let style = Style {
            size: size(relative(1.0).into(), relative(1.0).into()),
            ..Style::default()
        };
        (window.request_layout(style, None, cx), ())
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
            .use_state(cx, |_, _| ResizeState::new(&self.config))
            .read(cx)
            .clone();
        let reset = {
            let mut inner = state.0.borrow_mut();
            let changed =
                inner.source != self.config.source || inner.signal != *self.config.signal.id();
            if changed {
                inner.source = self.config.source.clone();
                inner.signal = self.config.signal.id().clone();
                inner.dragging = false;
            }
            changed
        };
        if reset {
            let events = self.events.clone();
            let signal = self.config.signal.clone();
            window.defer(cx, move |_, cx| {
                let _ = events.write_signal(&signal, SignalValue::OptionalFloat(None), cx);
            });
        }
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::BlockMouseExceptScroll);
        ResizePrepaint { hitbox, state }
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
        let dragged = prepaint.state.0.borrow().dragging;
        let hovered = prepaint.hitbox.is_hovered(window);
        let color = if dragged || hovered {
            self.config.active_color
        } else {
            self.config.idle_color
        };
        let line_width = if dragged { px(2.0) } else { px(1.0) };
        let line = Bounds::new(
            point(
                bounds.origin.x + (bounds.size.width - line_width) / 2.0,
                bounds.origin.y + px(4.0),
            ),
            size(line_width, (bounds.size.height - px(8.0)).max(px(1.0))),
        );
        window.paint_quad(fill(line, rgba(color.as_rgba_hex())));
        window.set_cursor_style(CursorStyle::ResizeColumn, &prepaint.hitbox);
        register_pointer_listeners(
            prepaint,
            self.config.clone(),
            self.events.clone(),
            window,
            cx,
        );
    }
}

fn register_pointer_listeners(
    prepaint: &ResizePrepaint,
    config: ResizeConfig,
    events: PrimitiveEventEmitter,
    window: &mut Window,
    _: &mut App,
) {
    let view = window.current_view();
    let hitbox = prepaint.hitbox.clone();
    let down_state = prepaint.state.clone();
    let down_config = config.clone();
    let down_events = events.clone();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble
            || event.button != MouseButton::Left
            || !hitbox.is_hovered(window)
        {
            return;
        }
        let Some(bounds) = down_events.element_bounds(&down_config.reference, cx) else {
            return;
        };
        let width = clamp_width(bounds.width, down_config.min, down_config.max);
        {
            let mut inner = down_state.0.borrow_mut();
            inner.dragging = true;
            inner.start_position = event.position;
            inner.start_width = width;
        }
        let _ = down_events.write_signal(
            &down_config.signal,
            SignalValue::OptionalFloat(Some(width)),
            cx,
        );
        cx.stop_propagation();
        cx.notify(view);
    });

    let move_state = prepaint.state.clone();
    let move_config = config.clone();
    let move_events = events.clone();
    let move_hitbox = prepaint.hitbox.clone();
    window.on_mouse_event(move |event: &MouseMoveEvent, _, window, cx| {
        let (dragging, start_position, start_width) = {
            let inner = move_state.0.borrow();
            (inner.dragging, inner.start_position, inner.start_width)
        };
        if !dragging || !event.dragging() {
            let hovered = move_hitbox.is_hovered(window);
            let mut inner = move_state.0.borrow_mut();
            if inner.hovered != hovered {
                inner.hovered = hovered;
                cx.notify(view);
            }
            return;
        }
        let delta = horizontal_delta(start_position, event.position, move_config.direction);
        let width = clamp_width(start_width + delta, move_config.min, move_config.max);
        let _ = move_events.write_signal(
            &move_config.signal,
            SignalValue::OptionalFloat(Some(width)),
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
        let (dragging, start_position, start_width) = {
            let inner = up_state.0.borrow();
            (inner.dragging, inner.start_position, inner.start_width)
        };
        if !dragging {
            return;
        }
        let delta = horizontal_delta(start_position, event.position, up_config.direction);
        let width = clamp_width(start_width + delta, up_config.min, up_config.max);
        {
            let mut inner = up_state.0.borrow_mut();
            inner.dragging = false;
        }
        let _ = up_events.write_signal(
            &up_config.signal,
            SignalValue::OptionalFloat(Some(width)),
            cx,
        );
        let events = up_events.clone();
        let payload = resize_payload(&up_config.column_key, width);
        window.defer(cx, move |window, cx| {
            let _ = events.emit("resize", payload, window, cx);
        });
        cx.stop_propagation();
        cx.notify(view);
    });
}

fn horizontal_delta(start: Point<Pixels>, current: Point<Pixels>, direction: TextDirection) -> f64 {
    let delta = f64::from(current.x - start.x);
    match direction {
        TextDirection::LeftToRight => delta,
        TextDirection::RightToLeft => -delta,
    }
}

fn clamp_width(width: f64, min: f64, max: f64) -> f64 {
    width.clamp(min, max)
}

fn resize_payload(column_key: &str, width: f64) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("key".to_owned(), UiValue::String(column_key.to_owned())),
        (
            "width".to_owned(),
            UiValue::Map(BTreeMap::from([
                ("kind".to_owned(), UiValue::String("fixed".to_owned())),
                ("value".to_owned(), UiValue::Float(width)),
            ])),
        ),
    ]))
}

#[derive(Default)]
pub struct ColumnResizePrimitiveHandler;

impl PrimitiveHandler for ColumnResizePrimitiveHandler {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        _: &mut Window,
        _: &mut App,
    ) -> Result<AnyElement, String> {
        let config = parse_config(&instance.node.props, theme)?;
        Ok(ColumnResizeHandle {
            config,
            events: events.clone(),
        }
        .into_any_element())
    }
}

fn parse_config(props: &PrimitiveProps, theme: &PrimitiveTheme) -> Result<ResizeConfig, String> {
    let column_key = string_prop(props, "column_key")
        .ok_or_else(|| "column resize handle requires column_key".to_owned())?;
    let source_kind = string_prop(props, "source_kind")
        .ok_or_else(|| "column resize handle requires source_kind".to_owned())?;
    let source_value = number_prop(props, "source_value")
        .ok_or_else(|| "column resize handle requires source_value".to_owned())?;
    let min = number_prop(props, "min_width").unwrap_or(DEFAULT_MIN_WIDTH);
    let max = number_prop(props, "max_width").unwrap_or(MAX_COLUMN_WIDTH);
    if !source_value.is_finite()
        || !min.is_finite()
        || !max.is_finite()
        || min <= 0.0
        || max < min
        || max > MAX_COLUMN_WIDTH
    {
        return Err("column resize widths must be finite and satisfy 0 < min <= max".to_owned());
    }
    let signal = signal_prop(props, "signal")
        .ok_or_else(|| "column resize handle requires signal".to_owned())?;
    let reference = ref_prop(props, "column_ref")
        .ok_or_else(|| "column resize handle requires column_ref".to_owned())?;
    Ok(ResizeConfig {
        id: format!(
            "gpui-rhai-column-resize:{}:{}",
            signal.id().component(),
            signal.id().key()
        ),
        column_key: column_key.clone(),
        signal,
        reference,
        source: SourceWidth {
            column_key,
            kind: source_kind,
            value: source_value,
            min,
            max,
        },
        min,
        max,
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

const fn resize_bound_schema() -> ValueSchema {
    ValueSchema::Number {
        min: None,
        max: Some(MAX_COLUMN_WIDTH),
        exclusive_min: Some(0.0),
        exclusive_max: None,
    }
}

/// Build the internal native Table column-resize primitive schema.
///
/// # Panics
///
/// Panics only if the static built-in primitive ID becomes invalid.
#[must_use]
pub fn column_resize_primitive_descriptor() -> PrimitiveDescriptor {
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.column_resize").expect("static primitive ID"),
        export: "ColumnResizePrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "column_key".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "source_kind".to_owned(),
                ObjectField::required(ValueSchema::String {
                    allowed: vec!["fixed".to_owned(), "percent".to_owned(), "flex".to_owned()],
                }),
            ),
            (
                "source_value".to_owned(),
                ObjectField::required(ValueSchema::positive_number()),
            ),
            (
                "min_width".to_owned(),
                ObjectField::optional(resize_bound_schema()),
            ),
            (
                "max_width".to_owned(),
                ObjectField::optional(resize_bound_schema()),
            ),
            (
                "signal".to_owned(),
                ObjectField::required(ValueSchema::Signal),
            ),
            (
                "column_ref".to_owned(),
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
                payload: ValueSchema::object(BTreeMap::from([
                    (
                        "key".to_owned(),
                        ObjectField::required(ValueSchema::string()),
                    ),
                    (
                        "width".to_owned(),
                        ObjectField::required(ValueSchema::object(BTreeMap::from([
                            (
                                "kind".to_owned(),
                                ObjectField::required(ValueSchema::String {
                                    allowed: vec!["fixed".to_owned()],
                                }),
                            ),
                            (
                                "value".to_owned(),
                                ObjectField::required(ValueSchema::positive_number()),
                            ),
                        ]))),
                    ),
                ])),
            },
        )]),
        state: ComponentStateSchema::default(),
        lifecycle: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_delta_and_clamping_follow_text_direction() {
        let start = point(px(100.0), px(0.0));
        let right = point(px(132.0), px(0.0));
        assert!(
            (horizontal_delta(start, right, TextDirection::LeftToRight) - 32.0).abs()
                < f64::EPSILON
        );
        assert!(
            (horizontal_delta(start, right, TextDirection::RightToLeft) + 32.0).abs()
                < f64::EPSILON
        );
        assert!((clamp_width(10.0, 48.0, 200.0) - 48.0).abs() < f64::EPSILON);
        assert!((clamp_width(240.0, 48.0, 200.0) - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn resize_event_uses_a_fixed_width_descriptor() {
        assert_eq!(
            resize_payload("name", 144.0),
            UiValue::Map(BTreeMap::from([
                ("key".to_owned(), UiValue::String("name".to_owned())),
                (
                    "width".to_owned(),
                    UiValue::Map(BTreeMap::from([
                        ("kind".to_owned(), UiValue::String("fixed".to_owned())),
                        ("value".to_owned(), UiValue::Float(144.0)),
                    ])),
                ),
            ]))
        );
    }
}
