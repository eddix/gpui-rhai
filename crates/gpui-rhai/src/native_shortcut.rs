use gpui::{Action, App, KeyBinding, KeyBindingContextPredicate};

/// A built-in native action using GPUI's runtime platform modifier:
/// Command on macOS, Control on Linux and Windows.
pub(crate) fn platform_binding(
    cx: &App,
    keys: &'static str,
    action: impl Action,
    context: &'static str,
) -> KeyBinding {
    let context = KeyBindingContextPredicate::parse(context).expect("built-in key context");
    KeyBinding::load(
        keys,
        Box::new(action),
        Some(context.into()),
        false,
        None,
        cx,
    )
    .expect("built-in platform shortcut")
}
