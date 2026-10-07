//! State shared by the native resize handles (`SplitPane`, `Resizable`) with a
//! decorative handle node: the node's bounds count as the handle for presses
//! and hover, and a string signal carries the handle's state to the node's
//! `signal_style` without a Rhai render.

use gpui::{DispatchPhase, Hitbox, MouseMoveEvent, Pixels, Point, Window};

use crate::{ObjectField, PrimitiveContext, SignalValue, UiValue, ValueSchema};

/// The primitive props of a handle that takes a decorative node: `line`,
/// `line_inset`, `state_signal` and `handle_ref`.
pub(crate) fn decoration_props() -> [(String, ObjectField); 4] {
    [
        (
            "line".to_owned(),
            ObjectField::optional(ValueSchema::Bool).with_default(UiValue::Bool(true)),
        ),
        (
            "line_inset".to_owned(),
            ObjectField::optional(ValueSchema::Number {
                min: Some(0.0),
                max: Some(256.0),
                exclusive_min: None,
                exclusive_max: None,
            }),
        ),
        (
            "state_signal".to_owned(),
            ObjectField::optional(ValueSchema::optional(ValueSchema::Signal)),
        ),
        (
            "handle_ref".to_owned(),
            ObjectField::optional(ValueSchema::optional(ValueSchema::Ref)),
        ),
    ]
}

/// Whether `position` is over the handle or over its decorative node.
pub(crate) fn over_handle(
    hitbox: &Hitbox,
    node: Option<crate::GeometryBounds>,
    position: Point<Pixels>,
    window: &Window,
) -> bool {
    hitbox.is_hovered(window)
        || node.is_some_and(|bounds| {
            let (x, y) = (f64::from(position.x), f64::from(position.y));
            x >= bounds.x
                && x < bounds.x + bounds.width
                && y >= bounds.y
                && y < bounds.y + bounds.height
        })
}

/// The state a handle publishes, by priority: four independent flags.
#[allow(clippy::fn_params_excessive_bools)]
pub(crate) fn handle_state(
    disabled: bool,
    dragged: bool,
    hovered: bool,
    focused: bool,
) -> &'static str {
    if disabled {
        "disabled"
    } else if dragged {
        "drag"
    } else if hovered {
        "hover"
    } else if focused {
        "focus"
    } else {
        "idle"
    }
}

/// Write `state` to the signal after this frame, when it differs from the
/// state last written (`written`).
pub(crate) fn publish_state(
    events: &PrimitiveContext,
    signal: &crate::NativeSignal,
    written: &mut Option<&'static str>,
    state: &'static str,
    window: &mut Window,
    cx: &mut gpui::App,
) {
    if *written == Some(state) {
        return;
    }
    *written = Some(state);
    let events = events.clone();
    let signal = signal.clone();
    window.defer(cx, move |_, cx| {
        let _ = events.write_signal(&signal, SignalValue::String(state.to_owned()), cx);
    });
}

/// Repaint when the pointer enters or leaves the handle or its node.
pub(crate) fn track_hover(
    hitbox: &Hitbox,
    node: Option<&crate::ElementRef>,
    events: &PrimitiveContext,
    was_hovered: bool,
    window: &mut Window,
) {
    let hitbox = hitbox.clone();
    let node = node.cloned();
    let events = events.clone();
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble {
            return;
        }
        let bounds = node
            .as_ref()
            .and_then(|reference| events.element_bounds(reference, cx));
        if over_handle(&hitbox, bounds, event.position, window) != was_hovered {
            window.refresh();
        }
    });
}
