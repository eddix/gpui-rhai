/* gpui-rhai
{
  "id": "charts/chart",
  "export": "Chart",
  "version": "0.1.5",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Native composable visualization surface. Small arrays are converted once to
// typed columns; NativeChartData keeps streaming and large data in Rust.
// Props: key, spec, data, key_dimension, selected_keys, hidden_series and
// semantic callbacks. State is controlled by the caller. Parts: root, loading,
// error. Events: select, zoom_change, brush_change, legend_change and
// annotation_activate.
// Example: Chart(#{ key: "sales", data: rows, key_dimension: "id", spec })

define_component(#{
    metadata: #{ id: "charts/chart", "export": "Chart", version: "0.1.5",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            spec: #{ schema: #{ type: "ui_value" }, required: true, sensitive: false },
            data: #{ schema: #{ type: "one_of", variants: [
                #{ type: "array", items: #{ type: "ui_value" }, max_items: 10000 },
                #{ type: "chart_data" },
            ] }, required: true, sensitive: false },
            key_dimension: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            selected_keys: #{ schema: #{ type: "array", items: #{ type: "string" }, max_items: 10000 }, required: false,
                sensitive: false, "default": #{ type: "array", value: [] } },
            hidden_series: #{ schema: #{ type: "array", items: #{ type: "string" }, max_items: 1000 }, required: false,
                sensitive: false, "default": #{ type: "array", value: [] } },
            zoom: #{ schema: #{ type: "number", min: 0.5, max: 20.0 }, required: false, sensitive: false,
                "default": #{ type: "float", value: 1.0 } },
            pan_x: #{ schema: #{ type: "number" }, required: false, sensitive: false,
                "default": #{ type: "float", value: 0.0 } },
            pan_y: #{ schema: #{ type: "number" }, required: false, sensitive: false,
                "default": #{ type: "float", value: 0.0 } },
            on_select: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_zoom_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_brush_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_legend_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_annotation_activate: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{
            select: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                series_key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                datum_key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                name: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                value: #{ schema: #{ type: "optional", value: #{ type: "number" } }, required: true, sensitive: false },
            } } },
            zoom_change: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                zoom: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                pan_x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                pan_y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            } } },
            brush_change: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                keys: #{ schema: #{ type: "array", items: #{ type: "string" }, max_items: 10000 }, required: true, sensitive: false },
                x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                width: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                height: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            } } },
            legend_change: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                series_key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                visible: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            } } },
            annotation_activate: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            } } },
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
        on_select: prop_or(props, "on_select", ()),
        on_zoom_change: prop_or(props, "on_zoom_change", ()),
        on_brush_change: prop_or(props, "on_brush_change", ()),
        on_legend_change: prop_or(props, "on_legend_change", ()),
        on_annotation_activate: prop_or(props, "on_annotation_activate", ()),
        style: prop_or(props, "style", style()),
        part_styles: prop_or(props, "part_styles", #{}),
    };
    if !("spec" in props) || props.spec == () {
        let coordinate = if coordinate_kind == "geo_2d" {
            #{ key: "main", kind: coordinate_kind, map: prop_or(props, "map", "map"),
                projection: prop_or(props, "projection", "equirectangular") }
        } else {
            #{ key: "main", kind: coordinate_kind }
        };
        chart_props.spec = #{
            title: prop_or(props, "title", ""),
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
    } else {
        chart_props.spec = props.spec;
        for series in chart_props.spec.series { series.kind = kind; }
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
    let root = ctx.component_style("root", style()
        .width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))
        .background(theme_color("surface")).border(px(1)).border_color(theme_color("border")));
    gpui_rhai::ChartPrimitive(#{
        key: props.key,
        spec: props.spec,
        data: props.data,
        key_dimension: prop_or(props, "key_dimension", ()),
        selected_keys: prop_or(props, "selected_keys", []),
        hidden_series: prop_or(props, "hidden_series", []),
        zoom: prop_or(props, "zoom", 1.0),
        pan_x: prop_or(props, "pan_x", 0.0),
        pan_y: prop_or(props, "pan_y", 0.0),
        on_select: if "on_select" in props && props.on_select != () { Fn("emit_select") } else { () },
        on_zoom_change: if "on_zoom_change" in props && props.on_zoom_change != () { Fn("emit_zoom") } else { () },
        on_brush_change: if "on_brush_change" in props && props.on_brush_change != () { Fn("emit_brush") } else { () },
        on_legend_change: if "on_legend_change" in props && props.on_legend_change != () { Fn("emit_legend") } else { () },
        on_annotation_activate: if "on_annotation_activate" in props && props.on_annotation_activate != () { Fn("emit_annotation") } else { () },
    }).with_key(props.key).with_style(root).accessibility_role("figure")
        .accessibility_label(if "title" in props.spec { props.spec.title } else { "Chart" })
}
