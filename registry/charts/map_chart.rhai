/* gpui-rhai
{
  "id": "charts/map_chart",
  "export": "MapChart",
  "version": "0.1.5",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": ["charts/chart"],
  "capabilities": {}
}
*/
// Source-owned single-series choropleth adapter over Chart.
// Props: key, data, encode, map, projection, key_dimension, title, controlled
// selection/legend/viewport, optional full spec, and semantic callbacks.
// State is controlled by the caller. Events are forwarded by charts/chart.
// Example: MapChart(#{ key: "regions", data: rows, map: "world", encode: #{ name: "code", value: "load" } })

import "charts/chart" as chart;

define_component(#{
    metadata: #{ id: "charts/map_chart", "export": "MapChart", version: "0.1.5",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: ["charts/chart"], capabilities: #{} },
    schema: #{ props: #{
        key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
        data: #{ schema: #{ type: "one_of", variants: [#{ type: "array", items: #{ type: "ui_value" }, max_items: 10000 }, #{ type: "chart_data" }] }, required: true, sensitive: false },
        encode: #{ schema: #{ type: "ui_value" }, required: true, sensitive: false },
        map: #{ schema: #{ type: "string" }, required: true, sensitive: false },
        projection: #{ schema: #{ type: "string" }, required: false, sensitive: false, "default": #{ type: "string", value: "equirectangular" } },
        key_dimension: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
        title: #{ schema: #{ type: "string" }, required: false, sensitive: false, "default": #{ type: "string", value: "Map" } },
        series_key: #{ schema: #{ type: "string" }, required: false, sensitive: false, "default": #{ type: "string", value: "series" } },
        series_name: #{ schema: #{ type: "string" }, required: false, sensitive: false, "default": #{ type: "string", value: "Series" } },
        spec: #{ schema: #{ type: "optional", value: #{ type: "ui_value" } }, required: false, sensitive: false },
        selected_keys: #{ schema: #{ type: "array", items: #{ type: "string" }, max_items: 10000 }, required: false, sensitive: false, "default": #{ type: "array", value: [] } },
        hidden_series: #{ schema: #{ type: "array", items: #{ type: "string" }, max_items: 1000 }, required: false, sensitive: false, "default": #{ type: "array", value: [] } },
        zoom: #{ schema: #{ type: "number", min: 0.5, max: 20.0 }, required: false, sensitive: false, "default": #{ type: "float", value: 1.0 } },
        pan_x: #{ schema: #{ type: "number" }, required: false, sensitive: false, "default": #{ type: "float", value: 0.0 } },
        pan_y: #{ schema: #{ type: "number" }, required: false, sensitive: false, "default": #{ type: "float", value: 0.0 } },
        on_select: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        on_zoom_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        on_brush_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        on_legend_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        on_annotation_activate: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
    }, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"] },
    render: Fn("render_MapChart")
});

fn MapChart(props) { render_component("charts/map_chart", props) }
fn render_MapChart(ctx, props) { chart::MapChart(props) }
