import "components/input" as input;
import "components/textarea" as textarea;

fn state_schema() {
    #{ fields: #{
        name: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        notes: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
    } }
}

fn name_changed(ctx, value) { ctx.set_state("name", value); }
fn notes_changed(ctx, value) { ctx.set_state("notes", value); }

fn view(ctx) {
    column([
        text("Resident script view").with_style(style().typography("title")),
        input::Input(#{ key: "resident-name", label: "Resident name",
            value: ctx.get_state("name"), placeholder: "Type with an IME",
            autofocus: true, on_change: Fn("name_changed") }),
        textarea::Textarea(#{ key: "resident-notes", label: "Resident notes",
            value: ctx.get_state("notes"), rows: 2, on_change: Fn("notes_changed") }),
        text(`resident:${ctx.get_state("name")}`).accessibility_role("status")
            .accessibility_label(`resident:${ctx.get_state("name")}`)
    ]).with_style(style().width(relative(1.0)).height(relative(1.0))
        .padding(theme_spacing("md")).gap(theme_spacing("sm"))
        .background(theme_color("surface_raised")))
}
