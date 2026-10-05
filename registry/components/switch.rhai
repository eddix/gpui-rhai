/* gpui-rhai
{
  "id": "components/switch",
  "export": "Switch",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "accent", "on_accent", "surface_hover", "surface", "focus_ring", "text_primary",
    "disabled", "metrics.mark", "metrics.control", "spacing.sm", "radius.sm",
    "typography.control", "typography.caption"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// Controlled binary switch with disabled and loading states.
// Props: checked, label, disabled, loading, on_change.
// Events: change(bool). Parts: root, track, thumb, label.
// State: controlled by the checked prop.
// Example: Switch(#{ checked: true, label: "Updates", on_change: Fn("changed") })

define_component(#{
    metadata: #{
        id: "components/switch", "export": "Switch", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "accent", "on_accent", "surface_hover", "surface", "focus_ring", "text_primary",
            "disabled", "metrics.mark", "metrics.control", "spacing.sm", "radius.sm",
            "typography.control", "typography.caption"
        ],
        environment: ["size", "density", "corners"],
    },
    schema: #{
        props: #{
            checked: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            loading: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            on_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{ change: #{ payload: #{ type: "bool" } } },
        slots: #{}, parts: ["root", "track", "thumb", "label"],
    },
    render: Fn("render_Switch")
});

fn Switch(props) { render_component("components/switch", props) }
fn render_Switch(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let loading = if "loading" in props { props.loading } else { false };
    let blocked = disabled || loading;
    let fill = if props.checked { theme_color("accent") } else { theme_color("surface_hover") };
    // A rectangular track with a square thumb (a capsule and a circle with round
    // corners). The 2px track border is reserved and takes the fill until focus turns
    // it ink.
    let thumb = text(if loading { "·" } else { "" }).with_style(ctx.component_style("thumb", style()
        .width(theme_length("metrics.mark") * 0.75).height(theme_length("metrics.mark") * 0.75)
        .radius(theme_radius("sm")).items_center().justify_center().typography("caption")
        .background(if props.checked { theme_color("on_accent") } else { theme_color("text_primary") })
        .text_color(if props.checked { theme_color("accent") } else { theme_color("surface") })
        .disabled(style().background(theme_color("disabled")))));
    let track_style = style()
        .width(theme_length("metrics.mark") * 2).height(theme_length("metrics.mark"))
        .items_center().radius(theme_radius("sm"))
        .border(px(2)).border_color(fill).background(fill)
        .group_focus(style().border_color(theme_color("focus_ring")))
        .disabled(style().border_color(theme_color("surface_hover")).background(theme_color("surface_hover")));
    track_style = if props.checked { track_style.justify_end() } else { track_style.justify_start() };
    let track = row([thumb]).with_style(ctx.component_style("track", track_style));
    let node = row([track, text(props.label).with_style(ctx.component_style("label", style()))])
        .with_style(ctx.component_style("root",
            style().min_height(theme_length("metrics.control")).gap(theme_spacing("sm"))
                .items_center().self_start()
                .typography("control")
                .text_color(theme_color("text_primary"))
                // Owns the focus state shown by the mark (group_focus).
                .focus(style())
                .disabled(style().text_color(theme_color("disabled")))))
        .disabled(blocked)
        .accessibility_role("switch")
        .accessibility_label(props.label)
        .accessibility_checked(props.checked);
    if !blocked && "on_change" in props && props.on_change != () {
        node.on_click_value(props.on_change, !props.checked)
    } else { node }
}
