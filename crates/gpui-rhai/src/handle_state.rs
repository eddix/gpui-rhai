//! State shared by the native resize handles (`SplitPane`, `Resizable`) with a
//! decorative handle node: where the node is visible and not covered, it counts
//! as the handle for presses and hover, and a string signal carries the
//! handle's state to the node's `signal_style` without a Rhai render.

use gpui::{DispatchPhase, Hitbox, HitboxId, MouseMoveEvent, Window};

use crate::{ObjectField, PrimitiveContext, SignalValue, UiValue, ValueSchema};

/// The primitive props of a handle that takes a decorative node: `line`,
/// `line_inset`, `state_signal` and `handle_ref`.
pub(crate) fn decoration_props() -> [(String, ObjectField); 4] {
    [
        (
            "line".to_owned(),
            ObjectField::optional(ValueSchema::Bool)
                .with_default(UiValue::Bool(true))
                .with_doc(
                    "Paints the native handle mark (a square on a corner); set `false` when a handle node draws its own.",
                ),
        ),
        (
            "line_inset".to_owned(),
            ObjectField::optional(ValueSchema::Number {
                min: Some(0.0),
                max: Some(256.0),
                exclusive_min: None,
                exclusive_max: None,
            })
            .with_doc(
                "How far the native line stops short of each end of the handle, in logical pixels; defaults to 4.",
            ),
        ),
        (
            "state_signal".to_owned(),
            ObjectField::optional(ValueSchema::optional(ValueSchema::Signal)).with_doc(
                "String signal that receives `idle`, `hover`, `drag`, `focus` or `disabled`, for a handle node's `signal_style`.",
            ),
        ),
        (
            "handle_ref".to_owned(),
            ObjectField::optional(ValueSchema::optional(ValueSchema::Ref)).with_doc(
                "Ref to a decorative handle node; presses and hover over its visible bounds count as the handle's own.",
            ),
        ),
    ]
}

/// Whether the pointer is over the handle or over its decorative node. Both
/// are hitboxes, so clipping and occluding content in front apply to the node
/// as they do to the handle.
pub(crate) fn over_handle(hitbox: &Hitbox, node: Option<HitboxId>, window: &Window) -> bool {
    hitbox.is_hovered(window) || node.is_some_and(|node| node.is_hovered(window))
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
    window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble {
            return;
        }
        let node = node
            .as_ref()
            .and_then(|reference| events.element_hitbox(reference, cx));
        if over_handle(&hitbox, node, window) != was_hovered {
            window.refresh();
        }
    });
}
