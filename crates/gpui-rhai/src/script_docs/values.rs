//! Script API docs: methods of other runtime values. One entry per registered signature;
//! see `super` for the format.

use super::ScriptFnDoc;

pub(super) const DOCS: &[ScriptFnDoc] = &[
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
        doc: "Clips a path command to an axis-aligned Canvas-local rectangle that does not rotate with it; other commands raise an error.",
    },
    ScriptFnDoc {
        signature: "color(_: &mut Span, _: gpui_rhai::style::ColorValue) -> Span",
        params: &["color"],
        doc: "Sets the span's text color.",
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
        doc: "The component-local key the ref was declared with in `element_ref(key)`.",
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
        doc: "The ref's full identity, `<component path>:<key>`, which tells apart refs with the same key in different components.",
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
        signature: "scale(_: &mut CanvasCommand, _: f64) -> core::result::Result<gpui_rhai::canvas::CanvasCommand,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["factor"],
        doc: "Sets the path's positive uniform scale, applied before rotation and translation; other commands raise an error.",
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
