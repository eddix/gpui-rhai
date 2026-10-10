/* gpui-rhai
{
  "id": "charts/map_chart",
  "export": "MapChart",
  "version": "0.1.7",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["charts/chart"],
  "capabilities": {}
}
*/
// Source-owned single-series choropleth adapter over Chart.
// State is controlled by the caller. Events are forwarded by charts/chart.
// Example: MapChart(#{ key: "regions", data: rows, map: "world", encode: #{ name: "code", value: "load" } })

import "charts/chart" as chart;

define_component(#{
    metadata: #{ id: "charts/map_chart", "export": "MapChart", version: "0.1.7",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: ["charts/chart"], capabilities: #{} },
    schema: #{ props: #{
        key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
            doc: "Stable chart identity; keeps native focus, viewport preview and transitions across renders." },
        data: #{ schema: #{ type: "one_of", variants: [#{ type: "array", items: #{ type: "ui_value" }, max_items: 10000 }, #{ type: "chart_data" }] }, required: true, sensitive: false,
            doc: "Row maps, converted once to typed columns, or a Host-owned `NativeChartData` handle for large or streaming data." },
        encode: #{ schema: #{ type: "optional", value: #{ type: "ui_value" } }, required: false, sensitive: false,
            doc: "Channels of the generated series, such as `#{ name: \"code\", value: \"load\" }`; required when `spec` is absent." },
        map: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
            doc: "Id of the Host-registered geo map (`RuntimeEngine::register_chart_map`) when `spec` is absent; `\"map\"` when omitted." },
        projection: #{ schema: #{ type: "string" }, required: false, sensitive: false, "default": #{ type: "string", value: "equirectangular" },
            doc: "Projection of the generated geo region: `\"equirectangular\"`, `\"mercator\"` or one the Host registered; ignored with `spec`." },
        key_dimension: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
            doc: "Dimension whose values identify each datum across updates, selection, focus and motion; without it updates replace every datum." },
        title: #{ schema: #{ type: "string" }, required: false, sensitive: false,
            doc: "Chart title when `spec` is absent; a blank title reserves no space." },
        series_key: #{ schema: #{ type: "string" }, required: false, sensitive: false, "default": #{ type: "string", value: "series" },
            doc: "Key of the generated series, as used by `hidden_series` and event payloads; ignored when `spec` is given." },
        series_name: #{ schema: #{ type: "string" }, required: false, sensitive: false, "default": #{ type: "string", value: "Series" },
            doc: "Legend name of the generated series; ignored when `spec` is given." },
        spec: #{ schema: #{ type: "optional", value: #{ type: "ui_value" } }, required: false, sensitive: false,
            doc: "Full `Chart` spec used instead of `encode`, `map`, `projection`, `title` and the series props; every series in it is drawn as `map`." },
        selected_keys: #{ schema: #{ type: "array", items: #{ type: "string" }, max_items: 10000 }, required: false, sensitive: false, "default": #{ type: "array", value: [] },
            doc: "Datum keys drawn as selected; the chart never selects by itself, so update it from `on_select` or `on_brush_change`." },
        hidden_series: #{ schema: #{ type: "array", items: #{ type: "string" }, max_items: 1000 }, required: false, sensitive: false, "default": #{ type: "array", value: [] },
            doc: "Keys of the series to hide; update it from `on_legend_change`." },
        zoom: #{ schema: #{ type: "number", min: 0.5, max: 20.0 }, required: false, sensitive: false, "default": #{ type: "float", value: 1.0 },
            doc: "Zoom factor of the scalar camera, a shorthand for `viewport` when one camera is enough." },
        pan_x: #{ schema: #{ type: "number" }, required: false, sensitive: false, "default": #{ type: "float", value: 0.0 },
            doc: "Horizontal pan of the scalar camera in logical pixels." },
        pan_y: #{ schema: #{ type: "number" }, required: false, sensitive: false, "default": #{ type: "float", value: 0.0 },
            doc: "Vertical pan of the scalar camera in logical pixels." },
        viewport: #{ schema: #{ type: "optional", value: #{ type: "ui_value" } }, required: false, sensitive: false,
            doc: "Controlled typed viewport, Cartesian axis windows or a Geo camera; store the `viewport` of each `zoom_change`." },
        viewport_revision: #{ schema: #{ type: "integer", min: 0 }, required: false, sensitive: false, "default": #{ type: "integer", value: 0 },
            doc: "Write back the `viewport_revision` of each `zoom_change`, with `viewport`, to accept or reject its native preview." },
        on_select: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
            doc: "Called with the `charts/chart` `select` payload when a data mark is clicked (without `spec.brush`) or activated with Enter or Space." },
        on_zoom_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
            doc: "Called with the `charts/chart` `zoom_change` payload when a zoom or pan gesture commits." },
        on_brush_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
            doc: "Called with the `charts/chart` `brush_change` payload when a brush drag ends; `spec.brush` turns brushing on." },
        on_legend_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
            doc: "Called with the `charts/chart` `legend_change` payload when a legend entry is clicked." },
        on_annotation_activate: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
            doc: "Called with the `charts/chart` `annotation_activate` payload when an annotation is clicked or activated from the keyboard." },
    }, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"] },
    render: Fn("render_MapChart")
});

fn MapChart(props) { render_component("charts/map_chart", props) }
// The adapter's own root rule and caller style reach the chart as its root style.
fn render_MapChart(ctx, props) {
    let chart_props = props;
    chart_props.style = ctx.component_style("root", style());
    chart::MapChart(chart_props)
}
