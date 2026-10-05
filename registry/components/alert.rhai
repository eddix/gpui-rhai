/* gpui-rhai
{
  "id": "components/alert",
  "export": "Alert",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_raised", "accent", "success", "warning", "danger", "text_primary", "text_muted",
    "metrics.inset", "metrics.icon", "spacing.xxs", "spacing.sm", "space.related", "radius.lg",
    "typography.body"
  ],
  "environment": ["density", "corners"]
}
*/
// Persistent inline feedback with semantic variants and composable actions.
// Props: title, optional description/icon/content/actions, and variant.
// Events/state: stateless and emits no events.
// Parts: root, icon, lamp, body, title, description, content, actions.
// Example: Alert(#{ title: "Saved", description: "Changes are live.", variant: "success" })

define_component(#{
    metadata: #{ id: "components/alert", "export": "Alert", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_raised", "accent", "success", "warning", "danger", "text_primary",
            "text_muted", "metrics.inset", "metrics.icon", "spacing.xxs", "spacing.sm",
            "space.related", "radius.lg", "typography.body"
        ],
        environment: ["density", "corners"] },
    schema: #{
        props: #{
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            icon: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            actions: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            variant: #{ schema: #{ type: "string", allowed: ["info", "success", "warning", "danger"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "info" } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ icon: #{ required: false, multiple: false }, content: #{ required: false, multiple: false },
            actions: #{ required: false, multiple: true } },
        parts: ["root", "icon", "lamp", "body", "title", "description", "content", "actions"],
    },
    render: Fn("render_Alert")
});

fn alert_color(variant) {
    if variant == "success" || variant == "warning" || variant == "danger" { theme_color(variant) }
    else { theme_color("accent") }
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
        column(body).with_style(ctx.component_style("body", style().gap(theme_spacing("xxs")).flex_grow())),
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
