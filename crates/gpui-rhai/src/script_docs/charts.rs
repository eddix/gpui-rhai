//! Script API docs: functions the `charts` feature adds. One entry per registered signature;
//! see `super` for the format.

use super::ScriptFnDoc;

pub(super) const DOCS: &[ScriptFnDoc] = &[
    ScriptFnDoc {
        signature: "ChartPrimitive(_)",
        params: &["props"],
        doc: "Native retained 2D chart with typed data, transforms, hover, selection, brush and zoom; `charts/chart` wraps it.",
    },
    ScriptFnDoc {
        signature: "chart_validate(_: map, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["spec", "data", "key_dimension"],
        doc: "Checks that `spec` is a valid chart spec and `data` is `NativeChartData` or up to 10,000 row maps keyed by `key_dimension` or `()`.",
    },
    ScriptFnDoc {
        signature: "get$revision(_: &mut NativeChartData) -> i64",
        params: &[],
        doc: "Returns the data's revision, which increases by one each time the Host replaces its datasets.",
    },
    ScriptFnDoc {
        signature: "get_native_chart_data(_: &mut UiContext, _: string) -> core::result::Result<gpui_rhai::chart::data::NativeChartData,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["name"],
        doc: "Returns the Host-registered chart data `name`; the handle notifies its own updates, so the read adds no rerender dependency.",
    },
    ScriptFnDoc {
        signature: "len(_: &mut NativeChartData, _: string) -> i64",
        params: &["dataset"],
        doc: "Returns the number of rows in `dataset` at the current revision, or 0 for an unknown dataset.",
    },
];
