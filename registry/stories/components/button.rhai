import "components/button" as button;
import "components/badge" as badge;
import "components/tag" as tag;

fn state_schema() {
    #{ fields: #{
        last_action: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "No action yet" } },
    } }
}

fn activated(label, ctx, payload) { ctx.set_state("last_action", label); }

fn view(ctx) {
    column([
        text("Button, Badge, and Tag")
            .with_style(style().typography("heading")),
        text("Actions reserve a larger target; status and metadata remain compact.")
            .with_style(style().typography("body_small").text_color(theme_color("text_muted"))),
        row([
            button::Button(#{ text: "Deploy", variant: "primary",
                on_click: Fn("activated").curry("Deploy") }),
            badge::Badge(#{ text: "Deploy", variant: "accent" }),
            tag::Tag(#{ text: "Deploy", variant: "accent" }),
        ]).with_style(style().gap(theme_spacing("sm")).items_center()),
        text(ctx.get_state("last_action"))
            .accessibility_role("status").accessibility_label("Last action")
            .with_style(style().typography("body_small").text_color(theme_color("text_muted"))),
    ]).with_style(style().padding(theme_spacing("lg")).gap(theme_spacing("md")))
}
