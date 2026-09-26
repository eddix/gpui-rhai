import "charts/chart" as chart;
import "charts/bar_chart" as bar_chart;
import "charts/line_chart" as line_chart;
import "charts/pie_chart" as pie_chart;
import "charts/map_chart" as map_chart;

fn series_rows() {
    [
        #{ id: "jan", category: "Jan", value: 18.0, other: 12.0, size: 40.0 },
        #{ id: "feb", category: "Feb", value: 31.0, other: 22.0, size: 80.0 },
        #{ id: "mar", category: "Mar", value: 26.0, other: 35.0, size: 25.0 },
        #{ id: "apr", category: "Apr", value: 44.0, other: 29.0, size: 120.0 },
        #{ id: "may", category: "May", value: 38.0, other: 41.0, size: 64.0 },
    ]
}

fn candle_rows() {
    [
        #{ id: "m", day: "Mon", open: 24.0, close: 29.0, low: 20.0, high: 33.0 },
        #{ id: "t", day: "Tue", open: 29.0, close: 25.0, low: 22.0, high: 32.0 },
        #{ id: "w", day: "Wed", open: 25.0, close: 36.0, low: 24.0, high: 39.0 },
        #{ id: "r", day: "Thu", open: 36.0, close: 34.0, low: 30.0, high: 41.0 },
    ]
}

fn geo_rows() {
    [#{ id: "north", name: "north", value: 72.0, lon: 4.0, lat: 7.0,
       source_lon: 2.0, source_lat: 2.0, target_lon: 8.0, target_lat: 8.0 },
     #{ id: "south", name: "south", value: 34.0, lon: 6.0, lat: 3.0,
       source_lon: 8.0, source_lat: 2.0, target_lon: 2.0, target_lat: 8.0 }]
}

fn card(title, content) {
    column([
        text(title).with_style(theme_typography("subtitle")),
        content.with_style(style().width(relative(1.0)).height(px(220))),
    ]).with_style(style().width(px(340)).padding(theme_spacing("md"))
        .gap(theme_spacing("sm")).border(px(1)).border_color(theme_color("border"))
        .background(theme_color("surface_raised")))
}

fn single(kind, key, encode, data) {
    chart::Chart(#{ key: key, data: data, key_dimension: "id", spec: #{
        title: kind,
        interaction: #{ wheel_zoom: "modifier" },
        series: [#{ key: key, name: kind, kind: kind, encode: encode }],
        annotations: if kind == "bar" { [
            #{ key: "target", kind: "baseline", axis: "y", value: 30.0, label: "Target" }
        ] } else { [] },
        brush: if kind == "scatter" { "xy" } else { "none" },
    } })
}

fn polar(kind, key) {
    chart::Chart(#{ key: key, data: series_rows(), key_dimension: "id", spec: #{
        title: kind,
        regions: [#{ key: "main", kind: "polar" }],
        series: [#{ key: key, name: kind, kind: kind,
            encode: #{ name: "category", value: "value" } }],
    } })
}

fn geo(kind, key, encode) {
    chart::Chart(#{ key: key, data: geo_rows(), key_dimension: "id", spec: #{
        title: kind,
        regions: [#{ key: "main", kind: "geo_2d", map: "demo_map",
            projection: "equirectangular" }],
        series: [#{ key: key, name: kind, kind: kind, encode: encode }],
    } })
}

fn view(ctx) {
    let rows = series_rows();
    let streaming = ctx.get_native_chart_data("gallery_stream");
    column([
        text("CHARTS").with_style(theme_typography("heading")),
        text("Native scenes, stable datum identity, Rust transforms, and Motion Runtime policy.")
            .with_style(theme_typography("body_small").text_color(theme_color("text_muted"))),
        text("SPECIALIZED ADAPTERS").with_style(theme_typography("caption").font_weight(700)),
        row([
            card("BarChart", bar_chart::BarChart(#{ key: "adapter-bar", data: rows,
                key_dimension: "id", title: "Bar adapter",
                encode: #{ x: "category", y: "value" } })),
            card("LineChart", line_chart::LineChart(#{ key: "adapter-line", data: rows,
                key_dimension: "id", title: "Line adapter",
                encode: #{ x: "category", y: "value" } })),
            card("PieChart", pie_chart::PieChart(#{ key: "adapter-pie", data: rows,
                key_dimension: "id", title: "Pie adapter",
                encode: #{ name: "category", value: "value" } })),
            card("MapChart", map_chart::MapChart(#{ key: "adapter-map", data: geo_rows(),
                key_dimension: "id", title: "Map adapter", map: "demo_map",
                encode: #{ name: "name", value: "value" } })),
        ]).with_style(style().gap(theme_spacing("md")).flex_wrap()),
        text("CARTESIAN").with_style(theme_typography("caption").font_weight(700)),
        row([
            card("Bar + annotation", single("bar", "bar", #{ x: "category", y: "value" }, rows)),
            card("Line · Host stream", single("line", "line", #{ x: "x", y: "y" }, streaming)),
            card("Area", single("area", "area", #{ x: "category", y: "value" }, series_rows())),
            card("Scatter + brush", single("scatter", "scatter",
                #{ x: "value", y: "other", size: "size" }, series_rows())),
            card("Heatmap", single("heatmap", "heat",
                #{ x: "category", y: "other", value: "value" }, series_rows())),
            card("Candlestick", chart::CandlestickChart(#{ key: "candle", data: candle_rows(),
                key_dimension: "id", encode: #{ x: "day", open: "open", close: "close",
                    low: "low", high: "high" } })),
        ]).with_style(style().gap(theme_spacing("md")).flex_wrap()),
        text("POLAR AND GEO").with_style(theme_typography("caption").font_weight(700)),
        row([
            card("Pie", polar("pie", "pie")),
            card("Donut", polar("donut", "donut")),
            card("Radar", polar("radar", "radar")),
            card("Gauge", polar("gauge", "gauge")),
            card("Funnel", polar("funnel", "funnel")),
            card("Choropleth", geo("map", "map", #{ name: "name", value: "value" })),
            card("Geo scatter", geo("geo_scatter", "geo_scatter",
                #{ longitude: "lon", latitude: "lat", value: "value" })),
            card("Geo lines", geo("geo_lines", "geo_lines",
                #{ source_longitude: "source_lon", source_latitude: "source_lat",
                    target_longitude: "target_lon", target_latitude: "target_lat" })),
        ]).with_style(style().gap(theme_spacing("md")).flex_wrap()),
    ]).with_style(style().width(relative(1.0)).min_height(relative(1.0))
        .padding(theme_spacing("lg")).gap(theme_spacing("lg")).overflow_y_scroll()
        .background(theme_color("surface")).text_color(theme_color("text_primary")))
        .scrollbars("hidden", "auto")
}
