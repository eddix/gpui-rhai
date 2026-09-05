/* gpui-rhai
{
  "id": "components/kbd",
  "export": "Kbd",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Keyboard shortcut or key-cap presentation.
// Props: text and optional accessible label.
// Events/state: stateless and emits no events.
// Parts: root, label.
// Example: Kbd(#{ text: "⌘ K", label: "Command K" })

define_component(#{
    metadata: #{ id: "components/kbd", "export": "Kbd", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            text: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root", "label"],
    },
    render: Fn("render_Kbd")
});

fn Kbd(props) { render_component("components/kbd", props) }
fn render_Kbd(ctx, props) {
    text(props.text).with_style(component_style(props, "root", component_style(props, "label", style()
        .min_width(px(20)).height(px(20)).padding_x(px(5)).items_center().justify_center()
        .radius(theme_radius("sm")).border(px(1)).border_color(theme_color("border"))
        .background(theme_color("surface_raised")).text_color(theme_color("text_muted"))
        .font_size(px(10)).line_height(px(14)).font_weight(700))))
        .accessibility_role("text").accessibility_label(
            if "label" in props && props.label != () { props.label } else { props.text })
}
