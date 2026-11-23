/* gpui-rhai
{
  "id": "components/switch",
  "export": "Switch",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["accent","choice.fill","choice.description","disabled","focus_ring","metrics.choice_line","metrics.control","metrics.mark","metrics.switch_h","metrics.switch_inset","metrics.switch_thumb","metrics.switch_w","on_accent","radius.pill","spacing.sm","surface","surface_hover","switch.checked_thumb","switch.off","switch.thumb","switch.thumb_off_ink","switch.thumb_off","switch.thumb_on","switch.track_off","switch.track_on","text_primary","typography.caption","typography.control","typography.hint"],
  "environment": ["size", "density"]
}
*/
// Controlled binary switch with disabled and loading states.
// State: controlled by the checked prop.
// Example: Switch(#{ checked: true, label: "Updates", on_change: Fn("changed") })

define_component(#{
    metadata: #{
        id: "components/switch", "export": "Switch", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: ["accent","choice.fill","choice.description","disabled","focus_ring","metrics.choice_line","metrics.control","metrics.mark","metrics.switch_h","metrics.switch_inset","metrics.switch_thumb","metrics.switch_w","on_accent","radius.pill","spacing.sm","surface","surface_hover","switch.checked_thumb","switch.off","switch.thumb","switch.thumb_off_ink","switch.thumb_off","switch.thumb_on","switch.track_off","switch.track_on","text_primary","typography.caption","typography.control","typography.hint"],
        environment: ["size", "density"],
    },
    schema: #{
        props: #{
            checked: #{
                schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether the switch is on; the caller stores the `change` payload and passes it back.",
            },
            label: #{
                schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Text beside the track; also the accessible name.",
            },
            description: #{
                schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Secondary text below the label and the switch's accessible description; clicking it proposes the same change.",
            },
            placement: #{
                schema: #{ type: "string", allowed: ["start", "end"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "start" },
                doc: "Logical start puts the track before the text. End fills the available row with text first and the track at the opposite edge; respects RTL.",
            },
            size: #{
                schema: #{ type: "optional", value: #{ type: "string", allowed: ["sm", "md"] } },
                required: false, sensitive: false,
                doc: "Overrides the inherited size. Stock sm tracks are 26×16, md 32×20; geometry still follows theme and reading scale.",
            },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Blocks clicks and shows the disabled style.",
            },
            loading: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Marks a pending change: the thumb shows a dot and the switch blocks clicks like `disabled`.",
            },
            on_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
                doc: "Called with the new checked value, the opposite of `checked`, when the switch is clicked.",
            },
        },
        state: #{ fields: #{} },
        events: #{ change: #{ payload: #{ type: "bool" },
            doc: "Emitted when the switch is clicked; the payload is the new checked value." } },
        slots: #{}, parts: ["root", "root_end", "control", "track", "thumb", "text", "label", "description"],
    },
    render: Fn("render_Switch")
});

fn Switch(props) { render_component("components/switch", props) }
fn render_Switch(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let loading = if "loading" in props { props.loading } else { false };
    let blocked = disabled || loading;
    let fill = if props.checked { theme_color("choice.fill") } else { theme_color("switch.off") };
    // The compact track/thumb geometry is independent from the text button.
    // Its visual track borrows the row's focus scope without adding a border.
    let thumb = text(if loading { "·" } else { "" }).with_key("thumb").with_style(ctx.component_style("thumb", style()
        .width(theme_length("metrics.switch_thumb")).height(theme_length("metrics.switch_thumb"))
        .radius(theme_radius("pill")).items_center().justify_center().typography("caption")
        .background(if props.checked { theme_color("switch.checked_thumb") } else { theme_color("switch.thumb") })
        .text_color(if props.checked { theme_color("choice.fill") } else { theme_color("switch.thumb_off_ink") }),
        ctx.theme_style(if props.checked {"switch.thumb_on"}else{"switch.thumb_off"})));
    let track_style = style()
        .width(theme_length("metrics.switch_w")).height(theme_length("metrics.switch_h"))
        .items_center().radius(theme_radius("pill"))
        .padding(theme_length("metrics.switch_inset")).border(px(0)).background(fill).group_focus(style());
    track_style = if props.checked { track_style.justify_end() } else { track_style.justify_start() };
    let track = row([thumb]).with_key("track").with_style(ctx.component_style("track", track_style,
        ctx.theme_style(if props.checked {"switch.track_on"}else{"switch.track_off"})));
    let control=row([track]).with_key("control").with_style(ctx.component_style("control",style()
        .width(theme_length("metrics.switch_w")).height(theme_length("metrics.choice_line"))
        .min_height(theme_length("metrics.switch_h")).flex_shrink(false).items_center().justify_center()));
    let at_end = props.placement == "end";
    let copy = column([
        text(props.label).with_key("label").with_style(ctx.component_style("label", style().min_width(px(0)))),
        text(props.description).with_key("description")
            .with_style(ctx.component_style("description", style().typography("hint").text_color(theme_color("choice.description"))))
            .accessibility_hidden(true).retained_page(props.description != ""),
    ]).with_key("text").with_style(ctx.component_style("text",style().min_width(px(0)).gap(px(0))));
    let node = row(if at_end {[copy,control]}else{[control,copy]})
        .with_style(ctx.component_style(if at_end {["root","root_end"]}else{["root"]},
            style().min_height(theme_length("metrics.control")).gap(theme_spacing("sm"))
                .items_center().self_start()
                .typography("control")
                .text_color(theme_color("text_primary"))
                // Owns the focus state shown by the mark (group_focus).
                .focus(style()).hover(style()),
            if loading && !disabled {style().disabled(style().opacity(1.0))}else{style()}))
        .disabled(blocked)
        .accessibility_role("switch")
        .accessibility_label(props.label)
        .accessibility_description(props.description)
        .accessibility_checked(props.checked);
    if !blocked && "on_change" in props && props.on_change != () {
        node = node.on_click_value(props.on_change, !props.checked);
    }
    if "size" in props && props.size != () {node.env(#{size:props.size})}else{node}
}
