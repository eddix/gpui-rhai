//! Generic retained single-value range-input behavior for styled Rhai components.

use std::collections::BTreeMap;

use gpui::{
    AnyElement, App, AppContext, Bounds, Context, CursorStyle, Element, ElementId, Entity,
    FocusHandle, GlobalElementId, InspectorElementId, InteractiveElement, IntoElement,
    KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, Point, Render, Styled, Window, div, px, relative,
};

use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveDescriptor, PrimitiveEventEmitter,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId, PrimitiveProps,
    PrimitiveTheme, PrimitiveValue, Style, TextDirection, UiValue, ValueSchema,
};

#[derive(Clone)]
struct RangeInputConfig {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    orientation: RangeOrientation,
    disabled: bool,
    track_style: Style,
    fill_style: Style,
    thumb_style: Style,
    theme: PrimitiveTheme,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RangeOrientation {
    Horizontal,
    Vertical,
}

struct RangeInputEntity {
    focus: FocusHandle,
    controlled: f64,
    preview: f64,
    min: f64,
    max: f64,
    step: f64,
    orientation: RangeOrientation,
    disabled: bool,
    dragging: bool,
    bounds: Option<Bounds<Pixels>>,
    events: PrimitiveEventEmitter,
    track_style: Style,
    fill_style: Style,
    thumb_style: Style,
    theme: PrimitiveTheme,
}

impl RangeInputEntity {
    fn new(
        config: RangeInputConfig,
        events: PrimitiveEventEmitter,
        cx: &mut Context<Self>,
    ) -> Self {
        let value = normalize_value(config.value, config.min, config.max, config.step);
        Self {
            focus: cx.focus_handle(),
            controlled: value,
            preview: value,
            min: config.min,
            max: config.max,
            step: config.step,
            orientation: config.orientation,
            disabled: config.disabled,
            dragging: false,
            bounds: None,
            events,
            track_style: config.track_style,
            fill_style: config.fill_style,
            thumb_style: config.thumb_style,
            theme: config.theme,
        }
    }

    fn update_props(
        &mut self,
        config: RangeInputConfig,
        events: PrimitiveEventEmitter,
        cx: &mut Context<Self>,
    ) {
        self.min = config.min;
        self.max = config.max;
        self.step = config.step;
        self.orientation = config.orientation;
        self.disabled = config.disabled;
        self.controlled = normalize_value(config.value, config.min, config.max, config.step);
        if !self.dragging || self.disabled {
            self.preview = self.controlled;
        }
        if self.disabled {
            self.dragging = false;
        }
        self.focus = self.focus.clone().tab_stop(!self.disabled);
        self.events = events;
        self.track_style = config.track_style;
        self.fill_style = config.fill_style;
        self.thumb_style = config.thumb_style;
        self.theme = config.theme;
        cx.notify();
    }

    fn ratio(&self) -> f64 {
        ((self.preview - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    fn value_at(&self, position: Point<Pixels>) -> f64 {
        let Some(bounds) = self.bounds else {
            return self.preview;
        };
        let ratio = match self.orientation {
            RangeOrientation::Horizontal => {
                let width = f64::from(bounds.size.width).max(f64::EPSILON);
                let ratio =
                    ((f64::from(position.x) - f64::from(bounds.origin.x)) / width).clamp(0.0, 1.0);
                if self.theme.direction() == TextDirection::RightToLeft {
                    1.0 - ratio
                } else {
                    ratio
                }
            }
            RangeOrientation::Vertical => {
                let height = f64::from(bounds.size.height).max(f64::EPSILON);
                1.0 - ((f64::from(position.y) - f64::from(bounds.origin.y)) / height)
                    .clamp(0.0, 1.0)
            }
        };
        normalize_value(
            self.min + ratio * (self.max - self.min),
            self.min,
            self.max,
            self.step,
        )
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.focus.focus(window);
        self.dragging = true;
        self.preview = self.value_at(event.position);
        cx.notify();
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.dragging && !self.disabled {
            self.preview = self.value_at(event.position);
            cx.notify();
        }
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dragging || self.disabled {
            return;
        }
        self.preview = self.value_at(event.position);
        self.dragging = false;
        self.emit_change(self.preview, window, cx);
        cx.notify();
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let key = event.keystroke.key.as_str();
        let direction = self.theme.direction();
        let delta = match (self.orientation, key, direction) {
            (RangeOrientation::Horizontal, "left", TextDirection::LeftToRight)
            | (RangeOrientation::Horizontal, "right", TextDirection::RightToLeft)
            | (RangeOrientation::Vertical, "down", _) => Some(-self.step),
            (RangeOrientation::Horizontal, "right", TextDirection::LeftToRight)
            | (RangeOrientation::Horizontal, "left", TextDirection::RightToLeft)
            | (RangeOrientation::Vertical, "up", _) => Some(self.step),
            _ => None,
        };
        let next = if key == "home" {
            Some(self.min)
        } else if key == "end" {
            Some(self.max)
        } else {
            delta.map(|delta| self.preview + delta)
        };
        if let Some(next) = next {
            self.preview = normalize_value(next, self.min, self.max, self.step);
            self.emit_change(self.preview, window, cx);
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn emit_change(&self, value: f64, window: &mut Window, cx: &mut Context<Self>) {
        let events = self.events.clone();
        window.defer(cx, move |window, cx| {
            let _ = events.emit("change", UiValue::Float(value), window, cx);
        });
    }
}

impl Render for RangeInputEntity {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ratio = self.ratio();
        let direction = self.theme.direction();
        let mut fill = crate::renderer::apply_style_override(
            div().absolute(),
            &self.fill_style,
            &self.theme,
            direction,
        );
        let mut thumb = crate::renderer::apply_style_override(
            div().absolute(),
            &self.thumb_style,
            &self.theme,
            direction,
        );
        let track = crate::renderer::apply_style_override(
            div().relative(),
            &self.track_style,
            &self.theme,
            direction,
        );
        let track = match self.orientation {
            RangeOrientation::Horizontal => {
                let visual_ratio = horizontal_thumb_ratio(ratio, direction);
                fill = fill.top(px(0.0)).bottom(px(0.0));
                fill = if direction == TextDirection::RightToLeft {
                    fill.right(px(0.0)).w(relative(fraction_f32(ratio)))
                } else {
                    fill.left(px(0.0)).w(relative(fraction_f32(ratio)))
                };
                thumb = thumb
                    .left(relative(fraction_f32(visual_ratio)))
                    .top(relative(0.5))
                    .ml(px(-7.0))
                    .mt(px(-7.0));
                track.child(fill).child(thumb)
            }
            RangeOrientation::Vertical => {
                fill = fill
                    .left(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .h(relative(fraction_f32(ratio)));
                thumb = thumb
                    .bottom(relative(fraction_f32(ratio)))
                    .left(relative(0.5))
                    .mb(px(-7.0))
                    .ml(px(-7.0));
                track.child(fill).child(thumb)
            }
        };
        div()
            .id("gpui-rhai-range-input")
            .relative()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.focus)
            .tab_stop(!self.disabled)
            .cursor(if self.disabled {
                CursorStyle::OperationNotAllowed
            } else {
                CursorStyle::PointingHand
            })
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .child(track)
            .child(RangeBoundsRecorder { input: cx.entity() })
            .opacity(if self.disabled { 0.62 } else { 1.0 })
    }
}

struct RangeBoundsRecorder {
    input: Entity<RangeInputEntity>,
}

impl Element for RangeBoundsRecorder {
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
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child = div()
            .absolute()
            .top(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .left(px(0.0))
            .into_any_element();
        let layout = child.request_layout(window, cx);
        (layout, child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.prepaint(window, cx);
        self.input
            .update(cx, |input, _| input.bounds = Some(bounds));
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        (): &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}

impl IntoElement for RangeBoundsRecorder {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[derive(Default)]
pub struct RangeInputPrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, Entity<RangeInputEntity>>,
}

impl PrimitiveHandler for RangeInputPrimitiveHandler {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        _: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, String> {
        let id = instance
            .id
            .clone()
            .ok_or_else(|| "RangeInputPrimitive requires a stable key".to_owned())?;
        let config = parse_config(&instance.node.props, theme)?;
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let events = events.clone();
            let entity = cx.new(|cx| RangeInputEntity::new(config.clone(), events, cx));
            self.instances.insert(id, entity.clone());
            entity
        };
        entity.update(cx, |input, cx| {
            input.update_props(config, events.clone(), cx);
        });
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

fn parse_config(
    props: &PrimitiveProps,
    theme: &PrimitiveTheme,
) -> Result<RangeInputConfig, String> {
    let value = number_prop(props, "value").ok_or_else(|| "range value is required".to_owned())?;
    let min = number_prop(props, "min").unwrap_or(0.0);
    let max = number_prop(props, "max").unwrap_or(100.0);
    let step = number_prop(props, "step").unwrap_or(1.0);
    if !value.is_finite() || !min.is_finite() || !max.is_finite() || !step.is_finite() {
        return Err("range values must be finite".to_owned());
    }
    if max <= min {
        return Err("range max must be greater than min".to_owned());
    }
    if step <= 0.0 || step > max - min {
        return Err("range step must be positive and no larger than max - min".to_owned());
    }
    let orientation = match string_prop(props, "orientation").as_deref() {
        None | Some("horizontal") => RangeOrientation::Horizontal,
        Some("vertical") => RangeOrientation::Vertical,
        Some(other) => return Err(format!("unknown range orientation `{other}`")),
    };
    Ok(RangeInputConfig {
        value,
        min,
        max,
        step,
        orientation,
        disabled: bool_prop(props, "disabled").unwrap_or(false),
        track_style: style_prop(props, "track_style"),
        fill_style: style_prop(props, "fill_style"),
        thumb_style: style_prop(props, "thumb_style"),
        theme: theme.clone(),
    })
}

fn normalize_value(value: f64, min: f64, max: f64, step: f64) -> f64 {
    let snapped = min + ((value.clamp(min, max) - min) / step).round() * step;
    snapped.clamp(min, max)
}

fn horizontal_thumb_ratio(ratio: f64, direction: TextDirection) -> f64 {
    if direction == TextDirection::RightToLeft {
        1.0 - ratio
    } else {
        ratio
    }
}

fn fraction_f32(value: f64) -> f32 {
    value.to_string().parse().unwrap_or(0.0)
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

fn style_prop(props: &PrimitiveProps, name: &str) -> Style {
    match props.get(name) {
        Some(PrimitiveValue::Style(style)) => (**style).clone(),
        _ => Style::new(),
    }
}

/// Build the compile-time generic range-input primitive schema.
///
/// # Panics
///
/// Panics only if the static built-in primitive ID becomes invalid.
#[must_use]
pub fn range_input_primitive_descriptor() -> PrimitiveDescriptor {
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.range_input").expect("static primitive ID"),
        export: "RangeInputPrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "value".to_owned(),
                ObjectField::required(ValueSchema::number()),
            ),
            (
                "min".to_owned(),
                ObjectField::required(ValueSchema::number()),
            ),
            (
                "max".to_owned(),
                ObjectField::required(ValueSchema::number()),
            ),
            (
                "step".to_owned(),
                ObjectField::required(ValueSchema::number()),
            ),
            (
                "orientation".to_owned(),
                ObjectField::required(ValueSchema::String {
                    allowed: vec!["horizontal".to_owned(), "vertical".to_owned()],
                }),
            ),
            (
                "disabled".to_owned(),
                ObjectField::optional(ValueSchema::Bool).with_default(UiValue::Bool(false)),
            ),
            (
                "track_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "fill_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "thumb_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "on_change".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
            ),
        ]),
        events: BTreeMap::from([(
            "change".to_owned(),
            EventSchema {
                payload: ValueSchema::number(),
            },
        )]),
        state: ComponentStateSchema::default(),
        lifecycle: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_values_clamp_and_snap_to_step() {
        assert!(normalize_value(-2.0, 0.0, 10.0, 0.5).abs() < f64::EPSILON);
        assert!((normalize_value(3.26, 0.0, 10.0, 0.5) - 3.5).abs() < f64::EPSILON);
        assert!((normalize_value(12.0, 0.0, 10.0, 0.5) - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn horizontal_rtl_geometry_keeps_the_fill_between_minimum_and_thumb() {
        let ratio = 0.2;
        let ltr_thumb = horizontal_thumb_ratio(ratio, TextDirection::LeftToRight);
        let rtl_thumb = horizontal_thumb_ratio(ratio, TextDirection::RightToLeft);
        assert!((ltr_thumb - 0.2_f64).abs() < f64::EPSILON);
        assert!((rtl_thumb - 0.8_f64).abs() < f64::EPSILON);
        // The LTR fill occupies [0, thumb]. The RTL fill is right-anchored and
        // occupies [thumb, 1], so both represent the same semantic value.
        assert!((ratio - (1.0 - rtl_thumb)).abs() < f64::EPSILON);
    }
}
