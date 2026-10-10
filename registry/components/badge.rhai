/* gpui-rhai
{
  "id": "components/badge",
  "export": "Badge",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "text_primary", "text_muted", "surface", "accent", "on_accent", "success", "on_success",
    "warning", "on_warning", "danger", "on_danger", "metrics.marker", "metrics.marker_small",
    "spacing.xs", "spacing.sm", "radius.sm", "typography.caption"
  ],
  "environment": ["corners"]
}
*/
// Badge marks a read-only status: a square lamp followed by its label.
//
// Example: Badge(#{ text: "Ready", variant: "success" })
//
// `emphasis: "strong"` turns the badge into a solid status block; reserve it for
// states that require action. Markers keep their size in both densities.

define_component(#{
    metadata: #{
        id: "components/badge", "export": "Badge", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "text_primary", "text_muted", "surface", "accent", "on_accent", "success", "on_success",
            "warning", "on_warning", "danger", "on_danger", "metrics.marker", "metrics.marker_small",
            "spacing.xs", "spacing.sm", "radius.sm", "typography.caption"
        ],
        environment: ["corners"],
    },
    schema: #{
        props: #{
            text: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "The status label; also the accessible name." },
            variant: #{
                schema: #{ type: "string", allowed: ["neutral", "accent", "success", "warning", "danger"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "neutral" },
                doc: "Status color of the lamp, or of the block when `strong`; `neutral` is muted ink.",
            },
            emphasis: #{
                schema: #{ type: "string", allowed: ["subtle", "strong"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "subtle" },
                doc: "`strong` draws a solid block with a bold label and no lamp, for states that need action.",
            },
            dot: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Shows the 6px square lamp before the label; a `strong` badge never shows it." },
            size: #{ schema: #{ type: "string", allowed: ["sm", "md"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "md" },
                doc: "Badge height: `md` is `metrics.marker`, `sm` is `metrics.marker_small`." },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{},
        parts: ["root", "lamp", "label"],
    },
    render: Fn("render_Badge")
});

fn badge_color(variant) {
    if variant == "neutral" { theme_color("text_muted") } else { theme_color(variant) }
}

fn badge_on_color(variant) {
    if variant == "neutral" { theme_color("surface") } else { theme_color(`on_${variant}`) }
}

fn Badge(props) { render_component("components/badge", props) }
fn render_Badge(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "neutral" };
    let strong = "emphasis" in props && props.emphasis == "strong";
    let dot = if "dot" in props { props.dot } else { true };
    let height = if "size" in props && props.size == "sm" {
        theme_length("metrics.marker_small")
    } else {
        theme_length("metrics.marker")
    };
    let root = style()
        .flex_row()
        .items_center()
        .min_height(height)
        .gap(theme_spacing("xs"))
        .typography("caption");
    if strong {
        // A neutral strong badge inverts ink and paper.
        let fill = if variant == "neutral" { theme_color("text_primary") } else { badge_color(variant) };
        root = root
            .padding_x(theme_spacing("sm"))
            .radius(theme_radius("sm"))
            .background(fill)
            .text_color(badge_on_color(variant))
            .font_weight(600);
    } else {
        root = root.text_color(theme_color("text_primary"));
    }
    let children = [];
    if dot && !strong {
        // The 6px square lamp is a structural constant.
        children.push(row([]).with_style(ctx.component_style("lamp", style()
            .width(px(6)).height(px(6)).background(badge_color(variant)))));
    }
    children.push(text(props.text).with_style(ctx.component_style("label", style())));
    row(children)
        .with_style(ctx.component_style("root", root))
        .accessibility_role("status")
        .accessibility_label(props.text)
}
