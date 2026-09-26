import "charts/chart" as chart;
import "components/scroll_area" as scroll_area;
import "components/button" as button;

fn state_schema() {
    #{ fields: #{
        zoom: #{ schema: #{ type: "float", min: 0.5, max: 20.0 },
            "default": #{ type: "float", value: 1.0 } },
        viewport_revision: #{ schema: #{ type: "integer", min: 0 },
            "default": #{ type: "integer", value: 0 } },
        broken: #{ schema: #{ type: "bool" },
            "default": #{ type: "bool", value: false } },
    } }
}

fn zoomed(ctx, proposal) {
    ctx.set_state("zoom", proposal.zoom);
    ctx.set_state("viewport_revision", proposal.viewport_revision);
}
fn break_chart(ctx, payload) { ctx.set_state("broken", true); }

fn line_chart(ctx, key, title) {
    let spec = #{
        interaction: #{ wheel_zoom: "modifier" },
        legend: #{ visible: false },
        series: [#{ key: "latency", name: "Latency", kind: "line",
            encode: #{ x: "minute", y: "latency" } }],
    };
    if title != "" { spec.title = title; }
    chart::Chart(#{ key: key, key_dimension: "id",
        data: [#{ id: "m1", minute: "09:00", latency: 42 },
            #{ id: "m2", minute: "09:05", latency: 51 },
            #{ id: "m3", minute: "09:10", latency: 38 }],
        spec: spec, zoom: ctx.get_state("zoom"),
        viewport_revision: ctx.get_state("viewport_revision"),
        on_zoom_change: Fn("zoomed")
    }).with_style(style().width(relative(1.0)).height(px(240)))
}

fn diagnostic_chart(ctx) {
    chart::Chart(#{ key: "diagnostic-chart", key_dimension: "id",
        data: [#{ id: "a", minute: "09:00", latency: 42 },
            #{ id: "b", minute: "09:05", latency: 51 }],
        spec: #{ title: "Diagnostic chart", legend: #{ visible: false },
            series: [#{ key: "latency", name: "Latency",
                kind: if ctx.get_state("broken") { "piee" } else { "line" },
                encode: #{ x: "minute", y: "latency" } }] }
    }).with_style(style().width(relative(1.0)).height(px(220)))
}

fn view(ctx) {
    scroll_area::ScrollArea(#{ key: "chart-story-scroll", label: "Scrollable charts",
        height: px(420), vertical_scrollbar: "always",
        content: column([
            text("Plain wheel scrolls this page; Command/Control + wheel zooms the chart.")
                .with_style(style().typography("body_small").text_color(theme_color("text_muted"))),
            text("No title").with_style(style().typography("title")),
            line_chart(ctx, "untitled-chart", ""),
            text("Explicit title").with_style(style().typography("title")),
            line_chart(ctx, "titled-chart", "Latency trend"),
            text("Diagnostics").with_style(style().typography("title")),
            button::Button(#{ text: "Break chart spec", variant: "danger",
                on_click: Fn("break_chart") }).test_id("break-chart"),
            diagnostic_chart(ctx),
        ]).with_style(style().padding(theme_spacing("lg")).gap(theme_spacing("md")))
    })
}
