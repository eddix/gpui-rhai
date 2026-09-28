fn view(ctx) {
    column([
        text("HOST EMBEDDING").with_style(style().typography("heading")),
        text("The outer Rhai view declares an opaque boundary; Rust supplies an independently mounted resident Rhai form.")
            .with_style(style().typography("body_small").text_color(theme_color("text_muted"))),
        gpui_rhai::HostSlot(#{ key: "resident-form-slot", name: "resident-form" })
            .with_style(style().width(relative(1.0)).height(px(220))
                .border(px(1)).border_color(theme_color("border")))
    ]).with_style(style().width(relative(1.0)).height(relative(1.0))
        .padding(theme_spacing("lg")).gap(theme_spacing("md")))
}
