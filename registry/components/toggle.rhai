/* gpui-rhai
{
  "id": "components/toggle",
  "export": "Toggle",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/button"],
  "capabilities": {},
  "tokens": ["selection", "text_primary"],
  "environment": ["size", "density"]
}
*/
// Controlled pressed-state tool button, distinct from Checkbox and Switch.
// State: caller owns pressed state.
// Example: Toggle(#{ text: "Bold", pressed: true, on_pressed_change: Fn("changed") })

import "components/button" as button;

define_component(#{
    metadata: #{ id: "components/toggle", "export": "Toggle", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/button"], capabilities: #{},
        tokens: ["selection", "text_primary"], environment: ["size", "density"] },
    schema: #{
        props: #{
            text: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Label text; also the accessible name." },
            pressed: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether the toggle is on; the caller stores the `pressed_change` payload and passes it back." },
            prefix: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Node before the label, such as an icon." },
            suffix: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Node after the label." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false, doc: "Control size; leave unset to inherit the environment size." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false }, doc: "Blocks clicks and shows the disabled style." },
            on_pressed_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
                doc: "Called with the new pressed value, the opposite of `pressed`, when the toggle is clicked." },
        },
        state: #{ fields: #{} }, events: #{ pressed_change: #{ payload: #{ type: "bool" },
            doc: "Emitted when the toggle is clicked; the payload is the new pressed value." } },
        slots: #{
            prefix: #{ required: false, multiple: false, doc: "Node before the label, such as an icon." },
            suffix: #{ required: false, multiple: false, doc: "Node after the label." },
        },
        parts: ["root", "label", "prefix", "suffix"],
    },
    render: Fn("render_Toggle")
});

fn Toggle(props) { render_component("components/toggle", props) }
fn pressed_action(pressed, ctx, payload) { ctx.emit("pressed_change", pressed); }
fn render_Toggle(ctx, props) {
    // A pressed Toggle fills its block with selection; weight stays 400 in both
    // states so the label does not change width.
    let fill = if props.pressed { theme_color("selection") } else { rgba(0x00000000) };
    let node = button::Button(#{
        text: props.text,
        prefix: if "prefix" in props { props.prefix } else { () },
        suffix: if "suffix" in props { props.suffix } else { () },
        variant: "ghost",
        size: if "size" in props { props.size } else { () },
        disabled: if "disabled" in props { props.disabled } else { false },
        hoverable: !props.pressed,
        on_click: if "on_pressed_change" in props && props.on_pressed_change != () {
            Fn("pressed_action").curry(!props.pressed)
        } else { () }
    }).with_style(ctx.component_style("root", style()
        .background(fill)
        .border_color(fill)
        .text_color(theme_color("text_primary"))
        .font_weight(400)));
    node.accessibility_role("button").accessibility_label(props.text)
        .accessibility_pressed(props.pressed)
}
