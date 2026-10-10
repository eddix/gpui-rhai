//! Generic retained single-value range-input behavior for styled Rhai components.

use std::collections::BTreeMap;

use gpui::{
    AnyElement, App, AppContext, Bounds, Context, CursorStyle, Element, ElementId, Entity,
    FocusHandle, GlobalElementId, InspectorElementId, InteractiveElement, IntoElement,
    KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render,
    Styled, Window, div, px, relative,
};

use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveContext, PrimitiveDescriptor,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId, PrimitiveProps,
    PrimitiveTheme, Style, TextDirection, UiValue, ValueSchema,
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
pub(crate) enum RangeOrientation {
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
    events: PrimitiveContext,
    interaction_key: String,
    track_style: Style,
    fill_style: Style,
    thumb_style: Style,
    theme: PrimitiveTheme,
}

impl RangeInputEntity {
    fn new(
        config: RangeInputConfig,
        events: PrimitiveContext,
        interaction_key: String,
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
            interaction_key,
            track_style: config.track_style,
            fill_style: config.fill_style,
            thumb_style: config.thumb_style,
            theme: config.theme,
        }
    }

    fn update_props(
        &mut self,
        config: RangeInputConfig,
        events: PrimitiveContext,
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
        range_value_at(
            bounds,
            position,
            self.orientation,
            self.theme.direction(),
            self.min,
            self.max,
            self.step,
        )
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.focus.focus(window, cx);
        self.dragging = true;
        self.preview = self.value_at(event.position);
        let entity = cx.weak_entity();
        let update_entity = entity.clone();
        let update =
            move |gesture: crate::interaction::GestureUpdate, _: &mut Window, cx: &mut App| {
                update_entity
                    .update(cx, |input, cx| {
                        if input.disabled {
                            return;
                        }
                        input.preview = input.value_at(gesture.current());
                        cx.notify();
                    })
                    .map_or(crate::interaction::InteractionFlow::Cancel, |()| {
                        crate::interaction::InteractionFlow::Continue
                    })
            };
        let finish_entity = entity.clone();
        let finish =
            move |gesture: crate::interaction::GestureUpdate, window: &mut Window, cx: &mut App| {
                let _ = finish_entity.update(cx, |input, cx| {
                    if input.disabled {
                        input.dragging = false;
                        input.preview = input.controlled;
                        cx.notify();
                        return;
                    }
                    input.preview = input.value_at(gesture.current());
                    input.dragging = false;
                    if input.preview.to_bits() != input.controlled.to_bits() {
                        input.emit_change(input.preview, window, cx);
                    }
                    cx.notify();
                });
            };
        let cancel = move |_: &mut Window, cx: &mut App| {
            let _ = entity.update(cx, |input, cx| {
                input.dragging = false;
                input.preview = input.controlled;
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
            let before = self.preview;
            self.preview = normalize_value(next, self.min, self.max, self.step);
            if self.preview.to_bits() != before.to_bits() {
                self.emit_change(self.preview, window, cx);
            }
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn emit_change(&self, value: f64, window: &mut Window, cx: &mut Context<Self>) {
        self.events
            .propose("change", UiValue::Float(value), window, cx);
    }
}

impl Render for RangeInputEntity {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = self.events.interaction_owner(&self.interaction_key);
        self.events.present_interaction(owner.clone());
        if self.disabled {
            self.events.cancel_interaction(&owner, window, cx);
        }
        let ratio = self.ratio();
        let direction = self.theme.direction();
        let mut fill = crate::renderer::apply_style_override(
            div().absolute(),
            &self.fill_style,
            &self.theme,
            direction,
        );
        let thumb = crate::renderer::apply_style_override_in(
            div().flex_none(),
            &self.thumb_style,
            &crate::renderer::part_interaction(self.focus.is_focused(window), self.disabled),
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
                let anchor = div()
                    .absolute()
                    .left(relative(fraction_f32(visual_ratio)))
                    .top(relative(0.5));
                track
                    .child(fill)
                    .child(crate::renderer::centered_on_anchor(anchor, thumb))
            }
            RangeOrientation::Vertical => {
                fill = fill
                    .left(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .h(relative(fraction_f32(ratio)));
                let anchor = div()
                    .absolute()
                    .bottom(relative(fraction_f32(ratio)))
                    .left(relative(0.5));
                track
                    .child(fill)
                    .child(crate::renderer::centered_on_anchor(anchor, thumb))
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
    fn accessibility_actions(
        &self,
        _instance: &PrimitiveInstanceId,
    ) -> Vec<gpui::AccessibleAction> {
        vec![
            gpui::AccessibleAction::Focus,
            gpui::AccessibleAction::Decrement,
            gpui::AccessibleAction::Increment,
            gpui::AccessibleAction::SetValue,
        ]
    }

    fn perform_accessibility_action(
        &mut self,
        instance: &PrimitiveInstanceId,
        action: gpui::AccessibleAction,
        data: Option<&gpui::accesskit::ActionData>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<(), String> {
        let entity = self
            .instances
            .get(instance)
            .cloned()
            .ok_or_else(|| "range input accessibility target is stale".to_owned())?;
        if action == gpui::AccessibleAction::Focus {
            let (disabled, focus) = {
                let input = entity.read(cx);
                (input.disabled, input.focus.clone())
            };
            if disabled {
                return Err("disabled range input cannot receive focus".to_owned());
            }
            focus.focus(window, cx);
            return Ok(());
        }
        entity.update(cx, |input, cx| {
            if input.disabled {
                return Err("disabled range input cannot change value".to_owned());
            }
            let requested = match action {
                gpui::AccessibleAction::Decrement => input.preview - input.step,
                gpui::AccessibleAction::Increment => input.preview + input.step,
                gpui::AccessibleAction::SetValue => match data {
                    Some(gpui::accesskit::ActionData::NumericValue(value)) => *value,
                    Some(gpui::accesskit::ActionData::Value(value)) => value
                        .parse::<f64>()
                        .map_err(|_| "range input SetValue requires numeric data".to_owned())?,
                    _ => return Err("range input SetValue requires numeric data".to_owned()),
                },
                _ => return Err("unsupported range input accessibility action".to_owned()),
            };
            let before = input.preview;
            input.preview = normalize_value(requested, input.min, input.max, input.step);
            if input.preview.to_bits() != before.to_bits() {
                input.emit_change(input.preview, window, cx);
            }
            cx.notify();
            Ok(())
        })
    }

    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveContext,
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
            let interaction_key = format!("{}:{}", id.key(), id.node());
            let entity =
                cx.new(|cx| RangeInputEntity::new(config.clone(), events, interaction_key, cx));
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
    let value = props
        .number("value")
        .ok_or_else(|| "range value is required".to_owned())?;
    let min = props.number("min").unwrap_or(0.0);
    let max = props.number("max").unwrap_or(100.0);
    let step = props.number("step").unwrap_or(1.0);
    if !value.is_finite() || !min.is_finite() || !max.is_finite() || !step.is_finite() {
        return Err("range values must be finite".to_owned());
    }
    if max <= min {
        return Err("range max must be greater than min".to_owned());
    }
    if step <= 0.0 || step > max - min {
        return Err("range step must be positive and no larger than max - min".to_owned());
    }
    let orientation = match props.string("orientation") {
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
        disabled: props.boolean("disabled").unwrap_or(false),
        track_style: props.style("track_style").cloned().unwrap_or_default(),
        fill_style: props.style("fill_style").cloned().unwrap_or_default(),
        thumb_style: props.style("thumb_style").cloned().unwrap_or_default(),
        theme: theme.clone(),
    })
}

pub(crate) fn normalize_value(value: f64, min: f64, max: f64, step: f64) -> f64 {
    let snapped = min + ((value.clamp(min, max) - min) / step).round() * step;
    snapped.clamp(min, max)
}

pub(crate) fn horizontal_thumb_ratio(ratio: f64, direction: TextDirection) -> f64 {
    if direction == TextDirection::RightToLeft {
        1.0 - ratio
    } else {
        ratio
    }
}

pub(crate) fn fraction_f32(value: f64) -> f32 {
    value.to_string().parse().unwrap_or(0.0)
}

pub(crate) fn range_value_at(
    bounds: Bounds<Pixels>,
    position: Point<Pixels>,
    orientation: RangeOrientation,
    direction: TextDirection,
    min: f64,
    max: f64,
    step: f64,
) -> f64 {
    let ratio = match orientation {
        RangeOrientation::Horizontal => {
            let width = f64::from(bounds.size.width).max(f64::EPSILON);
            let ratio =
                ((f64::from(position.x) - f64::from(bounds.origin.x)) / width).clamp(0.0, 1.0);
            if direction == TextDirection::RightToLeft {
                1.0 - ratio
            } else {
                ratio
            }
        }
        RangeOrientation::Vertical => {
            let height = f64::from(bounds.size.height).max(f64::EPSILON);
            1.0 - ((f64::from(position.y) - f64::from(bounds.origin.y)) / height).clamp(0.0, 1.0)
        }
    };
    normalize_value(min + ratio * (max - min), min, max, step)
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
                ObjectField::required(ValueSchema::number()).with_doc(
                    "The selected value, snapped to `step` from `min`; controlled, so store each `change` payload or the thumb snaps back.",
                ),
            ),
            (
                "min".to_owned(),
                ObjectField::required(ValueSchema::number()).with_doc(
                    "Value at the start of the track: the leading edge, or the bottom when vertical; must be less than `max`.",
                ),
            ),
            (
                "max".to_owned(),
                ObjectField::required(ValueSchema::number())
                    .with_doc("Value at the end of the track; must be greater than `min`."),
            ),
            (
                "step".to_owned(),
                ObjectField::required(ValueSchema::number()).with_doc(
                    "Positive increment the value snaps to, counted from `min`, and the arrow-key step; at most `max - min`. When `max - min` is not a multiple of it, the last step below `max` is the highest value.",
                ),
            ),
            (
                "orientation".to_owned(),
                ObjectField::required(ValueSchema::String {
                    allowed: vec!["horizontal".to_owned(), "vertical".to_owned()],
                })
                .with_doc(
                    "`horizontal` runs the track in the reading direction (mirrored in RTL); `vertical` runs it bottom to top.",
                ),
            ),
            (
                "disabled".to_owned(),
                ObjectField::optional(ValueSchema::Bool)
                    .with_default(UiValue::Bool(false))
                    .with_doc("Blocks pointer and keyboard input, leaves the tab order and dims the control."),
            ),
            (
                "track_style".to_owned(),
                ObjectField::required(ValueSchema::Style).with_doc(
                    "Style of the track that holds the fill and thumb; give it its size here, such as full width by 4px.",
                ),
            ),
            (
                "fill_style".to_owned(),
                ObjectField::required(ValueSchema::Style).with_doc(
                    "Style of the fill from `min` to the thumb; its length along the track is set natively from the value.",
                ),
            ),
            (
                "thumb_style".to_owned(),
                ObjectField::required(ValueSchema::Style).with_doc(
                    "Style of the thumb centered on the value; its `focus` and `disabled` states apply.",
                ),
            ),
            (
                "on_change".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)).with_doc(
                    "Called with the snapped value when a drag, an arrow, Home or End key or an accessibility action changes it.",
                ),
            ),
        ]),
        events: BTreeMap::from([(
            "change".to_owned(),
            EventSchema {
                doc: Some(
                    "Emitted when a drag ends or an arrow, Home or End key acts, if the value changes; the payload is the proposed snapped value."
                        .to_owned(),
                ),
                payload: ValueSchema::number(),
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
