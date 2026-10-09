//! Script API docs: `UiNode` methods. One entry per registered signature;
//! see `super` for the format.

use super::ScriptFnDoc;

pub(super) const DOCS: &[ScriptFnDoc] = &[
    ScriptFnDoc {
        signature: "accessibility_checked(_: &mut UiNode, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["checked"],
        doc: "Sets the checked state, `true`, `false` or `\"mixed\"`; tabs, options, rows and menu items report it as selected, a combobox as expanded.",
    },
    ScriptFnDoc {
        signature: "accessibility_column_count(_: &mut UiNode, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["count"],
        doc: "Sets the total number of columns of a table or grid, at least 1, including columns that are not rendered.",
    },
    ScriptFnDoc {
        signature: "accessibility_column_header(_: &mut UiNode, _: string, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["label", "column"],
        doc: "Makes the node a `columnheader` with the accessible name `label` at the 1-based column index `column`.",
    },
    ScriptFnDoc {
        signature: "accessibility_column_index(_: &mut UiNode, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["index"],
        doc: "Sets the node's 1-based column index within its table or grid.",
    },
    ScriptFnDoc {
        signature: "accessibility_current(_: &mut UiNode, _: string) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["current"],
        doc: "Marks the node as the current item of a set: `page`, `step`, `location`, `date`, `time` or `true`; other values raise an error.",
    },
    ScriptFnDoc {
        signature: "accessibility_described_by(_: &mut UiNode, _: string) -> UiNode",
        params: &["ids"],
        doc: "Takes the accessible description from the labels or text of the nodes with these space-separated `accessibility_id`s.",
    },
    ScriptFnDoc {
        signature: "accessibility_expanded(_: &mut UiNode, _: bool) -> UiNode",
        params: &["expanded"],
        doc: "Sets whether the content the node controls, such as a menu or a disclosure panel, is expanded.",
    },
    ScriptFnDoc {
        signature: "accessibility_id(_: &mut UiNode, _: string) -> UiNode",
        params: &["id"],
        doc: "Gives the node a semantic ID, unique in the view, for `accessibility_labelled_by`, `accessibility_described_by` and automation.",
    },
    ScriptFnDoc {
        signature: "accessibility_invalid(_: &mut UiNode, _: bool) -> UiNode",
        params: &["invalid"],
        doc: "Marks the node's value as invalid, as for a form field that fails validation.",
    },
    ScriptFnDoc {
        signature: "accessibility_key_shortcuts(_: &mut UiNode, _: string) -> UiNode",
        params: &["shortcuts"],
        doc: "Announces the shortcuts that activate the node, in ARIA `keyshortcuts` form such as `\"Meta+S\"`; it binds no keys.",
    },
    ScriptFnDoc {
        signature: "accessibility_label(_: &mut UiNode, _: string) -> UiNode",
        params: &["label"],
        doc: "Sets the node's accessible name, which takes precedence over `accessibility_labelled_by` and the node's own text.",
    },
    ScriptFnDoc {
        signature: "accessibility_labelled_by(_: &mut UiNode, _: string) -> UiNode",
        params: &["ids"],
        doc: "Names the node from the labels or text of the nodes with these space-separated `accessibility_id`s, unless it has a label.",
    },
    ScriptFnDoc {
        signature: "accessibility_level(_: &mut UiNode, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["level"],
        doc: "Sets the node's hierarchical level, at least 1, such as a heading level or a tree item's depth.",
    },
    ScriptFnDoc {
        signature: "accessibility_option(_: &mut UiNode, _: string, _: bool, _: i64, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["label", "selected", "position", "size"],
        doc: "Makes the node an `option` named `label`, with its selected state, at the 1-based `position` in a set of `size` options.",
    },
    ScriptFnDoc {
        signature: "accessibility_orientation(_: &mut UiNode, _: string) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["orientation"],
        doc: "Sets the orientation of a slider, toolbar or list, `horizontal` or `vertical`; other values raise an error.",
    },
    ScriptFnDoc {
        signature: "accessibility_placeholder(_: &mut UiNode, _: string) -> UiNode",
        params: &["placeholder"],
        doc: "Sets the placeholder text assistive technology announces for an empty input.",
    },
    ScriptFnDoc {
        signature: "accessibility_position_in_set(_: &mut UiNode, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["position"],
        doc: "Sets the node's 1-based position among the items of its set, such as the options of a listbox.",
    },
    ScriptFnDoc {
        signature: "accessibility_pressed(_: &mut UiNode, _: bool) -> UiNode",
        params: &["pressed"],
        doc: "Sets the pressed state of a toggle button, reported as toggled on or off.",
    },
    ScriptFnDoc {
        signature: "accessibility_read_only(_: &mut UiNode, _: bool) -> UiNode",
        params: &["read_only"],
        doc: "Marks the node as read-only; accessibility actions can no longer set its value.",
    },
    ScriptFnDoc {
        signature: "accessibility_required(_: &mut UiNode, _: bool) -> UiNode",
        params: &["required"],
        doc: "Marks the node as a form field that must be filled in.",
    },
    ScriptFnDoc {
        signature: "accessibility_role(_: &mut UiNode, _: string) -> UiNode",
        params: &["role"],
        doc: "Sets the node's semantic role, such as `button`, `checkbox` or `presentation`; an unknown role rejects the render at commit.",
    },
    ScriptFnDoc {
        signature: "accessibility_row_count(_: &mut UiNode, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["count"],
        doc: "Sets the total number of rows of a table or grid, at least 1, including rows that are not rendered.",
    },
    ScriptFnDoc {
        signature: "accessibility_row_index(_: &mut UiNode, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["index"],
        doc: "Sets the node's 1-based row index within its table or grid.",
    },
    ScriptFnDoc {
        signature: "accessibility_selected(_: &mut UiNode, _: bool) -> UiNode",
        params: &["selected"],
        doc: "Sets whether the node, such as a tab, an option or a row, is selected.",
    },
    ScriptFnDoc {
        signature: "accessibility_size_of_set(_: &mut UiNode, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["size"],
        doc: "Sets how many items, at least 1, are in the set the node belongs to.",
    },
    ScriptFnDoc {
        signature: "accessibility_table_cell(_: &mut UiNode, _: string, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["label", "column"],
        doc: "Makes the node a `gridcell` with the accessible name `label` at the 1-based column index `column`.",
    },
    ScriptFnDoc {
        signature: "accessibility_table_row(_: &mut UiNode, _: string, _: bool, _: i64, _: i64, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["label", "selected", "row", "position", "size"],
        doc: "Makes the node a `row` named `label`, with its selected state, 1-based row index `row`, and `position` in a set of `size`.",
    },
    ScriptFnDoc {
        signature: "accessibility_value(_: &mut UiNode, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Sets the value assistive technology reads; it is announced as text, and a number is also reported as the numeric value.",
    },
    ScriptFnDoc {
        signature: "accessibility_value_max(_: &mut UiNode, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["max"],
        doc: "Sets the maximum of the node's numeric range, such as a slider's; `max` must be a finite number.",
    },
    ScriptFnDoc {
        signature: "accessibility_value_min(_: &mut UiNode, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["min"],
        doc: "Sets the minimum of the node's numeric range, such as a slider's; `min` must be a finite number.",
    },
    ScriptFnDoc {
        signature: "audit_allow(_: &mut UiNode, _: array) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["rules"],
        doc: "Exempts the node from the named composition audit rules, such as `\"mixed-type-in-row\"`; an unknown rule raises an error.",
    },
    ScriptFnDoc {
        signature: "bind_parent_signal(_: &mut UiNode, _: gpui_rhai::context::UiContext, _: string, _: string) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["ctx", "property", "key"],
        doc: "Drives an optional-float `property` such as `width_override` from the signal `key` of the nearest ancestor component.",
    },
    ScriptFnDoc {
        signature: "bind_signal(_: &mut UiNode, _: string, _: gpui_rhai::signal::NativeSignal) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["property", "signal"],
        doc: "Drives `property` (`opacity`, `translate_x`, `width`, `background`, ...) from a native signal of its type, without a rerender.",
    },
    ScriptFnDoc {
        signature: "disabled(_: &mut UiNode, _: bool) -> UiNode",
        params: &["disabled"],
        doc: "Disables the node and its subtree: their handlers stop firing, `disabled` styles apply, and assistive technology reports it.",
    },
    ScriptFnDoc {
        signature: "enter_motion(_: &mut UiNode, _: gpui_rhai::motion::MotionSource) -> UiNode",
        params: &["source"],
        doc: "Declares a motion source that plays when the keyed node mounts or its replay key changes; the same as `motion`.",
    },
    ScriptFnDoc {
        signature: "env(_: &mut UiNode, _: map) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["values"],
        doc: "Sets environment axes for the node and its subtree, such as `#{ density: \"compact\" }`, merged with earlier `env` values on the node.",
    },
    ScriptFnDoc {
        signature: "exit_motion(_: &mut UiNode, _: gpui_rhai::motion::MotionSource) -> UiNode",
        params: &["source"],
        doc: "Declares a motion played on a paint-only copy of the keyed node after it is removed; replaces an exit source for that property.",
    },
    ScriptFnDoc {
        signature: "heading_elsewhere(_: &mut UiNode) -> UiNode",
        params: &[],
        doc: "Tells the composition audit that this container's heading is drawn elsewhere, such as in a host's panel header.",
    },
    ScriptFnDoc {
        signature: "layout_motion(_: &mut UiNode, _: i64, _: string) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["duration_ms", "easing"],
        doc: "Animates the node between committed layout positions over `duration_ms`, with `linear`, `ease_in`, `ease_out` or `ease_in_out`.",
    },
    ScriptFnDoc {
        signature: "motion(_: &mut UiNode, _: gpui_rhai::motion::MotionSource) -> UiNode",
        params: &["source"],
        doc: "Declares a motion source for one property of the keyed node, replacing an earlier source for it; rerenders keep its progress.",
    },
    ScriptFnDoc {
        signature: "motion_particle_count(_: &mut UiNode, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["count"],
        doc: "Declares how many motion particles the node uses, counted against the runtime's `motion_particles` budget; at least 0.",
    },
    ScriptFnDoc {
        signature: "motion_replay_key(_: &mut UiNode, _: string) -> UiNode",
        params: &["key"],
        doc: "Replays the node's unchanged motion declarations whenever `key` changes, such as when a semantically new value arrives.",
    },
    ScriptFnDoc {
        signature: "on(_: &mut UiNode, _: string, _: Fn) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["event", "handler"],
        doc: "Appends a target-phase handler for `event` (`click`, `pointer_down`, `wheel`, `key:escape`, ...), called with `ctx` and the payload.",
    },
    ScriptFnDoc {
        signature: "on(_: &mut UiNode, _: string, _: gpui_rhai::native_handler::NativeHandlerRef) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["event", "handler"],
        doc: "Appends a Rust native handler for `event` in the target phase; the handler's descriptor must declare `event`.",
    },
    ScriptFnDoc {
        signature: "on_bubble(_: &mut UiNode, _: string, _: Fn) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["event", "handler"],
        doc: "Appends a bubble-phase handler for a pointer, wheel or `key:` event, run after target handlers, from inner to outer nodes.",
    },
    ScriptFnDoc {
        signature: "on_bubble(_: &mut UiNode, _: string, _: gpui_rhai::native_handler::NativeHandlerRef) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["event", "handler"],
        doc: "Appends a Rust native handler for the bubble phase of `event`; the handler's descriptor must declare `event`.",
    },
    ScriptFnDoc {
        signature: "on_capture(_: &mut UiNode, _: string, _: Fn) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["event", "handler"],
        doc: "Appends a capture-phase handler for a pointer, wheel or `key:` event, run from outer to inner nodes before target handlers.",
    },
    ScriptFnDoc {
        signature: "on_capture(_: &mut UiNode, _: string, _: gpui_rhai::native_handler::NativeHandlerRef) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["event", "handler"],
        doc: "Appends a Rust native handler for the capture phase of `event`; the handler's descriptor must declare `event`.",
    },
    ScriptFnDoc {
        signature: "on_change(_: &mut UiNode, _: Fn) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handler"],
        doc: "Appends a handler for the `change` event a primitive such as a text input or slider emits with its proposed value.",
    },
    ScriptFnDoc {
        signature: "on_click(_: &mut UiNode, _: Fn) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handler"],
        doc: "Appends a click handler, run on a mouse click or Enter or Space while focused, with payload `()`; makes the node a tab stop.",
    },
    ScriptFnDoc {
        signature: "on_click(_: &mut UiNode, _: gpui_rhai::native_handler::NativeHandlerRef) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handler"],
        doc: "Appends a Rust native click handler, run like a script one; the handler's descriptor must declare `click`.",
    },
    ScriptFnDoc {
        signature: "on_click_value(_: &mut UiNode, _: Fn, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handler", "value"],
        doc: "Appends a click handler that receives `value` as its payload, so one named function can serve many nodes.",
    },
    ScriptFnDoc {
        signature: "on_click_value(_: &mut UiNode, _: gpui_rhai::native_handler::NativeHandlerRef, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handler", "value"],
        doc: "Appends a Rust native click handler that receives `value` as its payload; its descriptor must declare `click`.",
    },
    ScriptFnDoc {
        signature: "on_hover_change(_: &mut UiNode, _: Fn) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handler"],
        doc: "Appends a handler called with `true` when the pointer enters the node and `false` when it leaves.",
    },
    ScriptFnDoc {
        signature: "on_hover_value(_: &mut UiNode, _: Fn, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handler", "value"],
        doc: "Appends a hover handler whose payload is `#{ hovered, value }`: `hovered` is `true` on enter and `false` on leave.",
    },
    ScriptFnDoc {
        signature: "on_key_value(_: &mut UiNode, _: string, _: Fn, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "handler", "value"],
        doc: "Appends a handler for `key` (`escape`, `ctrl+s`, ...) pressed while focus is on or inside the node, with `value` as payload.",
    },
    ScriptFnDoc {
        signature: "on_open_change(_: &mut UiNode, _: Fn) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handler"],
        doc: "Appends a handler for an overlay's `open_change` event, called with the requested open state, such as `false` on dismissal.",
    },
    ScriptFnDoc {
        signature: "progress_motion(_: &mut UiNode, _: gpui_rhai::motion::MotionProgressBinding) -> UiNode",
        params: &["binding"],
        doc: "Attaches a natively driven motion such as `motion_hover(...)` or `motion_scroll(...)`, replacing one for the same property.",
    },
    ScriptFnDoc {
        signature: "scrollbars(_: &mut UiNode, _: string, _: string) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["horizontal", "vertical"],
        doc: "Overlays themed scrollbars on a scrollable node; each axis is `auto`, `always` or `hidden`, which changes presentation only.",
    },
    ScriptFnDoc {
        signature: "selectable(_: &mut UiNode, _: bool) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["selectable"],
        doc: "Lets the user drag-select a `text()` node and copy it with the platform copy action; `true` on other nodes raises an error.",
    },
    ScriptFnDoc {
        signature: "shared_layout(_: &mut UiNode, _: string) -> UiNode",
        params: &["id"],
        doc: "Gives the node the shared-layout identity `id` in the group of the enclosing `motion_group`, linking its layout motion.",
    },
    ScriptFnDoc {
        signature: "shared_layout(_: &mut UiNode, _: string, _: string) -> UiNode",
        params: &["group", "id"],
        doc: "Gives the node the shared-layout identity `id` in `group`, overriding an enclosing `motion_group`; unique in the window.",
    },
    ScriptFnDoc {
        signature: "signal_style(_: &mut UiNode, _: gpui_rhai::signal::NativeSignal, _: map) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["signal", "states"],
        doc: "Merges `states[value]` over the node's style, where `value` is the current value of a string signal; switches without a rerender.",
    },
    ScriptFnDoc {
        signature: "tab_group(_: &mut UiNode) -> UiNode",
        params: &[],
        doc: "Makes the node a tab group, so the `tab_index` values of its descendants order focus locally.",
    },
    ScriptFnDoc {
        signature: "tab_index(_: &mut UiNode, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["index"],
        doc: "Sets the node's tab order within its tab group, from -32768 to 32767, and makes it a tab stop unless `tab_stop(false)`.",
    },
    ScriptFnDoc {
        signature: "tab_stop(_: &mut UiNode, _: bool) -> UiNode",
        params: &["tab_stop"],
        doc: "Sets whether Tab reaches the node; `false` keeps it focusable from code but out of keyboard traversal.",
    },
    ScriptFnDoc {
        signature: "test_id(_: &mut UiNode, _: string) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["id"],
        doc: "Sets a non-semantic automation ID of 1-128 letters, digits, `_`, `-`, `.` or `:` that automation locators find.",
    },
    ScriptFnDoc {
        signature: "timeline(_: &mut UiNode, _: gpui_rhai::motion::MotionTimeline) -> UiNode",
        params: &["timeline"],
        doc: "Attaches a motion timeline to the keyed node, replacing one of the same name; `ctx.motion_handle(name)` controls it.",
    },
    ScriptFnDoc {
        signature: "translate_wheel(_: &mut UiNode) -> UiNode",
        params: &[],
        doc: "Lets a one-axis scroll container also scroll with the other wheel axis, as a tab strip under a vertical mouse wheel.",
    },
    ScriptFnDoc {
        signature: "window_drag_area(_: &mut UiNode) -> UiNode",
        params: &[],
        doc: "Makes a press on the node move the window and a double press run the title-bar action; inert unless the Host allows it.",
    },
    ScriptFnDoc {
        signature: "with_key(_: &mut UiNode, _: string) -> UiNode",
        params: &["key"],
        doc: "Sets the node's stable identity among its siblings, which keeps its state, focus, motion and refs across reorders.",
    },
    ScriptFnDoc {
        signature: "with_part_style(_: &mut UiNode, _: string, _: gpui_rhai::style::Style) -> UiNode",
        params: &["part", "style"],
        doc: "Sets the style of a named part the node draws, such as `scrollbar_thumb` or a layer's `backdrop`, replacing an earlier one.",
    },
    ScriptFnDoc {
        signature: "with_ref(_: &mut UiNode, _: gpui_rhai::element_ref::ElementRef) -> UiNode",
        params: &["reference"],
        doc: "Attaches an `element_ref`, through which the component reads the node's committed bounds, focuses it or scrolls it.",
    },
    ScriptFnDoc {
        signature: "with_style(_: &mut UiNode, _: gpui_rhai::style::Style) -> UiNode",
        params: &["style"],
        doc: "Merges `style` over the node's style: the properties it sets override earlier ones and the rest are kept.",
    },
    ScriptFnDoc {
        signature: "with_table_column(_: &mut UiNode, _: i64) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["index"],
        doc: "Places the node in the 0-based column `index` (below 256) of the enclosing table track, which sets its width.",
    },
    ScriptFnDoc {
        signature: "with_table_track(_: &mut UiNode, _: array, _: array) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["columns", "width_signals"],
        doc: "Makes a box a table column plan of `#{ key, width: #{ kind, value } }` maps; `width_signals` is `[]` or one override signal per column.",
    },
];
