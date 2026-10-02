import "components/table" as table;

fn state_schema() {
    #{ fields: #{
        selected: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "string" } },
            "default": #{ type: "array", value: [#{ type: "string", value: "edge-01" }] } },
        sort_key: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "host" } },
        sort_direction: #{ schema: #{ type: "string", allowed: ["ascending", "descending"] },
            "default": #{ type: "string", value: "ascending" } },
        widths: #{ schema: #{ type: "map", values: #{ type: "number" } },
            "default": #{ type: "map", value: #{} } },
    } }
}

fn selected(ctx, value) { ctx.set_state("selected", value); }
fn sorted(ctx, value) {
    if value != () {
        ctx.set_state("sort_key", value.key);
        ctx.set_state("sort_direction", value.direction);
    }
}
fn resized(ctx, value) {
    let widths = ctx.get_state("widths");
    widths[value.key] = value.width.value;
    ctx.set_state("widths", widths);
}

fn view(ctx) {
    let scenario = __TABLE_CASE__;
    let columns = if scenario == "wide" {
        [#{ key: "host", title: "Host", width: #{ kind: "fixed", value: 360 }, sortable: true },
         #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 300 } },
         #{ key: "state", title: "Status", width: #{ kind: "fixed", value: 260 } }]
    } else {
        [#{ key: "host", title: "Host", width: #{ kind: "flex", value: 1 }, sortable: true,
                adornments: [#{ text_key: "state", variant_key: "state_variant", dot: true }] },
         #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 140 } }]
    };
    let widths = ctx.get_state("widths");
    for index in 0..columns.len {
        let column = columns[index];
        if column.key in widths {
            column.width = #{ kind: "fixed", value: widths[column.key] };
            columns[index] = column;
        }
    }
    let props = #{
        key: "story-hosts", label: "Hosts", row_key: "id", height: 180,
        rows: [
            #{ id: "edge-01", host: "edge-01", region: "Tokyo",
                state: "Healthy", state_variant: "success" },
            #{ id: "edge-02", host: "edge-02", region: "Berlin",
                state: "Deploying", state_variant: "accent" },
            #{ id: "edge-03", host: "edge-03", region: "Virginia",
                state: "Drift", state_variant: "warning" },
        ],
        columns: columns, resizable_columns: true, on_column_resize: Fn("resized"),
        selection_mode: "multiple", selected_keys: ctx.get_state("selected"),
        sort: #{ key: ctx.get_state("sort_key"), direction: ctx.get_state("sort_direction") },
        on_selection_change: Fn("selected"), on_sort_change: Fn("sorted"),
    };
    if scenario == "loading" { props.loading = true; }
    if scenario == "empty" { props.rows = []; }
    if scenario == "projected-empty" { props.query = "missing"; props.search_fields = ["host", "region"]; }
    let info = ctx.theme_variant();
    column([text(`${info.family} / ${info.name} · ${info.mode}`)
        .with_style(style().typography("caption").text_color(theme_color("text_muted"))),
        table::Table(props)]).with_style(style().gap(theme_spacing("sm")))
}
