/* gpui-rhai
{
  "id": "components/status_bar",
  "export": "StatusBar",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_raised", "border", "text_muted", "metrics.statusbar", "metrics.inset",
    "spacing.md", "typography.caption"
  ],
  "environment": ["density"]
}
*/
// Compact application footer with stable logical start, center, and end regions.
// Example: StatusBar(#{ label: "Editor status", start: [text("Ready")], end: [text("UTF-8")] })

define_component(#{
    metadata: #{ id: "components/status_bar", "export": "StatusBar", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_raised", "border", "text_muted", "metrics.statusbar", "metrics.inset",
            "spacing.md", "typography.caption"
        ],
        environment: ["density"] },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the bar, which is announced as a status bar." },
            start: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Fields at the start edge; they truncate when space runs out." },
            center: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Fields centered in the bar." },
            end: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Fields at the end edge; they keep their width." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ start: #{ required: false, multiple: true, doc: "Fields at the start edge." },
            center: #{ required: false, multiple: true, doc: "Fields centered in the bar." },
            end: #{ required: false, multiple: true, doc: "Fields at the end edge." } },
        parts: ["root", "start", "center", "end"],
    },
    render: Fn("render_StatusBar")
});

fn StatusBar(props) { render_component("components/status_bar", props) }
// Symmetric side tracks protect a real center. Without one, the end only
// reserves its intrinsic width and the start gets the remaining space.
fn status_region(ctx, part, children, centered) {
    let base = style().height(relative(1.0)).items_center().gap(theme_spacing("md"))
        .whitespace_nowrap();
    base = if part == "center" { base.justify_center().flex_shrink(false) }
        else if part == "end" {
            if centered { base.justify_end().flex_grow().flex_basis(relative(0)) }
            else { base.justify_end().flex_shrink(false) }
        }
        else { base.flex_grow().flex_basis(relative(0)).min_width(px(0)).overflow_hidden().text_ellipsis() };
    let styled = ctx.component_style(part, base);
    // Keep the slot's retained position without charging two inter-slot
    // gaps for an empty center.
    row(children).with_style(if part == "center" && !centered { styled.hidden() } else { styled })
}
fn render_StatusBar(ctx, props) {
    let center = if "center" in props { props.center } else { [] };
    let centered = center.len > 0;
    row([
        status_region(ctx, "start", if "start" in props { props.start } else { [] }, centered),
        status_region(ctx, "center", center, centered),
        status_region(ctx, "end", if "end" in props { props.end } else { [] }, centered),
    ]).with_style(ctx.component_style("root", style()
        .width(relative(1.0)).height(theme_length("metrics.statusbar")).flex_shrink(false)
        .padding_x(theme_length("metrics.inset")).items_center()
        .border_top(px(1)).border_color(theme_color("border"))
        .background(theme_color("surface_raised"))
        .typography("caption").text_color(theme_color("text_muted"))))
        .accessibility_role("statusbar").accessibility_label(props.label)
}
