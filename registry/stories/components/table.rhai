import "components/table" as table;

fn state_schema() {
    #{ fields: #{
        selected: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "string" } },
            "default": #{ type: "array", value: [#{ type: "string", value: "edge-01" }] } },
        sort_key: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "host" } },
        sort_direction: #{ schema: #{ type: "string", allowed: ["ascending", "descending"] },
            "default": #{ type: "string", value: "ascending" } },
    } }
}

fn selected(ctx, value) { ctx.set_state("selected", value); }
fn sorted(ctx, value) {
    if value != () {
        ctx.set_state("sort_key", value.key);
        ctx.set_state("sort_direction", value.direction);
    }
}

fn view(ctx) {
    table::Table(#{
        key: "story-hosts", label: "Hosts", row_key: "id", height: 180,
        rows: [
            #{ id: "edge-01", host: "edge-01", region: "Tokyo",
                state: "Healthy", state_variant: "success" },
            #{ id: "edge-02", host: "edge-02", region: "Berlin",
                state: "Deploying", state_variant: "accent" },
            #{ id: "edge-03", host: "edge-03", region: "Virginia",
                state: "Drift", state_variant: "warning" },
        ],
        columns: [
            #{ key: "host", title: "Host", width: #{ kind: "flex", value: 1 }, sortable: true,
                adornments: [#{ text_key: "state", variant_key: "state_variant", dot: true }] },
            #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 140 } },
        ],
        selection_mode: "multiple", selected_keys: ctx.get_state("selected"),
        sort: #{ key: ctx.get_state("sort_key"), direction: ctx.get_state("sort_direction") },
        on_selection_change: Fn("selected"), on_sort_change: Fn("sorted"),
    })
}
