/* gpui-rhai
{
  "id": "components/toggle",
  "export": "Toggle",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/button"],
  "capabilities": {}
}
*/
// Controlled pressed-state tool button, distinct from Checkbox and Switch.
// Props: text, pressed, optional prefix/suffix, size, disabled, and on_pressed_change.
// Events: pressed_change(bool).
// State: caller owns pressed state.
// Parts: root, label, prefix, suffix.
// Example: Toggle(#{ text: "Bold", pressed: true, on_pressed_change: Fn("changed") })

import "components/button" as button;

define_component(#{
    metadata: #{ id: "components/toggle", "export": "Toggle", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/button"], capabilities: #{} },
    schema: #{
        props: #{
            text: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            pressed: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            prefix: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            suffix: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            size: #{ schema: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "md" } },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_pressed_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ pressed_change: #{ payload: #{ type: "bool" } } },
        slots: #{ prefix: #{ required: false, multiple: false }, suffix: #{ required: false, multiple: false } },
        parts: ["root", "label", "prefix", "suffix"],
    },
    render: Fn("render_Toggle")
});

fn Toggle(props) { render_component("components/toggle", props) }
fn pressed_action(pressed, ctx, payload) { ctx.emit("pressed_change", pressed); }
fn render_Toggle(ctx, props) {
    let node = button::Button(#{
        text: props.text,
        prefix: if "prefix" in props { props.prefix } else { () },
        suffix: if "suffix" in props { props.suffix } else { () },
        variant: if props.pressed { "secondary" } else { "ghost" },
        size: if "size" in props { props.size } else { "md" },
        disabled: if "disabled" in props { props.disabled } else { false },
        on_click: if "on_pressed_change" in props && props.on_pressed_change != () {
            Fn("pressed_action").curry(!props.pressed)
        } else { () }
    }).with_style(component_style(props, "root", style()
        .background(if props.pressed { theme_color("selection") } else { rgba(0x00000000) })
        .border_color(if props.pressed { theme_color("accent") } else { rgba(0x00000000) })));
    node.accessibility_role("button").accessibility_label(props.text)
        .accessibility_pressed(props.pressed)
}
