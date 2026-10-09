/* gpui-rhai
{
  "id": "components/checkbox",
  "export": "Checkbox",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/check.svg", "icons/minus.svg"],
  "tokens": [
    "accent", "on_accent", "border", "surface_raised", "surface_hover", "focus_ring",
    "text_primary", "disabled", "metrics.mark", "metrics.control", "spacing.sm", "radius.xs",
    "typography.control"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// Controlled checkbox with checked, unchecked, and indeterminate states.
// State: controlled by checked/indeterminate props.
// Example: Checkbox(#{ checked: true, label: "Remember", on_change: Fn("changed") })

define_component(#{
    metadata: #{
        id: "components/checkbox", "export": "Checkbox", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "accent", "on_accent", "border", "surface_raised", "surface_hover", "focus_ring",
            "text_primary", "disabled", "metrics.mark", "metrics.control", "spacing.sm",
            "radius.xs", "typography.control"
        ],
        environment: ["size", "density", "corners"], assets: ["icons/check.svg", "icons/minus.svg"],
    },
    schema: #{
        props: #{
            checked: #{
                schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether the box is checked; the caller stores `checked` from the `change` payload and passes it back.",
            },
            indeterminate: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Shows the minus mark and a mixed accessible state over `checked`; the caller stores it from the `change` payload.",
            },
            label: #{
                schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Text beside the box; also the accessible name.",
            },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Blocks clicks and shows the disabled style.",
            },
            on_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
                doc: "Called with the next `#{ checked, indeterminate }` state when the box is clicked.",
            },
        },
        state: #{ fields: #{} },
        events: #{
            change: #{
                payload: #{
                    type: "object", allow_unknown: false,
                    fields: #{
                        checked: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                            doc: "The next checked value: true when the box was unchecked or mixed." },
                        indeterminate: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                            doc: "Always false, since a click resolves the mixed state." },
                    },
                },
                doc: "Emitted when the box is clicked; the payload is its next state, and a mixed box becomes checked.",
            },
        },
        slots: #{},
        parts: ["root", "box", "mark", "label"],
    },
    render: Fn("render_Checkbox")
});

fn Checkbox(props) { render_component("components/checkbox", props) }
fn render_Checkbox(ctx, props) {
    let mixed = if "indeterminate" in props { props.indeterminate } else { false };
    let disabled = if "disabled" in props { props.disabled } else { false };
    let active = props.checked || mixed;
    // The box is a small field: a 2px frame on the well, filled with accent when
    // checked. Focus turns the frame ink, so the box stays on the content edge.
    let mark_node = image_source(asset(if mixed { "app/icons/minus" } else { "app/icons/check" }))
        .with_style(ctx.component_style("mark", style()
            .width(theme_length("metrics.mark") * 0.75).height(theme_length("metrics.mark") * 0.75)
            .text_color(theme_color("on_accent"))
            .opacity(if active { 1.0 } else { 0.0 })));
    let box_node = row([mark_node]).with_style(ctx.component_style("box", style()
        .width(theme_length("metrics.mark")).height(theme_length("metrics.mark"))
        .items_center().justify_center().radius(theme_radius("xs"))
        .border(px(2)).border_color(if active { theme_color("accent") } else { theme_color("border") })
        .background(if active { theme_color("accent") } else { theme_color("surface_raised") })
        .group_focus(style().border_color(theme_color("focus_ring")))
        .disabled(style().border_color(theme_color("border")).background(theme_color("surface_hover")))));
    let node = row([
        box_node,
        text(props.label).with_style(ctx.component_style("label", style()))
    ])
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
