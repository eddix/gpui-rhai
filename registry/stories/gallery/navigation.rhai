import "components/input" as input;

fn state_schema() {
    #{ fields: #{
        query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
    } }
}

fn view(ctx) {
    column([
        text("EXPLORE").with_style(style().typography("body_small").font_weight(700)),
        input::Input(#{ key: "gallery-search", label: "Search stories",
            value: ctx.get_state("query"), placeholder: "Search stories",
            on_change: native_handler("gallery.search") })
    ]).with_style(style().width(relative(1.0)).padding(theme_spacing("sm"))
        .gap(theme_spacing("xs")))
}
