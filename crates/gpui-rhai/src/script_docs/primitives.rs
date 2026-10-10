//! Script API docs: native primitives in the `gpui_rhai` module. One entry per registered signature;
//! see `super` for the format.

use super::ScriptFnDoc;

pub(super) const DOCS: &[ScriptFnDoc] = &[
    ScriptFnDoc {
        signature: "CodeViewPrimitive(_)",
        params: &["props"],
        doc: "Native read-only, virtualized code view with syntax highlighting, selection, search and wrapping; `components/code_viewer` wraps it.",
    },
    ScriptFnDoc {
        signature: "ColumnResizePrimitive(_)",
        params: &["props"],
        doc: "Native table column separator that previews a dragged width and proposes `resize`; double-click auto-fits; `components/table` uses it.",
    },
    ScriptFnDoc {
        signature: "DiffViewPrimitive(_)",
        params: &["props"],
        doc: "Native read-only diff of two documents, unified or split, with folds, search and change navigation; `components/diff_viewer` wraps it.",
    },
    ScriptFnDoc {
        signature: "DragSourcePrimitive(_)",
        params: &["props"],
        doc: "Native drag source that carries a typed payload to a `DropZonePrimitive` and reports `drag_end`; `components/drag_source` wraps it.",
    },
    ScriptFnDoc {
        signature: "DraggablePrimitive(_)",
        params: &["props"],
        doc: "Native pointer and keyboard mover that previews an object's position and proposes one `move`; `components/draggable` wraps it.",
    },
    ScriptFnDoc {
        signature: "DropZonePrimitive(_)",
        params: &["props"],
        doc: "Native drop target that accepts typed payloads, paints drop feedback and proposes one `drop`; `components/drop_zone` wraps it.",
    },
    ScriptFnDoc {
        signature: "IntrinsicTextMeasurePrimitive(_)",
        params: &["props"],
        doc: "Zero-size native element that measures one line of text for column auto-fit; `components/table` pairs it with `ColumnResizePrimitive`.",
    },
    ScriptFnDoc {
        signature: "PanZoomPrimitive(_)",
        params: &["props"],
        doc: "Native pan and zoom over a Canvas that previews its transform and proposes `transform_change`; `components/pan_zoom` wraps it.",
    },
    ScriptFnDoc {
        signature: "RangeInputPrimitive(_)",
        params: &["props"],
        doc: "Native one-thumb range control with styled track, fill and thumb that proposes `change`; `components/slider` wraps it.",
    },
    ScriptFnDoc {
        signature: "RangeSliderPrimitive(_)",
        params: &["props"],
        doc: "Native two-thumb range control that keeps `low` and `high` ordered and proposes `change`; `components/range_slider` wraps it.",
    },
    ScriptFnDoc {
        signature: "ResizableHandlePrimitive(_)",
        params: &["props"],
        doc: "Native edge or corner handle that previews a rectangle resize and proposes `resize`; `components/resizable` wraps it.",
    },
    ScriptFnDoc {
        signature: "RotatablePrimitive(_)",
        params: &["props"],
        doc: "Native rotation around a pivot that previews a Canvas angle and proposes `rotate`; `components/rotatable` wraps it.",
    },
    ScriptFnDoc {
        signature: "SelectionAreaPrimitive(_)",
        params: &["props"],
        doc: "Native click, range and marquee selection of Canvas objects that proposes `selection_change`; `components/selection_area` wraps it.",
    },
    ScriptFnDoc {
        signature: "SortableItemPrimitive(_)",
        params: &["props"],
        doc: "Native drag grip for one keyed item that previews insertion and proposes `reorder`; `components/sortable` and `components/tab_bar` use it.",
    },
    ScriptFnDoc {
        signature: "SplitResizePrimitive(_)",
        params: &["props"],
        doc: "Native separator between two panels that previews a split ratio and proposes `resize`; `components/split_pane` wraps it.",
    },
    ScriptFnDoc {
        signature: "TextInputPrimitive(_)",
        params: &["props"],
        doc: "Native single-line text field with IME, selection, clipboard and undo; `components/input` wraps it.",
    },
    ScriptFnDoc {
        signature: "TextareaPrimitive(_)",
        params: &["props"],
        doc: "Native multi-line text field with wrapping, auto-growing rows, IME, selection and undo; `components/textarea` wraps it.",
    },
];
