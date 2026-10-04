/* gpui-rhai
{
  "id": "components/input_group",
  "export": "InputGroup",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "border", "surface_raised", "focus_ring", "text_muted", "metrics.control",
    "metrics.control_pad", "radius.md", "typography.control"
  ],
  "environment": ["size", "density"]
}
*/
// Composes an input-like control with integrated prefix and suffix content.
// Props: control, optional prefix/suffix nodes, disabled, and accessible label.
// Events/state: stateless and emits no events; the control owns editing events.
// Parts: root, prefix, control, suffix.
// Example: InputGroup(#{ label: "Search", prefix: search_icon, control: input_node })

define_component(#{
    metadata: #{ id: "components/input_group", "export": "InputGroup", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "border", "surface_raised", "focus_ring", "text_muted", "metrics.control",
            "metrics.control_pad", "radius.md", "typography.control"
        ],
        environment: ["size", "density"] },
    schema: #{
        props: #{
            control: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            prefix: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            suffix: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false },
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
    // The group owns the field frame; the inner control drops its own frame and
    // the group frame takes the focus color while the control has focus.
    let children = [];
    if "prefix" in props && props.prefix != () {
        children.push(props.prefix.with_style(ctx.component_style("prefix", style()
            .padding_start(theme_length("metrics.control_pad")).items_center().justify_center()
            .text_color(theme_color("text_muted")))));
    }
    children.push(props.control.with_style(ctx.component_style("control", style()
        .flex_grow().min_width(px(0)).self_stretch().min_height(px(0))
        .border(px(0)).radius(px(0)).background(rgba(0x00000000)))));
    if "suffix" in props && props.suffix != () {
        children.push(props.suffix.with_style(ctx.component_style("suffix", style()
            .padding_end(theme_length("metrics.control_pad")).items_center().justify_center()
            .text_color(theme_color("text_muted")))));
    }
    let root = row(children).with_style(ctx.component_style("root", style()
        .min_height(theme_length("metrics.control")).items_center().clip().radius(theme_radius("md"))
        .border(px(2)).border_color(theme_color("border"))
        .background(theme_color("surface_raised"))
        .typography("control")
        .focus_within(style().border_color(theme_color("focus_ring")))
        .disabled(style().opacity(0.72))))
        .disabled("disabled" in props && props.disabled)
        .accessibility_role("group").accessibility_label(props.label);
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
