/* gpui-rhai
{
  "id": "components/checkbox",
  "export": "Checkbox",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled checkbox with checked, unchecked, and indeterminate states.
// Props: checked, indeterminate, label, disabled, on_change.
// Events: change({ checked, indeterminate }). Parts: root, box, mark, label.
// State: controlled by checked/indeterminate props.
// Example: Checkbox(#{ checked: true, label: "Remember", on_change: Fn("changed") })

export_component(#{
    metadata: #{
        id: "components/checkbox", "export": "Checkbox", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{},
    },
    schema: #{
        props: #{
            checked: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            indeterminate: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            on_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{
            change: #{
                payload: #{
                    type: "object", allow_unknown: false,
                    fields: #{
                        checked: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
                        indeterminate: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
                    },
                },
            },
        },
        slots: #{},
        parts: ["root", "box", "mark", "label"],
    },
});

fn Checkbox(props) {
    component_render("components/checkbox", props, Fn("render_Checkbox"))
}
fn render_Checkbox(ctx, props) {
    let mixed = if "indeterminate" in props { props.indeterminate } else { false };
    let disabled = if "disabled" in props { props.disabled } else { false };
    let active = props.checked || mixed;
    let mark = if mixed { "−" } else if props.checked { "✓" } else { "" };
    let box_background = if active { theme_color("accent") } else { theme_color("surface_raised") };
    let mark_node = text(mark).with_style(component_style(props, "mark",
        style().text_color(theme_color("on_accent"))
    ));
    let box_node = row([mark_node]).with_style(component_style(props, "box",
        style()
                .width(px(18)).height(px(18))
                .items_center().justify_center().radius(px(4))
                .border(px(1)).border_color(theme_color("border"))
                .background(box_background)
    ));
    let node = row([
        box_node,
        text(props.label).with_style(component_style(props, "label", style()))
    ])
        .with_style(component_style(props, "root",
            style().gap(px(8)).items_center()
                .text_color(if disabled { theme_color("disabled") } else { theme_color("text_primary") })
        ))
        .disabled(disabled)
        .accessibility_role("checkbox")
        .accessibility_label(props.label)
        .accessibility_checked(if mixed { "mixed" } else { props.checked });
    if !disabled && "on_change" in props && props.on_change != () {
        node.on_click_value(
            props.on_change,
            #{ checked: if mixed { true } else { !props.checked }, indeterminate: false }
        )
    } else { node }
}
