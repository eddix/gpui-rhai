import "charts/chart" as chart;

fn view(ctx) {
    column([
        text("NATIVE STREAMING LINE").with_style(style().typography("heading")),
        text("The Host owns typed columns and revisioned updates; Rhai declares one stable chart.")
            .with_style(style().typography("body_small").text_color(theme_color("text_muted"))),
        chart::Chart(#{ key: "streaming-line", data: ctx.get_native_chart_data("gallery_stream"),
            key_dimension: "id", spec: #{ title: "line", legend: #{ visible: false },
                interaction: #{ wheel_zoom: "modifier" },
                series: [#{ key: "line", name: "Latency", kind: "line",
                    encode: #{ x: "x", y: "y" } }] }
        }).with_style(style().width(relative(1.0)).height(relative(1.0)).min_height(px(0)))
    ]).with_style(style().width(relative(1.0)).height(relative(1.0)).min_height(px(0))
        .padding(theme_spacing("lg")).gap(theme_spacing("md")))
}
