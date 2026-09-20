//! Native hot-lane column resize handle used by the Rhai Table component.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, CursorStyle, DispatchPhase, Element, ElementId, GlobalElementId,
    Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, SharedString, Style, Window, fill, point, px,
    relative, rgba, size,
};

use crate::{
    ComponentStateSchema, EventSchema, Length, ObjectField, PrimitiveDescriptor,
    PrimitiveEventEmitter, PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId,
    PrimitiveProps, PrimitiveTheme, PrimitiveValue, Rgba8, SignalId, SignalKind, SignalValue,
    TextDirection, UiValue, ValueSchema,
};

const DEFAULT_MIN_WIDTH: f64 = 48.0;
const MAX_COLUMN_WIDTH: f64 = 16_384.0;

#[derive(Clone, Default)]
pub(crate) struct ColumnMeasurementRegistry(Rc<RefCell<BTreeMap<SignalId, BTreeMap<u64, f64>>>>);

impl ColumnMeasurementRegistry {
    fn report(&self, group: &SignalId, instance: u64, width: f64) {
        self.0
            .borrow_mut()
            .entry(group.clone())
            .or_default()
            .insert(instance, width);
    }

    fn remove(&self, group: &SignalId, instance: u64) {
        let mut groups = self.0.borrow_mut();
        let remove_group = groups.get_mut(group).is_some_and(|measurements| {
            measurements.remove(&instance);
            measurements.is_empty()
        });
        if remove_group {
            groups.remove(group);
        }
    }

    fn maximum(&self, group: &SignalId) -> Option<f64> {
        self.0
            .borrow()
            .get(group)
            .and_then(|measurements| measurements.values().copied().reduce(f64::max))
    }
}

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
    moved: bool,
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
            moved: false,
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
    measurements: ColumnMeasurementRegistry,
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
                inner.moved = false;
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
        _: &mut App,
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
            self.measurements.clone(),
            window,
        );
    }
}

fn register_pointer_listeners(
    prepaint: &ResizePrepaint,
    config: ResizeConfig,
    events: PrimitiveEventEmitter,
    measurements: ColumnMeasurementRegistry,
    window: &mut Window,
) {
    register_pointer_down(
        prepaint,
        config.clone(),
        events.clone(),
        measurements,
        window,
    );
    register_pointer_move(prepaint, config.clone(), events.clone(), window);
    register_pointer_up(prepaint, config, events, window);
}

fn register_pointer_down(
    prepaint: &ResizePrepaint,
    config: ResizeConfig,
    events: PrimitiveEventEmitter,
    measurements: ColumnMeasurementRegistry,
    window: &mut Window,
) {
    let view = window.current_view();
    let hitbox = prepaint.hitbox.clone();
    let down_state = prepaint.state.clone();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble
            || event.button != MouseButton::Left
            || !hitbox.is_hovered(window)
        {
            return;
        }
        let Some(bounds) = events.element_bounds(&config.reference, cx) else {
            return;
        };
        if event.click_count >= 2 {
            {
                let mut inner = down_state.0.borrow_mut();
                inner.dragging = false;
                inner.moved = false;
            }
            let measured = measurements
                .maximum(config.signal.id())
                .unwrap_or(bounds.width);
            let width = clamp_width(measured, config.min, config.max);
            let _ =
                events.write_signal(&config.signal, SignalValue::OptionalFloat(Some(width)), cx);
            let deferred_events = events.clone();
            let payload = resize_payload(&config.column_key, width);
            window.defer(cx, move |window, cx| {
                let _ = deferred_events.emit("resize", payload, window, cx);
            });
            cx.stop_propagation();
            cx.notify(view);
            return;
        }
        let width = clamp_width(bounds.width, config.min, config.max);
        {
            let mut inner = down_state.0.borrow_mut();
            inner.dragging = true;
            inner.moved = false;
            inner.start_position = event.position;
            inner.start_width = width;
        }
        cx.stop_propagation();
        cx.notify(view);
    });
}

fn register_pointer_move(
    prepaint: &ResizePrepaint,
    config: ResizeConfig,
    events: PrimitiveEventEmitter,
    window: &mut Window,
) {
    let view = window.current_view();
    let move_state = prepaint.state.clone();
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
        let delta = horizontal_delta(start_position, event.position, config.direction);
        if delta.abs() < f64::EPSILON {
            return;
        }
        move_state.0.borrow_mut().moved = true;
        let width = clamp_width(start_width + delta, config.min, config.max);
        let _ = events.write_signal(&config.signal, SignalValue::OptionalFloat(Some(width)), cx);
        cx.stop_propagation();
        cx.notify(view);
    });
}

fn register_pointer_up(
    prepaint: &ResizePrepaint,
    config: ResizeConfig,
    events: PrimitiveEventEmitter,
    window: &mut Window,
) {
    let view = window.current_view();
    let up_state = prepaint.state.clone();
    let up_hitbox = prepaint.hitbox.clone();
    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
            return;
        }
        let (dragging, moved, start_position, start_width) = {
            let inner = up_state.0.borrow();
            (
                inner.dragging,
                inner.moved,
                inner.start_position,
                inner.start_width,
            )
        };
        if !dragging {
            if event.click_count >= 2 && up_hitbox.is_hovered(window) {
                cx.stop_propagation();
            }
            return;
        }
        {
            let mut inner = up_state.0.borrow_mut();
            inner.dragging = false;
            inner.moved = false;
        }
        if !moved {
            cx.stop_propagation();
            cx.notify(view);
            return;
        }
        let delta = horizontal_delta(start_position, event.position, config.direction);
        let width = clamp_width(start_width + delta, config.min, config.max);
        let _ = events.write_signal(&config.signal, SignalValue::OptionalFloat(Some(width)), cx);
        let deferred_events = events.clone();
        let payload = resize_payload(&config.column_key, width);
        window.defer(cx, move |window, cx| {
            let _ = deferred_events.emit("resize", payload, window, cx);
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

#[derive(Clone)]
struct IntrinsicMeasureConfig {
    text: String,
    group: SignalId,
    horizontal_padding: Length,
    extra_width: f64,
}

struct IntrinsicTextMeasureElement {
    instance: u64,
    config: IntrinsicMeasureConfig,
    measurements: ColumnMeasurementRegistry,
}

impl IntoElement for IntrinsicTextMeasureElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for IntrinsicTextMeasureElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
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
            size: size(px(0.0).into(), px(0.0).into()),
            ..Style::default()
        };
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let runs = [text_style.to_run(self.config.text.len())];
        if let Ok(lines) = window.text_system().shape_text(
            SharedString::from(self.config.text.clone()),
            font_size,
            &runs,
            None,
            Some(1),
        ) {
            let text_width = lines
                .iter()
                .map(|line| f64::from(line.width()))
                .reduce(f64::max)
                .unwrap_or(0.0);
            let padding =
                absolute_length_pixels(self.config.horizontal_padding, window).unwrap_or_default();
            let width = (text_width + padding * 2.0 + self.config.extra_width).ceil();
            if width.is_finite() {
                self.measurements
                    .report(&self.config.group, self.instance, width);
            }
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        (): &mut Self::PrepaintState,
        _: &mut Window,
        _: &mut App,
    ) {
    }
}

fn absolute_length_pixels(value: Length, window: &Window) -> Option<f64> {
    match value {
        Length::Pixels(value) => Some(value),
        Length::Rems(value) => Some(value * f64::from(window.rem_size())),
        Length::Relative(_) | Length::ThemeSpacing(_) | Length::ThemeRadius(_) => None,
    }
}

#[derive(Default)]
pub(crate) struct IntrinsicTextMeasurePrimitiveHandler {
    measurements: ColumnMeasurementRegistry,
    owners: BTreeMap<PrimitiveInstanceId, SignalId>,
}

impl IntrinsicTextMeasurePrimitiveHandler {
    pub(crate) fn new(measurements: ColumnMeasurementRegistry) -> Self {
        Self {
            measurements,
            owners: BTreeMap::new(),
        }
    }
}

impl PrimitiveHandler for IntrinsicTextMeasurePrimitiveHandler {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        _: &PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        _: &mut Window,
        _: &mut App,
    ) -> Result<AnyElement, String> {
        let id = instance
            .id
            .clone()
            .ok_or_else(|| "intrinsic text measurement requires a keyed instance".to_owned())?;
        let config = parse_measure_config(&instance.node.props, theme)?;
        if let Some(previous) = self.owners.insert(id.clone(), config.group.clone())
            && previous != config.group
        {
            self.measurements.remove(&previous, id.node().get());
        }
        Ok(IntrinsicTextMeasureElement {
            instance: id.node().get(),
            config,
            measurements: self.measurements.clone(),
        }
        .into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        if let Some(group) = self.owners.remove(instance) {
            self.measurements.remove(&group, instance.node().get());
        }
    }
}

fn parse_measure_config(
    props: &PrimitiveProps,
    theme: &PrimitiveTheme,
) -> Result<IntrinsicMeasureConfig, String> {
    let text = string_prop(props, "text")
        .ok_or_else(|| "intrinsic text measurement requires text".to_owned())?;
    let group = signal_prop(props, "group")
        .ok_or_else(|| "intrinsic text measurement requires group".to_owned())?;
    if group.id().kind() != SignalKind::OptionalFloat {
        return Err("intrinsic text measurement group must be an optional-float signal".to_owned());
    }
    let horizontal_padding = match props.get("horizontal_padding") {
        Some(PrimitiveValue::Length(value)) => theme
            .resolve_length(*value)
            .ok_or_else(|| "intrinsic text measurement padding did not resolve".to_owned())?,
        None | Some(PrimitiveValue::Data(UiValue::Null)) => Length::Pixels(0.0),
        Some(_) => return Err("intrinsic text measurement padding must be Length".to_owned()),
    };
    if !matches!(horizontal_padding, Length::Pixels(_) | Length::Rems(_)) {
        return Err("intrinsic text measurement padding must resolve to pixels or rems".to_owned());
    }
    let extra_width = number_prop(props, "extra_width").unwrap_or(0.0);
    if !extra_width.is_finite() || !(0.0..=MAX_COLUMN_WIDTH).contains(&extra_width) {
        return Err(
            "intrinsic text measurement extra width must be between 0 and 16384".to_owned(),
        );
    }
    Ok(IntrinsicMeasureConfig {
        text,
        group: group.id().clone(),
        horizontal_padding,
        extra_width,
    })
}

#[derive(Default)]
pub struct ColumnResizePrimitiveHandler {
    measurements: ColumnMeasurementRegistry,
}

impl ColumnResizePrimitiveHandler {
    pub(crate) fn new(measurements: ColumnMeasurementRegistry) -> Self {
        Self { measurements }
    }
}

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
            measurements: self.measurements.clone(),
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
        effect: None,
    }
}

/// Build the internal intrinsic-text measurement primitive used by Table
/// auto-fit. The primitive is zero-size and reports only within Rust.
///
/// # Panics
///
/// Panics only if the static built-in primitive ID becomes invalid.
#[must_use]
pub(crate) fn intrinsic_text_measure_primitive_descriptor() -> PrimitiveDescriptor {
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.intrinsic_text_measure").expect("static primitive ID"),
        export: "IntrinsicTextMeasurePrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "text".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "group".to_owned(),
                ObjectField::required(ValueSchema::Signal),
            ),
            (
                "horizontal_padding".to_owned(),
                ObjectField::optional(ValueSchema::Length),
            ),
            (
                "extra_width".to_owned(),
                ObjectField::optional(ValueSchema::Number {
                    min: Some(0.0),
                    max: Some(MAX_COLUMN_WIDTH),
                    exclusive_min: None,
                    exclusive_max: None,
                }),
            ),
        ]),
        events: BTreeMap::new(),
        state: ComponentStateSchema::default(),
        lifecycle: true,
        effect: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measurement_group() -> SignalId {
        SignalId::new(
            crate::ComponentInstancePath::root("Table", "users"),
            "column-width-0",
            SignalKind::OptionalFloat,
        )
        .unwrap()
    }

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

    #[test]
    fn realized_measurements_replace_and_unmount_without_retaining_stale_maxima() {
        let registry = ColumnMeasurementRegistry::default();
        let group = measurement_group();
        registry.report(&group, 1, 80.0);
        registry.report(&group, 2, 144.0);
        assert_eq!(registry.maximum(&group), Some(144.0));

        registry.report(&group, 2, 96.0);
        assert_eq!(registry.maximum(&group), Some(96.0));
        registry.remove(&group, 2);
        assert_eq!(registry.maximum(&group), Some(80.0));
        registry.remove(&group, 1);
        assert_eq!(registry.maximum(&group), None);
    }

    #[test]
    fn intrinsic_measurement_is_a_keyed_lifecycle_primitive() {
        let descriptor = intrinsic_text_measure_primitive_descriptor();
        assert!(descriptor.lifecycle);
        assert_eq!(descriptor.id.as_str(), "gpui_rhai.intrinsic_text_measure");
    }
}
