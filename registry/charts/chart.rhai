/* gpui-rhai
{
  "id": "charts/chart",
  "export": "Chart",
  "version": "0.1.7",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Native composable visualization surface. Small arrays are converted once to
// typed columns; NativeChartData keeps streaming and large data in Rust.
// State is controlled by the caller.
// Example: Chart(#{ key: "sales", data: rows, key_dimension: "id", spec: spec })

define_component(#{
    metadata: #{ id: "charts/chart", "export": "Chart", version: "0.1.7",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable chart identity; keeps native focus, viewport preview and transitions across renders." },
            spec: #{ schema: #{ type: "ui_value" }, required: true, sensitive: false,
                doc: "Typed chart description: `title`, `regions`, `axes`, `series`, `legend`, `brush`, `annotations` and link settings." },
            data: #{ schema: #{ type: "one_of", variants: [
                #{ type: "array", items: #{ type: "ui_value" }, max_items: 10000 },
                #{ type: "chart_data" },
            ] }, required: true, sensitive: false,
                doc: "Row maps, converted once to typed columns, or a Host-owned `NativeChartData` handle for large or streaming data." },
            key_dimension: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Dimension whose values identify each datum across updates, selection, focus and motion; without it updates replace every datum." },
            selected_keys: #{ schema: #{ type: "array", items: #{ type: "string" }, max_items: 10000 }, required: false,
                sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Datum keys drawn as selected; the chart never selects by itself, so update it from `on_select` or `on_brush_change`." },
            hidden_series: #{ schema: #{ type: "array", items: #{ type: "string" }, max_items: 1000 }, required: false,
                sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Keys of the series to hide; update it from `on_legend_change`." },
            zoom: #{ schema: #{ type: "number", min: 0.5, max: 20.0 }, required: false, sensitive: false,
                "default": #{ type: "float", value: 1.0 },
                doc: "Zoom factor of the scalar camera, a shorthand for `viewport` when one camera is enough." },
            pan_x: #{ schema: #{ type: "number" }, required: false, sensitive: false,
                "default": #{ type: "float", value: 0.0 },
                doc: "Horizontal pan of the scalar camera in logical pixels." },
            pan_y: #{ schema: #{ type: "number" }, required: false, sensitive: false,
                "default": #{ type: "float", value: 0.0 },
                doc: "Vertical pan of the scalar camera in logical pixels." },
            viewport: #{ schema: #{ type: "optional", value: #{ type: "ui_value" } }, required: false, sensitive: false,
                doc: "Controlled typed viewport, Cartesian axis windows or a Geo camera; store the `viewport` of each `zoom_change`." },
            viewport_revision: #{ schema: #{ type: "integer", min: 0 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 0 },
                doc: "Write back the `viewport_revision` of each `zoom_change`, with `viewport`, to accept or reject its native preview." },
            on_select: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the `select` payload when a data mark is clicked or activated with Enter or Space; with `spec.brush`, a press that moves more than 4 pixels brushes instead." },
            on_zoom_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the `zoom_change` payload when a zoom or pan gesture commits." },
            on_brush_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the `brush_change` payload when a brush drag ends; `spec.brush` turns brushing on, and a press in the plot becomes a brush once the pointer moves more than 4 pixels." },
            on_legend_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the `legend_change` payload when a legend entry is clicked." },
            on_annotation_activate: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the `annotation_activate` payload when an annotation is clicked or activated from the keyboard." },
        },
        state: #{ fields: #{} },
        events: #{
            select: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                dataset: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Dataset the datum belongs to." },
                region_key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Coordinate region that holds the mark." },
                revision: #{ schema: #{ type: "integer", min: 0 }, required: true, sensitive: false,
                    doc: "Data revision the chart presented when the mark was activated." },
                series_key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Key of the series that holds the datum." },
                datum_key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Stable key of the datum, the value to put in `selected_keys`." },
                name: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Display label of the mark." },
                value: #{ schema: #{ type: "optional", value: #{ type: "number" } }, required: true, sensitive: false,
                    doc: "Numeric value of the mark, or `()` when it has none." },
            } }, doc: "Emitted when a data mark is clicked or activated from the keyboard; the payload identifies the datum." },
            zoom_change: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                zoom: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Proposed zoom factor of the scalar camera." },
                pan_x: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Proposed horizontal pan in logical pixels." },
                pan_y: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Proposed vertical pan in logical pixels." },
                viewport: #{ schema: #{ type: "ui_value" }, required: true, sensitive: false,
                    doc: "Proposed typed viewport, or `()`; store it in `viewport` to accept the proposal." },
                viewport_revision: #{ schema: #{ type: "integer", min: 0 }, required: true, sensitive: false,
                    doc: "Revision of this proposal; write it back to `viewport_revision`." },
            } }, doc: "Emitted when a zoom or pan gesture commits; the payload is the proposed camera and viewport with its revision." },
            brush_change: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                keys: #{ schema: #{ type: "array", items: #{ type: "string" }, max_items: 10000 }, required: true, sensitive: false,
                    doc: "Datum keys of the marks inside the brush." },
                data: #{ schema: #{ type: "array", max_items: 10000, items: #{ type: "object", allow_unknown: false, fields: #{
                    dataset: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Dataset the datum belongs to." },
                    series_key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Key of the series that holds the datum." },
                    datum_key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Stable key of the datum." },
                    region_key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Coordinate region that holds the mark." },
                } } }, required: true, sensitive: false,
                    doc: "Unambiguous references to the data inside the brush, for linked or multi-series charts." },
                revision: #{ schema: #{ type: "integer", min: 0 }, required: true, sensitive: false,
                    doc: "Data revision the chart presented when the brush ended." },
                x: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Left edge of the brush rectangle in chart-local logical pixels." },
                y: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Top edge of the brush rectangle in chart-local logical pixels." },
                width: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Width of the brush rectangle in logical pixels; a `\"y\"` brush spans the whole chart." },
                height: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Height of the brush rectangle in logical pixels; an `\"x\"` brush spans the whole chart." },
            } }, doc: "Emitted when a brush drag ends; the payload is the data inside the brush and the brush rectangle." },
            legend_change: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                series_key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Key of the series whose legend entry was clicked." },
                visible: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                    doc: "Visibility the click asks for: `false` to add the series to `hidden_series`, `true` to remove it." },
            } }, doc: "Emitted when a legend entry is clicked; the payload names the series and the visibility it asks for." },
            annotation_activate: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Stable key of the annotation." },
                label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Label of the annotation." },
            } }, doc: "Emitted when an annotation is clicked or activated from the keyboard; the payload is its key and label." },
        },
        slots: #{}, parts: ["root", "loading", "error"],
    },
    render: Fn("render_Chart")
});

fn Chart(props) { render_component("charts/chart", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn specialized_chart(props, kind, coordinate_kind) {
    let chart_props = #{
        key: props.key,
        data: props.data,
        key_dimension: prop_or(props, "key_dimension", ()),
        selected_keys: prop_or(props, "selected_keys", []),
        hidden_series: prop_or(props, "hidden_series", []),
        zoom: prop_or(props, "zoom", 1.0),
        pan_x: prop_or(props, "pan_x", 0.0),
        pan_y: prop_or(props, "pan_y", 0.0),
        viewport_revision: prop_or(props, "viewport_revision", 0),
        on_select: prop_or(props, "on_select", ()),
        on_zoom_change: prop_or(props, "on_zoom_change", ()),
        on_brush_change: prop_or(props, "on_brush_change", ()),
        on_legend_change: prop_or(props, "on_legend_change", ()),
        on_annotation_activate: prop_or(props, "on_annotation_activate", ()),
        style: prop_or(props, "style", style()),
        part_styles: prop_or(props, "part_styles", #{}),
    };
    if "viewport" in props && props.viewport != () { chart_props.viewport = props.viewport; }
    if !("spec" in props) || props.spec == () {
        let coordinate = if coordinate_kind == "geo_2d" {
            #{ key: "main", kind: coordinate_kind, map: prop_or(props, "map", "map"),
                projection: prop_or(props, "projection", "equirectangular") }
        } else {
            #{ key: "main", kind: coordinate_kind }
        };
        chart_props.spec = #{
            regions: [coordinate],
            series: [#{
                key: prop_or(props, "series_key", "series"),
                name: prop_or(props, "series_name", "Series"),
                kind: kind,
                dataset: "main",
                coordinate: "main",
                encode: props.encode,
            }],
        };
        if "title" in props && props.title.trim() != "" { chart_props.spec.title = props.title; }
    } else {
        chart_props.spec = props.spec;
        for index in 0..chart_props.spec.series.len {
            chart_props.spec.series[index].kind = kind;
        }
    }
    Chart(chart_props)
}

// Ergonomic single-kind adapters. Complex mixed charts call Chart directly.
fn BarChart(props) { specialized_chart(props, "bar", "cartesian_2d") }
fn LineChart(props) { specialized_chart(props, "line", "cartesian_2d") }
fn AreaChart(props) { specialized_chart(props, "area", "cartesian_2d") }
fn ScatterChart(props) { specialized_chart(props, "scatter", "cartesian_2d") }
fn HeatmapChart(props) { specialized_chart(props, "heatmap", "cartesian_2d") }
fn CandlestickChart(props) { specialized_chart(props, "candlestick", "cartesian_2d") }
fn PieChart(props) { specialized_chart(props, "pie", "polar") }
fn DonutChart(props) { specialized_chart(props, "donut", "polar") }
fn RadarChart(props) { specialized_chart(props, "radar", "polar") }
fn GaugeChart(props) { specialized_chart(props, "gauge", "polar") }
fn FunnelChart(props) { specialized_chart(props, "funnel", "polar") }
fn MapChart(props) { specialized_chart(props, "map", "geo_2d") }
fn GeoScatterChart(props) { specialized_chart(props, "geo_scatter", "geo_2d") }
fn GeoLinesChart(props) { specialized_chart(props, "geo_lines", "geo_2d") }
fn emit_select(ctx, payload) { ctx.emit("select", payload); }
fn emit_zoom(ctx, payload) { ctx.emit("zoom_change", payload); }
fn emit_brush(ctx, payload) { ctx.emit("brush_change", payload); }
fn emit_legend(ctx, payload) { ctx.emit("legend_change", payload); }
fn emit_annotation(ctx, payload) { ctx.emit("annotation_activate", payload); }

fn render_Chart(ctx, props) {
    chart_validate(props.spec, props.data, prop_or(props, "key_dimension", ()));
    let root = ctx.component_style("root", style()
        .width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))
        .background(theme_color("surface")).border(px(1)).border_color(theme_color("border")));
    let primitive_props = #{
        key: props.key,
        spec: props.spec,
        data: props.data,
        key_dimension: prop_or(props, "key_dimension", ()),
        selected_keys: prop_or(props, "selected_keys", []),
        hidden_series: prop_or(props, "hidden_series", []),
        zoom: prop_or(props, "zoom", 1.0),
        pan_x: prop_or(props, "pan_x", 0.0),
        pan_y: prop_or(props, "pan_y", 0.0),
        viewport_revision: prop_or(props, "viewport_revision", 0),
        on_select: if "on_select" in props && props.on_select != () { Fn("emit_select") } else { () },
        on_zoom_change: if "on_zoom_change" in props && props.on_zoom_change != () { Fn("emit_zoom") } else { () },
        on_brush_change: if "on_brush_change" in props && props.on_brush_change != () { Fn("emit_brush") } else { () },
        on_legend_change: if "on_legend_change" in props && props.on_legend_change != () { Fn("emit_legend") } else { () },
        on_annotation_activate: if "on_annotation_activate" in props && props.on_annotation_activate != () { Fn("emit_annotation") } else { () },
    };
    if "viewport" in props && props.viewport != () { primitive_props.viewport = props.viewport; }
    gpui_rhai::ChartPrimitive(primitive_props).with_key(props.key).with_style(root).accessibility_role("figure")
        .accessibility_label(if "title" in props.spec { props.spec.title } else { "Chart" })
}
