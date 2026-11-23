//! Script API docs: global functions. One entry per registered signature;
//! see `super` for the format.

use super::ScriptFnDoc;

pub(super) const DOCS: &[ScriptFnDoc] = &[
    ScriptFnDoc {
        signature: "conflict_radar_project(_: array, _: array, _: map) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["agent_statuses", "files", "resolutions"],
        doc: "Computes exact inclusive line bands covered by edits from distinct agents, validates resolution identity, and projects readiness without merging anything.",
    },
    ScriptFnDoc {
        signature: "parse_delimited_preview(_: string, _: bool) -> types::dynamic::Dynamic",
        params: &["source", "tab_separated"],
        doc: "Parses a bounded rectangular CSV or TSV preview, preserving quoted values; returns null for malformed or oversized data so the caller can show the original text.",
    },
    ScriptFnDoc {
        signature: "context_capacity(_: array, _: i64, _: i64) -> core::result::Result<alloc::collections::btree::map::BTreeMap<smartstring::SmartString<smartstring::config::LazyCompact>,rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["counts", "limit", "cells"],
        doc: "Projects token counts onto a bounded strip of cells and reports used, remaining, and overflow totals.",
    },
    ScriptFnDoc {
        signature: "context_partition(_: array, _: i64, _: f64, _: f64) -> core::result::Result<alloc::collections::btree::map::BTreeMap<smartstring::SmartString<smartstring::config::LazyCompact>,rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["counts", "limit", "width", "height"],
        doc: "Partitions a finite viewport into context-count rectangles, returning tooltip bounds and used, free, and overflow totals.",
    },
    ScriptFnDoc {
        signature: "diagnostic_document(_: types::dynamic::Dynamic) -> core::result::Result<alloc::collections::btree::map::BTreeMap<smartstring::SmartString<smartstring::config::LazyCompact>,rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value"],
        doc: "Formats bounded diagnostic JSON or plain text for display; marks oversized content omitted rather than returning partial text.",
    },
    ScriptFnDoc {
        signature: "schedule_capacity(_: map) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["plan"],
        doc: "Evaluates then proposes placements for a bounded set of tasks and capacity windows, without mutating the accepted plan.",
    },
    ScriptFnDoc {
        signature: "judge_bracket_project(_: array, _: map, _: i64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["contenders", "verdicts", "capacity"],
        doc: "Projects ordered elimination matches, distinguishing empty seeds from unresolved opponents. Only verdicts that still match both entrants can advance a winner.",
    },
    ScriptFnDoc {
        signature: "root_cause_projection(_: array, _: f64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["causes", "width"],
        doc: "Validates a bounded fault tree, lays out its branches, and projects complete AND/OR proof paths from accepted cause states.",
    },
    ScriptFnDoc {
        signature: "rate_plan_curve(_: array, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["samples", "width", "height", "ceiling"],
        doc: "Projects 2–1024 ordered rate-plan {at, v} samples into a positive native-sized plot and returns the shared Catmull–Rom path. The ceiling clips only the displayed loads. Keeps original points and minutes intact.",
    },
    ScriptFnDoc {
        signature: "rate_plan_load_path(_: array, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["samples", "width", "load_unit"],
        doc: "Projects 2–1024 ordered rate-plan {at, v} samples across the unbroken day axis into an open polyline, with ordinate -v / load_unit. Unlike the displayed curve, does not clamp or smooth the original values, so peak ordering and warning thresholds agree.",
    },
    ScriptFnDoc {
        signature: "rate_plan_read(_: array, _: map, _: array) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["runs", "settings", "measured"],
        doc: "Projects a 24-hour plan, exact minute-level peaks and overloads, a nearest safe placement, and a bounded set of chart points. Optional settings.plot_width draws on a shared grid of approximately one point per 4 logical pixels (2–1024 points), without changing the exact measured/forecast seam. Optional settings.hover is a minute offset (0–1439); the result's hover map carries value and measured from the accepted data, not from the plotted samples. Does not change the plan.",
    },
    ScriptFnDoc {
        signature: "rate_plan_spread(_: array, _: map) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["runs", "settings"],
        doc: "Proposes deterministic near-anchor moves for future runs that do not increase the forecast peak. Running, finished, and filtered runs are left unchanged.",
    },
    ScriptFnDoc {
        signature: "rate_plan_add_slot(_: array, _: map, _: i64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["runs", "settings", "peak"],
        doc: "Finds the first buffer-safe start minute for a default run, or the least-loaded valid start. Returns null when the run would not fit in the remaining window.",
    },
    ScriptFnDoc {
        signature: "trace_waterfall_projection(_: array, _: array, _: i64, _: i64, _: i64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[
            "spans",
            "collapsed",
            "now_ms",
            "min_range_ms",
            "tick_capacity",
        ],
        doc: "Validates a bounded flat trace and projects visible rows, nested cost and token sums, live ends, and ticks on one elapsed-time domain. Folding never changes the domain or the sums.",
    },
    ScriptFnDoc {
        signature: "validate_replay_input(_: string, _: string) -> bool",
        params: &["input", "format"],
        doc: "Accepts text or strict JSON of up to 8192 UTF-8 bytes. No evaluation, authorization or tool execution is performed.",
    },
    ScriptFnDoc {
        signature: "text_initial(_: string) -> string",
        params: &["text"],
        doc: "Returns the first Unicode letter or number, uppercased, or an empty string when absent. Skips punctuation and emoji; Unicode uppercase mappings may contain multiple characters.",
    },
    ScriptFnDoc {
        signature: "named_function(_: string) -> core::result::Result<rhai::types::fn_ptr::FnPtr,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["name"],
        doc: "Creates a named function pointer retaining this module's lexical environment. Fn(name) is an alias in the SDK engine; curry and synchronous call are supported. UI state ownership is assigned separately when retained as a callback.",
    },
    ScriptFnDoc {
        signature: "time_parts(_: i64, _: string) -> core::result::Result<alloc::collections::btree::map::BTreeMap<smartstring::SmartString<smartstring::config::LazyCompact>,rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["ms", "zone"],
        doc: "Returns date, time, offset_minutes and zone for an epoch instant; date/time are ISO and HH:MM:SS.",
    },
    ScriptFnDoc {
        signature: "time_local_candidates(_: string, _: string, _: string) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date", "time", "zone"],
        doc: "Returns ordered epoch-ms instants for a local date/time: zero in a DST gap, two in a fold, one otherwise; never guesses.",
    },
    ScriptFnDoc {
        signature: "time_range_resolve(_: map, _: i64) -> core::result::Result<alloc::collections::btree::map::BTreeMap<smartstring::SmartString<smartstring::config::LazyCompact>,rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["value", "now_ms"],
        doc: "Resolves a relative duration at query time or preserves an absolute range, returning start/end epoch milliseconds.",
    },
    ScriptFnDoc {
        signature: "time_range_error(_: map, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> string",
        params: &["value", "min", "max", "max_span_ms"],
        doc: "Returns an empty string for a valid relative/absolute range, otherwise a validation explanation; optional epoch bounds and span use ().",
    },
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
        signature: "alpha(_: ColorValue, _: i64) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["color", "factor"],
        doc: "Returns `color` with its alpha multiplied by `factor` (0 to 1); a token color resolves when the node renders.",
    },
    ScriptFnDoc {
        signature: "alpha(_: PaintColor, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::PaintColor,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["color", "factor"],
        doc: "Multiplies Canvas paint alpha by factor (zero to one), retaining its deferred native foreground. Integers and floats are accepted. PaintColor is not accepted by ordinary Style or theme-color setters.",
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
        signature: "canvas_circle(_: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "center_x", "center_y", "radius", "fill"],
        doc: "Creates a filled circle command centered at (`center_x`, `center_y`); radius must be positive and key unique. Numbers may be integers or floats; fill accepts ColorValue or native PaintColor such as current_color().",
    },
    ScriptFnDoc {
        signature: "canvas_fill_path(_: string, _: array, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "segments", "fill"],
        doc: "Creates a path command filled with a solid color; `segments` start with `path_move` and hold 2 to 10,000 segments.",
    },
    ScriptFnDoc {
        signature: "canvas_fill_path(_: string, _: array, _: PaintColor) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "segments", "fill"],
        doc: "Creates a filled path with native-context paint, such as current_color(). Segments start with path_move and hold 2 to 10,000 segments. Foreground resolution occurs during native paint, not Rhai render.",
    },
    ScriptFnDoc {
        signature: "canvas_fill_path(_: string, _: array, _: LinearGradientSpec) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "segments", "gradient"],
        doc: "Creates a path command filled with a two-stop `linear_gradient`; `segments` start with `path_move` and hold 2 to 10,000 segments.",
    },
    ScriptFnDoc {
        signature: "canvas_fill_path(_: string, _: array, _: Style) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "segments", "paint_style"],
        doc: "Uses only the Style's base background, so a theme part can own Canvas paint while geometry stays in the component. A solid or two-stop fill is supported; no background draws transparent, and layered fills are rejected. Layout, shadows, and pseudo-state styles do not affect the path.",
    },
    ScriptFnDoc {
        signature: "canvas_line(_: string, _: f64, _: f64, _: f64, _: f64, _: f64, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "from_x", "from_y", "to_x", "to_y", "width", "color"],
        doc: "Creates a line command from (`from_x`, `from_y`) to (`to_x`, `to_y`), stroked `width` pixels wide (positive).",
    },
    ScriptFnDoc {
        signature: "canvas_line(_: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "from_x", "from_y", "to_x", "to_y", "width", "color"],
        doc: "Creates a line command from (from_x, from_y) to (to_x, to_y), stroked a positive width. Numbers may be integers or floats; color accepts ColorValue or native PaintColor.",
    },
    ScriptFnDoc {
        signature: "canvas_morph_fill_path(_: string, _: array, _: array, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "from", "to", "paint"],
        doc: "Builds a filled path whose two endpoints have identical command topology. Uses the Canvas node's path_progress sample for native drawing and hits. Accepts a color, contextual paint, linear gradient or a Style with a supported fill, like canvas_fill_path.",
    },
    ScriptFnDoc {
        signature: "canvas_morph_stroke_path(_: string, _: array, _: array, _: f64, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "from", "to", "width", "color"],
        doc: "Creates a stroked path that morphs from `from` to `to` as its `path_progress` motion goes 0 to 1; both need the same segment kinds.",
    },
    ScriptFnDoc {
        signature: "canvas_morph_stroke_path(_: string, _: array, _: array, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "from", "to", "width", "color"],
        doc: "Creates a stroked path that morphs between `from` and `to` as path_progress goes zero to one; both paths need the same segment kinds. Width accepts integers/floats; color accepts ColorValue or native PaintColor.",
    },
    ScriptFnDoc {
        signature: "canvas_rect(_: string, _: f64, _: f64, _: f64, _: f64, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "x", "y", "width", "height", "fill"],
        doc: "Creates a filled rectangle command with its top-left corner at (`x`, `y`); `width` and `height` must be positive.",
    },
    ScriptFnDoc {
        signature: "canvas_rect(_: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "x", "y", "width", "height", "fill"],
        doc: "Creates a filled rectangle at (x, y) with positive width/height. Numbers may be integers or floats; fill accepts ColorValue or native PaintColor.",
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
        signature: "canvas_stroke_path(_: string, _: array, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "segments", "width", "color"],
        doc: "Creates a path with positive stroke width and 2 to 10,000 segments starting with path_move. Width accepts integers/floats; color accepts ColorValue or native PaintColor.",
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
        signature: "current_color() -> PaintColor",
        params: &[],
        doc: "Returns deferred paint from the owning element's actual native text color, after inherited and own state refinements. Supports alpha(), Canvas ink, Style background/border, shadow color, and layered-gradient stops. It is not an ordinary theme color.",
    },
    ScriptFnDoc {
        signature: "cubic_bezier(_: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<rhai::types::immutable_string::ImmutableString,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x1", "y1", "x2", "y2"],
        doc: "Returns the easing `cubic_bezier(x1, y1, x2, y2)` for a motion `easing` or a theme easing token: a CSS cubic Bézier from (0, 0) to (1, 1); `x1` and `x2` must lie in [0, 1], `y` may overshoot. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "date_checked_add_days(_: string, _: i64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["date", "days"],
        doc: "Adds signed `days` to an ISO `YYYY-MM-DD` date and returns the new ISO date, or `()` when it leaves years 1 to 9999.",
    },
    ScriptFnDoc {
        signature: "date_days_between(_: string, _: string) -> core::result::Result<i64,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["start", "end"],
        doc: "Returns the signed number of Gregorian days from start to end, independent of time zones. Add one for the length of an inclusive ordered range.",
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
        signature: "directional_image(_: OpaqueHandle, _: OpaqueHandle) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["left_to_right", "right_to_left"],
        doc: "Creates an image node that shows the first image handle in left-to-right layout and the second in right-to-left layout; a handle of another kind raises an error.",
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
        signature: "text_anchor_plan(_: string, _: i64, _: array) -> core::result::Result<gpui_rhai::text_anchors::TextAnchorPlan,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["source", "revision", "requests"],
        doc: "Builds an exact text/revision/request identity for up to 32 grapheme anchors in up to 16 KiB. A map with id and start requests a caret (optional affinity); add an exclusive end for a range.",
    },
    ScriptFnDoc {
        signature: "element_ref(_: string) -> core::result::Result<gpui_rhai::element_ref::ElementRef,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key"],
        doc: "Declares a component-local element ref during formal render; attach it with `with_ref` and read it with `ctx.element_bounds`.",
    },
    ScriptFnDoc {
        signature: "element_ref(_: string, _: string) -> core::result::Result<gpui_rhai::element_ref::ElementRef,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["namespace", "business_key"],
        doc: "Declares a ref for a role and business ID during formal render. Namespace: 1–32 ASCII letters, digits, underscores or hyphens. Business key: 1–128 UTF-8 bytes. Use the returned ref or its opaque key for focus and geometry.",
    },
    ScriptFnDoc {
        signature: "error_boundary(_: UiNode, _: UiNode) -> UiNode",
        params: &["child", "fallback"],
        doc: "Wraps `child` so that a native rendering failure in its subtree, or a mistake in the tree it declares (two siblings with one key), shows `fallback` instead.",
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
        signature: "grapheme_slice(_: string, _: i64, _: i64) -> types::dynamic::Dynamic",
        params: &["text", "start", "end"],
        doc: "Returns the substring in the half-open extended-grapheme range. Equal valid endpoints return an empty string; invalid endpoints return ().",
    },
    ScriptFnDoc {
        signature: "grapheme_index_at_byte(_: string, _: i64, _: bool) -> types::dynamic::Dynamic",
        params: &["text", "byte", "downstream"],
        doc: "Converts a UTF-8 character boundary to a grapheme boundary in the same text. Inside a cluster, rounds forward when downstream is true and backward otherwise; an invalid byte offset returns ().",
    },
    ScriptFnDoc {
        signature: "memory_panel_projection(_: array, _: array, _: string, _: string) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["items", "categories", "active_category", "query"],
        doc: "Validates a bounded list of memory entries and returns category counts, visible indices, total and paused count. Does not persist or inspect application data outside its arguments.",
    },
    ScriptFnDoc {
        signature: "memory_panel_normalize(_: string) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["text"],
        doc: "Trims and collapses whitespace in a single memory phrase, with a bounded input and output.",
    },
    ScriptFnDoc {
        signature: "prompt_template_parts(_: string) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["template"],
        doc: "Tokenizes literal text and {name} placeholders without evaluating code. Returns ordered maps with text or name. ASCII names allow letters, digits, underscore and hyphen; limits are 32 KiB, 32 distinct names and 128 uses.",
    },
    ScriptFnDoc {
        signature: "cost_waterfall_projection(_: array, _: string, _: array) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["steps", "mode", "collapsed"],
        doc: "Rolls up a bounded flat parent tree with exact integer currency micro-units and token counts. Returns one signed domain and row geometry for cost or tokens, including descendants hidden by folds.",
    },
    ScriptFnDoc {
        signature: "billing_schedule_projection(_: string, _: string, _: array) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["start", "duration", "lines"],
        doc: "Quotes a bounded billing phase with integer minor units and basis-point discounts. Returns the first invoice, discounted phase subtotal, minimum-adjusted total and optional inclusive end date. It does not execute billing.",
    },
    ScriptFnDoc {
        signature: "diff_timeline_projection(_: array, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["edits", "elapsed", "duration"],
        doc: "Projects up to 192 {file, at, added, removed} entries into stable axis bounds, visible edit source indices and guides, sampled cumulative points, and file lanes. Times are elapsed seconds; () uses the completed run and an inferred duration.",
    },
    ScriptFnDoc {
        signature: "fan_out_layout(_: f64, _: i64, _: i64, _: i64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["width", "cell", "gap", "count"],
        doc: "Plans up to 1,024 fixed tiles with distributed horizontal spacing. Returns cell size, column count, horizontal pitch, vertical stride and total height, in logical pixels.",
    },
    ScriptFnDoc {
        signature: "fallback_chain_summary(_: array, _: i64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["attempts", "now_ms"],
        doc: "Summarizes up to 24 host-owned fallback attempts with exact signed costs and separate live and completed times. It never changes a status on timeout.",
    },
    ScriptFnDoc {
        signature: "fallback_dash_route(_: array, _: f64, _: f64) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["segments", "on", "off"],
        doc: "Creates dashed Canvas branch segments with on and off lengths expressed as fractions of each path's arc length.",
    },
    ScriptFnDoc {
        signature: "fallback_dot_grid(_: f64, _: f64, _: ColorValue) -> core::result::Result<gpui_rhai::canvas::CanvasScene,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["width", "height", "color"],
        doc: "Builds a bounded, native dot grid with one solid Canvas path per row. Retains the supplied color and alpha without sprite tinting.",
    },
    ScriptFnDoc {
        signature: "fan_out_summary(_: array, _: i64, _: i64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["tasks", "now_ms", "workers"],
        doc: "Summarizes up to 1,024 uniquely keyed tasks. Returns signed total cost, ordered failed IDs, status counts and a remaining-time estimate after three successful duration samples. Pass -1 to infer workers.",
    },
    ScriptFnDoc {
        signature: "plan_vs_actual_projection(_: array, _: i64) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["steps", "now_ms"],
        doc: "Projects a directed dependency graph of at most 96 steps into independent plan and forecast spans, keeping observed timings intact. Returns row indices for prerequisites and dependents, signed cost sums, and a bounded time axis.",
    },
    ScriptFnDoc {
        signature: "replace_recipient_name(_: string, _: string, _: string) -> core::result::Result<alloc::string::String,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["text", "previous", "next"],
        doc: "Replaces the previous full recipient name and standalone first name in one literal, nonrecursive pass. Names are limited to 512 UTF-8 bytes, and the text and result to 64000 bytes.",
    },
    ScriptFnDoc {
        signature: "normalize_site_domain(_: string) -> types::dynamic::Dynamic",
        params: &["text"],
        doc: "Canonicalizes a site-address hostname to strict IDNA ASCII, allowing an optional HTTP(S) prefix and trailing slashes. Returns an empty string for a blank field or () for invalid input; accepts no port, path, query, fragment, credential or IP address. Does not contact the network.",
    },
    ScriptFnDoc {
        signature: "markdown_literal(_: string) -> string",
        params: &["text"],
        doc: "Quotes literal inline text as Markdown character references. Add paragraph separators and intended syntax separately; embedded punctuation and whitespace do not become structure.",
    },
    ScriptFnDoc {
        signature: "utf8_byte_count(_: string) -> i64",
        params: &["text"],
        doc: "Returns a UTF-8 byte count, not a grapheme or character count. Use for documented byte ranges into native text.",
    },
    ScriptFnDoc {
        signature: "text_match_ranges(_: string, _: array, _: bool) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["text", "terms", "words_only"],
        doc: "Finds at most 128 sorted, merged UTF-8 byte ranges in bounded literal text. Matches exact phrases, or shared case-insensitive Unicode words when words_only is true. Inputs are limited to 64 KiB per text/term collection and 128 terms.",
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
        signature: "image(_: OpaqueHandle) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["handle"],
        doc: "Creates an image node for an image handle from `ctx.load_image` or a capability; a handle of another kind raises an error.",
    },
    ScriptFnDoc {
        signature: "image_source(_: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["source"],
        doc: "Creates an image node from `source`, an `AssetId` or an image handle; other values are rejected.",
    },
    ScriptFnDoc {
        signature: "interval(_: string, _: i64, _: bool, _: Fn, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "period_ms", "paused", "callback", "payload"],
        doc: "Declares repeating timer `key` during formal render that calls `callback(ctx, payload)` every `period_ms` (16 ms to 24 h) while not `paused`; a late tick is one call, not several. Shares keys with `timeout`.",
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
        signature: "key_legend(_: string) -> types::dynamic::Dynamic",
        params: &["key"],
        doc: "Describes one named key or literal for this platform as `#{ label, spoken, glyph, platform }`. This does not parse or register a key binding. `mod` means Command on macOS and Control elsewhere; blank input stays blank.",
    },
    ScriptFnDoc {
        signature: "key_legend(_: string, _: string) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "platform"],
        doc: "Describes one key for explicit `platform` (`mac` or `other`); rejects other values. Named aliases and punctuation have spelled-out accessible names.",
    },
    ScriptFnDoc {
        signature: "key_shortcut(_: string) -> types::dynamic::Dynamic",
        params: &["text"],
        doc: "Formats `text` such as `\"cmd-p\"` as `#{ label, keystrokes, chords }` with platform key names. Each chord has `key`, `modifiers` and ordered `keys` descriptors (`label`, `spoken`, `glyph`, `platform`) for custom keycaps. Text that is not keystrokes stays one literal label with no inferred key boundaries.",
    },
    ScriptFnDoc {
        signature: "layer(_: UiNode, _: map) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["content", "config"],
        doc: "Places `content` in a window-level layer; `config` needs `id` and may set `placement` (default `top_right`), `inset` (12 px) and `priority`.",
    },
    ScriptFnDoc {
        signature: "layered_gradient(_: array) -> core::result::Result<gpui_rhai::style_fill::LayeredGradient,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["layers"],
        doc: "Creates a topmost-first stack of up to eight gradients. Each layer has 2–16 stops with ordered offsets in [0,1] and ColorValue or PaintColor inks. Optional angle, origin, clip, size and position control the gradient image; padding and border boxes are independent. Optional repeat_period (0.5–65536 logical pixels) repeats the stops by projected distance instead of stretching them. Empty clears a background. This stack is not a Canvas path paint.",
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
        signature: "over(_: ColorValue, _: ColorValue) -> ColorValue",
        params: &["foreground", "background"],
        doc: "Source-over composition in encoded sRGB, returning straight RGBA. Token leaves stay late-bound. For contrast calculations, composite translucent backgrounds onto their actual opaque backing first; this color expression does not implement isolated group opacity or a backdrop filter.",
    },
    ScriptFnDoc {
        signature: "mix(_: ColorValue, _: ColorValue, _: i64) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["base", "other", "weight"],
        doc: "Mixes `base` toward `other` by `weight` (0 to 1) in every channel, alpha included; token colors resolve when the node renders.",
    },
    ScriptFnDoc {
        signature: "motion_numbers(_: array) -> core::result::Result<gpui_rhai::number_motion::MotionNumbers,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["refs"],
        doc: "Groups 1–128 distinct ElementRefs with numeric tracks from one formal component. Values and layout projections are derived from the same native frame.",
    },
    ScriptFnDoc {
        signature: "mix_premultiplied(_: ColorValue, _: ColorValue, _: f64) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["base", "other", "weight"],
        doc: "Interpolates toward the other color by weight (0 to 1) in premultiplied encoded sRGB, returning straight RGBA. Transparent endpoints do not contribute hidden RGB. Token colors remain late-bound; mix keeps its independent-channel behavior.",
    },
    ScriptFnDoc {
        signature: "mix_premultiplied(_: ColorValue, _: ColorValue, _: i64) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["base", "other", "weight"],
        doc: "Interpolates toward the other color by weight (0 to 1) in premultiplied encoded sRGB, returning straight RGBA. Transparent endpoints do not contribute hidden RGB. Token colors remain late-bound; mix keeps its independent-channel behavior.",
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
        signature: "motion_signal(_: NativeSignal, _: array, _: MotionSource) -> core::result::Result<gpui_rhai::motion::MotionProgressBinding,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["signal", "active_states", "source"],
        doc: "Drives `source` forward while a native string signal matches an active state, and back otherwise. Sampling and reversal stay native and follow Host motion policy; takes a transition or keyframes without delay or repeat.",
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
        doc: "Drives source by this node's mouse press: forward while held, back on release inside or outside its bounds. Disabling returns it to rest; retained-page hiding, suspension or removing the binding revokes the transient press lease without resetting unrelated motion clocks. Initial presentation uses the declared inactive value. Takes a transition or keyframes without delay or repeat.",
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
        doc: "Splits text into grapheme spans. The default reveal profile accepts duration_ms, stagger_ms and easing. profile: \"sweep\" makes one bounded repeating opacity sweep (at most 48 graphemes; duration_ms is one cycle). Use a decorative sweep over an independently stable readable label.",
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
        doc: "Creates a transition of `property` from `from` to `to`; `config` sets `duration_ms` (180), `delay_ms`, `easing`, `iterations`, `intent`. In normal motion, `background_position_x` moves the front layered gradient image in fractions of remaining horizontal positioning space; otherwise the authored position is kept.",
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
        doc: "Returns a fuzzy-filtered, ranked and grouped view of a native collection for Command; config sets query and row field names. Optional hint_field contributes to search with keyword precedence. Optional icon_field/logo_field project logical asset ID strings only; they do not affect order-cache identity or load assets. Missing/null optional row fields are empty, not embedded UI nodes.",
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
        signature: "native_list_view(_: NativeCollection, _: array) -> core::result::Result<gpui_rhai::native_collection::NativeCollection,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["items", "selected_keys"],
        doc: "Normalizes a List row collection natively, projecting selected flags without enumerating rows in Rhai.",
    },
    ScriptFnDoc {
        signature: "native_list_view(_: array, _: array) -> core::result::Result<gpui_rhai::native_collection::NativeCollection,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["items", "selected_keys"],
        doc: "Normalizes an array of keyed List rows using the same projection as native data.",
    },
    ScriptFnDoc {
        signature: "native_list_neighbors(_: NativeCollection, _: array) -> core::result::Result<alloc::collections::btree::map::BTreeMap<smartstring::SmartString<smartstring::config::LazyCompact>,rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["items", "selected_keys"],
        doc: "Returns enabled List navigation targets first/last/previous/next/current/current_index and the current row's has_children flag; disabled and missing selections are skipped.",
    },
    ScriptFnDoc {
        signature: "native_selection_view(_: types::dynamic::Dynamic, _: array) -> core::result::Result<gpui_rhai::native_collection::NativeCollection,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["items", "selected_keys"],
        doc: "Normalizes SelectableList rows (an array of maps or a NativeCollection, keyed by `key`) natively: each row gains `selected` and a bool `disabled`, without enumerating rows in Rhai.",
    },
    ScriptFnDoc {
        signature: "native_selection_neighbors(_: types::dynamic::Dynamic, _: array) -> core::result::Result<alloc::collections::btree::map::BTreeMap<smartstring::SmartString<smartstring::config::LazyCompact>,rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["items", "selected_keys"],
        doc: "Returns SelectableList navigation targets first/last/previous/next/current/current_index; disabled rows and missing selections are skipped.",
    },
    ScriptFnDoc {
        signature: "native_table_view(_: array, _: map) -> core::result::Result<gpui_rhai::native_collection::NativeCollection,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["rows", "config"],
        doc: "Applies Table's native projection to array rows. Controlled sort names a declared column key; sort_key resolves its rank field. Null/missing values are last in both directions; ties preserve source order.",
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
        doc: "Anchors `content` to `trigger` in a window-level overlay; `config` needs `id` and sets `kind`, `open`, `placement`, `align`, `gap`, dismissal, and the panel's width against the trigger (`match_trigger_width` or `min_trigger_width`).",
    },
    ScriptFnDoc {
        signature: "path_catmull_rom(_: array) -> core::result::Result<alloc::vec::Vec<rhai::types::dynamic::Dynamic>,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["segments"],
        doc: "Converts one open move/line polyline to uniform Catmull–Rom cubic segments through the same vertices. First and last tangents use repeated endpoints. Corresponding controls interpolate with the vertices for compatible morph paths. Close an area after this conversion, not before.",
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
        doc: "Moves `color` toward `toward` just far enough to reach RGB contrast `ratio` (1 to 21) against `background`, when the node renders. Alpha is not composited by this helper: use `over` to account for translucent colors and their known opaque backing first.",
    },
    ScriptFnDoc {
        signature: "readable(_: ColorValue, _: ColorValue, _: ColorValue, _: i64) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["color", "toward", "background", "ratio"],
        doc: "Moves `color` toward `toward` just far enough to reach RGB contrast `ratio` (1 to 21) against `background`, when the node renders. Alpha is not composited by this helper: use `over` to account for translucent colors and their known opaque backing first.",
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
        doc: "Creates a box shadow from `#{ x, y, blur, spread, color, inset }` in logical pixels; color is required and accepts ColorValue or PaintColor. Offsets and spread are finite/signed, blur is non-negative, and inset is a bool defaulting to false. All numbers must be representable by the backend.",
    },
    ScriptFnDoc {
        signature: "signal(_: string, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::signal::NativeSignal,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["key", "initial"],
        doc: "Declares a component-local native signal holding a bool, int, float, string or color during formal render; `initial` applies on first mount.",
    },
    ScriptFnDoc {
        signature: "inline_prose(_: UiNode, _: array) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["prose", "targets"],
        doc: "Places up to 32 keyed empty link nodes over nonoverlapping grapheme ranges of a rich `text([...])` node. Targets are `#{ start, end, node }` maps with zero-based, end-exclusive indices. Each link remains a normal retained keyboard and accessibility node; its pointer hits follow the shaped line fragments, not the semantic rectangle.",
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
        signature: "theme_color(_: string) -> core::result::Result<gpui_rhai::style::ColorValue,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["token"],
        doc: "References theme color `token`, a snake_case `name` or `namespace.name` such as `\"accent\"` or `\"text.danger\"`, resolved when the node renders; other paths raise an error.",
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
        signature: "without_reading_scale(_: Length) -> Length",
        params: &["length"],
        doc: "Keeps a theme length late-bound to its token, size/density variants and explicit `times` factor, but omits the inherited reading-scale multiplier. Use for structural hairlines, not to freeze control or text sizing. Literal px/rem/relative lengths already bypass reading scale and remain unchanged.",
    },
    ScriptFnDoc {
        signature: "virtual_collection(_: map, _: Fn) -> core::result::Result<gpui_rhai::node::UiNode,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["config", "renderer"],
        doc: "Creates a virtual list of `config.data` rendering visible items via named `renderer(ctx, payload)`; needs `key`, `estimated_height` and exactly one sizing mode: numeric `height`, `fill_height: true`, or positive absolute Length `max_height` to fit actual measured content up to that cap. Theme/rem caps resolve in the inherited environment. Optional `item_height` is a positive px/rem/theme Length used for native uniform hints; actual rows remain measured. Optional numeric `content_padding_top` and `content_padding_bottom` add 0–512 logical pixels inside the scrolling content, without adding items. `overscan_items`, `batch_items` and `retain_items` configure render-ahead.",
    },
];
