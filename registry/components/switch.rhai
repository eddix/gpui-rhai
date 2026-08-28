/* gpui-rhai
{
  "id": "components/switch",
  "export": "Switch",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled binary switch with disabled and loading states.
// Props: checked, label, disabled, loading, on_change.
// Events: change(bool). Parts: root, track, thumb, label.
// State: controlled by the checked prop.
// Example: Switch(#{ checked: true, label: "Updates", on_change: Fn("changed") })

export_component(#{
    metadata: #{
        id: "components/switch", "export": "Switch", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{},
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
});

fn Switch(props) {
    component_render("components/switch", props, Fn("render_Switch"))
}
fn render_Switch(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let loading = if "loading" in props { props.loading } else { false };
    let blocked = disabled || loading;
    let thumb = text(if loading { "…" } else { "" }).with_style(component_style(props, "thumb",
        style()
            .width(px(18)).height(px(18)).radius(px(9))
            .background(theme_color("text_primary"))
            .text_color(theme_color("surface"))
    ));
    let track_style = style()
        .width(px(38)).height(px(22)).padding(px(2)).radius(px(11))
        .background(if props.checked { theme_color("accent") } else { theme_color("surface_hover") });
    track_style = if props.checked { track_style.justify_end() } else { track_style.justify_start() };
    let track = row([thumb]).with_style(component_style(props, "track", track_style));
    let node = row([track, text(props.label).with_style(component_style(props, "label", style()))])
        .with_style(component_style(props, "root", style().gap(px(8)).items_center()))
        .disabled(blocked)
        .accessibility_role("switch")
        .accessibility_label(props.label)
        .accessibility_checked(props.checked);
    if !blocked && "on_change" in props && props.on_change != () {
        node.on_click_value(props.on_change, !props.checked)
    } else { node }
}
