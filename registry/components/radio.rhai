/* gpui-rhai
{
  "id": "components/radio",
  "export": "Radio",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled radio option. RadioGroup owns roving keyboard behavior.
// Props: value, selected, label, disabled, on_select.
// Events: select(string). Parts: root, indicator, label.
// State: controlled by the selected prop.
// Example: Radio(#{ value: "team", selected: true, label: "Team", on_select: Fn("selected") })

define_component(#{
    metadata: #{
        id: "components/radio", "export": "Radio", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{},
    },
    schema: #{
        props: #{
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            selected: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            on_select: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{ select: #{ payload: #{ type: "string" } } },
        slots: #{}, parts: ["root", "indicator", "label"],
    },
    render: Fn("render_Radio")
});

fn Radio(props) { render_component("components/radio", props) }
fn render_Radio(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let dot = row([]).with_style(style()
        .width(px(6)).height(px(6)).radius(px(3))
        .background(if props.selected { theme_color("accent") } else { rgba(0x00000000) }));
    let indicator = row([dot]).with_style(ctx.component_style("indicator",
        style().width(px(16)).height(px(16)).radius(px(8))
            .items_center().justify_center().border(px(1))
            .border_color(if props.selected { theme_color("accent") }
                else { theme_color("border") }).background(theme_color("surface"))));
    let node = row([indicator, text(props.label).with_style(ctx.component_style("label", style()))])
        .with_key(props.value)
        .with_style(ctx.component_style("root",
            style().gap(px(8)).items_center().self_start().padding(px(3))
                .radius(theme_radius("sm"))
                .border(px(1)).border_color(rgba(0x00000000))
                .typography("body")
                .text_color(if disabled { theme_color("disabled") } else { theme_color("text_primary") })
                .hover(style().background(theme_color("surface_hover")))
                .focus(style().border_color(theme_color("focus_ring")))
                .active(style().opacity(0.82))
                .disabled(style().opacity(0.72))
        ))
        .disabled(disabled)
        .accessibility_role("radio")
        .accessibility_label(props.label)
        .accessibility_checked(props.selected)
        .accessibility_value(props.value);
    if !disabled && "on_select" in props && props.on_select != () {
        node.on_click_value(props.on_select, props.value)
    } else { node }
}
