/* gpui-rhai
{
  "id": "patterns/stat",
  "export": "Stat",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "text_primary", "text_muted", "text.accent", "text.success", "text.warning", "text.danger",
    "spacing.xxs", "spacing.xs", "space.section", "typography.display", "typography.body",
    "typography.caption", "typography.label"
  ],
  "environment": []
}
*/
// Stat shows one figure: a label-voice label, the value with its unit, and an optional
// change against a reference.
//
// Props: label (required), value (required), unit, delta, tone, description.
// Events/state: none. Parts: root, label, value, unit, delta, description.
// Example: Stat(#{ label: "Hosts", value: "128", delta: "+4 this week", tone: "success" })
//
// Figures use tabular digits; the unit follows the number in the muted color. `tone` colors
// only the delta and only when it deviates ("normal is quiet"). Use `stats(items)` for a
// row of figures.

define_component(#{
    metadata: #{
        id: "patterns/stat", "export": "Stat", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "text_primary", "text_muted", "text.accent", "text.success", "text.warning",
            "text.danger", "spacing.xxs", "spacing.xs", "space.section", "typography.display",
            "typography.body", "typography.caption", "typography.label"
        ],
        environment: [],
    },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            unit: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            delta: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            tone: #{ schema: #{ type: "string", allowed: ["neutral", "accent", "success", "warning", "danger"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "neutral" } },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{},
        parts: ["root", "label", "value", "unit", "delta", "description"],
    },
    render: Fn("render_Stat")
});

fn Stat(props) { render_component("patterns/stat", props) }

// A row of Stat figures, `section` apart.
fn stats(items) {
    let nodes = [];
    for item in items { nodes.push(Stat(item)); }
    row(nodes).with_style(style().flex_row().flex_wrap().items_start().gap(theme_length("space.section")))
}

fn render_Stat(ctx, props) {
    let tone = if "tone" in props { props.tone } else { "neutral" };
    let figure = [text(props.value).with_style(ctx.component_style("value", style()
        .typography("display").font_feature("tnum", 1).text_color(theme_color("text_primary"))))];
    if "unit" in props && props.unit != () {
        figure.push(text(props.unit).with_style(ctx.component_style("unit", style()
            .typography("body").text_color(theme_color("text_muted")))));
    }
    let children = [
        text(props.label.to_upper()).with_style(ctx.component_style("label", style()
            .typography("label").text_color(theme_color("text_muted")))),
        // A figure and its unit are one datum set in two sizes on purpose.
        row(figure).with_style(style().flex_row().items_end().gap(theme_spacing("xs")))
            .audit_allow(["mixed-type-in-row"]),
    ];
    if "delta" in props && props.delta != () {
        children.push(text(props.delta).with_style(ctx.component_style("delta", style()
            .typography("caption").font_feature("tnum", 1)
            .text_color(if tone == "neutral" { theme_color("text_muted") } else { theme_color(`text.${tone}`) }))));
    }
    if "description" in props && props.description != () {
        children.push(text(props.description).with_style(ctx.component_style("description", style()
            .typography("caption").text_color(theme_color("text_muted")))));
    }
    let name = if "unit" in props && props.unit != () { `${props.label}: ${props.value} ${props.unit}` }
        else { `${props.label}: ${props.value}` };
    column(children).with_style(ctx.component_style("root", style().flex_col().gap(theme_spacing("xxs"))))
        .accessibility_role("group").accessibility_label(name)
}
