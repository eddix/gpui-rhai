/* gpui-rhai
{
  "id": "components/input_group",
  "export": "InputGroup",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Composes an input-like control with integrated prefix and suffix content.
// Props: control, optional prefix/suffix nodes, disabled, and accessible label.
// Events/state: stateless and emits no events; the control owns editing events.
// Parts: root, prefix, control, suffix.
// Example: InputGroup(#{ label: "Search", prefix: text("⌕"), control: input_node })

define_component(#{
    metadata: #{ id: "components/input_group", "export": "InputGroup", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            control: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            prefix: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            suffix: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ control: #{ required: true, multiple: false }, prefix: #{ required: false, multiple: false },
            suffix: #{ required: false, multiple: false } },
        parts: ["root", "prefix", "control", "suffix"],
    },
    render: Fn("render_InputGroup")
});

fn InputGroup(props) { render_component("components/input_group", props) }
fn render_InputGroup(ctx, props) {
    let children = [];
    if "prefix" in props && props.prefix != () {
        children.push(props.prefix.with_style(component_style(props, "prefix", style()
            .height(px(30)).padding_start(px(9)).items_center().justify_center()
            .text_color(theme_color("text_muted")))));
    }
    children.push(props.control.with_style(component_style(props, "control", style()
        .height(px(30)).flex_grow().border(px(0)).radius(px(0)).background(rgba(0x00000000)))));
    if "suffix" in props && props.suffix != () {
        children.push(props.suffix.with_style(component_style(props, "suffix", style()
            .height(px(30)).padding_end(px(9)).items_center().justify_center()
            .text_color(theme_color("text_muted")))));
    }
    row(children).with_style(component_style(props, "root", style()
        .height(px(32)).items_center().clip().radius(theme_radius("md"))
        .border(px(1)).border_color(theme_color("border"))
        .background(theme_color("surface"))
        .focus(style().border_color(theme_color("focus_ring")))
        .disabled(style().background(theme_color("surface_raised")).opacity(0.68))))
        .disabled("disabled" in props && props.disabled)
        .accessibility_role("group").accessibility_label(props.label)
}
