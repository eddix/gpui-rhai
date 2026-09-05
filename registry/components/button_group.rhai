/* gpui-rhai
{
  "id": "components/button_group",
  "export": "ButtonGroup",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Visually joins related buttons into one horizontal or vertical control.
// Props: buttons, orientation, and accessible label.
// Events/state: stateless and emits no events; child buttons own actions.
// Parts: root, item, first, last.
// Example: ButtonGroup(#{ label: "View", buttons: [grid_button, list_button] })

define_component(#{
    metadata: #{ id: "components/button_group", "export": "ButtonGroup", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            buttons: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: true, sensitive: false },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "horizontal" } },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
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
        let item_style = style().radius(px(0));
        if vertical {
            if !first { item_style = item_style.margin_top(offset_px(-1)); }
        } else {
            if !first { item_style = item_style.margin_start(offset_px(-1)); }
        }
        let part = if first { "first" } else if last { "last" } else { "item" };
        items.push(props.buttons[index].with_style(component_style(props, part,
            component_style(props, "item", item_style))));
    }
    let root = if vertical { column(items) } else { row(items) };
    root.with_style(component_style(props, "root", style()
        .clip().radius(theme_radius("md"))))
        .accessibility_role("group").accessibility_label(props.label)
}
