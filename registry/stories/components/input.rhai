import "components/input" as input;
import "components/textarea" as textarea;

fn state_schema() {
    #{ fields: #{
        host: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "edge-01" } },
        note: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "Ready for deployment" } },
    } }
}

fn host_changed(ctx, value) { ctx.set_state("host", value); }
fn note_changed(ctx, value) { ctx.set_state("note", value); }

fn view(ctx) {
    column([
        input::Input(#{ key: "story-host", label: "Host name",
            value: ctx.get_state("host"), on_change: Fn("host_changed") }),
        textarea::Textarea(#{ key: "story-note", label: "Deployment note", rows: 3,
            value: ctx.get_state("note"), on_change: Fn("note_changed") }),
        text(`${ctx.get_state("host")} · ${ctx.get_state("note")}`)
            .accessibility_role("status").accessibility_label("Form summary"),
    ]).with_style(style().padding(theme_spacing("lg")).gap(theme_spacing("md")))
}
