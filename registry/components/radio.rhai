/* gpui-rhai
{
  "id": "components/radio",
  "export": "Radio",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "accent", "border", "control.border", "surface_raised", "surface_hover", "focus_ring", "text_primary",
    "disabled", "metrics.mark", "metrics.choice_line", "metrics.control", "spacing.sm", "typography.control",
    "choice.checked", "choice.unchecked", "choice.fill", "choice.on_fill"
  ,"space.spacing_unit","space.unit"],
  "environment": ["size", "density"]
}
*/
// Controlled radio option. RadioGroup owns roving keyboard behavior.
// State: controlled by the selected prop.
// Example: Radio(#{ value: "team", selected: true, label: "Team", on_select: Fn("selected") })

define_component(#{
    metadata: #{
        id: "components/radio", "export": "Radio", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "accent", "border", "control.border", "surface_raised", "surface_hover", "focus_ring", "text_primary",
            "disabled", "metrics.mark", "metrics.choice_line", "metrics.control", "spacing.sm", "typography.control",
            "choice.checked", "choice.unchecked", "choice.fill", "choice.on_fill"
        ,"space.spacing_unit","space.unit"],
        environment: ["size", "density"],
    },
    schema: #{
        props: #{
            value: #{
                schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Value this option stands for; passed to `on_select` and used as the option's key.",
            },
            selected: #{
                schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether this is the chosen option; the caller derives it from the value it stores.",
            },
            label: #{
                schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Text beside the circle; also the accessible name.",
            },
            leading: #{
                schema: #{ type: "optional", value: #{ type: "node" } },
                required: false, sensitive: false,
                doc: "Decorative element before the label. The label remains the only accessible name.",
            },
            appearance: #{
                schema: #{ type: "string", allowed: ["indicator", "chip"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "indicator" },
                doc: "The ordinary radio mark or a chip without a separate mark. Both retain radio semantics and controlled selection.",
            },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Blocks clicks and shows the disabled style.",
            },
            on_select: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
                doc: "Called with `value` when the option is clicked, even if it is already selected.",
            },
            focus_mark: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Whether focus shows the ring on this option's circle; RadioGroup sets it only on its keyboard cursor.",
            },
        },
        state: #{ fields: #{} },
        events: #{ select: #{ payload: #{ type: "string" },
            doc: "Emitted when the option is clicked, even if it is already selected; the payload is `value`." } },
        slots: #{}, parts: ["root", "control", "indicator", "dot", "label", "chip", "chip_selected"],
    },
    render: Fn("render_Radio")
});

fn Radio(props) { render_component("components/radio", props) }
fn attach_choice(node, props, disabled) {
    node = node.with_key(props.value).disabled(disabled)
        .accessibility_role("radio").accessibility_label(props.label)
        .accessibility_checked(props.selected).accessibility_value(props.value);
    if !disabled && "on_select" in props && props.on_select != () {
        node.on_click_value(props.on_select, props.value)
    } else { node }
}
fn render_Radio(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let focus_mark = if "focus_mark" in props { props.focus_mark } else { true };
    let content=[text(props.label).with_style(ctx.component_style("label",style().min_width(px(0))))];
    let has_leading="leading" in props && props.leading!=();
    if has_leading { content.insert(0,props.leading.with_key("leading").accessibility_hidden(true)); }
    if props.appearance == "chip" {
        let base=style().min_width(px(0)).min_height(theme_length("metrics.control")).padding_x(theme_length("space.spacing_unit").times(2))
            .items_center().justify_center().radius(theme_length("space.unit").times(2)).typography("control").text_color(theme_color("text_primary"))
            .background(theme_color(if props.selected{"surface_raised"}else{"surface_hover"}))
            .disabled(style().opacity(0.5));
        let ring=style().focus_ring(px(2),offset_px(-2),theme_color("focus_ring"));
        base=base.focus(ring).group_focus(if focus_mark{ring}else{style()});
        if has_leading { base=base.gap(theme_spacing("sm")); }
        let chip=row(content)
            .with_style(ctx.component_style(if props.selected{["chip","chip_selected"]}else{["chip"]},base));
        return attach_choice(chip,props,disabled);
    }
    // Derive Radio from the same choice surface/type recipe as Checkbox. The dot
    // and the first-line control box scale together, without moving to the middle
    // of a wrapped label. Keep topology stable while selection changes.
    let dot = row([]).with_style(ctx.component_style("dot", style()
        .width(theme_length("metrics.mark") * 0.375).height(theme_length("metrics.mark") * 0.375)
        .radius(theme_length("metrics.mark"))
        .background(theme_color("choice.on_fill"))
        .opacity(if props.selected { 1.0 } else { 0.0 })));
    let indicator = row([dot]).with_style(ctx.component_style("indicator", style()
        .width(theme_length("metrics.mark")).height(theme_length("metrics.mark"))
        .radius(theme_length("metrics.mark") * 0.5)
        .items_center().justify_center().border(px(0))
        .background(theme_color("surface_raised")),
        ctx.theme_style(if props.selected { "choice.checked" } else { "choice.unchecked" })
            .group_focus(if focus_mark { style().focus_ring(px(2), px(2), theme_color("focus_ring")) } else { style() })));
    let control = row([indicator]).with_style(ctx.component_style("control", style()
        .width(theme_length("metrics.mark")).height(theme_length("metrics.choice_line"))
        .min_height(theme_length("metrics.mark")).flex_shrink(false).items_center().justify_center()));
    content.insert(0,control);
    let node = row(content)
        .with_style(ctx.component_style("root",
            style().min_height(theme_length("metrics.control")).gap(theme_spacing("sm"))
                .items_center().self_start()
                .typography("control")
                .text_color(theme_color("text_primary"))
                // Owns the focus state shown by the mark (group_focus).
                .focus(style()).hover(style())
                .disabled(style().opacity(0.5))
        ));
    attach_choice(node,props,disabled)
}
