/* gpui-rhai
{
  "id": "components/alert",
  "export": "Alert",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_raised", "success", "warning", "danger", "text_primary", "text_muted",
    "metrics.inset", "metrics.icon", "spacing.xxs", "spacing.sm", "space.related", "radius.lg",
    "typography.body"
  ],
  "environment": ["density", "corners"]
}
*/
// Persistent inline feedback with semantic variants and composable actions.
//
// Example: Alert(#{ title: "Saved", description: "Changes are live.", variant: "success" })

define_component(#{
    metadata: #{ id: "components/alert", "export": "Alert", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_raised", "success", "warning", "danger", "text_primary",
            "text_muted", "metrics.inset", "metrics.icon", "spacing.xxs", "spacing.sm",
            "space.related", "radius.lg", "typography.body"
        ],
        environment: ["density", "corners"] },
    schema: #{
        props: #{
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "The message in bold body text; also the alert's accessible name." },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Muted text under the title; it wraps with the alert's width." },
            icon: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Replaces the square lamp; sized to `metrics.icon` and tinted with the variant color." },
            content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Extra content under the description, such as a list or a link." },
            actions: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Controls on the end side, `related` apart." },
            variant: #{ schema: #{ type: "string", allowed: ["info", "success", "warning", "danger"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "info" },
                doc: "Lamp or icon color, `text_muted` for `info` and the status color otherwise; `warning` and `danger` announce as an alert, the others as a status." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            icon: #{ required: false, multiple: false, doc: "A glyph in place of the lamp." },
            content: #{ required: false, multiple: false, doc: "Extra content under the description." },
            actions: #{ required: false, multiple: true, doc: "Controls on the end side." },
        },
        parts: ["root", "icon", "lamp", "body", "title", "description", "content", "actions"],
    },
    render: Fn("render_Alert")
});

// `info` is not a status color (accent marks action and selection): its lamp is muted
// ink, like a neutral Badge.
fn alert_color(variant) {
    if variant == "success" || variant == "warning" || variant == "danger" { theme_color(variant) }
    else { theme_color("text_muted") }
}

fn alert_role(variant) {
    if variant == "danger" || variant == "warning" { "alert" } else { "status" }
}

fn Alert(props) { render_component("components/alert", props) }
fn render_Alert(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "info" };
    let body = [text(props.title).with_style(ctx.component_style("title", style()
        .typography("body").font_weight(600)
        .text_color(theme_color("text_primary"))))];
    if "description" in props && props.description != () {
        body.push(text(props.description).with_style(ctx.component_style("description", style()
            .typography("body").text_color(theme_color("text_muted")))));
    }
    if "content" in props && props.content != () {
        body.push(props.content.with_style(ctx.component_style("content", style())));
    }
    // A quiet raised block marked by a square status lamp centered on the title line.
    let mark = if "icon" in props && props.icon != () {
        props.icon.with_style(ctx.component_style("icon", style()
            .width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
            .text_color(alert_color(variant))))
    } else {
        row([]).with_style(ctx.component_style("lamp", style()
            .width(px(6)).height(px(6)).background(alert_color(variant))))
    };
    let children = [
        // A zero-width body line gives the mark column the title's line box.
        row([mark, text("\u200B")]).with_style(style().items_center().typography("body")),
        // min_width(0): the body shrinks with the alert, so long text wraps.
        column(body).with_style(ctx.component_style("body", style().gap(theme_spacing("xxs")).flex_grow()
            .min_width(px(0)))),
    ];
    if "actions" in props && props.actions.len > 0 {
        children.push(row(props.actions).with_style(ctx.component_style("actions", style()
            .gap(theme_length("space.related")).items_center())));
    }
    row(children).with_style(ctx.component_style("root", style()
        .padding(theme_length("metrics.inset")).gap(theme_spacing("sm")).items_start()
        .radius(theme_radius("lg")).background(theme_color("surface_raised"))))
        .accessibility_role(alert_role(variant))
        .accessibility_label(props.title)
}
