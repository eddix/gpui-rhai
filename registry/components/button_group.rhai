/* gpui-rhai
{
  "id": "components/button_group",
  "export": "ButtonGroup",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["spacing.xxs", "radius.md"],
  "environment": ["size", "density"]
}
*/
// Visually joins related buttons into one horizontal or vertical control.
// Props: buttons, orientation, size, and accessible label.
// Events/state: stateless and emits no events; child buttons own actions.
// Parts: root, item, first, last.
// Example: ButtonGroup(#{ label: "View", buttons: [grid_button, list_button] })

define_component(#{
    metadata: #{ id: "components/button_group", "export": "ButtonGroup", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["spacing.xxs", "radius.md"], environment: ["size", "density"] },
    schema: #{
        props: #{
            buttons: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: true, sensitive: false },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "horizontal" } },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ buttons: #{ required: true, multiple: true } },
        parts: ["root", "item", "first", "last"],
    },
    render: Fn("render_ButtonGroup")
});

fn ButtonGroup(props) { render_component("components/button_group", props) }
fn render_ButtonGroup(ctx, props) {
    if props.buttons.len == 0 { throw "button group requires at least one button"; }
    let vertical = "orientation" in props && props.orientation == "vertical";
    let items = [];
    for index in 0..props.buttons.len {
        let first = index == 0;
        let last = index == props.buttons.len - 1;
        let part = if first { "first" } else if last { "last" } else { "item" };
        items.push(props.buttons[index].with_style(ctx.component_style(part,
            ctx.component_style("item", style().radius(px(0))))));
    }
    // Joined blocks keep a 2px seam, so each button keeps its own focus border and
    // outline frames never double up.
    let root = if vertical { column(items) } else { row(items) };
    root = root.with_style(ctx.component_style("root", style()
        .gap(theme_spacing("xxs")).clip().radius(theme_radius("md"))))
        .accessibility_role("group").accessibility_label(props.label);
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
