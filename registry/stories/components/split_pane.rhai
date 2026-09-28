import "components/button" as button;
import "components/split_pane" as split_pane;

fn state_schema() { #{ fields: #{
    size: #{ schema: #{ type: "number", min: 0.0, max: 1.0 },
        "default": #{ type: "float", value: 0.34 } },
    orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] },
        "default": #{ type: "string", value: "horizontal" } },
    source_collapsed: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
} } }

fn resized(ctx, value) { ctx.set_state("size", value); }
fn toggle_orientation(ctx, payload) {
    ctx.set_state("orientation", if ctx.get_state("orientation") == "horizontal" {
        "vertical"
    } else { "horizontal" });
}
fn toggle_source(ctx, payload) {
    ctx.set_state("source_collapsed", !ctx.get_state("source_collapsed"));
}

fn panel(title, body) {
    column([
        text(title).with_style(style().typography("title")),
        text(body).with_style(style().typography("body").text_color(theme_color("text_muted"))),
    ]).with_style(style().width(relative(1.0)).height(relative(1.0))
        .padding(theme_spacing("md")).gap(theme_spacing("sm"))
        .border(px(1)).border_color(theme_color("border")))
}

fn view(ctx) {
    let collapsed = ctx.get_state("source_collapsed");
    column([
        row([
            button::Button(#{ text: "Switch orientation", variant: "outline",
                on_click: Fn("toggle_orientation") }),
            button::Button(#{ text: if collapsed { "Show source" } else { "Hide source" },
                variant: "outline", on_click: Fn("toggle_source") }),
            text(`Start ratio: ${(ctx.get_state("size") * 100.0).round}%`)
                .accessibility_role("status"),
        ]).with_style(style().gap(theme_spacing("sm")).items_center()),
        split_pane::SplitPane(#{ key: "story-split", label: "Resize navigation and preview",
            orientation: ctx.get_state("orientation"), size: ctx.get_state("size"),
            min_start: 120.0, min_end: 180.0, end_collapsed: collapsed,
            start_key: "navigation", end_key: "preview",
            start: panel("Navigation", "The start panel keeps a stable key and controlled ratio."),
            end: panel("Preview", "Drag the separator or focus it and use the arrow keys."),
            on_resize: Fn("resized")
        }).with_style(style().flex_grow().min_height(px(0))),
    ]).with_style(style().width(relative(1.0)).height(relative(1.0))
        .padding(theme_spacing("md")).gap(theme_spacing("md")))
}
