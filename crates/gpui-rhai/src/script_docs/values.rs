//! Script API docs: methods of other runtime values. One entry per registered signature;
//! see `super` for the format.

use super::ScriptFnDoc;

pub(super) const DOCS: &[ScriptFnDoc] = &[
    ScriptFnDoc {
        signature: "project_y(_: &mut CanvasPeakRoute, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasPeakRoute,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[
            "source_top",
            "source_bottom",
            "display_top",
            "display_bottom",
        ],
        doc: "Clamps the winning ordinate to the authored range and maps it to the display range before placement offsets. Vertex ranking and canvas_peak_paint_above still use the original coordinates; clipped values do not become ties. Ranges must be finite and ordered.",
    },
    ScriptFnDoc {
        signature: "threshold_bands(_: &mut CanvasScene, _: CanvasCommand, _: types::dynamic::Dynamic, _: Style) -> core::result::Result<gpui_rhai::canvas::CanvasScene,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["polyline", "limit_y", "paint"],
        doc: "Uses the node's path_progress (default one) to sample a static or morph polyline with 2–1024 stationary, increasing x coordinates. Consecutive y samples strictly below limit_y open midpoint-edged bands onto a single viewport-anchored Style background. Bands paint before commands in the fixed viewport, have no hit target, and are replaced, or cleared by a style without a background.",
    },
    ScriptFnDoc {
        signature: "peak_route(_: &mut CanvasCommand) -> core::result::Result<gpui_rhai::canvas::CanvasPeakRoute,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &[],
        doc: "Extracts 2–1024 vertices of a single open polyline, including matching morph endpoints, for a retained peak placement. Selects the first highest sampled vertex, using the command transform and authored layout coordinates before any view-box projection.",
    },
    ScriptFnDoc {
        signature: "stroke_dash(_: &mut CanvasCommand, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["on", "off"],
        doc: "Sets a repeating stroke dash in the same units as stroke width. Each length must be in [0.5, 4096]. Paint and hits share the projected, interpolated path; paths beyond the native dash-work budget are neither painted nor hit.",
    },
    ScriptFnDoc {
        signature: "stroke_cap(_: &mut CanvasCommand, _: string) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["cap"],
        doc: "Sets both open stroke ends to butt, round or square; default butt. Applies to lines and stroked/morph paths, including the newly trimmed endpoint. Paint and hit testing share the actual native tessellation; filled-only commands and invalid names are errors.",
    },
    ScriptFnDoc {
        signature: "stroke_join(_: &mut CanvasCommand, _: string) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["join"],
        doc: "Sets stroke corners to miter, miter_clip, round or bevel; default miter with the native default miter limit. Applies only to lines and stroked/morph paths. Paint and hit geometry use the same options; invalid names and filled-only commands are errors.",
    },
    ScriptFnDoc {
        signature: "view_box(_: &mut CanvasScene, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasScene,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x", "y", "width", "height"],
        doc: "Fits source coordinates into the native drawable uniformly and centered (xMidYMid meet). Paint and presented hit testing share this mapping; layout still follows Style and theme lengths. Origins and dimensions must fit f32, dimensions must be at least f32::MIN_POSITIVE, and origin+dimension must remain distinct in f64. Raw scene hit tests/path samples remain in source coordinates; clipping remains an explicit Style/path choice.",
    },
    ScriptFnDoc {
        signature: "path_pulse(_: &mut CanvasScene, _: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: bool, _: bool) -> core::result::Result<gpui_rhai::canvas::CanvasScene,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["path_key", "radius", "paint", "hollow", "reverse"],
        doc: "Adds or replaces one decorative dot with a fading trail on a non-empty static path. The Canvas node's path_progress samples the marker by arc length in the presented projection, starting at zero if no motion is supplied. Hollow draws an outline; reverse samples the same path backward. The decoration owns no hit target.",
    },
    ScriptFnDoc {
        signature: "background(_: &mut Span, _: gpui_rhai::style::ColorValue) -> Span",
        params: &["color"],
        doc: "Paints `color` behind the span's text, as for inline code.",
    },
    ScriptFnDoc {
        signature: "bold(_: &mut Span) -> Span",
        params: &[],
        doc: "Sets the span's text in bold.",
    },
    ScriptFnDoc {
        signature: "capture_pointer(_: &mut EventResponse) -> EventResponse",
        params: &[],
        doc: "Captures the pointer for the handler's node, so its later move and up events go to that node until released.",
    },
    ScriptFnDoc {
        signature: "clip_rect(_: &mut CanvasCommand, _: f64, _: f64, _: f64, _: f64) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x", "y", "width", "height"],
        doc: "Clips a path command to an axis-aligned Canvas-local rectangle that does not rotate with it, replacing an earlier clip; other commands raise an error.",
    },
    ScriptFnDoc {
        signature: "clip_rect(_: &mut CanvasCommand, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x", "y", "width", "height"],
        doc: "Clips a path command to an axis-aligned Canvas-local rectangle that does not rotate with it, replacing an earlier clip; other commands raise an error. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "color(_: &mut Span, _: gpui_rhai::style::ColorValue) -> Span",
        params: &["color"],
        doc: "Sets the span's text color.",
    },
    ScriptFnDoc {
        signature: "ink_from(_: &mut Span, _: gpui_rhai::style::Style) -> Span",
        params: &["style"],
        doc: "Applies the style's authored text color to this inline span, if present. The paragraph still owns font size, alignment, wrapping and geometry.",
    },
    ScriptFnDoc {
        signature: "get$id(_: &mut NativeHandlerRef) -> string",
        params: &[],
        doc: "The `namespace.name` ID the Host registered this native handler under.",
    },
    ScriptFnDoc {
        signature: "get$identity(_: &mut NativeTextDocument) -> string",
        params: &[],
        doc: "The document's identity; a new identity starts a new reading session, while a new revision keeps it.",
    },
    ScriptFnDoc {
        signature: "get$key(_: &mut ElementRef) -> string",
        params: &[],
        doc: "The opaque component-local lookup key. Use it with `ctx.focus(key)` or geometry and scroll commands; a namespaced ref key need not equal the original business ID.",
    },
    ScriptFnDoc {
        signature: "get$key(_: &mut NativeSignal) -> string",
        params: &[],
        doc: "The signal's component-local key.",
    },
    ScriptFnDoc {
        signature: "get$key_field(_: &mut NativeCollection) -> string",
        params: &[],
        doc: "The name of the row field that holds each row's unique key.",
    },
    ScriptFnDoc {
        signature: "get$kind(_: &mut NativeSignal) -> string",
        params: &[],
        doc: "The signal's value type: `bool`, `integer`, `float`, `optional_float`, `string` or `color`.",
    },
    ScriptFnDoc {
        signature: "get$kind(_: &mut OpaqueHandle) -> string",
        params: &[],
        doc: "The kind of Rust-owned resource the handle refers to, such as `image`.",
    },
    ScriptFnDoc {
        signature: "get$len(_: &mut NativeCollection) -> i64",
        params: &[],
        doc: "The number of rows; Rhai can count the rows of a collection but not index or enumerate them.",
    },
    ScriptFnDoc {
        signature: "get$len(_: &mut NativeTextDocument) -> i64",
        params: &[],
        doc: "The length of the document's text in UTF-8 bytes.",
    },
    ScriptFnDoc {
        signature: "get$revision(_: &mut NativeTextDocument) -> i64",
        params: &[],
        doc: "The document's revision number, which increases with each published revision of the same identity.",
    },
    ScriptFnDoc {
        signature: "get$scope(_: &mut ElementRef) -> string",
        params: &[],
        doc: "A readable `<component path>:<key>` scope label for diagnostics. Displayed paths are not a lossless identity encoding; use semantic_id for accessibility relationships.",
    },
    ScriptFnDoc {
        signature: "get$semantic_id(_: &mut ElementRef) -> string",
        params: &[],
        doc: "An opaque, deterministic, whitespace-free accessibility ID encoded losslessly from the typed component path and ref key. Use it for accessibility_id and labelled_by/described_by relationships; unlike diagnostic scope, embedded path delimiters cannot alias another path.",
    },
    ScriptFnDoc {
        signature: "italic(_: &mut Span) -> Span",
        params: &[],
        doc: "Sets the span's text in italic.",
    },
    ScriptFnDoc {
        signature: "motion(_: &mut Span, _: gpui_rhai::motion::MotionSource) -> Span",
        params: &["source"],
        doc: "Declares an `opacity` motion for the span, replacing an earlier one; the span needs `with_key`, and other properties are rejected.",
    },
    ScriptFnDoc {
        signature: "prevent_default(_: &mut EventResponse) -> EventResponse",
        params: &[],
        doc: "Prevents the event's native default, such as an ancestor taking focus on press or an enclosing window drag area moving the window.",
    },
    ScriptFnDoc {
        signature: "release_pointer(_: &mut EventResponse) -> EventResponse",
        params: &[],
        doc: "Releases this pointer's capture, so its later events hit-test normally again.",
    },
    ScriptFnDoc {
        signature: "rotate(_: &mut CanvasCommand, _: f64) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["degrees"],
        doc: "Sets the path's rotation in degrees about the Canvas origin, replacing an earlier one; other commands raise an error.",
    },
    ScriptFnDoc {
        signature: "rotate(_: &mut CanvasCommand, _: i64) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["degrees"],
        doc: "Sets the path's rotation in degrees about the Canvas origin, replacing an earlier one; other commands raise an error.",
    },
    ScriptFnDoc {
        signature: "scale(_: &mut CanvasCommand, _: f64) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["factor"],
        doc: "Sets the path's positive uniform scale, applied before rotation and translation and replacing an earlier one; other commands raise an error.",
    },
    ScriptFnDoc {
        signature: "scale(_: &mut CanvasCommand, _: i64) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["factor"],
        doc: "Sets the path's positive uniform scale, applied before rotation and translation and replacing an earlier one; other commands raise an error.",
    },
    ScriptFnDoc {
        signature: "stop(_: &mut EventResponse) -> EventResponse",
        params: &[],
        doc: "Stops the event from reaching further nodes; the current node's remaining handlers for this phase still run.",
    },
    ScriptFnDoc {
        signature: "stop_immediate(_: &mut EventResponse) -> EventResponse",
        params: &[],
        doc: "Stops the event at once: no further handler runs, on this node or any other.",
    },
    ScriptFnDoc {
        signature: "to_string(_: &mut AssetId) -> string",
        params: &[],
        doc: "Returns the namespaced logical asset ID, such as `core/check`.",
    },
    ScriptFnDoc {
        signature: "to_string(_: &mut ImageDecodeHandle) -> string",
        params: &[],
        doc: "Returns a debug label for the pending decode, `image-decode#<n>`.",
    },
    ScriptFnDoc {
        signature: "to_string(_: &mut NativeCollection) -> string",
        params: &[],
        doc: "Returns a debug label with the row count, `NativeCollection(len=<n>)`.",
    },
    ScriptFnDoc {
        signature: "to_string(_: &mut NativeTextDocument) -> string",
        params: &[],
        doc: "Returns a debug label, `NativeTextDocument(<identity>, revision=<n>)`.",
    },
    ScriptFnDoc {
        signature: "to_string(_: &mut OpaqueHandle) -> string",
        params: &[],
        doc: "Returns a debug label, `<kind>#<id>`.",
    },
    ScriptFnDoc {
        signature: "to_string(_: &mut SubscriptionHandle) -> string",
        params: &[],
        doc: "Returns a debug label, `subscription#<n>`.",
    },
    ScriptFnDoc {
        signature: "to_string(_: &mut TaskHandle) -> string",
        params: &[],
        doc: "Returns a debug label, `task#<n>`.",
    },
    ScriptFnDoc {
        signature: "translate(_: &mut CanvasCommand, _: f64, _: f64) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x", "y"],
        doc: "Sets the path's translation, applied after scale and rotation and replacing an earlier one; other commands raise an error.",
    },
    ScriptFnDoc {
        signature: "translate(_: &mut CanvasCommand, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["x", "y"],
        doc: "Sets the path's translation, applied after scale and rotation and replacing an earlier one; other commands raise an error. Numbers may be integers or floats.",
    },
    ScriptFnDoc {
        signature: "strikethrough(_: &mut Span) -> Span",
        params: &[],
        doc: "Draws a strikethrough using the final span text color without changing text geometry.",
    },
    ScriptFnDoc {
        signature: "underline(_: &mut Span) -> Span",
        params: &[],
        doc: "Underlines the final span text without changing its layout.",
    },
    ScriptFnDoc {
        signature: "underline(_: &mut Span, _: f64, _: gpui_rhai::style::ColorValue) -> core::result::Result<gpui_rhai::node::Span,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["width", "color"],
        doc: "Underlines the span with a separate color and a finite width of 0–16 logical pixels without changing layout.",
    },
    ScriptFnDoc {
        signature: "underline(_: &mut Span, _: f64, _: gpui_rhai::style::ColorValue, _: f64) -> core::result::Result<gpui_rhai::node::Span,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["width", "color", "baseline_offset"],
        doc: "Underlines the span with its own color and a finite width of 0–16 logical pixels, at an offset of -16–32 logical pixels from every wrapped line's baseline.",
    },
    ScriptFnDoc {
        signature: "underline_hover(_: &mut Span, _: i64, _: string) -> core::result::Result<gpui_rhai::node::Span,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["duration_ms", "easing"],
        doc: "Transitions an explicit underline to the span's glyph color while its exact inline_prose link is hovered. Duration is 0–60000ms; the application motion preference applies.",
    },
    ScriptFnDoc {
        signature: "typography(_: &mut Span, _: string) -> Span",
        params: &["role"],
        doc: "Sets the span's family and weight from the typography `role`, such as `code`; size and line height stay the paragraph's.",
    },
    ScriptFnDoc {
        signature: "with_key(_: &mut Span, _: string) -> Span",
        params: &["key"],
        doc: "Names the span, unique within its text, so a span `motion` can target it.",
    },
];
