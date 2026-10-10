//! Script API docs: global functions. One entry per registered signature;
//! see `super` for the format.

use super::ScriptFnDoc;

pub(super) const DOCS: &[ScriptFnDoc] = &[
    ScriptFnDoc {
        signature: "*(_: Length, _: f64) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["length", "factor"],
        doc: "Scales `length` by a non-negative `factor`; a token keeps the factor until it resolves, and a relative result must stay within 0 to 1.",
    },
    ScriptFnDoc {
        signature: "*(_: Length, _: i64) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["length", "factor"],
        doc: "Scales `length` by a non-negative integer `factor`; a token keeps the factor until it resolves, and a relative result must stay within 0 to 1.",
    },
    ScriptFnDoc {
        signature: "alpha(_: ColorValue, _: f64) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["color", "factor"],
        doc: "Returns `color` with its alpha multiplied by `factor` (0 to 1); a token color resolves when the node renders.",
    },
    ScriptFnDoc {
        signature: "asset(_: string) -> core::result::Result<gpui_rhai::asset::AssetId,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["id"],
        doc: "Parses a logical asset ID of two or more `/`-separated segments of letters, digits, `_` or `-`, such as `\"app/icons/check\"`.",
    },
    ScriptFnDoc {
        signature: "auto() -> AutoLength",
        params: &[],
        doc: "Returns the `auto` layout length, accepted by the size, margin and inset setters: `style().width(auto())`.",
    },
    ScriptFnDoc {
        signature: "box(_: array) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["children"],
        doc: "Creates a box node, the general container for layout, paint and interaction; every child must be a `UiNode`.",
    },
    ScriptFnDoc {
        signature: "by_env(_: types::dynamic::Dynamic, _: map) -> core::result::Result<gpui_rhai::theme_source::EnvTableSource,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["keys", "table"],
        doc: "Varies a theme token with the environment: `keys` is an axis name or an array of names, and `table` maps their values, nested per axis.",
    },
    ScriptFnDoc {
        signature: "canvas(_: CanvasScene) -> UiNode",
        params: &["scene"],
        doc: "Creates a canvas node that paints a retained `canvas_scene` in canvas-local logical pixels.",
    },
    ScriptFnDoc {
        signature: "canvas_circle(_: string, _: f64, _: f64, _: f64, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "center_x", "center_y", "radius", "fill"],
        doc: "Creates a filled circle command centered at (`center_x`, `center_y`); `radius` must be positive and `key` unique in the scene.",
    },
    ScriptFnDoc {
        signature: "canvas_circle(_: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "center_x", "center_y", "radius", "fill"],
        doc: "Creates a filled circle command centered at (`center_x`, `center_y`); `radius` must be positive and `key` unique in the scene. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "canvas_fill_path(_: string, _: array, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "segments", "fill"],
        doc: "Creates a path command filled with a solid color; `segments` start with `path_move` and hold 2 to 10,000 segments.",
    },
    ScriptFnDoc {
        signature: "canvas_fill_path(_: string, _: array, _: LinearGradientSpec) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "segments", "gradient"],
        doc: "Creates a path command filled with a two-stop `linear_gradient`; `segments` start with `path_move` and hold 2 to 10,000 segments.",
    },
    ScriptFnDoc {
        signature: "canvas_line(_: string, _: f64, _: f64, _: f64, _: f64, _: f64, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "from_x", "from_y", "to_x", "to_y", "width", "color"],
        doc: "Creates a line command from (`from_x`, `from_y`) to (`to_x`, `to_y`), stroked `width` pixels wide (positive).",
    },
    ScriptFnDoc {
        signature: "canvas_line(_: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "from_x", "from_y", "to_x", "to_y", "width", "color"],
        doc: "Creates a line command from (`from_x`, `from_y`) to (`to_x`, `to_y`), stroked `width` pixels wide (positive). Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "canvas_morph_stroke_path(_: string, _: array, _: array, _: f64, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "from", "to", "width", "color"],
        doc: "Creates a stroked path that morphs from `from` to `to` as its `path_progress` motion goes 0 to 1; both need the same segment kinds.",
    },
    ScriptFnDoc {
        signature: "canvas_morph_stroke_path(_: string, _: array, _: array, _: i64, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "from", "to", "width", "color"],
        doc: "Creates a stroked path that morphs from `from` to `to` as its `path_progress` motion goes 0 to 1; both need the same segment kinds.",
    },
    ScriptFnDoc {
        signature: "canvas_rect(_: string, _: f64, _: f64, _: f64, _: f64, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "x", "y", "width", "height", "fill"],
        doc: "Creates a filled rectangle command with its top-left corner at (`x`, `y`); `width` and `height` must be positive.",
    },
    ScriptFnDoc {
        signature: "canvas_rect(_: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "x", "y", "width", "height", "fill"],
        doc: "Creates a filled rectangle command with its top-left corner at (`x`, `y`); `width` and `height` must be positive. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "canvas_scene(_: array) -> core::result::Result<gpui_rhai::canvas::CanvasScene,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["commands"],
        doc: "Builds a canvas scene from canvas commands, painted in order; every command key must be unique.",
    },
    ScriptFnDoc {
        signature: "canvas_stroke_path(_: string, _: array, _: f64, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "segments", "width", "color"],
        doc: "Creates a path command stroked `width` pixels wide (positive); `segments` start with `path_move` and hold 2 to 10,000 segments.",
    },
    ScriptFnDoc {
        signature: "canvas_stroke_path(_: string, _: array, _: i64, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "segments", "width", "color"],
        doc: "Creates a path command stroked `width` pixels wide (positive); `segments` start with `path_move` and hold 2 to 10,000 segments.",
    },
    ScriptFnDoc {
        signature: "color(_: string) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Parses a color literal: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, a CSS basic name, `transparent`, or an `rgb()`, `hsl()` or `hwb()` form.",
    },
    ScriptFnDoc {
        signature: "column(_: array) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["children"],
        doc: "Creates a box that lays its children out top to bottom (`flex_col`); every child must be a `UiNode`.",
    },
    ScriptFnDoc {
        signature: "date_checked_add_days(_: string, _: i64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date", "days"],
        doc: "Adds signed `days` to an ISO `YYYY-MM-DD` date and returns the new ISO date, or `()` when it leaves years 1 to 9999.",
    },
    ScriptFnDoc {
        signature: "date_checked_add_months(_: string, _: i64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date", "months"],
        doc: "Adds signed `months` to an ISO date, clamping the day into the target month; returns `()` when it leaves years 1 to 9999.",
    },
    ScriptFnDoc {
        signature: "date_clamp(_: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<rhai::types::immutable_string::ImmutableString,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date", "min", "max"],
        doc: "Clamps an ISO date into `min` to `max`, each an ISO date or `()` for no bound, and returns the ISO date.",
    },
    ScriptFnDoc {
        signature: "date_info(_: string) -> core::result::Result<alloc::collections::btree::map::BTreeMap<smartstring::SmartString<smartstring::config::LazyCompact>,rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date"],
        doc: "Parses an ISO `YYYY-MM-DD` date into `#{ iso, year, month, day, weekday }`, `weekday` being a lowercase English name.",
    },
    ScriptFnDoc {
        signature: "date_month_grid(_: string, _: string) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date", "first_weekday"],
        doc: "Returns the 42 cells (six weeks) of the month containing `date`, weeks starting on `first_weekday`; each is `#{ date, day, outside, weekday }`.",
    },
    ScriptFnDoc {
        signature: "date_month_intersects(_: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<bool,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date", "min", "max"],
        doc: "Returns whether any day of the month containing `date` lies within `min` to `max`, each an ISO date or `()` for no bound.",
    },
    ScriptFnDoc {
        signature: "date_month_start(_: string) -> core::result::Result<rhai::types::immutable_string::ImmutableString,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date"],
        doc: "Returns the ISO date of the first day of the month containing the ISO date `date`.",
    },
    ScriptFnDoc {
        signature: "date_week_edge(_: string, _: string, _: bool) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date", "first_weekday", "end"],
        doc: "Returns the ISO date that starts (or, when `end`, ends) the week containing `date`, weeks starting on `first_weekday`, or `()` out of range.",
    },
    ScriptFnDoc {
        signature: "define_component(_: map) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["definition"],
        doc: "Registers a formal component from `#{ metadata, schema, render }` at module top level; `render` must be a named, uncurried `Fn`.",
    },
    ScriptFnDoc {
        signature: "directional_image(_: AssetId, _: AssetId) -> UiNode",
        params: &["left_to_right", "right_to_left"],
        doc: "Creates an image node that shows the first asset in left-to-right layout and the second in right-to-left layout.",
    },
    ScriptFnDoc {
        signature: "directional_image(_: OpaqueHandle, _: OpaqueHandle) -> UiNode",
        params: &["left_to_right", "right_to_left"],
        doc: "Creates an image node that shows the first image handle in left-to-right layout and the second in right-to-left layout.",
    },
    ScriptFnDoc {
        signature: "directional_image_source(_: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["left_to_right", "right_to_left"],
        doc: "Creates an image node chosen by layout direction; each source is an `AssetId` or an image handle, and other values are rejected.",
    },
    ScriptFnDoc {
        signature: "effect(_: string, _: types::dynamic::Dynamic, _: Fn, _: Fn) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "dependencies", "start", "cleanup"],
        doc: "Declares effect `key` (listed in the schema) during formal render; `start(ctx, deps)` runs after commit, `cleanup` before a change or unmount.",
    },
    ScriptFnDoc {
        signature: "element_ref(_: string) -> core::result::Result<gpui_rhai::element_ref::ElementRef,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key"],
        doc: "Declares a component-local element ref during formal render; attach it with `with_ref` and read it with `ctx.element_bounds`.",
    },
    ScriptFnDoc {
        signature: "error_boundary(_: UiNode, _: UiNode) -> UiNode",
        params: &["child", "fallback"],
        doc: "Wraps `child` so that a native rendering failure in its subtree shows `fallback` instead.",
    },
    ScriptFnDoc {
        signature: "error_boundary_lazy(_: Fn, _: Fn) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["child", "fallback"],
        doc: "Builds `fallback()` then `child()` into an error boundary; when `child()` throws, returns the fallback with a `boundary_error` attribute.",
    },
    ScriptFnDoc {
        signature: "event_response() -> EventResponse",
        params: &[],
        doc: "Returns a blank event response that lets the event continue, to refine with `prevent_default()`, `stop()` or `capture_pointer()`.",
    },
    ScriptFnDoc {
        signature: "fragment(_: array) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["children"],
        doc: "Groups children without a layout box; a fragment carries only children, a key and its source, so style, handlers and refs are rejected.",
    },
    ScriptFnDoc {
        signature: "grapheme_count(_: string) -> i64",
        params: &["text"],
        doc: "Returns the number of extended grapheme clusters, the user-perceived characters, in `text`.",
    },
    ScriptFnDoc {
        signature: "handled() -> EventResponse",
        params: &[],
        doc: "Returns an event response that stops the event from reaching ancestor handlers.",
    },
    ScriptFnDoc {
        signature: "image(_: AssetId) -> UiNode",
        params: &["asset"],
        doc: "Creates an image node for a component-declared, preloaded asset; an undeclared asset renders an image error.",
    },
    ScriptFnDoc {
        signature: "image(_: OpaqueHandle) -> UiNode",
        params: &["handle"],
        doc: "Creates an image node for an image handle from `ctx.load_image` or a capability; a handle of another kind fails when rendered.",
    },
    ScriptFnDoc {
        signature: "image_source(_: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["source"],
        doc: "Creates an image node from `source`, an `AssetId` or an image handle; other values are rejected.",
    },
    ScriptFnDoc {
        signature: "is_native_collection(_: types::dynamic::Dynamic) -> bool",
        params: &["value"],
        doc: "Returns whether `value` is a Host-owned `NativeCollection`.",
    },
    ScriptFnDoc {
        signature: "is_native_text_document(_: types::dynamic::Dynamic) -> bool",
        params: &["value"],
        doc: "Returns whether `value` is a Host-owned `NativeTextDocument`.",
    },
    ScriptFnDoc {
        signature: "key_shortcut(_: string) -> types::dynamic::Dynamic",
        params: &["text"],
        doc: "Formats `text` such as `\"cmd-p\"` as `#{ label, keystrokes, chords }` with platform key names; text that is not keystrokes stays a plain label.",
    },
    ScriptFnDoc {
        signature: "layer(_: UiNode, _: map) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["content", "config"],
        doc: "Places `content` in a window-level layer; `config` needs `id` and may set `placement` (default `top_right`), `inset` (12 px) and `priority`.",
    },
    ScriptFnDoc {
        signature: "linear_gradient(_: map) -> core::result::Result<gpui_rhai::style::LinearGradientSpec,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["spec"],
        doc: "Creates a two-stop gradient from `#{ angle, from, to }`; `angle` is in degrees (default 180) and both colors are required.",
    },
    ScriptFnDoc {
        signature: "mix(_: ColorValue, _: ColorValue, _: f64) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["base", "other", "weight"],
        doc: "Mixes `base` toward `other` by `weight` (0 to 1) in every channel, alpha included; token colors resolve when the node renders.",
    },
    ScriptFnDoc {
        signature: "motion_delay(_: i64) -> core::result::Result<gpui_rhai::motion::MotionTimelineStep,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["duration_ms"],
        doc: "Creates a timeline step that waits `duration_ms` milliseconds (non-negative).",
    },
    ScriptFnDoc {
        signature: "motion_focus(_: MotionSource) -> core::result::Result<gpui_rhai::motion::MotionProgressBinding,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["source"],
        doc: "Drives `source` by focus: forward while the node is focused, back after. Takes a transition or keyframes without delay or repeat.",
    },
    ScriptFnDoc {
        signature: "motion_group(_: string, _: array) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["id", "children"],
        doc: "Groups children in a layout-transparent fragment whose `shared_layout` nodes join group `id` (1 to 128 letters, digits, `_`, `-`).",
    },
    ScriptFnDoc {
        signature: "motion_hover(_: MotionSource) -> core::result::Result<gpui_rhai::motion::MotionProgressBinding,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["source"],
        doc: "Drives `source` by hover: forward while the pointer is over the node, back after. Takes a transition or keyframes without delay or repeat.",
    },
    ScriptFnDoc {
        signature: "motion_in_view(_: MotionSource) -> core::result::Result<gpui_rhai::motion::MotionProgressBinding,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["source"],
        doc: "Maps `source` to the visible fraction of the node in the viewport, 0 to 1. Takes a transition or keyframes without delay or repeat.",
    },
    ScriptFnDoc {
        signature: "motion_inertia(_: string, _: f64, _: f64, _: map) -> core::result::Result<gpui_rhai::motion::MotionSource,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["property", "from", "velocity", "config"],
        doc: "Creates an inertia source gliding `property` from `from` at `velocity` units/s; `config` sets `friction`, `min`, `max`, `bounce`, `snap_points`.",
    },
    ScriptFnDoc {
        signature: "motion_inertia(_: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: map) -> core::result::Result<gpui_rhai::motion::MotionSource,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["property", "from", "velocity", "config"],
        doc: "Creates an inertia source gliding `property` from `from` at `velocity` units/s; `config` sets `friction`, `min`, `max`, `bounce`, `snap_points`. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "motion_keyframes(_: string, _: array, _: map) -> core::result::Result<gpui_rhai::motion::MotionSource,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["property", "frames", "config"],
        doc: "Creates a keyframe source for `property` from `#{ offset, value, easing }` maps; `config` sets `duration_ms` (240), `delay_ms`, `iterations`.",
    },
    ScriptFnDoc {
        signature: "motion_parallel(_: array) -> core::result::Result<gpui_rhai::motion::MotionTimelineStep,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["steps"],
        doc: "Creates a timeline step that starts all `steps` together and ends with the longest.",
    },
    ScriptFnDoc {
        signature: "motion_path_follow(_: gpui_rhai::canvas::CanvasScene, _: string, _: string, _: map) -> core::result::Result<gpui_rhai::motion::MotionSource,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["scene", "key", "property", "config"],
        doc: "Creates keyframes moving `translate_x`, `translate_y` or `rotate` along path `key` of `scene`; `config` sets `samples` (16 to 512), `duration_ms`.",
    },
    ScriptFnDoc {
        signature: "motion_press(_: MotionSource) -> core::result::Result<gpui_rhai::motion::MotionProgressBinding,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["source"],
        doc: "Drives `source` by press: forward while the node is held down, back on release. Takes a transition or keyframes without delay or repeat.",
    },
    ScriptFnDoc {
        signature: "motion_scroll(_: string, _: MotionSource) -> core::result::Result<gpui_rhai::motion::MotionProgressBinding,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["axis", "source"],
        doc: "Maps `source` to the scroll position on `axis` (`\"x\"` or `\"y\"`) of the nearest scrollable node, 0 at the start to 1 at the end.",
    },
    ScriptFnDoc {
        signature: "motion_sequence(_: array) -> core::result::Result<gpui_rhai::motion::MotionTimelineStep,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["steps"],
        doc: "Creates a timeline step that runs `steps` one after another.",
    },
    ScriptFnDoc {
        signature: "motion_spring(_: string, _: f64, _: f64, _: map) -> core::result::Result<gpui_rhai::motion::MotionSource,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["property", "from", "to", "config"],
        doc: "Creates a spring source from `from` to `to` on `property`; `config` sets `stiffness` (180), `damping` (24), `mass` (1), `initial_velocity`.",
    },
    ScriptFnDoc {
        signature: "motion_spring(_: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: map) -> core::result::Result<gpui_rhai::motion::MotionSource,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["property", "from", "to", "config"],
        doc: "Creates a spring source from `from` to `to` on `property`; `config` sets `stiffness` (180), `damping` (24), `mass` (1), `initial_velocity`. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "motion_stagger(_: array, _: i64) -> core::result::Result<gpui_rhai::motion::MotionTimelineStep,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["steps", "interval_ms"],
        doc: "Creates a timeline step that starts each of `steps` `interval_ms` milliseconds after the previous one.",
    },
    ScriptFnDoc {
        signature: "motion_text_spans(_: string, _: map) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["text", "config"],
        doc: "Splits `text` into one fading-in span per grapheme for `text(spans)`; `config` sets `duration_ms` (180), `stagger_ms` (24), `easing`.",
    },
    ScriptFnDoc {
        signature: "motion_timeline(_: string, _: MotionTimelineStep, _: map) -> core::result::Result<gpui_rhai::motion::MotionTimeline,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["name", "root", "config"],
        doc: "Creates timeline `name` from a root step for `node.timeline`; `config` sets `autoplay` (true), `iterations`, `on_complete`, `on_cancel`.",
    },
    ScriptFnDoc {
        signature: "motion_track(_: string, _: MotionSource) -> MotionTimelineStep",
        params: &["target", "source"],
        doc: "Creates a timeline step that plays `source` on `target`: `\".\"` for the timeline's own node, or a `/`-separated path of descendant keys.",
    },
    ScriptFnDoc {
        signature: "motion_transition(_: string, _: f64, _: f64, _: map) -> core::result::Result<gpui_rhai::motion::MotionSource,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["property", "from", "to", "config"],
        doc: "Creates a transition of `property` from `from` to `to`; `config` sets `duration_ms` (180), `delay_ms`, `easing`, `iterations`, `intent`.",
    },
    ScriptFnDoc {
        signature: "motion_transition(_: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: map) -> core::result::Result<gpui_rhai::motion::MotionSource,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["property", "from", "to", "config"],
        doc: "Creates a transition of `property` from `from` to `to`; `config` sets `duration_ms` (180), `delay_ms`, `easing`, `iterations`, `intent`. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "motion_viewport(_: MotionSource) -> core::result::Result<gpui_rhai::motion::MotionProgressBinding,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["source"],
        doc: "Maps `source` to the node's travel through the viewport: 0 as it enters at the bottom, 1 as it leaves at the top.",
    },
    ScriptFnDoc {
        signature: "native_fuzzy_adjacent(_: NativeCollection, _: string, _: i64) -> core::result::Result<rhai::types::immutable_string::ImmutableString,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["collection", "active", "step"],
        doc: "Returns the key of the enabled row one before (`step` < 0) or after (`step` > 0) `active` in a fuzzy view, wrapping; `step` 0 returns `active` itself. An `active` that is not an enabled row counts as the first one; `\"\"` when none is enabled.",
    },
    ScriptFnDoc {
        signature: "native_fuzzy_edge(_: NativeCollection, _: bool) -> core::result::Result<rhai::types::immutable_string::ImmutableString,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["collection", "last"],
        doc: "Returns the key of the first (or, when `last`, the last) enabled row of a fuzzy view, or `\"\"` when it has none.",
    },
    ScriptFnDoc {
        signature: "native_fuzzy_view(_: NativeCollection, _: map) -> core::result::Result<gpui_rhai::native_collection::NativeCollection,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["collection", "config"],
        doc: "Returns a fuzzy-filtered, ranked and grouped view of a native collection for Command; `config` sets `query` and the row field names.",
    },
    ScriptFnDoc {
        signature: "native_handler(_: string) -> core::result::Result<gpui_rhai::native_handler::NativeHandlerRef,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["id"],
        doc: "Resolves the Host-registered native handler `\"namespace.name\"` for use as a callback; fails when no handler has that ID.",
    },
    ScriptFnDoc {
        signature: "native_table_neighbors(_: NativeCollection, _: array) -> map",
        params: &["collection", "selected_keys"],
        doc: "Returns the Table navigation keys `#{ first, last, previous, next, current, current_index }`; `current` is the first shown selected row. Non-string keys match no row.",
    },
    ScriptFnDoc {
        signature: "native_table_view(_: NativeCollection, _: map) -> core::result::Result<gpui_rhai::native_collection::NativeCollection,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["collection", "config"],
        doc: "Returns a sorted, filtered, paged and grouped view of a native collection, as the Table's normalized `config` describes.",
    },
    ScriptFnDoc {
        signature: "offset_px(_: f64) -> core::result::Result<gpui_rhai::style::SignedLength,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Creates a signed offset in logical pixels for margins and insets; any finite value, negative included.",
    },
    ScriptFnDoc {
        signature: "offset_px(_: i64) -> core::result::Result<gpui_rhai::style::SignedLength,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Creates a signed offset in logical pixels for margins and insets; negative values are allowed.",
    },
    ScriptFnDoc {
        signature: "offset_relative(_: f64) -> core::result::Result<gpui_rhai::style::SignedLength,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["fraction"],
        doc: "Creates a signed offset as a fraction of the parent's size, from -10 to 10, for margins and insets.",
    },
    ScriptFnDoc {
        signature: "offset_relative(_: i64) -> core::result::Result<gpui_rhai::style::SignedLength,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["fraction"],
        doc: "Creates a signed offset as a whole multiple of the parent's size, from -10 to 10, for margins and insets.",
    },
    ScriptFnDoc {
        signature: "offset_rem(_: f64) -> core::result::Result<gpui_rhai::style::SignedLength,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Creates a signed offset in rems for margins and insets; any finite value, negative included.",
    },
    ScriptFnDoc {
        signature: "offset_rem(_: i64) -> core::result::Result<gpui_rhai::style::SignedLength,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Creates a signed offset in rems for margins and insets; negative values are allowed.",
    },
    ScriptFnDoc {
        signature: "optional_float_signal(_: string) -> core::result::Result<gpui_rhai::signal::NativeSignal,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key"],
        doc: "Declares a component-local optional-float signal that starts as `()` during formal render, for example to bind as `width_override`.",
    },
    ScriptFnDoc {
        signature: "outline_projection(_: array, _: array) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["items", "expanded_keys"],
        doc: "Flattens `#{ key, parent, label }` tree items into visible rows with `depth` and `has_children`, opening only `expanded_keys`.",
    },
    ScriptFnDoc {
        signature: "overlay(_: UiNode, _: UiNode, _: map) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["trigger", "content", "config"],
        doc: "Anchors `content` to `trigger` in a window-level overlay; `config` needs `id` and sets `kind`, `open`, `placement`, `align`, `gap` and dismissal.",
    },
    ScriptFnDoc {
        signature: "path_close() -> CanvasPathSegment",
        params: &[],
        doc: "Creates a path segment that closes the current subpath back to its start.",
    },
    ScriptFnDoc {
        signature: "path_cubic(_: f64, _: f64, _: f64, _: f64, _: f64, _: f64) -> core::result::Result<gpui_rhai::canvas::CanvasPathSegment,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[
            "x",
            "y",
            "control_a_x",
            "control_a_y",
            "control_b_x",
            "control_b_y",
        ],
        doc: "Creates a cubic curve segment to (`x`, `y`) with control points (`control_a_x`, `control_a_y`) and (`control_b_x`, `control_b_y`).",
    },
    ScriptFnDoc {
        signature: "path_cubic(_: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasPathSegment,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[
            "x",
            "y",
            "control_a_x",
            "control_a_y",
            "control_b_x",
            "control_b_y",
        ],
        doc: "Creates a cubic curve segment to (`x`, `y`) with control points (`control_a_x`, `control_a_y`) and (`control_b_x`, `control_b_y`). Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "path_line(_: f64, _: f64) -> core::result::Result<gpui_rhai::canvas::CanvasPathSegment,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x", "y"],
        doc: "Creates a path segment that draws a straight line to (`x`, `y`).",
    },
    ScriptFnDoc {
        signature: "path_line(_: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasPathSegment,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x", "y"],
        doc: "Creates a path segment that draws a straight line to (`x`, `y`). Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "path_move(_: f64, _: f64) -> core::result::Result<gpui_rhai::canvas::CanvasPathSegment,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x", "y"],
        doc: "Creates a path segment that starts a new subpath at (`x`, `y`); every path begins with one.",
    },
    ScriptFnDoc {
        signature: "path_move(_: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasPathSegment,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x", "y"],
        doc: "Creates a path segment that starts a new subpath at (`x`, `y`); every path begins with one. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "path_quadratic(_: f64, _: f64, _: f64, _: f64) -> core::result::Result<gpui_rhai::canvas::CanvasPathSegment,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x", "y", "control_x", "control_y"],
        doc: "Creates a quadratic curve segment to (`x`, `y`) with control point (`control_x`, `control_y`); the end point comes first.",
    },
    ScriptFnDoc {
        signature: "path_quadratic(_: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasPathSegment,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x", "y", "control_x", "control_y"],
        doc: "Creates a quadratic curve segment to (`x`, `y`) with control point (`control_x`, `control_y`); the end point comes first. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "propagate() -> EventResponse",
        params: &[],
        doc: "Returns an event response that lets the event continue to ancestor handlers; a handler returning any other value stops it.",
    },
    ScriptFnDoc {
        signature: "px(_: f64) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Creates a length in logical pixels; `value` must be finite and non-negative.",
    },
    ScriptFnDoc {
        signature: "px(_: i64) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Creates a length in logical pixels; `value` must be non-negative.",
    },
    ScriptFnDoc {
        signature: "readable(_: ColorValue, _: ColorValue, _: ColorValue, _: f64) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["color", "toward", "background", "ratio"],
        doc: "Moves `color` toward `toward` just far enough to reach WCAG contrast `ratio` (1 to 21) against `background`, when the node renders.",
    },
    ScriptFnDoc {
        signature: "relative(_: f64) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["fraction"],
        doc: "Creates a length as a fraction of the parent's size, from 0 to 1: `relative(0.5)` is half.",
    },
    ScriptFnDoc {
        signature: "relative(_: i64) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["fraction"],
        doc: "Creates a length as a fraction of the parent's size; as an integer only 0 or 1, and `relative(1)` fills the parent.",
    },
    ScriptFnDoc {
        signature: "rem(_: f64) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Creates a length in rems, multiples of the window's rem size; `value` must be finite and non-negative.",
    },
    ScriptFnDoc {
        signature: "rem(_: i64) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Creates a length in rems, multiples of the window's rem size; `value` must be non-negative.",
    },
    ScriptFnDoc {
        signature: "render_component(_: string, _: map) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["id", "props"],
        doc: "Renders the formal component registered as `id` with `props`, validated and defaulted; only during view render.",
    },
    ScriptFnDoc {
        signature: "rgb(_: i64) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["hex"],
        doc: "Creates an opaque color from `0xRRGGBB` (0 to 0xffffff).",
    },
    ScriptFnDoc {
        signature: "rgba(_: i64) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["hex"],
        doc: "Creates a color from `0xRRGGBBAA`, alpha in the low byte (0 to 0xffffffff).",
    },
    ScriptFnDoc {
        signature: "row(_: array) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["children"],
        doc: "Creates a box that lays its children out side by side (`flex_row`); every child must be a `UiNode`.",
    },
    ScriptFnDoc {
        signature: "shadow(_: map) -> core::result::Result<gpui_rhai::style::ShadowSpec,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["spec"],
        doc: "Creates a box shadow from `#{ x, y, blur, spread, color }` in pixels; only `color` is required, and `blur` and `spread` are non-negative.",
    },
    ScriptFnDoc {
        signature: "signal(_: string, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::signal::NativeSignal,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "initial"],
        doc: "Declares a component-local native signal holding a bool, int, float, string or color during formal render; `initial` applies on first mount.",
    },
    ScriptFnDoc {
        signature: "span(_: string) -> Span",
        params: &["text"],
        doc: "Creates an inline run for `text([...])`, refined with `color`, `background`, `typography`, `bold` and `italic`.",
    },
    ScriptFnDoc {
        signature: "stack(_: array) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["children"],
        doc: "Creates a relatively positioned box, so children can be placed with `style().absolute()` and edge offsets.",
    },
    ScriptFnDoc {
        signature: "style() -> Style",
        params: &[],
        doc: "Returns an empty `Style` to build with chained setters.",
    },
    ScriptFnDoc {
        signature: "svg(_: string) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["markup"],
        doc: "Creates an inline SVG node from self-contained markup of at most 64 KiB and 2,048 elements; scripts and external references are rejected.",
    },
    ScriptFnDoc {
        signature: "text(_: array) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["spans"],
        doc: "Creates a rich text node from `span` values; no span may split a grapheme cluster.",
    },
    ScriptFnDoc {
        signature: "text(_: string) -> UiNode",
        params: &["content"],
        doc: "Creates a plain text node.",
    },
    ScriptFnDoc {
        signature: "theme_color(_: string) -> ColorValue",
        params: &["token"],
        doc: "References theme color `token`, such as `\"accent\"` or `\"charts.series_a\"`, resolved against the active theme when the node renders.",
    },
    ScriptFnDoc {
        signature: "theme_length(_: string) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["path"],
        doc: "References length token `path` (`namespace.name`, such as `\"metrics.row\"`), resolved against theme and environment when rendered.",
    },
    ScriptFnDoc {
        signature: "theme_radius(_: string) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["name"],
        doc: "References radius token `radius.<name>`, such as `theme_radius(\"md\")`, resolved when the node renders.",
    },
    ScriptFnDoc {
        signature: "theme_spacing(_: string) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["name"],
        doc: "References spacing token `spacing.<name>`, such as `theme_spacing(\"sm\")`, resolved when the node renders.",
    },
    ScriptFnDoc {
        signature: "theme_typography(_: string) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["role"],
        doc: "Returns a `Style` with typography `role`, like `style().typography(role)`; the theme decides at render whether the role exists.",
    },
    ScriptFnDoc {
        signature: "timeout(_: string, _: i64, _: bool, _: Fn, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "delay_ms", "paused", "callback", "payload"],
        doc: "Declares one-shot timer `key` during formal render that calls `callback(ctx, payload)` `delay_ms` (1 ms to 24 h) later, unless `paused`.",
    },
    ScriptFnDoc {
        signature: "times(_: Length, _: f64) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["length", "factor"],
        doc: "Scales `length` by a non-negative `factor`, like `length * factor`; a relative result must stay within 0 to 1.",
    },
    ScriptFnDoc {
        signature: "times(_: Length, _: i64) -> core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["length", "factor"],
        doc: "Scales `length` by a non-negative integer `factor`, like `length * factor`; a relative result must stay within 0 to 1.",
    },
    ScriptFnDoc {
        signature: "virtual_collection(_: map, _: Fn) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["config", "renderer"],
        doc: "Creates a virtual list of `config.data` rendering visible items via named `renderer(ctx, payload)`; needs `key`, `estimated_height`, `height` or `fill_height`.",
    },
];
