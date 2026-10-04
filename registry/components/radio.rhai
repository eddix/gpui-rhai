/* gpui-rhai
{
  "id": "components/radio",
  "export": "Radio",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "accent", "border", "surface_raised", "surface_hover", "focus_ring", "text_primary",
    "disabled", "metrics.mark", "metrics.control", "spacing.sm", "typography.control"
  ],
  "environment": ["size", "density"]
}
*/
// Controlled radio option. RadioGroup owns roving keyboard behavior.
// Props: value, selected, label, disabled, on_select.
// Events: select(string). Parts: root, indicator, label.
// State: controlled by the selected prop.
// Example: Radio(#{ value: "team", selected: true, label: "Team", on_select: Fn("selected") })

define_component(#{
    metadata: #{
        id: "components/radio", "export": "Radio", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "accent", "border", "surface_raised", "surface_hover", "focus_ring", "text_primary",
            "disabled", "metrics.mark", "metrics.control", "spacing.sm", "typography.control"
        ],
        environment: ["size", "density"],
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
            focus_mark: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
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
    let focus_mark = if "focus_mark" in props { props.focus_mark } else { true };
    // The ring is a semantic circle; the 6px dot is a structural constant.
    let dot = row([]).with_style(style()
        .width(px(6)).height(px(6)).radius(px(3))
        .background(if props.selected { theme_color("accent") } else { rgba(0x00000000) })
        .disabled(style().background(if props.selected { theme_color("disabled") } else { rgba(0x00000000) })));
    let indicator = row([dot]).with_style(ctx.component_style("indicator", style()
        .width(theme_length("metrics.mark")).height(theme_length("metrics.mark"))
        .radius(theme_length("metrics.mark") * 0.5)
        .items_center().justify_center().border(px(2))
        .border_color(if props.selected { theme_color("accent") } else { theme_color("border") })
        .background(theme_color("surface_raised"))
        .group_focus(if focus_mark { style().border_color(theme_color("focus_ring")) } else { style() })
        .disabled(style().border_color(theme_color("border")).background(theme_color("surface_hover")))));
    let node = row([indicator, text(props.label).with_style(ctx.component_style("label", style()))])
        .with_key(props.value)
        .with_style(ctx.component_style("root",
            style().min_height(theme_length("metrics.control")).gap(theme_spacing("sm"))
                .items_center().self_start()
                .typography("control")
                .text_color(theme_color("text_primary"))
                // Owns the focus state shown by the mark (group_focus).
                .focus(style())
                .disabled(style().text_color(theme_color("disabled")))
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
