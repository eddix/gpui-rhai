//! Native pointer hot lane for the public controlled `Draggable` component.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, CursorStyle, DispatchPhase, Element, ElementId, FocusHandle,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, InteractiveElement, IntoElement,
    KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, ParentElement, Pixels, Style, Styled,
    Window, div, relative, size,
};

use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveContext, PrimitiveDescriptor,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveProps, PrimitiveTheme, SignalKind,
    SignalValue, UiValue, ValueSchema,
};

const MAX_POSITION: f64 = 1_000_000.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DragAxes {
    Both,
    Horizontal,
    Vertical,
}

#[derive(Clone)]
struct DragConfig {
    id: String,
    source_x: f64,
    source_y: f64,
    axes: DragAxes,
    contain: bool,
    threshold: f64,
    snap_x: Option<f64>,
    snap_y: Option<f64>,
    keyboard_step: f64,
    disabled: bool,
    boundary_ref: crate::ElementRef,
    object_ref: crate::ElementRef,
    handle_ref: Option<crate::ElementRef>,
    x_signal: crate::NativeSignal,
    y_signal: crate::NativeSignal,
    focus: Option<FocusHandle>,
}

#[derive(Clone, Debug, PartialEq)]
struct DragSourceIdentity {
    x: f64,
    y: f64,
    x_signal: crate::SignalId,
    y_signal: crate::SignalId,
    disabled: bool,
}

#[derive(Clone)]
struct DragState(Rc<RefCell<DragSourceIdentity>>);

impl DragState {
    fn new(config: &DragConfig) -> Self {
        Self(Rc::new(RefCell::new(source_identity(config))))
    }
}

fn source_identity(config: &DragConfig) -> DragSourceIdentity {
    DragSourceIdentity {
        x: config.source_x,
        y: config.source_y,
        x_signal: config.x_signal.id().clone(),
        y_signal: config.y_signal.id().clone(),
        disabled: config.disabled,
    }
}

struct DragPrepaint {
    hitbox: Hitbox,
}

struct DragHandleElement {
    config: DragConfig,
    context: PrimitiveContext,
}

impl IntoElement for DragHandleElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for DragHandleElement {
    type RequestLayoutState = ();
    type PrepaintState = DragPrepaint;

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
    ) -> DragPrepaint {
        let state = window
            .use_state(cx, |_, _| DragState::new(&self.config))
            .read(cx)
            .clone();
        let changed = {
            let next = source_identity(&self.config);
            let mut source = state.0.borrow_mut();
            let changed = *source != next;
            *source = next;
            changed
        };
        let owner = self.context.interaction_owner(&self.config.id);
        if changed && !self.context.cancel_interaction(&owner, window, cx) {
            clear_preview(&self.context, &self.config, cx);
        }
        DragPrepaint {
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut (),
        prepaint: &mut DragPrepaint,
        window: &mut Window,
        _: &mut App,
    ) {
        let owner = self.context.interaction_owner(&self.config.id);
        self.context.present_interaction(owner);
        if !self.config.disabled {
            window.set_cursor_style(CursorStyle::OpenHand, &prepaint.hitbox);
        }
        register_pointer_down(prepaint, &self.config, &self.context, window);
    }
}

fn register_pointer_down(
    prepaint: &DragPrepaint,
    config: &DragConfig,
    context: &PrimitiveContext,
    window: &mut Window,
) {
    let hitbox = prepaint.hitbox.clone();
    let config = config.clone();
    let context = context.clone();
    let view = window.current_view();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble
            || event.button != MouseButton::Left
            || config.disabled
            || !hitbox.is_hovered(window)
        {
            return;
        }
        let eligible_ref = config.handle_ref.as_ref().unwrap_or(&config.object_ref);
        let Some(eligible) = context.element_bounds(eligible_ref, cx) else {
            return;
        };
        if !contains(eligible, event.position) {
            return;
        }
        let (Some(boundary), Some(object)) = (
            context.element_bounds(&config.boundary_ref, cx),
            context.element_bounds(&config.object_ref, cx),
        ) else {
            return;
        };
        if config.contain
            && (config.source_x < 0.0
                || config.source_y < 0.0
                || config.source_x + object.width > boundary.width + 0.5
                || config.source_y + object.height > boundary.height + 0.5)
        {
            return;
        }
        if let Some(focus) = config.focus.as_ref() {
            focus.focus(window, cx);
        }
        let boundary_size = (boundary.width, boundary.height);
        let object_size = (object.width, object.height);
        let update_config = config.clone();
        let update_context = context.clone();
        let update =
            move |gesture: crate::interaction::GestureUpdate, _: &mut Window, cx: &mut App| {
                let Some(boundary) = update_context.element_bounds(&update_config.boundary_ref, cx)
                else {
                    return crate::interaction::InteractionFlow::Cancel;
                };
                if (boundary.width - boundary_size.0).abs() > 0.5
                    || (boundary.height - boundary_size.1).abs() > 0.5
                {
                    return crate::interaction::InteractionFlow::Cancel;
                }
                if gesture.moved() {
                    let next =
                        drag_position(&update_config, gesture.delta(), boundary_size, object_size);
                    write_preview(&update_context, &update_config, Some(next), cx);
                }
                crate::interaction::InteractionFlow::Continue
            };
        let finish_config = config.clone();
        let finish_context = context.clone();
        let finish =
            move |gesture: crate::interaction::GestureUpdate, window: &mut Window, cx: &mut App| {
                clear_preview(&finish_context, &finish_config, cx);
                if gesture.moved() {
                    let next =
                        drag_position(&finish_config, gesture.delta(), boundary_size, object_size);
                    if position_changed(&finish_config, next) {
                        finish_context.propose("move", position_value(next), window, cx);
                    }
                }
            };
        let cancel_config = config.clone();
        let cancel_context = context.clone();
        let cancel = move |_: &mut Window, cx: &mut App| {
            clear_preview(&cancel_context, &cancel_config, cx);
        };
        let owner = context.interaction_owner(&config.id);
        context.begin_interaction(
            crate::interaction::NativeGesture::new(
                owner,
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

fn contains(bounds: crate::GeometryBounds, point: gpui::Point<Pixels>) -> bool {
    let x = f64::from(point.x);
    let y = f64::from(point.y);
    x >= bounds.x && x <= bounds.x + bounds.width && y >= bounds.y && y <= bounds.y + bounds.height
}

fn drag_position(
    config: &DragConfig,
    delta: (f64, f64),
    boundary: (f64, f64),
    object: (f64, f64),
) -> (f64, f64) {
    let mut x = config.source_x;
    let mut y = config.source_y;
    if matches!(config.axes, DragAxes::Both | DragAxes::Horizontal) {
        x += delta.0;
        x = snap(x, config.snap_x);
    }
    if matches!(config.axes, DragAxes::Both | DragAxes::Vertical) {
        y += delta.1;
        y = snap(y, config.snap_y);
    }
    if config.contain {
        x = x.clamp(0.0, (boundary.0 - object.0).max(0.0));
        y = y.clamp(0.0, (boundary.1 - object.1).max(0.0));
    }
    (x, y)
}

fn snap(value: f64, step: Option<f64>) -> f64 {
    step.map_or(value, |step| (value / step).round() * step)
}

fn write_preview(
    context: &PrimitiveContext,
    config: &DragConfig,
    position: Option<(f64, f64)>,
    cx: &mut App,
) {
    let _ = context.write_signals(
        [
            (
                config.x_signal.clone(),
                SignalValue::OptionalFloat(position.map(|position| position.0 - config.source_x)),
            ),
            (
                config.y_signal.clone(),
                SignalValue::OptionalFloat(position.map(|position| position.1 - config.source_y)),
            ),
        ],
        cx,
    );
}

fn clear_preview(context: &PrimitiveContext, config: &DragConfig, cx: &mut App) {
    write_preview(context, config, None, cx);
}

fn position_value(position: (f64, f64)) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("x".to_owned(), UiValue::Float(position.0)),
        ("y".to_owned(), UiValue::Float(position.1)),
    ]))
}

fn position_changed(config: &DragConfig, position: (f64, f64)) -> bool {
    (position.0 - config.source_x).abs() > f64::EPSILON
        || (position.1 - config.source_y).abs() > f64::EPSILON
}

#[derive(Default)]
pub struct DraggablePrimitiveHandler;

impl PrimitiveHandler for DraggablePrimitiveHandler {
    fn uses_primary_focus(&self) -> bool {
        true
    }

    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        context: &PrimitiveContext,
        _: &PrimitiveTheme,
        _: &mut Window,
        _: &mut App,
    ) -> Result<AnyElement, String> {
        let config = parse_config(&instance.node.props, instance.focus_handle().cloned())?;
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
                let step = key_config.keyboard_step * multiplier;
                let delta = match event.keystroke.key.as_str() {
                    "left" => (-step, 0.0),
                    "right" => (step, 0.0),
                    "up" => (0.0, -step),
                    "down" => (0.0, step),
                    _ => return,
                };
                let (Some(boundary), Some(object)) = (
                    key_context.element_bounds(&key_config.boundary_ref, cx),
                    key_context.element_bounds(&key_config.object_ref, cx),
                ) else {
                    return;
                };
                let next = drag_position(
                    &key_config,
                    delta,
                    (boundary.width, boundary.height),
                    (object.width, object.height),
                );
                if position_changed(&key_config, next) {
                    key_context.propose("move", position_value(next), window, cx);
                }
                cx.stop_propagation();
            })
            .child(DragHandleElement {
                config,
                context: context.clone(),
            })
            .into_any_element())
    }
}

fn parse_config(props: &PrimitiveProps, focus: Option<FocusHandle>) -> Result<DragConfig, String> {
    let source_x = required_number(props, "x")?;
    let source_y = required_number(props, "y")?;
    if source_x.abs() > MAX_POSITION || source_y.abs() > MAX_POSITION {
        return Err("draggable position exceeds the supported range".to_owned());
    }
    let axes = match props.string("axes") {
        None | Some("both") => DragAxes::Both,
        Some("horizontal") => DragAxes::Horizontal,
        Some("vertical") => DragAxes::Vertical,
        Some(_) => return Err("draggable axes must be both, horizontal, or vertical".to_owned()),
    };
    let threshold = props.number("threshold").unwrap_or(4.0);
    let keyboard_step = props.number("keyboard_step").unwrap_or(8.0);
    let snap_x = optional_positive_number(props, "snap_x")?;
    let snap_y = optional_positive_number(props, "snap_y")?;
    if !threshold.is_finite()
        || !(0.0..=64.0).contains(&threshold)
        || !keyboard_step.is_finite()
        || !(0.0..=512.0).contains(&keyboard_step)
        || keyboard_step == 0.0
    {
        return Err("draggable threshold or keyboard_step is invalid".to_owned());
    }
    let x_signal = optional_float_signal(props, "x_signal")?;
    let y_signal = optional_float_signal(props, "y_signal")?;
    Ok(DragConfig {
        id: format!(
            "gpui-rhai-draggable:{}:{}",
            x_signal.id().component(),
            x_signal.id().key()
        ),
        source_x,
        source_y,
        axes,
        contain: props.boolean("contain").unwrap_or(true),
        threshold,
        snap_x,
        snap_y,
        keyboard_step,
        disabled: props.boolean("disabled").unwrap_or(false),
        boundary_ref: props
            .element_ref("boundary_ref")
            .cloned()
            .ok_or_else(|| "draggable requires boundary_ref".to_owned())?,
        object_ref: props
            .element_ref("object_ref")
            .cloned()
            .ok_or_else(|| "draggable requires object_ref".to_owned())?,
        handle_ref: props.element_ref("handle_ref").cloned(),
        x_signal,
        y_signal,
        focus,
    })
}

fn required_number(props: &PrimitiveProps, name: &str) -> Result<f64, String> {
    props
        .number(name)
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("draggable requires finite numeric {name}"))
}

fn optional_positive_number(props: &PrimitiveProps, name: &str) -> Result<Option<f64>, String> {
    match props.data(name) {
        None | Some(UiValue::Null) => Ok(None),
        Some(_) => props
            .number(name)
            .filter(|value| value.is_finite() && *value > 0.0)
            .map(Some)
            .ok_or_else(|| format!("draggable {name} must be a positive finite number")),
    }
}

fn optional_float_signal(
    props: &PrimitiveProps,
    name: &str,
) -> Result<crate::NativeSignal, String> {
    let signal = props
        .signal(name)
        .cloned()
        .ok_or_else(|| format!("draggable requires signal {name}"))?;
    if signal.id().kind() != SignalKind::OptionalFloat {
        return Err(format!("draggable {name} must be optional_float"));
    }
    Ok(signal)
}

fn position_schema() -> ValueSchema {
    ValueSchema::object(BTreeMap::from([
        ("x".to_owned(), ObjectField::required(ValueSchema::number())),
        ("y".to_owned(), ObjectField::required(ValueSchema::number())),
    ]))
}

/// Build the native controlled-position interaction schema.
///
/// # Panics
///
/// Panics only if the static built-in primitive ID becomes invalid.
#[must_use]
#[allow(clippy::too_many_lines)] // One declarative list of documented props and events.
pub fn draggable_primitive_descriptor() -> PrimitiveDescriptor {
    let optional_number = || ObjectField::optional(ValueSchema::optional(ValueSchema::number()));
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.draggable").expect("static primitive ID"),
        export: "DraggablePrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "x".to_owned(),
                ObjectField::required(ValueSchema::number()).with_doc(
                    "Controlled left edge of the object in `boundary_ref`'s local logical pixels.",
                ),
            ),
            (
                "y".to_owned(),
                ObjectField::required(ValueSchema::number()).with_doc(
                    "Controlled top edge of the object in `boundary_ref`'s local logical pixels.",
                ),
            ),
            (
                "axes".to_owned(),
                ObjectField::optional(ValueSchema::String {
                    allowed: vec![
                        "both".to_owned(),
                        "horizontal".to_owned(),
                        "vertical".to_owned(),
                    ],
                })
                .with_default(UiValue::String("both".to_owned()))
                .with_doc("Directions the object moves in: `both`, `horizontal` or `vertical`."),
            ),
            (
                "contain".to_owned(),
                ObjectField::optional(ValueSchema::Bool)
                    .with_default(UiValue::Bool(true))
                    .with_doc(
                        "Keeps the object inside `boundary_ref`; presses are ignored while the controlled position lies outside it.",
                    ),
            ),
            (
                "threshold".to_owned(),
                ObjectField::optional(ValueSchema::bounded_number(Some(0.0), Some(64.0))).with_doc(
                    "Pointer movement in logical pixels before a press becomes a drag; defaults to 4.",
                ),
            ),
            (
                "snap_x".to_owned(),
                optional_number().with_doc(
                    "Grid step in logical pixels the left edge rounds to, or `()` to move freely.",
                ),
            ),
            (
                "snap_y".to_owned(),
                optional_number().with_doc(
                    "Grid step in logical pixels the top edge rounds to, or `()` to move freely.",
                ),
            ),
            (
                "keyboard_step".to_owned(),
                ObjectField::optional(ValueSchema::positive_number()).with_doc(
                    "Logical pixels one arrow-key press moves the object, up to 512; Shift multiplies it by four; defaults to 8.",
                ),
            ),
            (
                "disabled".to_owned(),
                ObjectField::optional(ValueSchema::Bool)
                    .with_default(UiValue::Bool(false))
                    .with_doc(
                        "Ignores presses and arrow keys and removes the object from the tab order.",
                    ),
            ),
            (
                "boundary_ref".to_owned(),
                ObjectField::required(ValueSchema::Ref).with_doc(
                    "Ref to the container the position is local to; resizing it during a drag cancels the drag.",
                ),
            ),
            (
                "object_ref".to_owned(),
                ObjectField::required(ValueSchema::Ref).with_doc(
                    "Ref to the moved object; its size bounds `contain`, and without `handle_ref` a press on it starts a drag.",
                ),
            ),
            (
                "handle_ref".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Ref)).with_doc(
                    "Ref to a drag handle inside the object; when set, only presses inside it start a drag.",
                ),
            ),
            (
                "x_signal".to_owned(),
                ObjectField::required(ValueSchema::Signal).with_doc(
                    "Optional-float signal that receives the previewed horizontal offset from `x` during a drag, or `()` when idle.",
                ),
            ),
            (
                "y_signal".to_owned(),
                ObjectField::required(ValueSchema::Signal).with_doc(
                    "Optional-float signal that receives the previewed vertical offset from `y` during a drag, or `()` when idle.",
                ),
            ),
            (
                "on_move".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)).with_doc(
                    "Called with the proposed `{x, y}` when a drag ends or an arrow key is pressed.",
                ),
            ),
        ]),
        events: BTreeMap::from([(
            "move".to_owned(),
            EventSchema {
                doc: Some(
                    "Emitted once when a drag ends or an arrow key moves the object; the payload is the next `{x, y}`."
                        .to_owned(),
                ),
                payload: position_schema(),
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

    fn config(axes: DragAxes) -> DragConfig {
        let component = crate::ComponentInstancePath::root("Draggable", "card");
        DragConfig {
            id: "card".to_owned(),
            source_x: 20.0,
            source_y: 30.0,
            axes,
            contain: true,
            threshold: 4.0,
            snap_x: Some(10.0),
            snap_y: None,
            keyboard_step: 8.0,
            disabled: false,
            boundary_ref: crate::ElementRef::new(
                crate::ElementRefId::new(component.clone(), "boundary").unwrap(),
            ),
            object_ref: crate::ElementRef::new(
                crate::ElementRefId::new(component.clone(), "object").unwrap(),
            ),
            handle_ref: None,
            x_signal: crate::NativeSignal::new(
                crate::SignalId::new(component.clone(), "x", SignalKind::OptionalFloat).unwrap(),
            ),
            y_signal: crate::NativeSignal::new(
                crate::SignalId::new(component, "y", SignalKind::OptionalFloat).unwrap(),
            ),
            focus: None,
        }
    }

    #[test]
    fn drag_axes_snap_and_containment_are_one_pure_policy() {
        assert_eq!(
            drag_position(
                &config(DragAxes::Both),
                (17.0, 50.0),
                (100.0, 90.0),
                (30.0, 20.0)
            ),
            (40.0, 70.0)
        );
        assert_eq!(
            drag_position(
                &config(DragAxes::Horizontal),
                (99.0, 50.0),
                (100.0, 90.0),
                (30.0, 20.0)
            ),
            (70.0, 30.0)
        );
    }
}
