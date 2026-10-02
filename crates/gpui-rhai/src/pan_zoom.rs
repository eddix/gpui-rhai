//! Controlled Canvas pan/zoom interaction over the shared affine geometry path.

use std::collections::BTreeMap;
use std::time::Duration;

use gpui::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render,
    ScrollWheelEvent, Styled, Window, div, px,
};

use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveContext, PrimitiveDescriptor,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId, PrimitiveProps,
    PrimitiveTheme, SignalKind, SignalValue, UiValue, ValueSchema,
};

const WHEEL_COMMIT_DELAY: Duration = Duration::from_millis(80);
const MAX_TRANSLATION: f64 = 1_000_000.0;
const MAX_RENDER_SCALE: f64 = 1_000_000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct ViewTransform {
    x: f64,
    y: f64,
    scale: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PanAxes {
    Both,
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WheelZoom {
    Off,
    Modifier,
    Always,
}

#[derive(Clone)]
struct PanZoomConfig {
    id: String,
    source_token: String,
    source: ViewTransform,
    minimum_scale: f64,
    maximum_scale: f64,
    axes: PanAxes,
    wheel_zoom: WheelZoom,
    pan_button: MouseButton,
    threshold: f64,
    keyboard_pan_step: f64,
    keyboard_zoom_factor: f64,
    disabled: bool,
    viewport_ref: crate::ElementRef,
    x_signal: crate::NativeSignal,
    y_signal: crate::NativeSignal,
    scale_x_signal: crate::NativeSignal,
    scale_y_signal: crate::NativeSignal,
    wheel_generation_signal: crate::NativeSignal,
    wheel_active_signal: crate::NativeSignal,
    wheel_pending_signal: crate::NativeSignal,
    source_token_signal: crate::NativeSignal,
    focus: Option<FocusHandle>,
}

struct PanZoomEntity {
    focus: FocusHandle,
    config: PanZoomConfig,
    context: PrimitiveContext,
    preview: ViewTransform,
    suspended: bool,
}

impl PanZoomEntity {
    fn new(mut config: PanZoomConfig, context: PrimitiveContext, cx: &mut Context<Self>) -> Self {
        let focus = config.focus.clone().unwrap_or_else(|| cx.focus_handle());
        config.focus = Some(focus.clone());
        let preview = read_transform(&context, &config, cx).unwrap_or(config.source);
        Self {
            focus,
            preview,
            config,
            context,
            suspended: false,
        }
    }

    fn update_config(
        &mut self,
        mut config: PanZoomConfig,
        context: PrimitiveContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        config.focus = Some(self.focus.clone());
        self.context = context;
        let contract_changed = read_string_signal(&self.context, &config.source_token_signal, cx)
            .as_deref()
            != Some(config.source_token.as_str());
        if contract_changed {
            let owner = self.context.interaction_owner(&self.config.id);
            self.context.cancel_interaction(&owner, window, cx);
            invalidate_wheel(&self.context, &config, cx);
            self.preview = config.source;
            write_transform(&self.context, &config, config.source, cx);
            write_string_signal(
                &self.context,
                &config.source_token_signal,
                &config.source_token,
                cx,
            );
        } else {
            self.preview = read_transform(&self.context, &config, cx).unwrap_or(config.source);
        }
        self.config = config;
    }

    fn set_preview(&mut self, transform: ViewTransform, cx: &mut App) {
        self.preview = transform;
        write_transform(&self.context, &self.config, transform, cx);
    }

    fn restore_source(&mut self, cx: &mut App) {
        self.preview = self.config.source;
        write_transform(&self.context, &self.config, self.config.source, cx);
    }

    fn propose(&mut self, transform: ViewTransform, window: &mut Window, cx: &mut App) {
        self.restore_source(cx);
        if transform_changed(self.config.source, transform) {
            self.context
                .propose("transform_change", transform_value(transform), window, cx);
        }
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.suspended || self.config.disabled || event.button != self.config.pan_button {
            return;
        }
        let Some(viewport) = self.context.element_bounds(&self.config.viewport_ref, cx) else {
            return;
        };
        if !contains(viewport, event.position) {
            return;
        }
        self.focus.focus(window, cx);
        if transform_changed(self.config.source, self.preview)
            || read_bool_signal(&self.context, &self.config.wheel_active_signal, cx)
                .unwrap_or(false)
        {
            invalidate_wheel(&self.context, &self.config, cx);
        }
        let start = self.preview;
        let axes = self.config.axes;
        let update_entity = cx.entity();
        let update =
            move |gesture: crate::interaction::GestureUpdate, _: &mut Window, cx: &mut App| {
                if gesture.moved() {
                    let delta = gesture.delta();
                    let next = pan_transform(start, delta, axes);
                    update_entity.update(cx, |entity, cx| entity.set_preview(next, cx));
                }
                crate::interaction::InteractionFlow::Continue
            };
        let finish_entity = cx.entity();
        let finish =
            move |gesture: crate::interaction::GestureUpdate, window: &mut Window, cx: &mut App| {
                if gesture.moved() {
                    let next = pan_transform(start, gesture.delta(), axes);
                    finish_entity.update(cx, |entity, cx| entity.propose(next, window, cx));
                } else {
                    finish_entity.update(cx, |entity, cx| entity.restore_source(cx));
                }
            };
        let cancel_entity = cx.entity();
        let cancel = move |_: &mut Window, cx: &mut App| {
            cancel_entity.update(cx, |entity, cx| entity.restore_source(cx));
        };
        let owner = self.context.interaction_owner(&self.config.id);
        self.context.begin_interaction(
            crate::interaction::NativeGesture::new(
                owner,
                event.position,
                cx.entity_id(),
                update,
                finish,
                cancel,
            )
            .with_button(self.config.pan_button)
            .with_threshold(self.config.threshold),
            window,
            cx,
        );
        cx.stop_propagation();
    }

    fn wheel(&mut self, event: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Self>) {
        let explicit =
            read_bool_signal(&self.context, &self.config.wheel_active_signal, cx).unwrap_or(false);
        if self.suspended || self.config.disabled {
            return;
        }
        if matches!(event.touch_phase, gpui::TouchPhase::Cancelled) {
            if explicit {
                invalidate_wheel(&self.context, &self.config, cx);
                self.restore_source(cx);
                cx.stop_propagation();
            }
            return;
        }
        let ending = matches!(event.touch_phase, gpui::TouchPhase::Ended) && explicit;
        let enabled = match self.config.wheel_zoom {
            WheelZoom::Off => false,
            WheelZoom::Modifier => event.modifiers.control || event.modifiers.platform || ending,
            WheelZoom::Always => true,
        };
        if !enabled {
            return;
        }
        if ending {
            write_bool_signal(&self.context, &self.config.wheel_active_signal, false, cx);
            self.commit_wheel(window, cx);
            cx.stop_propagation();
            return;
        }
        if matches!(event.touch_phase, gpui::TouchPhase::Started) {
            invalidate_wheel(&self.context, &self.config, cx);
            write_bool_signal(&self.context, &self.config.wheel_active_signal, true, cx);
        }
        let Some(viewport) = self.context.element_bounds(&self.config.viewport_ref, cx) else {
            return;
        };
        let delta = event.delta.pixel_delta(px(16.0));
        let logical = if delta.y.abs() >= delta.x.abs() {
            f64::from(delta.y)
        } else {
            f64::from(delta.x)
        };
        if logical.abs() <= f64::EPSILON {
            return;
        }
        let factor = (-logical / 400.0).exp();
        let anchor = (
            f64::from(event.position.x) - viewport.x,
            f64::from(event.position.y) - viewport.y,
        );
        let next = zoom_transform(
            self.preview,
            factor,
            anchor,
            (viewport.width, viewport.height),
            self.config.minimum_scale,
            self.config.maximum_scale,
        );
        if !transform_changed(self.preview, next) {
            return;
        }
        write_bool_signal(&self.context, &self.config.wheel_pending_signal, true, cx);
        self.set_preview(next, cx);
        let cancel_entity = cx.entity();
        let owner = self.context.interaction_owner(&self.config.id);
        self.context.register_interaction_cancellation(
            owner,
            cx.entity_id(),
            move |_, cx| {
                cancel_entity.update(cx, |entity, cx| {
                    invalidate_wheel(&entity.context, &entity.config, cx);
                    entity.restore_source(cx);
                });
            },
            window,
            cx,
        );
        cx.stop_propagation();
        if explicit || matches!(event.touch_phase, gpui::TouchPhase::Started) {
            return;
        }
        if event.delta.precise() {
            self.schedule_wheel_commit(window, cx);
        } else {
            self.commit_wheel(window, cx);
        }
    }

    fn schedule_wheel_commit(&self, window: &mut Window, cx: &mut Context<Self>) {
        let generation = next_wheel_generation(&self.context, &self.config, cx);
        let context = self.context.clone();
        let config = self.config.clone();
        cx.spawn_in(window, async move |_, cx| {
            cx.background_executor().timer(WHEEL_COMMIT_DELAY).await;
            let _ = cx.update(|window, cx| {
                if read_integer_signal(&context, &config.wheel_generation_signal, cx)
                    == Some(generation)
                    && let Some(transform) = read_transform(&context, &config, cx)
                {
                    invalidate_wheel(&context, &config, cx);
                    write_transform(&context, &config, config.source, cx);
                    if transform_changed(config.source, transform) {
                        context.propose("transform_change", transform_value(transform), window, cx);
                    }
                }
            });
        })
        .detach();
    }

    fn commit_wheel(&mut self, window: &mut Window, cx: &mut App) {
        let Some(next) = read_transform(&self.context, &self.config, cx) else {
            return;
        };
        invalidate_wheel(&self.context, &self.config, cx);
        self.propose(next, window, cx);
    }
}

impl Render for PanZoomEntity {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.context
            .present_interaction(self.context.interaction_owner(&self.config.id));
        div()
            .size_full()
            .on_mouse_down(self.config.pan_button, cx.listener(Self::mouse_down))
            .on_scroll_wheel(cx.listener(Self::wheel))
    }
}

#[derive(Default)]
pub struct PanZoomPrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, Entity<PanZoomEntity>>,
}

impl PrimitiveHandler for PanZoomPrimitiveHandler {
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
        let id = instance
            .id
            .clone()
            .ok_or_else(|| "PanZoomPrimitive requires a stable key".to_owned())?;
        let config = parse_config(&instance.node.props, instance.focus_handle().cloned())?;
        let keyboard_config = config.clone();
        let keyboard_context = context.clone();
        let keyboard_focus = config.focus.clone();
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let entity = cx.new(|cx| PanZoomEntity::new(config.clone(), context.clone(), cx));
            self.instances.insert(id.clone(), entity.clone());
            entity
        };
        entity.update(cx, |pan_zoom, cx| {
            pan_zoom.update_config(config, context.clone(), window, cx);
        });
        let mut root = div().size_full().child(entity);
        if let Some(focus) = keyboard_focus {
            root = root.track_focus(&focus.tab_stop(!keyboard_config.disabled));
        }
        Ok(root
            .on_key_down(move |event: &KeyDownEvent, window, cx: &mut App| {
                if let Some(next) =
                    keyboard_transform(&keyboard_config, &keyboard_context, event, cx)
                {
                    keyboard_context.propose("transform_change", transform_value(next), window, cx);
                    cx.stop_propagation();
                }
            })
            .into_any_element())
    }

    fn suspend(&mut self, instance: &PrimitiveInstanceId, cx: &mut App) {
        if let Some(entity) = self.instances.get(instance) {
            entity.update(cx, |pan_zoom, cx| {
                invalidate_wheel(&pan_zoom.context, &pan_zoom.config, cx);
                pan_zoom.restore_source(cx);
                pan_zoom.suspended = true;
            });
        }
    }

    fn resume(&mut self, instance: &PrimitiveInstanceId, cx: &mut App) {
        if let Some(entity) = self.instances.get(instance) {
            entity.update(cx, |pan_zoom, _| pan_zoom.suspended = false);
        }
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

fn parse_config(
    props: &PrimitiveProps,
    focus: Option<FocusHandle>,
) -> Result<PanZoomConfig, String> {
    let source = ViewTransform {
        x: required_number(props, "x")?,
        y: required_number(props, "y")?,
        scale: required_number(props, "scale")?,
    };
    let minimum_scale = required_number(props, "min_scale")?;
    let maximum_scale = required_number(props, "max_scale")?;
    if source.x.abs() > MAX_TRANSLATION
        || source.y.abs() > MAX_TRANSLATION
        || minimum_scale <= 0.0
        || maximum_scale > MAX_RENDER_SCALE
        || maximum_scale < minimum_scale
        || source.scale < minimum_scale
        || source.scale > maximum_scale
    {
        return Err("pan zoom transform or scale range is invalid".to_owned());
    }
    let axes = match props.string("axes") {
        None | Some("both") => PanAxes::Both,
        Some("horizontal") => PanAxes::Horizontal,
        Some("vertical") => PanAxes::Vertical,
        Some(_) => return Err("pan zoom axes must be both, horizontal, or vertical".to_owned()),
    };
    let wheel_zoom = match props.string("wheel_zoom") {
        None | Some("modifier") => WheelZoom::Modifier,
        Some("always") => WheelZoom::Always,
        Some("off") => WheelZoom::Off,
        Some(_) => return Err("pan zoom wheel_zoom must be off, modifier, or always".to_owned()),
    };
    let pan_button = match props.string("pan_button") {
        None | Some("left") => MouseButton::Left,
        Some("middle") => MouseButton::Middle,
        Some(_) => return Err("pan zoom pan_button must be left or middle".to_owned()),
    };
    let threshold = bounded_number(props, "threshold", 0.0, 64.0, 4.0)?;
    let keyboard_pan_step = bounded_number(props, "keyboard_pan_step", 0.1, 512.0, 16.0)?;
    let keyboard_zoom_factor = bounded_number(props, "keyboard_zoom_factor", 1.001, 4.0, 1.2)?;
    let x_signal = float_signal(props, "x_signal")?;
    let y_signal = float_signal(props, "y_signal")?;
    let scale_signals = (
        float_signal(props, "scale_x_signal")?,
        float_signal(props, "scale_y_signal")?,
    );
    let wheel_generation_signal =
        typed_signal(props, "wheel_generation_signal", SignalKind::Integer)?;
    let wheel_active_signal = typed_signal(props, "wheel_active_signal", SignalKind::Bool)?;
    let wheel_pending_signal = typed_signal(props, "wheel_pending_signal", SignalKind::Bool)?;
    let source_token_signal = typed_signal(props, "source_token_signal", SignalKind::String)?;
    let source_token = props
        .string("source_token")
        .filter(|value| {
            !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
        })
        .ok_or_else(|| "pan zoom source_token must be 1-512 non-control characters".to_owned())?
        .to_owned();
    Ok(PanZoomConfig {
        id: format!(
            "gpui-rhai-pan-zoom:{}:{}",
            x_signal.id().component(),
            x_signal.id().key()
        ),
        source_token,
        source,
        minimum_scale,
        maximum_scale,
        axes,
        wheel_zoom,
        pan_button,
        threshold,
        keyboard_pan_step,
        keyboard_zoom_factor,
        disabled: props.boolean("disabled").unwrap_or(false),
        viewport_ref: props
            .element_ref("viewport_ref")
            .cloned()
            .ok_or_else(|| "pan zoom viewport_ref is required".to_owned())?,
        x_signal,
        y_signal,
        scale_x_signal: scale_signals.0,
        scale_y_signal: scale_signals.1,
        wheel_generation_signal,
        wheel_active_signal,
        wheel_pending_signal,
        source_token_signal,
        focus,
    })
}

fn required_number(props: &PrimitiveProps, name: &str) -> Result<f64, String> {
    props
        .number(name)
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("pan zoom requires finite numeric {name}"))
}

fn bounded_number(
    props: &PrimitiveProps,
    name: &str,
    minimum: f64,
    maximum: f64,
    fallback: f64,
) -> Result<f64, String> {
    let value = props.number(name).unwrap_or(fallback);
    if value.is_finite() && (minimum..=maximum).contains(&value) {
        Ok(value)
    } else {
        Err(format!("pan zoom {name} must be in [{minimum},{maximum}]"))
    }
}

fn float_signal(props: &PrimitiveProps, name: &str) -> Result<crate::NativeSignal, String> {
    typed_signal(props, name, SignalKind::Float)
}

fn typed_signal(
    props: &PrimitiveProps,
    name: &str,
    kind: SignalKind,
) -> Result<crate::NativeSignal, String> {
    let signal = props
        .signal(name)
        .cloned()
        .ok_or_else(|| format!("pan zoom requires signal {name}"))?;
    if signal.id().kind() != kind {
        return Err(format!("pan zoom {name} must be {}", kind.as_str()));
    }
    Ok(signal)
}

fn write_transform(
    context: &PrimitiveContext,
    config: &PanZoomConfig,
    transform: ViewTransform,
    cx: &mut App,
) {
    let _ = context.write_signals(
        [
            (config.x_signal.clone(), SignalValue::Float(transform.x)),
            (config.y_signal.clone(), SignalValue::Float(transform.y)),
            (
                config.scale_x_signal.clone(),
                SignalValue::Float(transform.scale),
            ),
            (
                config.scale_y_signal.clone(),
                SignalValue::Float(transform.scale),
            ),
        ],
        cx,
    );
}

fn read_transform(
    context: &PrimitiveContext,
    config: &PanZoomConfig,
    cx: &App,
) -> Option<ViewTransform> {
    Some(ViewTransform {
        x: read_float_signal(context, &config.x_signal, cx)?,
        y: read_float_signal(context, &config.y_signal, cx)?,
        scale: read_float_signal(context, &config.scale_x_signal, cx)?,
    })
}

fn read_float_signal(
    context: &PrimitiveContext,
    signal: &crate::NativeSignal,
    cx: &App,
) -> Option<f64> {
    match context.read_signal(signal, cx).ok()? {
        SignalValue::Float(value) => Some(value),
        _ => None,
    }
}

fn read_integer_signal(
    context: &PrimitiveContext,
    signal: &crate::NativeSignal,
    cx: &App,
) -> Option<i64> {
    match context.read_signal(signal, cx).ok()? {
        SignalValue::Integer(value) => Some(value),
        _ => None,
    }
}

fn read_bool_signal(
    context: &PrimitiveContext,
    signal: &crate::NativeSignal,
    cx: &App,
) -> Option<bool> {
    match context.read_signal(signal, cx).ok()? {
        SignalValue::Bool(value) => Some(value),
        _ => None,
    }
}

fn read_string_signal(
    context: &PrimitiveContext,
    signal: &crate::NativeSignal,
    cx: &App,
) -> Option<String> {
    match context.read_signal(signal, cx).ok()? {
        SignalValue::String(value) => Some(value),
        _ => None,
    }
}

fn write_bool_signal(
    context: &PrimitiveContext,
    signal: &crate::NativeSignal,
    value: bool,
    cx: &mut App,
) {
    let _ = context.write_signal(signal, SignalValue::Bool(value), cx);
}

fn write_string_signal(
    context: &PrimitiveContext,
    signal: &crate::NativeSignal,
    value: &str,
    cx: &mut App,
) {
    let _ = context.write_signal(signal, SignalValue::String(value.to_owned()), cx);
}

fn next_wheel_generation(context: &PrimitiveContext, config: &PanZoomConfig, cx: &mut App) -> i64 {
    let generation = read_integer_signal(context, &config.wheel_generation_signal, cx)
        .unwrap_or(0)
        .saturating_add(1);
    let _ = context.write_signal(
        &config.wheel_generation_signal,
        SignalValue::Integer(generation),
        cx,
    );
    generation
}

fn invalidate_wheel(context: &PrimitiveContext, config: &PanZoomConfig, cx: &mut App) {
    context.clear_interaction_cancellation(&context.interaction_owner(&config.id));
    let _ = next_wheel_generation(context, config, cx);
    write_bool_signal(context, &config.wheel_active_signal, false, cx);
    write_bool_signal(context, &config.wheel_pending_signal, false, cx);
}

fn pan_transform(start: ViewTransform, delta: (f64, f64), axes: PanAxes) -> ViewTransform {
    ViewTransform {
        x: if matches!(axes, PanAxes::Both | PanAxes::Horizontal) {
            (start.x + delta.0).clamp(-MAX_TRANSLATION, MAX_TRANSLATION)
        } else {
            start.x
        },
        y: if matches!(axes, PanAxes::Both | PanAxes::Vertical) {
            (start.y + delta.1).clamp(-MAX_TRANSLATION, MAX_TRANSLATION)
        } else {
            start.y
        },
        scale: start.scale,
    }
}

fn zoom_transform(
    start: ViewTransform,
    factor: f64,
    anchor: (f64, f64),
    viewport: (f64, f64),
    minimum: f64,
    maximum: f64,
) -> ViewTransform {
    let scale = (start.scale * factor).clamp(minimum, maximum);
    if (scale - start.scale).abs() <= f64::EPSILON {
        return start;
    }
    let ratio = scale / start.scale;
    let center = (viewport.0 / 2.0, viewport.1 / 2.0);
    ViewTransform {
        x: ((1.0 - ratio) * (anchor.0 - center.0) + ratio * start.x)
            .clamp(-MAX_TRANSLATION, MAX_TRANSLATION),
        y: ((1.0 - ratio) * (anchor.1 - center.1) + ratio * start.y)
            .clamp(-MAX_TRANSLATION, MAX_TRANSLATION),
        scale,
    }
}

fn keyboard_transform(
    config: &PanZoomConfig,
    context: &PrimitiveContext,
    event: &KeyDownEvent,
    cx: &App,
) -> Option<ViewTransform> {
    if config.disabled {
        return None;
    }
    let multiplier = if event.keystroke.modifiers.shift {
        4.0
    } else {
        1.0
    };
    let pan = config.keyboard_pan_step * multiplier;
    let next = match event.keystroke.key.as_str() {
        "left" => pan_transform(config.source, (-pan, 0.0), config.axes),
        "right" => pan_transform(config.source, (pan, 0.0), config.axes),
        "up" => pan_transform(config.source, (0.0, -pan), config.axes),
        "down" => pan_transform(config.source, (0.0, pan), config.axes),
        "+" | "=" => centered_zoom(config, config.keyboard_zoom_factor, context, cx),
        "-" => centered_zoom(config, 1.0 / config.keyboard_zoom_factor, context, cx),
        "0" => ViewTransform {
            x: 0.0,
            y: 0.0,
            scale: 1.0f64.clamp(config.minimum_scale, config.maximum_scale),
        },
        _ => return None,
    };
    transform_changed(config.source, next).then_some(next)
}

fn centered_zoom(
    config: &PanZoomConfig,
    factor: f64,
    context: &PrimitiveContext,
    cx: &App,
) -> ViewTransform {
    context
        .element_bounds(&config.viewport_ref, cx)
        .map_or_else(
            || ViewTransform {
                scale: (config.source.scale * factor)
                    .clamp(config.minimum_scale, config.maximum_scale),
                ..config.source
            },
            |viewport| {
                zoom_transform(
                    config.source,
                    factor,
                    (viewport.width / 2.0, viewport.height / 2.0),
                    (viewport.width, viewport.height),
                    config.minimum_scale,
                    config.maximum_scale,
                )
            },
        )
}

fn contains(bounds: crate::GeometryBounds, point: Point<Pixels>) -> bool {
    let x = f64::from(point.x);
    let y = f64::from(point.y);
    x >= bounds.x && x <= bounds.x + bounds.width && y >= bounds.y && y <= bounds.y + bounds.height
}

fn transform_changed(left: ViewTransform, right: ViewTransform) -> bool {
    (left.x - right.x).abs() > f64::EPSILON
        || (left.y - right.y).abs() > f64::EPSILON
        || (left.scale - right.scale).abs() > f64::EPSILON
}

fn transform_value(transform: ViewTransform) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("x".to_owned(), UiValue::Float(transform.x)),
        ("y".to_owned(), UiValue::Float(transform.y)),
        ("scale".to_owned(), UiValue::Float(transform.scale)),
    ]))
}

fn transform_schema() -> ValueSchema {
    ValueSchema::object(BTreeMap::from([
        ("x".to_owned(), ObjectField::required(ValueSchema::number())),
        ("y".to_owned(), ObjectField::required(ValueSchema::number())),
        (
            "scale".to_owned(),
            ObjectField::required(ValueSchema::positive_number()),
        ),
    ]))
}

fn pan_zoom_signal_props() -> BTreeMap<String, ObjectField> {
    [
        "x_signal",
        "y_signal",
        "scale_x_signal",
        "scale_y_signal",
        "wheel_generation_signal",
        "wheel_active_signal",
        "wheel_pending_signal",
        "source_token_signal",
    ]
    .into_iter()
    .map(|name| (name.to_owned(), ObjectField::required(ValueSchema::Signal)))
    .collect()
}

/// Build the native controlled Canvas pan/zoom schema.
///
/// # Panics
///
/// Panics only if the static primitive ID becomes invalid.
#[must_use]
pub fn pan_zoom_primitive_descriptor() -> PrimitiveDescriptor {
    let mut props = BTreeMap::from([
        (
            "source_token".to_owned(),
            ObjectField::required(ValueSchema::string()),
        ),
        ("x".to_owned(), ObjectField::required(ValueSchema::number())),
        ("y".to_owned(), ObjectField::required(ValueSchema::number())),
        (
            "scale".to_owned(),
            ObjectField::required(ValueSchema::positive_number()),
        ),
        (
            "min_scale".to_owned(),
            ObjectField::required(ValueSchema::positive_number()),
        ),
        (
            "max_scale".to_owned(),
            ObjectField::required(ValueSchema::positive_number()),
        ),
    ]);
    props.extend(pan_zoom_signal_props());
    props.extend([
        (
            "axes".to_owned(),
            ObjectField::optional(ValueSchema::String {
                allowed: vec![
                    "both".to_owned(),
                    "horizontal".to_owned(),
                    "vertical".to_owned(),
                ],
            })
            .with_default(UiValue::String("both".to_owned())),
        ),
        (
            "wheel_zoom".to_owned(),
            ObjectField::optional(ValueSchema::String {
                allowed: vec!["off".to_owned(), "modifier".to_owned(), "always".to_owned()],
            })
            .with_default(UiValue::String("modifier".to_owned())),
        ),
        (
            "pan_button".to_owned(),
            ObjectField::optional(ValueSchema::String {
                allowed: vec!["left".to_owned(), "middle".to_owned()],
            })
            .with_default(UiValue::String("left".to_owned())),
        ),
        (
            "threshold".to_owned(),
            ObjectField::optional(ValueSchema::bounded_number(Some(0.0), Some(64.0))),
        ),
        (
            "keyboard_pan_step".to_owned(),
            ObjectField::optional(ValueSchema::bounded_number(Some(0.1), Some(512.0))),
        ),
        (
            "keyboard_zoom_factor".to_owned(),
            ObjectField::optional(ValueSchema::bounded_number(Some(1.001), Some(4.0))),
        ),
        (
            "disabled".to_owned(),
            ObjectField::optional(ValueSchema::Bool).with_default(UiValue::Bool(false)),
        ),
        (
            "viewport_ref".to_owned(),
            ObjectField::required(ValueSchema::Ref),
        ),
        (
            "on_transform_change".to_owned(),
            ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
        ),
    ]);
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.pan_zoom").expect("static primitive ID"),
        export: "PanZoomPrimitive".to_owned(),
        props,
        events: BTreeMap::from([(
            "transform_change".to_owned(),
            EventSchema {
                payload: transform_schema(),
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
    fn pointer_anchored_zoom_preserves_the_content_point() {
        let start = ViewTransform {
            x: 14.0,
            y: -8.0,
            scale: 1.25,
        };
        let anchor = (70.0, 40.0);
        let viewport = (300.0, 180.0);
        let next = zoom_transform(start, 1.6, anchor, viewport, 0.25, 8.0);
        let center = (viewport.0 / 2.0, viewport.1 / 2.0);
        let content = (
            center.0 + (anchor.0 - start.x - center.0) / start.scale,
            center.1 + (anchor.1 - start.y - center.1) / start.scale,
        );
        let projected = (
            center.0 + next.scale * (content.0 - center.0) + next.x,
            center.1 + next.scale * (content.1 - center.1) + next.y,
        );
        assert!((projected.0 - anchor.0).abs() < 0.000_001);
        assert!((projected.1 - anchor.1).abs() < 0.000_001);
    }
}
