//! Controlled Canvas rotation around an explicit local pivot.

use std::collections::BTreeMap;

use gpui::{
    AnyElement, App, Bounds, DispatchPhase, Element, ElementId, GlobalElementId, Hitbox,
    HitboxBehavior, InspectorElementId, InteractiveElement, IntoElement, KeyDownEvent, LayoutId,
    MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Style, Styled, Window, div,
    relative, size,
};

use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveContext, PrimitiveDescriptor,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveProps, PrimitiveTheme, SignalKind,
    SignalValue, UiValue, ValueSchema,
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct RotationPreview {
    angle: f64,
    translate_x: f64,
    translate_y: f64,
}

#[derive(Clone)]
struct RotatableConfig {
    id: String,
    source_token: String,
    angle: f64,
    pivot: (f64, f64),
    snap: Option<f64>,
    keyboard_step: f64,
    threshold: f64,
    disabled: bool,
    content_ref: crate::ElementRef,
    angle_signal: crate::NativeSignal,
    x_signal: crate::NativeSignal,
    y_signal: crate::NativeSignal,
    source_token_signal: crate::NativeSignal,
    focus: Option<gpui::FocusHandle>,
}

struct RotationPrepaint {
    hitbox: Hitbox,
}

struct RotationElement {
    config: RotatableConfig,
    context: PrimitiveContext,
}

impl IntoElement for RotationElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for RotationElement {
    type RequestLayoutState = ();
    type PrepaintState = RotationPrepaint;

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
    ) -> (LayoutId, ()) {
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
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> RotationPrepaint {
        if let Some(viewport) = self.context.canvas_bounds(&self.config.content_ref, cx) {
            sync_controlled_source(&self.context, &self.config, viewport, window, cx);
        }
        RotationPrepaint {
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut (),
        prepaint: &mut RotationPrepaint,
        window: &mut Window,
        _: &mut App,
    ) {
        let owner = self.context.interaction_owner(&self.config.id);
        self.context.present_interaction(owner.clone());
        if self.config.disabled {
            return;
        }
        let hitbox = prepaint.hitbox.clone();
        let config = self.config.clone();
        let context = self.context.clone();
        let view = window.current_view();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble
                || event.button != MouseButton::Left
                || !hitbox.is_hovered(window)
            {
                return;
            }
            let Some(viewport) = context.canvas_bounds(&config.content_ref, cx) else {
                return;
            };
            if let Some(focus) = config.focus.as_ref() {
                focus.focus(window, cx);
            }
            sync_controlled_source(&context, &config, viewport, window, cx);
            let pointer_start = pointer_angle(event.position, viewport, config.pivot);
            let source_angle = config.angle;
            let update_context = context.clone();
            let update_config = config.clone();
            let update = move |gesture: crate::interaction::GestureUpdate,
                               _: &mut Window,
                               cx: &mut App| {
                if gesture.moved() {
                    let Some(current_viewport) =
                        update_context.canvas_bounds(&update_config.content_ref, cx)
                    else {
                        return crate::interaction::InteractionFlow::Cancel;
                    };
                    if current_viewport != viewport {
                        return crate::interaction::InteractionFlow::Cancel;
                    }
                    let pointer =
                        pointer_angle(gesture.current(), current_viewport, update_config.pivot);
                    let next = rotated_angle(&update_config, source_angle, pointer - pointer_start);
                    write_preview(&update_context, &update_config, current_viewport, next, cx);
                }
                crate::interaction::InteractionFlow::Continue
            };
            let finish_context = context.clone();
            let finish_config = config.clone();
            let finish = move |gesture: crate::interaction::GestureUpdate,
                               window: &mut Window,
                               cx: &mut App| {
                let Some(current_viewport) =
                    finish_context.canvas_bounds(&finish_config.content_ref, cx)
                else {
                    return;
                };
                if current_viewport != viewport {
                    write_preview(
                        &finish_context,
                        &finish_config,
                        current_viewport,
                        finish_config.angle,
                        cx,
                    );
                    return;
                }
                let pointer =
                    pointer_angle(gesture.current(), current_viewport, finish_config.pivot);
                let next = rotated_angle(&finish_config, source_angle, pointer - pointer_start);
                if gesture.moved() && angle_changed(finish_config.angle, next) {
                    propose_angle(
                        &finish_context,
                        &finish_config,
                        current_viewport,
                        next,
                        window,
                        cx,
                    );
                } else {
                    write_preview(
                        &finish_context,
                        &finish_config,
                        current_viewport,
                        finish_config.angle,
                        cx,
                    );
                }
            };
            let cancel_context = context.clone();
            let cancel_config = config.clone();
            let cancel = move |_: &mut Window, cx: &mut App| {
                if let Some(current_viewport) =
                    cancel_context.canvas_bounds(&cancel_config.content_ref, cx)
                {
                    write_preview(
                        &cancel_context,
                        &cancel_config,
                        current_viewport,
                        cancel_config.angle,
                        cx,
                    );
                }
            };
            context.begin_interaction(
                crate::interaction::NativeGesture::new(
                    owner.clone(),
                    event.position,
                    view,
                    update,
                    finish,
                    cancel,
                )
                .with_threshold(config.threshold),
                window,
                cx,
            );
            cx.stop_propagation();
        });
    }
}

#[derive(Default)]
pub struct RotatablePrimitiveHandler;

impl PrimitiveHandler for RotatablePrimitiveHandler {
    fn uses_primary_focus(&self) -> bool {
        true
    }

    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        context: &PrimitiveContext,
        _: &PrimitiveTheme,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, String> {
        let config = parse_config(&instance.node.props, instance.focus_handle().cloned())?;
        if let Some(viewport) = context.canvas_bounds(&config.content_ref, cx) {
            sync_controlled_source(context, &config, viewport, window, cx);
        }
        let key_config = config.clone();
        let key_context = context.clone();
        let mut root = div().size_full();
        if let Some(focus) = config.focus.as_ref() {
            root = root.track_focus(&focus.clone().tab_stop(!config.disabled));
        }
        Ok(root
            .on_key_down(move |event: &KeyDownEvent, window, cx: &mut App| {
                if key_config.disabled {
                    return;
                }
                let multiplier = if event.keystroke.modifiers.shift {
                    4.0
                } else {
                    1.0
                };
                let step = key_config.snap.unwrap_or(key_config.keyboard_step) * multiplier;
                let delta = match event.keystroke.key.as_str() {
                    "left" | "down" => -step,
                    "right" | "up" => step,
                    "home" => -key_config.angle,
                    _ => return,
                };
                let next = rotated_angle(&key_config, key_config.angle, delta);
                if angle_changed(key_config.angle, next) {
                    if let Some(viewport) = key_context.canvas_bounds(&key_config.content_ref, cx) {
                        propose_angle(&key_context, &key_config, viewport, next, window, cx);
                    } else {
                        key_context.propose("rotate", UiValue::Float(next), window, cx);
                    }
                }
                cx.stop_propagation();
            })
            .child(RotationElement {
                config,
                context: context.clone(),
            })
            .into_any_element())
    }
}

fn parse_config(
    props: &PrimitiveProps,
    focus: Option<gpui::FocusHandle>,
) -> Result<RotatableConfig, String> {
    let angle = required_number(props, "angle")?;
    let pivot = (
        required_number(props, "pivot_x")?,
        required_number(props, "pivot_y")?,
    );
    let snap = match props.data("snap") {
        None | Some(UiValue::Null) => None,
        Some(_) => Some(required_number(props, "snap")?),
    };
    if snap.is_some_and(|snap| snap <= 0.0 || snap > 360.0)
        || pivot.0.abs() > 1_000_000.0
        || pivot.1.abs() > 1_000_000.0
    {
        return Err("rotatable pivot or snap is invalid".to_owned());
    }
    let keyboard_step = bounded_number(props, "keyboard_step", 0.1, 180.0, 5.0)?;
    let threshold = bounded_number(props, "threshold", 0.0, 64.0, 4.0)?;
    let angle_signal = typed_signal(props, "angle_signal", SignalKind::Float)?;
    let x_signal = typed_signal(props, "x_signal", SignalKind::Float)?;
    let y_signal = typed_signal(props, "y_signal", SignalKind::Float)?;
    let source_token_signal = typed_signal(props, "source_token_signal", SignalKind::String)?;
    let source_token = props
        .string("source_token")
        .filter(|value| !value.is_empty() && value.len() <= 512)
        .ok_or_else(|| "rotatable source_token is required".to_owned())?
        .to_owned();
    Ok(RotatableConfig {
        id: format!(
            "gpui-rhai-rotatable:{}:{}",
            angle_signal.id().component(),
            angle_signal.id().key()
        ),
        source_token,
        angle: wrap_angle(angle),
        pivot,
        snap,
        keyboard_step,
        threshold,
        disabled: props.boolean("disabled").unwrap_or(false),
        content_ref: props
            .element_ref("content_ref")
            .cloned()
            .ok_or_else(|| "rotatable content_ref is required".to_owned())?,
        angle_signal,
        x_signal,
        y_signal,
        source_token_signal,
        focus,
    })
}

fn required_number(props: &PrimitiveProps, name: &str) -> Result<f64, String> {
    props
        .number(name)
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("rotatable requires finite numeric {name}"))
}

fn bounded_number(
    props: &PrimitiveProps,
    name: &str,
    minimum: f64,
    maximum: f64,
    fallback: f64,
) -> Result<f64, String> {
    let value = props.number(name).unwrap_or(fallback);
    (value.is_finite() && (minimum..=maximum).contains(&value))
        .then_some(value)
        .ok_or_else(|| format!("rotatable {name} is outside its supported range"))
}

fn typed_signal(
    props: &PrimitiveProps,
    name: &str,
    kind: SignalKind,
) -> Result<crate::NativeSignal, String> {
    let signal = props
        .signal(name)
        .cloned()
        .ok_or_else(|| format!("rotatable requires signal {name}"))?;
    (signal.id().kind() == kind)
        .then_some(signal)
        .ok_or_else(|| format!("rotatable {name} must be {}", kind.as_str()))
}

fn pointer_angle(
    position: Point<Pixels>,
    viewport: crate::GeometryBounds,
    pivot: (f64, f64),
) -> f64 {
    let x = f64::from(position.x) - viewport.x - pivot.0;
    let y = f64::from(position.y) - viewport.y - pivot.1;
    y.atan2(x).to_degrees()
}

fn rotated_angle(config: &RotatableConfig, source: f64, delta: f64) -> f64 {
    let angle = wrap_angle(source + normalize_delta(delta));
    config
        .snap
        .map_or(angle, |snap| wrap_angle((angle / snap).round() * snap))
}

fn wrap_angle(angle: f64) -> f64 {
    angle.rem_euclid(360.0)
}

fn normalize_delta(delta: f64) -> f64 {
    (delta + 180.0).rem_euclid(360.0) - 180.0
}

fn angle_changed(left: f64, right: f64) -> bool {
    normalize_delta(right - left).abs() > 0.000_001
}

fn preview_for(angle: f64, pivot: (f64, f64), viewport: crate::GeometryBounds) -> RotationPreview {
    let center = (viewport.width / 2.0, viewport.height / 2.0);
    let offset = (pivot.0 - center.0, pivot.1 - center.1);
    let radians = angle.to_radians();
    let (sin, cos) = radians.sin_cos();
    let rotated = (
        cos.mul_add(offset.0, -sin * offset.1),
        sin.mul_add(offset.0, cos * offset.1),
    );
    RotationPreview {
        angle,
        translate_x: offset.0 - rotated.0,
        translate_y: offset.1 - rotated.1,
    }
}

fn write_preview(
    context: &PrimitiveContext,
    config: &RotatableConfig,
    viewport: crate::GeometryBounds,
    angle: f64,
    cx: &mut App,
) {
    let preview = preview_for(angle, config.pivot, viewport);
    let _ = context.write_signals(
        [
            (
                config.angle_signal.clone(),
                SignalValue::Float(preview.angle),
            ),
            (
                config.x_signal.clone(),
                SignalValue::Float(preview.translate_x),
            ),
            (
                config.y_signal.clone(),
                SignalValue::Float(preview.translate_y),
            ),
        ],
        cx,
    );
}

fn presentation_token(config: &RotatableConfig, viewport: crate::GeometryBounds) -> String {
    format!(
        "{}|{}|{}",
        config.source_token, viewport.width, viewport.height
    )
}

/// Show the controlled angle when the source (or the viewport) changed. A
/// proposal leaves the token as `pending:<token>`: the same source then means
/// the Host rejected it and the preview returns to the source.
///
/// This also runs while a frame is drawn, after the content read the signals,
/// when GPUI drops the redraw a write asks for; a changed preview asks for the
/// next frame itself.
fn sync_controlled_source(
    context: &PrimitiveContext,
    config: &RotatableConfig,
    viewport: crate::GeometryBounds,
    window: &mut Window,
    cx: &mut App,
) {
    let presentation_token = presentation_token(config, viewport);
    let token_matches = matches!(
        context.read_signal(&config.source_token_signal, cx),
        Ok(SignalValue::String(value)) if value == presentation_token
    );
    if token_matches {
        return;
    }
    let shown = matches!(
        context.read_signal(&config.angle_signal, cx),
        Ok(SignalValue::Float(angle)) if !angle_changed(angle, preview_for(config.angle, config.pivot, viewport).angle)
    );
    write_preview(context, config, viewport, config.angle, cx);
    let _ = context.write_signal(
        &config.source_token_signal,
        SignalValue::String(presentation_token),
        cx,
    );
    if !shown {
        window.defer(cx, |window, _| window.refresh());
    }
}

/// Propose an angle and keep showing it until the next source decides.
fn propose_angle(
    context: &PrimitiveContext,
    config: &RotatableConfig,
    viewport: crate::GeometryBounds,
    angle: f64,
    window: &mut Window,
    cx: &mut App,
) {
    write_preview(context, config, viewport, angle, cx);
    let _ = context.write_signal(
        &config.source_token_signal,
        SignalValue::String(format!("pending:{}", presentation_token(config, viewport))),
        cx,
    );
    context.propose("rotate", UiValue::Float(angle), window, cx);
    // A frame must follow the answer even when the Host changes nothing.
    window.defer(cx, |window, _| window.refresh());
}

fn descriptor_signal_props() -> BTreeMap<String, ObjectField> {
    [
        "angle_signal",
        "x_signal",
        "y_signal",
        "source_token_signal",
    ]
    .into_iter()
    .map(|name| (name.to_owned(), ObjectField::required(ValueSchema::Signal)))
    .collect()
}

/// Build the native controlled Canvas rotation schema.
///
/// # Panics
///
/// Panics only if the static primitive ID becomes invalid.
#[must_use]
pub fn rotatable_primitive_descriptor() -> PrimitiveDescriptor {
    let mut props = BTreeMap::from([
        (
            "source_token".to_owned(),
            ObjectField::required(ValueSchema::string()),
        ),
        (
            "angle".to_owned(),
            ObjectField::required(ValueSchema::number()),
        ),
        (
            "pivot_x".to_owned(),
            ObjectField::required(ValueSchema::number()),
        ),
        (
            "pivot_y".to_owned(),
            ObjectField::required(ValueSchema::number()),
        ),
        (
            "snap".to_owned(),
            ObjectField::optional(ValueSchema::optional(ValueSchema::number())),
        ),
        (
            "keyboard_step".to_owned(),
            ObjectField::optional(ValueSchema::bounded_number(Some(0.1), Some(180.0))),
        ),
        (
            "threshold".to_owned(),
            ObjectField::optional(ValueSchema::bounded_number(Some(0.0), Some(64.0))),
        ),
        (
            "disabled".to_owned(),
            ObjectField::optional(ValueSchema::Bool).with_default(UiValue::Bool(false)),
        ),
        (
            "content_ref".to_owned(),
            ObjectField::required(ValueSchema::Ref),
        ),
        (
            "on_rotate".to_owned(),
            ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
        ),
    ]);
    props.extend(descriptor_signal_props());
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.rotatable").expect("static primitive ID"),
        export: "RotatablePrimitive".to_owned(),
        props,
        events: BTreeMap::from([(
            "rotate".to_owned(),
            EventSchema {
                doc: None,
                payload: ValueSchema::number(),
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
    fn arbitrary_pivot_stays_fixed_under_rotation_compensation() {
        let viewport = crate::GeometryBounds::new(0.0, 0.0, 300.0, 180.0).unwrap();
        let pivot = (40.0, 70.0);
        let preview = preview_for(90.0, pivot, viewport);
        let transformed = crate::canvas::canvas_motion_point(
            viewport.width,
            viewport.height,
            pivot.0,
            pivot.1,
            crate::geometry::CanvasMotionTransform {
                rotate: preview.angle,
                ..crate::geometry::CanvasMotionTransform::default()
            },
        );
        assert!((transformed.0 + preview.translate_x - pivot.0).abs() < 0.000_001);
        assert!((transformed.1 + preview.translate_y - pivot.1).abs() < 0.000_001);
    }
}
