/* gpui-rhai
{
  "id": "components/button_group",
  "export": "ButtonGroup",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["spacing.xxs", "radius.md", "radius.lg"],
  "environment": ["size", "density", "corners"]
}
*/
// Visually joins related buttons into one horizontal or vertical control.
// Example: ButtonGroup(#{ label: "View", buttons: [grid_button, list_button] })

define_component(#{
    metadata: #{ id: "components/button_group", "export": "ButtonGroup", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["spacing.xxs", "radius.md", "radius.lg"], environment: ["size", "density", "corners"] },
    schema: #{
        props: #{
            buttons: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: true, sensitive: false,
                doc: "Buttons to join, in order, at least one; each keeps its own action and only the group's outer corners round." },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "horizontal" },
                doc: "`horizontal` joins the buttons in a row, `vertical` in a column." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the group; it is not drawn." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false,
                doc: "Control size for the buttons inside; unset, they inherit the `size` environment." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ buttons: #{ required: true, multiple: true, doc: "The joined buttons." } },
        parts: ["root", "item", "first", "last"],
    },
    render: Fn("render_ButtonGroup")
});

fn ButtonGroup(props) { render_component("components/button_group", props) }
fn render_ButtonGroup(ctx, props) {
    if props.buttons.len == 0 { throw "button group requires at least one button"; }
    let vertical = "orientation" in props && props.orientation == "vertical";
    // Only the group's outer corners round: a horizontal group is a capsule with round
    // corners, a vertical one rounds like a panel.
    let corner = theme_radius(if vertical { "lg" } else { "md" });
    let items = [];
    for index in 0..props.buttons.len {
        let first = index == 0;
        let last = index == props.buttons.len - 1;
        let joined = style().radius(px(0));
        if vertical {
            if first { joined = joined.radius_top_left(corner).radius_top_right(corner); }
            if last { joined = joined.radius_bottom_left(corner).radius_bottom_right(corner); }
        } else {
            if first { joined = joined.radius_start(corner); }
            if last { joined = joined.radius_end(corner); }
        }
        // Every button is an `item`; a lone button is both `first` and `last`.
        let resolved = ctx.component_style("item", joined);
        if first { resolved = ctx.component_style("first", resolved); }
        if last { resolved = ctx.component_style("last", resolved); }
        items.push(props.buttons[index].with_style(resolved));
    }
    // Joined blocks keep a 2px seam, so each button keeps its own focus border and
    // outline frames never double up.
    let root = if vertical { column(items) } else { row(items) };
    root = root.with_style(ctx.component_style("root", style()
        .gap(theme_spacing("xxs")).clip().radius(corner)))
        .accessibility_role("group").accessibility_label(props.label);
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
