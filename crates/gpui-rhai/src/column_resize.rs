//! Native hot-lane column resize handle used by the Rhai Table component.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, CursorStyle, DispatchPhase, Element, ElementId, GlobalElementId,
    Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    Pixels, SharedString, Style, Window, fill, point, px, relative, rgba, size,
};

use crate::interaction::{GestureUpdate, InteractionFlow, ResizeGestureSpace};
use crate::{
    ComponentStateSchema, EventSchema, Length, NativeSignal, ObjectField, PrimitiveContext,
    PrimitiveDescriptor, PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId,
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
    direction: TextDirection,
}

impl ResizeState {
    fn new(config: &ResizeConfig) -> Self {
        Self(Rc::new(RefCell::new(ResizeStateInner {
            source: config.source.clone(),
            signal: config.signal.id().clone(),
            direction: config.direction,
        })))
    }
}

struct ResizePrepaint {
    hitbox: Hitbox,
}

struct ColumnResizeHandle {
    config: ResizeConfig,
    events: PrimitiveContext,
    measurements: ColumnMeasurementRegistry,
}

#[cfg(feature = "vendor-gpui")]
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
        let (reset, axes_changed) = {
            let mut inner = state.0.borrow_mut();
            let changed =
                inner.source != self.config.source || inner.signal != *self.config.signal.id();
            let axes_changed = inner.direction != self.config.direction;
            inner.direction = self.config.direction;
            if changed {
                inner.source = self.config.source.clone();
                inner.signal = self.config.signal.id().clone();
            }
            (changed, axes_changed)
        };
        let owner = self.events.interaction_owner(&self.config.id);
        let canceled =
            (reset || axes_changed) && self.events.cancel_interaction(&owner, window, cx);
        if reset && !canceled {
            let events = self.events.clone();
            let signal = self.config.signal.clone();
            window.defer(cx, move |_, cx| {
                let _ = events.write_signal(&signal, SignalValue::OptionalFloat(None), cx);
            });
        }
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::BlockMouseExceptScroll);
        ResizePrepaint { hitbox }
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
        let owner = self.events.interaction_owner(&self.config.id);
        self.events.present_interaction(owner.clone());
        let dragged = self.events.interaction_is_active(&owner);
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
        register_pointer_down(
            prepaint,
            self.config.clone(),
            self.events.clone(),
            self.measurements.clone(),
            window,
        );
    }
}

fn register_pointer_down(
    prepaint: &ResizePrepaint,
    config: ResizeConfig,
    events: PrimitiveContext,
    measurements: ColumnMeasurementRegistry,
    window: &mut Window,
) {
    let view = window.current_view();
    let hitbox = prepaint.hitbox.clone();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble
            || event.button != MouseButton::Left
            || !hitbox.is_hovered(window)
        {
            return;
        }
        let Some(header) = events.element_geometry(&config.reference, cx) else {
            return;
        };
        if event.click_count >= 2 {
            let owner = events.interaction_owner(&config.id);
            events.cancel_interaction(&owner, window, cx);
            let measured = measurements
                .maximum(config.signal.id())
                .unwrap_or(header.layout.width);
            let width = clamp_width(measured, config.min, config.max);
            let _ =
                events.write_signal(&config.signal, SignalValue::OptionalFloat(Some(width)), cx);
            let payload = resize_payload(&config.column_key, width);
            events.propose("resize", payload, window, cx);
            if events.observes("resize") {
                schedule_preview_clear(&events, &config.signal, window, cx);
            }
            cx.stop_propagation();
            cx.notify(view);
            return;
        }
        if let Some(drag) = ColumnDrag::start(&events, &config, header, cx) {
            drag.arm(&events, &config, event.position, view, window, cx);
            cx.stop_propagation();
        }
    });
}

#[derive(Clone)]
struct ColumnDrag {
    space: ResizeGestureSpace,
    width: f64,
    scroll_x: f64,
    previous_override: SignalValue,
}

impl ColumnDrag {
    fn start(
        events: &PrimitiveContext,
        config: &ResizeConfig,
        header: crate::PresentedGeometry,
        cx: &App,
    ) -> Option<Self> {
        let (boundary, scroll_x) = match events.table_resize_viewport() {
            Some(viewport) => (
                viewport.presented()?,
                viewport.logical_scroll_x(config.direction),
            ),
            None => (standalone_resize_axes(header)?, 0.),
        };
        Some(Self {
            space: ResizeGestureSpace::new(boundary)?,
            width: clamp_width(header.layout.width, config.min, config.max),
            scroll_x,
            previous_override: events
                .read_signal(&config.signal, cx)
                .unwrap_or(SignalValue::OptionalFloat(None)),
        })
    }

    fn proposed_width(
        &self,
        gesture: GestureUpdate,
        events: &PrimitiveContext,
        config: &ResizeConfig,
        cx: &App,
    ) -> Option<f64> {
        let boundary = if let Some(viewport) = events.table_resize_viewport() {
            if (viewport.logical_scroll_x(config.direction) - self.scroll_x).abs() > 0.5 {
                return None;
            }
            viewport.presented()
        } else {
            events
                .element_geometry(&config.reference, cx)
                .and_then(standalone_resize_axes)
        };
        let (dx, _) = self.space.displacement(boundary, gesture)?;
        Some(clamp_width(
            self.width + logical_delta(dx, config.direction),
            config.min,
            config.max,
        ))
    }

    fn arm(
        self,
        events: &PrimitiveContext,
        config: &ResizeConfig,
        origin: gpui::Point<Pixels>,
        view: gpui::EntityId,
        window: &mut Window,
        cx: &mut App,
    ) {
        let update_drag = self.clone();
        let update_config = config.clone();
        let update_events = events.clone();
        let update = move |gesture: GestureUpdate, _: &mut Window, cx: &mut App| {
            let Some(width) =
                update_drag.proposed_width(gesture, &update_events, &update_config, cx)
            else {
                return InteractionFlow::Cancel;
            };
            if gesture.moved() {
                write_width(
                    &update_events,
                    &update_config.signal,
                    SignalValue::OptionalFloat(Some(width)),
                    cx,
                );
            }
            InteractionFlow::Continue
        };
        let finish_config = config.clone();
        let finish_events = events.clone();
        let cancel_signal = config.signal.clone();
        let cancel_events = events.clone();
        let rollback = self.previous_override.clone();
        let finish = move |gesture: GestureUpdate, window: &mut Window, cx: &mut App| {
            if !gesture.moved() {
                return;
            }
            let Some(width) = self.proposed_width(gesture, &finish_events, &finish_config, cx)
            else {
                write_width(
                    &finish_events,
                    &finish_config.signal,
                    self.previous_override.clone(),
                    cx,
                );
                return;
            };
            write_width(
                &finish_events,
                &finish_config.signal,
                SignalValue::OptionalFloat(Some(width)),
                cx,
            );
            finish_events.propose(
                "resize",
                resize_payload(&finish_config.column_key, width),
                window,
                cx,
            );
            if finish_events.observes("resize") {
                schedule_preview_clear(&finish_events, &finish_config.signal, window, cx);
            }
        };
        let cancel = move |_: &mut Window, cx: &mut App| {
            write_width(&cancel_events, &cancel_signal, rollback.clone(), cx);
        };
        let owner = events.interaction_owner(&config.id);
        events.begin_interaction(
            crate::interaction::NativeGesture::new(owner, origin, view, update, finish, cancel),
            window,
            cx,
        );
    }
}

fn write_width(events: &PrimitiveContext, signal: &NativeSignal, value: SignalValue, cx: &mut App) {
    let _ = events.write_signal(signal, value, cx);
}

/// Direct users of the primitive have no Table shell. Keep the header
/// reference's native axes, but not its moving origin or changing width.
fn standalone_resize_axes(
    mut header: crate::PresentedGeometry,
) -> Option<crate::PresentedGeometry> {
    let (xx, xy, yx, yy, _, _) = header.local_to_window.components();
    header.local_to_window = crate::Affine2D::new(xx, xy, yx, yy, 0., 0.).ok()?;
    header.window_to_local = header.local_to_window.inverse();
    header.layout.width = 0.;
    Some(header)
}

fn schedule_preview_clear(
    events: &PrimitiveContext,
    signal: &crate::NativeSignal,
    window: &mut Window,
    cx: &mut App,
) {
    let events = events.clone();
    let signal = signal.clone();
    window.defer(cx, move |_, cx| {
        let _ = events.write_signal(&signal, SignalValue::OptionalFloat(None), cx);
    });
}

fn logical_delta(delta: f64, direction: TextDirection) -> f64 {
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
    extra_width: Length,
    inline_border_width: Length,
}

struct IntrinsicTextMeasureElement {
    instance: u64,
    config: IntrinsicMeasureConfig,
    measurements: ColumnMeasurementRegistry,
}

#[cfg(feature = "vendor-gpui")]
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
            let Some(extra) = absolute_length_pixels(self.config.extra_width, window)
                .filter(|width| width.is_finite() && (0.0..=MAX_COLUMN_WIDTH).contains(width))
            else {
                return;
            };
            let Some(border) = absolute_length_pixels(self.config.inline_border_width, window)
                .filter(|width| width.is_finite() && (0.0..=MAX_COLUMN_WIDTH).contains(width))
            else {
                return;
            };
            let width = (text_width + padding * 2.0 + extra + border).ceil();
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
        Length::Relative(_) | Length::Token(_) => None,
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
        _: &PrimitiveContext,
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
    let text = props
        .string("text")
        .map(ToOwned::to_owned)
        .ok_or_else(|| "intrinsic text measurement requires text".to_owned())?;
    let group = props
        .signal("group")
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
    if !matches!(horizontal_padding, Length::Pixels(value) | Length::Rems(value) if value.is_finite() && (0.0..=MAX_COLUMN_WIDTH).contains(&value))
    {
        return Err(
            "intrinsic text measurement padding must resolve to pixels or rems between 0 and 16384"
                .to_owned(),
        );
    }
    let extra_width = match props.get("extra_width") {
        Some(PrimitiveValue::Length(value)) => theme
            .resolve_length(*value)
            .ok_or_else(|| "intrinsic text measurement extra width did not resolve".to_owned())?,
        None | Some(PrimitiveValue::Data(UiValue::Null)) => Length::Pixels(0.0),
        Some(PrimitiveValue::Data(UiValue::Integer(_) | UiValue::Float(_))) => Length::Pixels(
            props
                .number("extra_width")
                .ok_or("extra width must be numeric")?,
        ),
        Some(_) => {
            return Err(
                "intrinsic text measurement extra width must be a number or Length".to_owned(),
            );
        }
    };
    if !matches!(extra_width,Length::Pixels(value)|Length::Rems(value) if value.is_finite() && (0.0..=MAX_COLUMN_WIDTH).contains(&value))
    {
        return Err(
            "intrinsic text measurement extra width must be between 0 and 16384".to_owned(),
        );
    }
    let inline_border_width = match props.get("inline_border_width") {
        Some(PrimitiveValue::Length(value)) => theme
            .resolve_length(*value)
            .ok_or_else(|| "intrinsic text measurement border width did not resolve".to_owned())?,
        None | Some(PrimitiveValue::Data(UiValue::Null)) => Length::Pixels(0.0),
        Some(_) => return Err("intrinsic text measurement border width must be Length".to_owned()),
    };
    if !matches!(inline_border_width,Length::Pixels(value)|Length::Rems(value) if value.is_finite() && (0.0..=MAX_COLUMN_WIDTH).contains(&value))
    {
        return Err("intrinsic text measurement border width must resolve to pixels or rems between 0 and 16384".to_owned());
    }
    Ok(IntrinsicMeasureConfig {
        text,
        group: group.id().clone(),
        horizontal_padding,
        extra_width,
        inline_border_width,
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
        events: &PrimitiveContext,
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
    let column_key = props
        .string("column_key")
        .map(ToOwned::to_owned)
        .ok_or_else(|| "column resize handle requires column_key".to_owned())?;
    let source_kind = props
        .string("source_kind")
        .map(ToOwned::to_owned)
        .ok_or_else(|| "column resize handle requires source_kind".to_owned())?;
    let source_value = props
        .number("source_value")
        .ok_or_else(|| "column resize handle requires source_value".to_owned())?;
    let min = props.number("min_width").unwrap_or(DEFAULT_MIN_WIDTH);
    let max = props.number("max_width").unwrap_or(MAX_COLUMN_WIDTH);
    if !source_value.is_finite()
        || !min.is_finite()
        || !max.is_finite()
        || min <= 0.0
        || max < min
        || max > MAX_COLUMN_WIDTH
    {
        return Err("column resize widths must be finite and satisfy 0 < min <= max".to_owned());
    }
    let signal = props
        .signal("signal")
        .cloned()
        .ok_or_else(|| "column resize handle requires signal".to_owned())?;
    if signal.id().kind() != SignalKind::OptionalFloat {
        return Err("column resize signal must be optional_float".to_owned());
    }
    let reference = props
        .element_ref("column_ref")
        .cloned()
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
                ObjectField::required(ValueSchema::string()).with_doc(
                    "Key of the column this handle resizes; it is `key` in the `resize` payload.",
                ),
            ),
            (
                "source_kind".to_owned(),
                ObjectField::required(ValueSchema::String {
                    allowed: vec!["fixed".to_owned(), "percent".to_owned(), "flex".to_owned()],
                })
                .with_doc(
                    "Kind of the column's controlled width; with `source_value` it identifies the source, and a change cancels a drag.",
                ),
            ),
            (
                "source_value".to_owned(),
                ObjectField::required(ValueSchema::positive_number()).with_doc(
                    "Value of the column's controlled width; a change cancels a drag and clears the preview.",
                ),
            ),
            (
                "min_width".to_owned(),
                ObjectField::optional(resize_bound_schema()).with_doc(
                    "Smallest width a drag or auto-fit may propose, in logical pixels; defaults to 48.",
                ),
            ),
            (
                "max_width".to_owned(),
                ObjectField::optional(resize_bound_schema()).with_doc(
                    "Largest width a drag or auto-fit may propose, in logical pixels; defaults to 16,384.",
                ),
            ),
            (
                "signal".to_owned(),
                ObjectField::required(ValueSchema::Signal).with_doc(
                    "Optional-float signal for the previewed width, kept after release without `on_resize`; text measured with it as `group` sets auto-fit.",
                ),
            ),
            (
                "column_ref".to_owned(),
                ObjectField::required(ValueSchema::Ref).with_doc(
                    "Ref to the column's header cell; a drag starts from its width, and auto-fit uses it when no text was measured.",
                ),
            ),
            (
                "on_resize".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)).with_doc(
                    "Called with the proposed fixed width when a drag ends or a double-click auto-fits the column.",
                ),
            ),
        ]),
        events: BTreeMap::from([(
            "resize".to_owned(),
            EventSchema {
                doc: Some(
                    "Emitted when a drag ends or a double-click auto-fits the column; the payload is `key` and a `fixed` `width` in logical pixels."
                        .to_owned(),
                ),
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
                ObjectField::required(ValueSchema::string()).with_doc(
                    "Text measured as one line in the text style inherited here; place it inside the cell it measures.",
                ),
            ),
            (
                "group".to_owned(),
                ObjectField::required(ValueSchema::Signal).with_doc(
                    "The column's optional-float signal, shared with its `ColumnResizePrimitive`; auto-fit takes the widest measure.",
                ),
            ),
            (
                "horizontal_padding".to_owned(),
                ObjectField::optional(ValueSchema::Length).with_doc(
                    "Padding added on both sides of the text, in pixels or rems, such as the cell's inset; 0 when omitted.",
                ),
            ),
            (
                "extra_width".to_owned(),
                ObjectField::optional(ValueSchema::one_of([ValueSchema::Length,ValueSchema::Number {
                    min: Some(0.0),
                    max: Some(MAX_COLUMN_WIDTH),
                    exclusive_min: None,
                    exclusive_max: None,
                }]))
                .with_doc(
                    "Additional width added once, as logical pixels or an absolute Length (including late-bound theme/reading-scale lengths); 0 when omitted.",
                ),
            ),
            (
                "inline_border_width".to_owned(),
                ObjectField::optional(ValueSchema::Length).with_doc(
                    "Total inline border width, added once after padding and extra content; pixels or rems, including late-bound theme lengths; 0 when omitted.",
                ),
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
    fn the_preview_signal_must_be_an_optional_float() {
        let component = crate::ComponentInstancePath::root("Table", "users");
        let props = |kind| {
            PrimitiveProps::new()
                .with(
                    "column_key",
                    PrimitiveValue::Data(UiValue::String("name".to_owned())),
                )
                .with(
                    "source_kind",
                    PrimitiveValue::Data(UiValue::String("fixed".to_owned())),
                )
                .with("source_value", PrimitiveValue::Data(UiValue::Float(120.0)))
                .with(
                    "signal",
                    PrimitiveValue::Signal(crate::NativeSignal::new(
                        SignalId::new(component.clone(), "width", kind).unwrap(),
                    )),
                )
                .with(
                    "column_ref",
                    PrimitiveValue::Ref(crate::ElementRef::new(
                        crate::ElementRefId::new(component.clone(), "header").unwrap(),
                    )),
                )
        };
        let theme = PrimitiveTheme::default();
        assert!(parse_config(&props(SignalKind::OptionalFloat), &theme).is_ok());
        assert_eq!(
            parse_config(&props(SignalKind::Float), &theme).err(),
            Some("column resize signal must be optional_float".to_owned())
        );
    }

    #[test]
    fn width_delta_and_clamping_follow_text_direction() {
        assert!((logical_delta(32.0, TextDirection::LeftToRight) - 32.0).abs() < f64::EPSILON);
        assert!((logical_delta(32.0, TextDirection::RightToLeft) + 32.0).abs() < f64::EPSILON);
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

    #[test]
    fn intrinsic_measurement_preserves_absolute_chrome_lengths() {
        let props = PrimitiveProps::new()
            .with("text", PrimitiveValue::Data(UiValue::String("Name".into())))
            .with(
                "group",
                PrimitiveValue::Signal(crate::NativeSignal::new(measurement_group())),
            )
            .with(
                "horizontal_padding",
                PrimitiveValue::Length(Length::Rems(0.75)),
            )
            .with("extra_width", PrimitiveValue::Length(Length::Rems(1.1875)))
            .with(
                "inline_border_width",
                PrimitiveValue::Length(Length::Rems(0.0625)),
            );
        let parsed = parse_measure_config(&props, &PrimitiveTheme::default()).unwrap();
        assert_eq!(parsed.horizontal_padding, Length::Rems(0.75));
        assert_eq!(parsed.extra_width, Length::Rems(1.1875));
        assert_eq!(parsed.inline_border_width, Length::Rems(0.0625));
        for (name, value, expected_ok) in [
            (
                "extra_width",
                PrimitiveValue::Data(UiValue::Float(19.0)),
                true,
            ),
            (
                "extra_width",
                PrimitiveValue::Data(UiValue::Float(f64::NAN)),
                false,
            ),
            (
                "extra_width",
                PrimitiveValue::Length(Length::Relative(0.5)),
                false,
            ),
            (
                "inline_border_width",
                PrimitiveValue::Length(Length::Pixels(-1.0)),
                false,
            ),
            (
                "horizontal_padding",
                PrimitiveValue::Length(Length::Pixels(f64::INFINITY)),
                false,
            ),
        ] {
            assert_eq!(
                parse_measure_config(&props.clone().with(name, value), &PrimitiveTheme::default())
                    .is_ok(),
                expected_ok,
                "{name}"
            );
        }
    }
}
