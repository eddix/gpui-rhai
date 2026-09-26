import "components/tabs" as tabs;

fn state_schema() {
    #{ fields: #{
        selected: #{ schema: #{ type: "string", allowed: ["status", "events"] },
            "default": #{ type: "string", value: "status" } },
    } }
}

fn selected(ctx, value) { ctx.set_state("selected", value); }

fn view(ctx) {
    tabs::Tabs(#{
        label: "Host details",
        value: ctx.get_state("selected"),
        layout: "equal",
        motion_key: "story-tabs",
        tabs: [
            #{ value: "status", label: "Status",
                content: text("Healthy · 42 days uptime") },
            #{ value: "events", label: "Events",
                content: column([text("Deploy completed"), text("Configuration verified")]) },
            #{ value: "locked", label: "Locked", content: text(""), disabled: true },
        ],
        on_change: Fn("selected"),
    }).with_style(style().width(relative(1.0)))
}
