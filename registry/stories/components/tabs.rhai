import "components/tabs" as tabs;
import "components/table" as table;
import "charts/chart" as chart;

fn state_schema() {
    #{ fields: #{
        selected: #{ schema: #{ type: "string", allowed: ["hosts", "latency"] },
            "default": #{ type: "string", value: "hosts" } },
        selected_hosts: #{ schema: #{ type: "array", max_items: 3, items: #{ type: "string" } },
            "default": #{ type: "array", value: [#{ type: "string", value: "edge-01" }] } },
    } }
}

fn selected(ctx, value) { ctx.set_state("selected", value); }
fn hosts_selected(ctx, value) { ctx.set_state("selected_hosts", value); }

fn hosts_table(ctx) {
    table::Table(#{ key: "tabs-hosts", label: "Hosts in Tabs", row_key: "id", fill_height: true,
        rows: [
            #{ id: "edge-01", host: "edge-01", region: "Tokyo", status: "Healthy", status_variant: "success" },
            #{ id: "edge-02", host: "edge-02", region: "Berlin", status: "Deploying", status_variant: "accent" },
            #{ id: "edge-03", host: "edge-03", region: "Virginia", status: "Drift", status_variant: "warning" },
        ],
        columns: [
            #{ key: "host", title: "Host", width: #{ kind: "flex", value: 1 },
                adornments: [#{ text_key: "status", variant_key: "status_variant", dot: true }] },
            #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 160 } },
        ],
        selection_mode: "multiple", selected_keys: ctx.get_state("selected_hosts"),
        on_selection_change: Fn("hosts_selected")
    })
}

fn latency_chart() {
    chart::Chart(#{ key: "tabs-latency", key_dimension: "id",
        data: [#{ id: "a", minute: "09:00", latency: 42 },
            #{ id: "b", minute: "09:05", latency: 51 },
            #{ id: "c", minute: "09:10", latency: 38 }],
        spec: #{ title: "Latency", legend: #{ visible: false },
            interaction: #{ wheel_zoom: "modifier" },
            series: [#{ key: "latency", kind: "line",
                encode: #{ x: "minute", y: "latency" } }] }
    }).with_style(style().width(relative(1.0)).height(relative(1.0)).min_height(px(0)))
}

fn view(ctx) {
    column([
        text("CONTENT-BEARING TABS").with_style(style().typography("heading")),
        text("The root stretches; relative-size Table and Chart panels consume the available content box.")
            .with_style(style().typography("body_small").text_color(theme_color("text_muted"))),
        tabs::Tabs(#{
            label: "Host details",
            value: ctx.get_state("selected"),
            layout: "content",
            motion_key: "story-tabs",
            tabs: [
                #{ value: "hosts", label: "Hosts", content: hosts_table(ctx) },
                #{ value: "latency", label: "Latency", content: latency_chart() },
                #{ value: "locked", label: "Locked", content: text(""), disabled: true },
            ],
            on_change: Fn("selected"),
        }).with_style(style().width(relative(1.0)).height(px(420)).min_height(px(0)))
    ]).with_style(style().width(relative(1.0)).height(relative(1.0)).min_height(px(0))
        .padding(theme_spacing("lg")).gap(theme_spacing("md")))
}
