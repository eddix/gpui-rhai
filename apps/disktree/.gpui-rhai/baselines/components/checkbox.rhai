/* gpui-rhai
{
  "id": "components/checkbox",
  "export": "Checkbox",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["checkbox.stroke_width","checkbox.draw_ms","checkbox.draw_delay_ms","checkbox.mixed_draw_ms","accent","border","choice.checked","choice.fill","choice.on_fill","choice.unchecked","control.border","disabled","focus_ring","metrics.choice_line","metrics.control","metrics.mark","on_accent","radius.xs","spacing.sm","surface.field","surface_hover","surface_raised","text_primary","typography.control"],
  "environment": ["size", "density"]
}
*/
// Controlled checkbox with checked, unchecked, and indeterminate states.
// State: controlled by checked/indeterminate props.
// Example: Checkbox(#{ checked: true, label: "Remember", on_change: Fn("changed") })

define_component(#{
    metadata: #{
        id: "components/checkbox", "export": "Checkbox", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: ["checkbox.stroke_width","checkbox.draw_ms","checkbox.draw_delay_ms","checkbox.mixed_draw_ms","accent","border","choice.checked","choice.fill","choice.on_fill","choice.unchecked","control.border","disabled","focus_ring","metrics.choice_line","metrics.control","metrics.mark","on_accent","radius.xs","spacing.sm","surface.field","surface_hover","surface_raised","text_primary","typography.control"],
        environment: ["size", "density"],
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
        parts: ["root", "control", "box", "mark", "label"],
    },
    render: Fn("render_Checkbox")
});

fn checkbox_setting(ctx,name,fallback) {
    if ctx.theme_variant() == () {fallback}else{ctx.theme_token(`checkbox.${name}`)}
}

fn Checkbox(props) { render_component("components/checkbox", props) }
fn render_Checkbox(ctx, props) {
    let mixed = if "indeterminate" in props { props.indeterminate } else { false };
    let disabled = if "disabled" in props { props.disabled } else { false };
    let active = props.checked || mixed;
    let path = if mixed {
        [path_move(4.5,8.0),path_line(11.5,8.0)]
    } else {
        [path_move(4.0,8.5),path_line(6.75,11.25),path_line(12.0,5.0)]
    };
    let scene = canvas_scene([canvas_stroke_path("mark",path,
        checkbox_setting(ctx,"stroke_width",1.8),current_color()).stroke_cap("round").stroke_join("round")])
        .view_box(0,0,16,16);
    let mark_node = canvas(scene).with_key("mark").accessibility_role("presentation")
        .with_style(ctx.component_style("mark", style()
            .width(theme_length("metrics.mark")).height(theme_length("metrics.mark"))
            .text_color(theme_color("choice.on_fill"))
            .opacity(if active { 1.0 } else { 0.0 })))
        // First mount is settled. Only an accepted controlled state change draws
        // the new path; an unchecked mark clears immediately, not by fading.
        .motion(motion_transition("path_progress",0.0,if active {1.0}else{0.0},#{
            duration_ms: if !active {1} else if mixed {checkbox_setting(ctx,"mixed_draw_ms",200.0).to_int()}
                else {checkbox_setting(ctx,"draw_ms",260.0).to_int()},
            delay_ms: if active && !mixed {checkbox_setting(ctx,"draw_delay_ms",40.0).to_int()}else{0},
            easing:cubic_bezier(0.22,1.0,0.36,1.0)
        }))
        .motion_on_change(if mixed {"mixed"}else if props.checked {"checked"}else{"off"});
    let box_node = row([mark_node]).with_style(ctx.component_style("box", style()
        .width(theme_length("metrics.mark")).height(theme_length("metrics.mark"))
        .items_center().justify_center().radius(theme_radius("xs"))
        .border(px(0))
        .background(if active { theme_color("choice.fill") } else { theme_color("surface.field") })
        .group_focus(style()),
        ctx.theme_style(if active { "choice.checked" } else { "choice.unchecked" })));
    let node = row([
        row([box_node]).with_style(ctx.component_style("control",style()
            .width(theme_length("metrics.mark")).height(theme_length("metrics.choice_line"))
            .min_height(theme_length("metrics.mark")).flex_shrink(false).items_center().justify_center())),
        text(props.label).with_style(ctx.component_style("label", style().min_width(px(0))))
    ])
        .with_style(ctx.component_style("root",
            style().min_height(theme_length("metrics.control")).gap(theme_spacing("sm"))
                .items_center().self_start()
                .typography("control")
                .text_color(theme_color("text_primary"))
                // Owns the focus state shown by the mark (group_focus).
                .focus(style()).hover(style())
                .disabled(style().opacity(0.4))
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
