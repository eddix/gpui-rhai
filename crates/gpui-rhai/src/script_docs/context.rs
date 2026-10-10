//! Script API docs: `UiContext` methods (`ctx`). One entry per registered signature;
//! see `super` for the format.

use super::ScriptFnDoc;

pub(super) const DOCS: &[ScriptFnDoc] = &[
    ScriptFnDoc {
        signature: "action_enabled(_: &mut UiContext, _: string) -> core::result::Result<bool,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["action"],
        doc: "Returns whether the action `action` (a `namespace.name` id) is registered and enabled; `false` when it is unregistered.",
    },
    ScriptFnDoc {
        signature: "action_shortcut(_: &mut UiContext, _: string) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["action"],
        doc: "Returns the key binding declared to this view for `action` as `#{ label, keystrokes, chords }`, or `()` when it has none.",
    },
    ScriptFnDoc {
        signature: "actions(_: &mut UiContext) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Lists every registered action as `#{ id, enabled, shortcut }` for a command palette; `shortcut` is `()` when unbound.",
    },
    ScriptFnDoc {
        signature: "calendar(_: &mut UiContext) -> core::result::Result<alloc::collections::btree::map::BTreeMap<smartstring::SmartString<smartstring::config::LazyCompact>,rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Returns the selected locale's calendar metadata: `first_weekday`, `months`, `weekdays` and `date_patterns`. Records a locale dependency in render.",
    },
    ScriptFnDoc {
        signature: "call_capability(_: &mut UiContext, _: string, _: string, _: types::dynamic::Dynamic) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["capability", "method", "input"],
        doc: "Synchronously calls `method` of the declared capability `capability` with `input` and returns its output; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "cancel_image_decode(_: &mut UiContext, _: ImageDecodeHandle) -> core::result::Result<bool,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handle"],
        doc: "Cancels a pending image decode so its callbacks never run; returns `false` when it is no longer pending. Raises for another component's handle.",
    },
    ScriptFnDoc {
        signature: "cancel_motion(_: &mut UiContext, _: gpui_rhai::motion::MotionHandle) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handle"],
        doc: "Cancels the timeline behind `handle`; raises an error for a stale handle and is not allowed during render.",
    },
    ScriptFnDoc {
        signature: "cancel_subscription(_: &mut UiContext, _: SubscriptionHandle) -> core::result::Result<bool,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handle"],
        doc: "Cancels a subscription; returns `false` when it already closed, raises for another component's handle. The producer is told on commit.",
    },
    ScriptFnDoc {
        signature: "cancel_task(_: &mut UiContext, _: TaskHandle) -> core::result::Result<bool,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handle"],
        doc: "Cancels a task so its callbacks never run and signals its work on commit; returns `false` when it already finished, raises for another component's handle.",
    },
    ScriptFnDoc {
        signature: "cancel_timeout(_: &mut UiContext, _: string) -> core::result::Result<bool,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key"],
        doc: "Cancels this component's declared `timeout` `key` until its declaration changes; returns `false` when no such timer is active.",
    },
    ScriptFnDoc {
        signature: "clear_close_handler(_: &mut UiContext) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Removes the current window's close handler, so a native close request closes the window again; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "close_window(_: &mut UiContext, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["id"],
        doc: "Queues a forced close of window `id` that skips its close handler; needs window-command authority and is not allowed during render.",
    },
    ScriptFnDoc {
        signature: "component_style(_: &mut UiContext, _: string, _: gpui_rhai::style::Style) -> gpui_rhai::style::Style",
        params: &["part", "base"],
        doc: "Returns `base` merged with the stylesheet rule, the caller's `style` (root only) and `part_styles` for the declared part `part`.",
    },
    ScriptFnDoc {
        signature: "dispatch_action(_: &mut UiContext, _: string, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["action", "payload"],
        doc: "Queues the action `action` with `payload` to run after the current callback; raises an error when it is unknown or disabled.",
    },
    ScriptFnDoc {
        signature: "element_bounds(_: &mut UiContext, _: ElementRef) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["reference"],
        doc: "Returns a ref's last committed `#{ layout, visual, clip }` window bounds, or `()` before first layout; records a dependency in render.",
    },
    ScriptFnDoc {
        signature: "element_bounds(_: &mut UiContext, _: string) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key"],
        doc: "Returns the `#{ layout, visual, clip }` window bounds of a component-local ref key, or `()`; the form event callbacks use.",
    },
    ScriptFnDoc {
        signature: "emit(_: &mut UiContext, _: string, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["event", "payload"],
        doc: "Emits the declared event `event` with a schema-checked `payload`; the caller's handler runs after this callback. Not allowed during render.",
    },
    ScriptFnDoc {
        signature: "event_target_bounds(_: &mut UiContext) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Returns the `#{ x, y, width, height }` window bounds of the node running the current handler, or `()`; untracked, event callbacks only.",
    },
    ScriptFnDoc {
        signature: "focus(_: &mut UiContext, _: ElementRef) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["reference"],
        doc: "Queues keyboard focus for a ref's node, applied after the transaction commits; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "focus(_: &mut UiContext, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key"],
        doc: "Queues keyboard focus for the node of a component-local ref key, applied after the transaction commits; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "focus_window(_: &mut UiContext, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["id"],
        doc: "Queues activation of window `id`; needs window-command authority and is not allowed during render.",
    },
    ScriptFnDoc {
        signature: "format_date(_: &mut UiContext, _: string, _: string) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date", "style"],
        doc: "Formats an ISO `YYYY-MM-DD` date in the selected locale; `style` is `short`, `medium` or `long`. Records a locale dependency in render.",
    },
    ScriptFnDoc {
        signature: "format_month_year(_: &mut UiContext, _: string) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date"],
        doc: "Formats the month and year of an ISO `YYYY-MM-DD` date with the selected locale's pattern. Records a locale dependency in render.",
    },
    ScriptFnDoc {
        signature: "format_number(_: &mut UiContext, _: f64) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Formats a finite number with the selected locale's digits and separators. Records a locale dependency in render.",
    },
    ScriptFnDoc {
        signature: "format_number(_: &mut UiContext, _: f64, _: map) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value", "options"],
        doc: "Formats a finite number in the selected locale; `options` takes `min_fraction_digits`, `max_fraction_digits` (0 to 12) and `grouping`.",
    },
    ScriptFnDoc {
        signature: "format_number(_: &mut UiContext, _: i64) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Formats an integer with the selected locale's digits and separators. Records a locale dependency in render.",
    },
    ScriptFnDoc {
        signature: "format_number(_: &mut UiContext, _: i64, _: map) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value", "options"],
        doc: "Formats an integer in the selected locale; `options` takes `min_fraction_digits`, `max_fraction_digits` (0 to 12) and `grouping`.",
    },
    ScriptFnDoc {
        signature: "get_app_store(_: &mut UiContext, _: string, _: string) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["store", "field"],
        doc: "Reads `field` of the app store `store`; in render it subscribes the component to the whole field.",
    },
    ScriptFnDoc {
        signature: "get_app_store_path(_: &mut UiContext, _: string, _: string, _: array) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["store", "field", "path"],
        doc: "Reads one nested path of an app store field, in render subscribing to that path only; segments are keys, indexes or `#{ by, key }`.",
    },
    ScriptFnDoc {
        signature: "get_native_collection(_: &mut UiContext, _: string) -> core::result::Result<gpui_rhai::native_collection::NativeCollection,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["name"],
        doc: "Returns the Host collection `name` as an opaque view for collection-aware components; in render it subscribes the component to it.",
    },
    ScriptFnDoc {
        signature: "get_native_text_document(_: &mut UiContext, _: string) -> core::result::Result<gpui_rhai::document::NativeTextDocument,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["name"],
        doc: "Returns the Host text document `name`; in render it subscribes the component to its replacement.",
    },
    ScriptFnDoc {
        signature: "get_signal(_: &mut UiContext, _: NativeSignal) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["signal"],
        doc: "Reads a native signal's current value without recording a dependency; in render it disables reuse of the component's last render.",
    },
    ScriptFnDoc {
        signature: "get_signal(_: &mut UiContext, _: string) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key"],
        doc: "Reads this component's signal `key` without recording a dependency; raises an error when no such signal is mounted.",
    },
    ScriptFnDoc {
        signature: "get_state(_: &mut UiContext, _: string) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["field"],
        doc: "Reads a declared state field of the current component. It needs no dependency: a change to local state rerenders its component.",
    },
    ScriptFnDoc {
        signature: "get_state_path(_: &mut UiContext, _: string, _: array) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["field", "path"],
        doc: "Reads one existing nested path of a state field; segments are keys, indexes or `#{ by, key }`. The component stays the dependency.",
    },
    ScriptFnDoc {
        signature: "get_window_store(_: &mut UiContext, _: string, _: string) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["store", "field"],
        doc: "Reads `field` of the current window's store `store`, in render subscribing to the whole field; raises an error outside a window.",
    },
    ScriptFnDoc {
        signature: "get_window_store_path(_: &mut UiContext, _: string, _: string, _: array) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["store", "field", "path"],
        doc: "Reads one nested path of a field of the current window's store, in render subscribing to that path only.",
    },
    ScriptFnDoc {
        signature: "load_image(_: &mut UiContext, _: AssetId) -> core::result::Result<gpui_rhai::value::OpaqueHandle,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["asset"],
        doc: "Loads and caches the logical image `asset` and returns its opaque handle; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "motion_distance(_: &mut UiContext, _: string) -> core::result::Result<f64,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["role"],
        doc: "Returns the theme motion distance in pixels for `role`, such as `subtle`; raises an error for an unknown role.",
    },
    ScriptFnDoc {
        signature: "motion_duration(_: &mut UiContext, _: string) -> core::result::Result<i64,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["role"],
        doc: "Returns the theme motion duration in milliseconds for `role`, such as `fast`; tracked in render, an error for an unknown role.",
    },
    ScriptFnDoc {
        signature: "motion_easing(_: &mut UiContext, _: string) -> core::result::Result<rhai::types::immutable_string::ImmutableString,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["role"],
        doc: "Returns the theme easing for `role` as `linear`, `ease_in`, `ease_out` or `ease_in_out`; raises an error for an unknown role.",
    },
    ScriptFnDoc {
        signature: "motion_handle(_: &mut UiContext, _: string) -> core::result::Result<gpui_rhai::motion::MotionHandle,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["name"],
        doc: "Returns a handle to the timeline `name` this component declared in the current view; raises an error when missing or duplicated.",
    },
    ScriptFnDoc {
        signature: "motion_quality(_: &mut UiContext) -> string",
        params: &[],
        doc: "Returns the Host's motion quality tier: `low`, `medium` or `high`.",
    },
    ScriptFnDoc {
        signature: "motion_spring(_: &mut UiContext, _: string) -> core::result::Result<alloc::collections::btree::map::BTreeMap<smartstring::SmartString<smartstring::config::LazyCompact>,rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["role"],
        doc: "Returns the theme spring preset for `role` as `#{ stiffness, damping, mass }`; raises an error for an unknown role.",
    },
    ScriptFnDoc {
        signature: "motion_stagger(_: &mut UiContext, _: string) -> core::result::Result<i64,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["role"],
        doc: "Returns the theme stagger interval in milliseconds for `role`, such as `tight`; raises an error for an unknown role.",
    },
    ScriptFnDoc {
        signature: "number(_: &mut UiContext) -> core::result::Result<alloc::collections::btree::map::BTreeMap<smartstring::SmartString<smartstring::config::LazyCompact>,rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Returns the selected locale's number metadata: `digits`, separators, group sizes and `minus_sign`. Records a locale dependency in render.",
    },
    ScriptFnDoc {
        signature: "open_window(_: &mut UiContext, _: string, _: string, _: i64, _: i64, _: bool) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["id", "title", "width", "height", "focus"],
        doc: "Queues a new window `id` that runs the same entry, sides 200 to 4096 pixels; needs window-command authority, not during render.",
    },
    ScriptFnDoc {
        signature: "pause_motion(_: &mut UiContext, _: gpui_rhai::motion::MotionHandle) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handle"],
        doc: "Pauses the timeline behind `handle` at its current position; raises an error for a stale handle and is not allowed during render.",
    },
    ScriptFnDoc {
        signature: "pause_timeout(_: &mut UiContext, _: string) -> core::result::Result<bool,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key"],
        doc: "Pauses this component's declared `timeout` `key`; returns `false` when no such timer is active. Not allowed during render.",
    },
    ScriptFnDoc {
        signature: "play_motion(_: &mut UiContext, _: gpui_rhai::motion::MotionHandle) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handle"],
        doc: "Plays the timeline behind `handle`, resuming a paused position; idempotent while playing and not allowed during render.",
    },
    ScriptFnDoc {
        signature: "register_action(_: &mut UiContext, _: string, _: Fn) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["action", "callback"],
        doc: "Registers or replaces the app-wide action `action` (`namespace.name`), enabled, running `callback`; removed when the component unmounts.",
    },
    ScriptFnDoc {
        signature: "restart_motion(_: &mut UiContext, _: gpui_rhai::motion::MotionHandle) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handle"],
        doc: "Restarts the timeline behind `handle` from zero, the only way to rewind; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "resume_motion(_: &mut UiContext, _: gpui_rhai::motion::MotionHandle) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handle"],
        doc: "Resumes the timeline behind `handle`, the same as `play_motion`; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "resume_timeout(_: &mut UiContext, _: string) -> core::result::Result<bool,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key"],
        doc: "Clears an explicit pause of this component's declared `timeout` `key`; returns `false` when no such timer is active.",
    },
    ScriptFnDoc {
        signature: "scroll_into_view(_: &mut UiContext, _: ElementRef) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["reference"],
        doc: "Queues scrolling the nearest retained scroll ancestor so a ref's node shows on the next frame; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "scroll_into_view(_: &mut UiContext, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key"],
        doc: "Queues scrolling the nearest retained scroll ancestor to reveal the node of a component-local ref key; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "scroll_to(_: &mut UiContext, _: ElementRef, _: f64, _: f64) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["reference", "x", "y"],
        doc: "Queues a scroll offset for a ref's scroll container, applied after commit; `x` and `y` must be finite and non-negative.",
    },
    ScriptFnDoc {
        signature: "scroll_to(_: &mut UiContext, _: ElementRef, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["reference", "x", "y"],
        doc: "Queues a scroll offset for a ref's scroll container, applied after commit; `x` and `y` must be finite and non-negative. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "scroll_to(_: &mut UiContext, _: string, _: f64, _: f64) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "x", "y"],
        doc: "Queues a scroll offset for the scroll container of a component-local ref key; `x` and `y` must be finite and non-negative.",
    },
    ScriptFnDoc {
        signature: "scroll_to(_: &mut UiContext, _: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "x", "y"],
        doc: "Queues a scroll offset for the scroll container of a component-local ref key; `x` and `y` must be finite and non-negative. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "seek_motion(_: &mut UiContext, _: gpui_rhai::motion::MotionHandle, _: i64) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handle", "position_ms"],
        doc: "Moves the timeline behind `handle` to an absolute position in milliseconds; raises an error for a negative position or stale handle.",
    },
    ScriptFnDoc {
        signature: "set_action_enabled(_: &mut UiContext, _: string, _: bool) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["action", "enabled"],
        doc: "Enables or disables the registered action `action`; raises an error when it is unknown and is not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_app_store(_: &mut UiContext, _: string, _: string, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["store", "field", "value"],
        doc: "Writes a schema-checked `value` to `field` of app store `store` and rerenders its readers; not during render, undone on failure.",
    },
    ScriptFnDoc {
        signature: "set_app_store_path(_: &mut UiContext, _: string, _: string, _: array, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["store", "field", "path", "value"],
        doc: "Replaces one existing nested path of an app store field, schema-checking the whole field; rerenders only readers of that path.",
    },
    ScriptFnDoc {
        signature: "set_close_handler(_: &mut UiContext, _: Fn) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["callback"],
        doc: "Calls `callback` instead of closing when the user closes the current window; it may confirm, then call `close_window`.",
    },
    ScriptFnDoc {
        signature: "set_local_locale(_: &mut UiContext, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["locale"],
        doc: "Selects `locale` for the current component subtree and rerenders its locale readers; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_local_theme(_: &mut UiContext, _: string, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["family", "variant"],
        doc: "Selects the theme variant `variant` of `family` for the current component subtree; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_local_theme_system(_: &mut UiContext, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["family"],
        doc: "Makes the current component subtree follow the system light or dark appearance within theme `family`; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_locale(_: &mut UiContext, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["locale"],
        doc: "Selects the app-wide `locale` and rerenders locale readers, keeping state; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_reduced_motion(_: &mut UiContext, _: bool) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["reduced"],
        doc: "Requests reduced or normal motion; reduced settles active animation at once. A stricter Host preference still wins.",
    },
    ScriptFnDoc {
        signature: "set_signal(_: &mut UiContext, _: NativeSignal, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["signal", "value"],
        doc: "Writes a native signal's value, which must match its type; bound properties update without rerunning Rhai. Not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_signal(_: &mut UiContext, _: string, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "value"],
        doc: "Writes this component's signal `key`, which must match its type; bound properties update without rerunning Rhai. Not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_state(_: &mut UiContext, _: string, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["field", "value"],
        doc: "Writes a schema-checked value to a declared state field and rerenders the component; not during render, undone if the callback fails.",
    },
    ScriptFnDoc {
        signature: "set_state_path(_: &mut UiContext, _: string, _: array, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["field", "path", "value"],
        doc: "Replaces one existing nested path of a state field, schema-checking the whole field, and rerenders the component; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_theme(_: &mut UiContext, _: string, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["family", "variant"],
        doc: "Selects the app-wide theme variant `variant` of `family` without recompiling or resetting state; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_theme_system(_: &mut UiContext, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["family"],
        doc: "Makes the app follow the system light or dark appearance within theme `family`; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_window_locale(_: &mut UiContext, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["locale"],
        doc: "Selects `locale` for the current window and rerenders its locale readers; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_window_store(_: &mut UiContext, _: string, _: string, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["store", "field", "value"],
        doc: "Writes a schema-checked `value` to `field` of the current window's store and rerenders its readers; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_window_store_path(_: &mut UiContext, _: string, _: string, _: array, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["store", "field", "path", "value"],
        doc: "Replaces one existing nested path of a field of the current window's store; rerenders only readers of that path.",
    },
    ScriptFnDoc {
        signature: "set_window_theme(_: &mut UiContext, _: string, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["family", "variant"],
        doc: "Selects the theme variant `variant` of `family` for the current window; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "set_window_theme_system(_: &mut UiContext, _: string) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["family"],
        doc: "Makes the current window follow the system light or dark appearance within theme `family`; not allowed during render.",
    },
    ScriptFnDoc {
        signature: "start_image_decode(_: &mut UiContext, _: AssetId, _: Fn, _: Fn) -> core::result::Result<gpui_rhai::asset::ImageDecodeHandle,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["asset", "on_success", "on_error"],
        doc: "Decodes the image `asset` in the background, then calls `on_success` or `on_error`, and returns a handle; from `init`, callbacks, effects or `resume`.",
    },
    ScriptFnDoc {
        signature: "start_subscription(_: &mut UiContext, _: string, _: string, _: types::dynamic::Dynamic, _: Fn, _: Fn, _: map) -> core::result::Result<gpui_rhai::async_runtime::SubscriptionHandle,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[
            "capability",
            "method",
            "input",
            "on_value",
            "on_error",
            "options",
        ],
        doc: "Starts a capability stream calling `on_value` per item, only from an effect start; `options` takes `delivery`, `capacity`, `throttle_ms`.",
    },
    ScriptFnDoc {
        signature: "start_task(_: &mut UiContext, _: string, _: string, _: types::dynamic::Dynamic, _: Fn, _: Fn) -> core::result::Result<gpui_rhai::async_runtime::TaskHandle,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["capability", "method", "input", "on_success", "on_error"],
        doc: "Runs a capability `method` in the background, then calls `on_success` or `on_error`; from `init`, callbacks, effects or `resume`.",
    },
    ScriptFnDoc {
        signature: "t(_: &mut UiContext, _: string) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key"],
        doc: "Returns the message `key` in the current locale, falling back to the default locale; records a locale dependency in render.",
    },
    ScriptFnDoc {
        signature: "text_direction(_: &mut UiContext) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Returns `ltr` or `rtl` for the locale of the current component scope; records a locale dependency in render.",
    },
    ScriptFnDoc {
        signature: "theme_variant(_: &mut UiContext) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Returns the resolved theme variant as `#{ family, name, mode }`, or `()` without a theme; records a theme dependency in render.",
    },
    ScriptFnDoc {
        signature: "theme_variants(_: &mut UiContext) -> array",
        params: &[],
        doc: "Lists every loaded theme variant as `#{ family, name, mode }`, ordered by family and name, for a theme picker.",
    },
    ScriptFnDoc {
        signature: "today(_: &mut UiContext) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Returns today's date as an ISO `YYYY-MM-DD` string from the Host calendar clock; available during render.",
    },
    ScriptFnDoc {
        signature: "view_id(_: &mut UiContext) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Returns the identity of the mounted script view; raises an error when the context has no view.",
    },
    ScriptFnDoc {
        signature: "viewport_class(_: &mut UiContext) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Returns `compact`, `regular` or `wide` for the current window's width; records a viewport dependency in render.",
    },
    ScriptFnDoc {
        signature: "virtual_item_bounds(_: &mut UiContext, _: string, _: i64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["collection", "index"],
        doc: "Returns the window bounds of the item at display `index` of this component's virtual collection `collection`, or `()`; untracked.",
    },
    ScriptFnDoc {
        signature: "window_id(_: &mut UiContext) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Returns the ID of the current window; raises an error outside a window.",
    },
];
