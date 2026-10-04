//! Controlled two-thumb range slider using the shared range axis math.

use std::collections::BTreeMap;

use gpui::{
    AnyElement, App, AppContext, Bounds, Context, CursorStyle, Element, ElementId, Entity,
    FocusHandle, GlobalElementId, InspectorElementId, InteractiveElement, IntoElement,
    KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render,
    Role, StatefulInteractiveElement, Styled, Window, div, px, relative,
};

use crate::range_input::{
    RangeOrientation, fraction_f32, horizontal_thumb_ratio, normalize_value, range_value_at,
};
use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveContext, PrimitiveDescriptor,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId, PrimitiveProps,
    PrimitiveTheme, Style, TextDirection, UiValue, ValueSchema,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RangeThumb {
    Low,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RangePair {
    low: f64,
    high: f64,
}

#[derive(Clone)]
struct RangeSliderConfig {
    values: RangePair,
    min: f64,
    max: f64,
    step: f64,
    minimum_gap: f64,
    orientation: RangeOrientation,
    disabled: bool,
    low_label: String,
    high_label: String,
    track_style: Style,
    fill_style: Style,
    thumb_style: Style,
    theme: PrimitiveTheme,
}

struct RangeSliderEntity {
    low_focus: FocusHandle,
    high_focus: FocusHandle,
    controlled: RangePair,
    preview: RangePair,
    min: f64,
    max: f64,
    step: f64,
    minimum_gap: f64,
    orientation: RangeOrientation,
    disabled: bool,
    dragging: bool,
    active: RangeThumb,
    bounds: Option<Bounds<Pixels>>,
    events: PrimitiveContext,
    interaction_key: String,
    low_label: String,
    high_label: String,
    track_style: Style,
    fill_style: Style,
    thumb_style: Style,
    theme: PrimitiveTheme,
    revision: u64,
}

impl RangeSliderEntity {
    fn new(
        config: RangeSliderConfig,
        events: PrimitiveContext,
        interaction_key: String,
        cx: &mut Context<Self>,
    ) -> Self {
        let values = normalize_pair(config.values, &config);
        Self {
            low_focus: cx.focus_handle(),
            high_focus: cx.focus_handle(),
            controlled: values,
            preview: values,
            min: config.min,
            max: config.max,
            step: config.step,
            minimum_gap: config.minimum_gap,
            orientation: config.orientation,
            disabled: config.disabled,
            dragging: false,
            active: RangeThumb::Low,
            bounds: None,
            events,
            interaction_key,
            low_label: config.low_label,
            high_label: config.high_label,
            track_style: config.track_style,
            fill_style: config.fill_style,
            thumb_style: config.thumb_style,
            theme: config.theme,
            revision: 1,
        }
    }

    fn update_props(
        &mut self,
        config: RangeSliderConfig,
        events: PrimitiveContext,
        cx: &mut Context<Self>,
    ) -> bool {
        let values = normalize_pair(config.values, &config);
        let changed = self.controlled != values
            || self.min.to_bits() != config.min.to_bits()
            || self.max.to_bits() != config.max.to_bits()
            || self.step.to_bits() != config.step.to_bits()
            || self.minimum_gap.to_bits() != config.minimum_gap.to_bits()
            || self.orientation != config.orientation
            || self.disabled != config.disabled;
        if changed {
            self.revision = self.revision.saturating_add(1);
            self.preview = values;
            self.dragging = false;
        }
        self.controlled = values;
        if !self.dragging || config.disabled {
            self.preview = values;
        }
        self.min = config.min;
        self.max = config.max;
        self.step = config.step;
        self.minimum_gap = config.minimum_gap;
        self.orientation = config.orientation;
        self.disabled = config.disabled;
        self.dragging &= !config.disabled;
        self.events = events;
        self.low_label = config.low_label;
        self.high_label = config.high_label;
        self.track_style = config.track_style;
        self.fill_style = config.fill_style;
        self.thumb_style = config.thumb_style;
        self.theme = config.theme;
        cx.notify();
        changed
    }

    fn ratio(&self, value: f64) -> f64 {
        ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    fn value_at(&self, position: Point<Pixels>) -> f64 {
        self.bounds.map_or(self.preview.low, |bounds| {
            range_value_at(
                bounds,
                position,
                self.orientation,
                self.theme.direction(),
                self.min,
                self.max,
                self.step,
            )
        })
    }

    fn nearest_thumb(&self, value: f64) -> RangeThumb {
        let low_distance = (value - self.preview.low).abs();
        let high_distance = (value - self.preview.high).abs();
        if low_distance < high_distance {
            RangeThumb::Low
        } else if high_distance < low_distance {
            RangeThumb::High
        } else {
            self.active
        }
    }

    fn set_active_value(&mut self, value: f64) {
        match self.active {
            RangeThumb::Low => {
                self.preview.low = normalize_constrained_value(
                    value,
                    self.min,
                    self.min,
                    self.preview.high - self.minimum_gap,
                    self.max,
                    self.step,
                );
            }
            RangeThumb::High => {
                self.preview.high = normalize_constrained_value(
                    value,
                    self.min,
                    self.preview.low + self.minimum_gap,
                    self.max,
                    self.max,
                    self.step,
                );
            }
        }
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let value = self.value_at(event.position);
        self.active = self.nearest_thumb(value);
        match self.active {
            RangeThumb::Low => self.low_focus.focus(window, cx),
            RangeThumb::High => self.high_focus.focus(window, cx),
        }
        self.dragging = true;
        let revision = self.revision;
        self.set_active_value(value);
        let entity = cx.entity();
        let update_entity = entity.clone();
        let update =
            move |gesture: crate::interaction::GestureUpdate, _: &mut Window, cx: &mut App| {
                let current = update_entity.update(cx, |slider, cx| {
                    if slider.disabled || slider.revision != revision {
                        return false;
                    }
                    slider.set_active_value(slider.value_at(gesture.current()));
                    cx.notify();
                    true
                });
                if current {
                    crate::interaction::InteractionFlow::Continue
                } else {
                    crate::interaction::InteractionFlow::Cancel
                }
            };
        let finish_entity = entity.clone();
        let finish =
            move |gesture: crate::interaction::GestureUpdate, window: &mut Window, cx: &mut App| {
                finish_entity.update(cx, |slider, cx| {
                    if !slider.disabled && slider.revision == revision {
                        slider.set_active_value(slider.value_at(gesture.current()));
                        slider.emit_change(window, cx);
                    }
                    slider.dragging = false;
                    slider.preview = slider.controlled;
                    cx.notify();
                });
            };
        let cancel = move |_: &mut Window, cx: &mut App| {
            entity.update(cx, |slider, cx| {
                slider.dragging = false;
                slider.preview = slider.controlled;
                cx.notify();
            });
        };
        let owner = self.events.interaction_owner(&self.interaction_key);
        self.events.begin_interaction(
            crate::interaction::NativeGesture::new(
                owner,
                event.position,
                cx.entity_id(),
                update,
                finish,
                cancel,
            ),
            window,
            cx,
        );
        cx.stop_propagation();
    }

    fn keyboard(
        &mut self,
        thumb: RangeThumb,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        self.active = thumb;
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
        let current = match thumb {
            RangeThumb::Low => self.preview.low,
            RangeThumb::High => self.preview.high,
        };
        let next = if key == "home" {
            Some(match thumb {
                RangeThumb::Low => self.min,
                RangeThumb::High => self.preview.low + self.minimum_gap,
            })
        } else if key == "end" {
            Some(match thumb {
                RangeThumb::Low => self.preview.high - self.minimum_gap,
                RangeThumb::High => self.max,
            })
        } else {
            delta.map(|delta| current + delta)
        };
        if let Some(next) = next {
            let before = self.preview;
            self.set_active_value(next);
            if self.preview != before {
                self.emit_change(window, cx);
            }
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn emit_change(&self, window: &mut Window, cx: &mut App) {
        self.events
            .propose("change", pair_value(self.preview), window, cx);
    }
}

impl Render for RangeSliderEntity {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = self.events.interaction_owner(&self.interaction_key);
        self.events.present_interaction(owner.clone());
        if self.disabled {
            self.events.cancel_interaction(&owner, window, cx);
        }
        let low_ratio = self.ratio(self.preview.low);
        let high_ratio = self.ratio(self.preview.high);
        let direction = self.theme.direction();
        let track = crate::renderer::apply_style_override(
            div().relative(),
            &self.track_style,
            &self.theme,
            direction,
        );
        let mut fill = crate::renderer::apply_style_override(
            div().absolute(),
            &self.fill_style,
            &self.theme,
            direction,
        );
        let low_thumb = self.thumb(RangeThumb::Low, low_ratio, window, cx);
        let high_thumb = self.thumb(RangeThumb::High, high_ratio, window, cx);
        let track = match self.orientation {
            RangeOrientation::Horizontal => {
                let low_visual = horizontal_thumb_ratio(low_ratio, direction);
                let high_visual = horizontal_thumb_ratio(high_ratio, direction);
                let start = low_visual.min(high_visual);
                let span = (high_visual - low_visual).abs();
                fill = fill
                    .left(relative(fraction_f32(start)))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .w(relative(fraction_f32(span)));
                track.child(fill).child(low_thumb).child(high_thumb)
            }
            RangeOrientation::Vertical => {
                fill = fill
                    .bottom(relative(fraction_f32(low_ratio)))
                    .left(px(0.0))
                    .right(px(0.0))
                    .h(relative(fraction_f32(high_ratio - low_ratio)));
                track.child(fill).child(low_thumb).child(high_thumb)
            }
        };
        div()
            .id("gpui-rhai-range-slider")
            .relative()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .cursor(if self.disabled {
                CursorStyle::OperationNotAllowed
            } else {
                CursorStyle::PointingHand
            })
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .child(track)
            .child(RangeSliderBoundsRecorder {
                slider: cx.entity(),
            })
            .opacity(if self.disabled { 0.62 } else { 1.0 })
    }
}

impl RangeSliderEntity {
    fn thumb(
        &self,
        thumb: RangeThumb,
        ratio: f64,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let (focus, label, value) = match thumb {
            RangeThumb::Low => (&self.low_focus, self.low_label.clone(), self.preview.low),
            RangeThumb::High => (&self.high_focus, self.high_label.clone(), self.preview.high),
        };
        let element = crate::renderer::apply_style_override_in(
            div().flex_none(),
            &self.thumb_style,
            &crate::renderer::part_interaction(focus.is_focused(window), self.disabled),
            &self.theme,
            self.theme.direction(),
        );
        let anchor = match self.orientation {
            RangeOrientation::Horizontal => div()
                .absolute()
                .left(relative(fraction_f32(horizontal_thumb_ratio(
                    ratio,
                    self.theme.direction(),
                ))))
                .top(relative(0.5)),
            RangeOrientation::Vertical => div()
                .absolute()
                .bottom(relative(fraction_f32(ratio)))
                .left(relative(0.5)),
        };
        let entity = cx.entity();
        let element = element
            .id(match thumb {
                RangeThumb::Low => "gpui-rhai-range-slider-low",
                RangeThumb::High => "gpui-rhai-range-slider-high",
            })
            .track_focus(&focus.clone().tab_stop(!self.disabled))
            .role(Role::Slider)
            .aria_label(label)
            .aria_numeric_value(value)
            .aria_min_numeric_value(self.min)
            .aria_max_numeric_value(self.max)
            .aria_numeric_value_step(self.step)
            .on_key_down(move |event, window, cx| {
                entity.update(cx, |slider, cx| {
                    slider.keyboard(thumb, event, window, cx);
                });
            });
        crate::renderer::centered_on_anchor(anchor, element)
    }
}

struct RangeSliderBoundsRecorder {
    slider: Entity<RangeSliderEntity>,
}

impl Element for RangeSliderBoundsRecorder {
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
        self.slider
            .update(cx, |slider, _| slider.bounds = Some(bounds));
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

impl IntoElement for RangeSliderBoundsRecorder {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[derive(Default)]
pub struct RangeSliderPrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, Entity<RangeSliderEntity>>,
}

impl PrimitiveHandler for RangeSliderPrimitiveHandler {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveContext,
        theme: &PrimitiveTheme,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, String> {
        let id = instance
            .id
            .clone()
            .ok_or_else(|| "RangeSliderPrimitive requires a stable key".to_owned())?;
        let config = parse_config(&instance.node.props, theme)?;
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let events = events.clone();
            let interaction_key = format!("{}:{}", id.key(), id.node());
            let entity =
                cx.new(|cx| RangeSliderEntity::new(config.clone(), events, interaction_key, cx));
            self.instances.insert(id.clone(), entity.clone());
            entity
        };
        let changed = entity.update(cx, |slider, cx| {
            slider.update_props(config, events.clone(), cx)
        });
        if changed {
            let owner = events.interaction_owner(&format!("{}:{}", id.key(), id.node()));
            events.cancel_interaction(&owner, window, cx);
        }
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

fn parse_config(
    props: &PrimitiveProps,
    theme: &PrimitiveTheme,
) -> Result<RangeSliderConfig, String> {
    let min = props.number("min").unwrap_or(0.0);
    let max = props.number("max").unwrap_or(100.0);
    let step = props.number("step").unwrap_or(1.0);
    let minimum_gap = props.number("minimum_gap").unwrap_or(0.0);
    let values = RangePair {
        low: props
            .number("low")
            .ok_or_else(|| "range slider low is required".to_owned())?,
        high: props
            .number("high")
            .ok_or_else(|| "range slider high is required".to_owned())?,
    };
    if ![min, max, step, minimum_gap, values.low, values.high]
        .into_iter()
        .all(f64::is_finite)
        || max <= min
        || step <= 0.0
        || minimum_gap < 0.0
        || minimum_gap > max - min
        || values.low < min
        || values.high > max
        || values.low + minimum_gap > values.high
    {
        return Err("range slider values, step, bounds, or minimum_gap are invalid".to_owned());
    }
    let orientation = match props.string("orientation") {
        None | Some("horizontal") => RangeOrientation::Horizontal,
        Some("vertical") => RangeOrientation::Vertical,
        Some(other) => return Err(format!("unknown range slider orientation `{other}`")),
    };
    Ok(RangeSliderConfig {
        values,
        min,
        max,
        step,
        minimum_gap,
        orientation,
        disabled: props.boolean("disabled").unwrap_or(false),
        low_label: props.string("low_label").unwrap_or("Minimum").to_owned(),
        high_label: props.string("high_label").unwrap_or("Maximum").to_owned(),
        track_style: props.style("track_style").cloned().unwrap_or_default(),
        fill_style: props.style("fill_style").cloned().unwrap_or_default(),
        thumb_style: props.style("thumb_style").cloned().unwrap_or_default(),
        theme: theme.clone(),
    })
}

fn normalize_pair(values: RangePair, config: &RangeSliderConfig) -> RangePair {
    normalize_pair_values(
        values,
        config.min,
        config.max,
        config.step,
        config.minimum_gap,
    )
}

fn normalize_pair_values(
    values: RangePair,
    min: f64,
    max: f64,
    step: f64,
    minimum_gap: f64,
) -> RangePair {
    let low = normalize_value(values.low, min, max, step);
    let high = normalize_value(values.high, min, max, step);
    if high - low >= minimum_gap {
        return RangePair { low, high };
    }
    let raise_high = first_step_at_or_above(low + minimum_gap, min, max, step)
        .map(|high| RangePair { low, high });
    let lower_low = last_step_at_or_below(high - minimum_gap, min, max, step)
        .map(|low| RangePair { low, high });
    [raise_high, lower_low]
        .into_iter()
        .flatten()
        .min_by(|left, right| {
            pair_distance(*left, values)
                .partial_cmp(&pair_distance(*right, values))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(RangePair {
            low: min,
            high: max,
        })
}

fn first_step_at_or_above(value: f64, origin: f64, max: f64, step: f64) -> Option<f64> {
    let stepped = origin + ((value - origin) / step).ceil() * step;
    if stepped <= max {
        Some(stepped.max(origin))
    } else {
        (max >= value).then_some(max)
    }
}

fn last_step_at_or_below(value: f64, min: f64, max: f64, step: f64) -> Option<f64> {
    let stepped = min + ((value - min) / step).floor() * step;
    if stepped >= min {
        Some(stepped.min(max))
    } else {
        (min <= value).then_some(min)
    }
}

fn pair_distance(pair: RangePair, source: RangePair) -> f64 {
    (pair.low - source.low).abs() + (pair.high - source.high).abs()
}

fn normalize_constrained_value(
    value: f64,
    origin: f64,
    feasible_min: f64,
    feasible_max: f64,
    global_max: f64,
    step: f64,
) -> f64 {
    let snapped = normalize_value(value, origin, global_max, step);
    if snapped >= feasible_min && snapped <= feasible_max {
        return snapped;
    }
    let first = origin + ((feasible_min - origin) / step).ceil() * step;
    let last = origin + ((feasible_max - origin) / step).floor() * step;
    if first <= last {
        snapped.clamp(first, last)
    } else {
        value.clamp(feasible_min, feasible_max)
    }
}

fn pair_value(values: RangePair) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("low".to_owned(), UiValue::Float(values.low)),
        ("high".to_owned(), UiValue::Float(values.high)),
    ]))
}

fn pair_schema() -> ValueSchema {
    ValueSchema::object(BTreeMap::from([
        (
            "low".to_owned(),
            ObjectField::required(ValueSchema::number()),
        ),
        (
            "high".to_owned(),
            ObjectField::required(ValueSchema::number()),
        ),
    ]))
}

/// Build the native controlled two-thumb range slider schema.
///
/// # Panics
///
/// Panics only if the static primitive ID becomes invalid.
#[must_use]
pub fn range_slider_primitive_descriptor() -> PrimitiveDescriptor {
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.range_slider").expect("static primitive ID"),
        export: "RangeSliderPrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "low".to_owned(),
                ObjectField::required(ValueSchema::number()),
            ),
            (
                "high".to_owned(),
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
                ObjectField::required(ValueSchema::positive_number()),
            ),
            (
                "minimum_gap".to_owned(),
                ObjectField::optional(ValueSchema::bounded_number(Some(0.0), None)),
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
                "low_label".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "high_label".to_owned(),
                ObjectField::required(ValueSchema::string()),
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
                payload: pair_schema(),
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

    #[test]
    fn pair_normalization_preserves_order_and_gap() {
        assert_eq!(
            normalize_pair_values(
                RangePair {
                    low: 31.0,
                    high: 84.0,
                },
                0.0,
                100.0,
                5.0,
                10.0,
            ),
            RangePair {
                low: 30.0,
                high: 85.0,
            }
        );
    }

    #[test]
    fn pair_normalization_solves_legal_endpoint_constraints_jointly() {
        assert_eq!(
            normalize_pair_values(
                RangePair {
                    low: 95.0,
                    high: 100.0,
                },
                0.0,
                100.0,
                10.0,
                5.0,
            ),
            RangePair {
                low: 90.0,
                high: 100.0,
            }
        );
    }

    #[test]
    fn normalized_pairs_are_bounded_gapped_and_idempotent() {
        for minimum in [0.0_f64, 3.0] {
            for maximum in [97.0, 100.0] {
                for step in [3.0, 10.0] {
                    for gap in [0.0, 5.0, 17.0] {
                        if gap > maximum - minimum {
                            continue;
                        }
                        for low in [minimum, maximum - gap, maximum] {
                            for high in [minimum, minimum + gap, maximum] {
                                let source = RangePair {
                                    low: low.min(high),
                                    high: low.max(high),
                                };
                                if source.high - source.low < gap {
                                    continue;
                                }
                                let normalized =
                                    normalize_pair_values(source, minimum, maximum, step, gap);
                                assert!(normalized.low >= minimum);
                                assert!(normalized.high <= maximum);
                                assert!(normalized.high - normalized.low + 1e-9 >= gap);
                                assert_eq!(
                                    normalize_pair_values(normalized, minimum, maximum, step, gap,),
                                    normalized
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
