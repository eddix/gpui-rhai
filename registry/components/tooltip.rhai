/* gpui-rhai
{
  "id": "components/tooltip",
  "export": "Tooltip",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Native delayed tooltip using the per-window overlay scheduler.
// Props: key, trigger/content slots, placement, show/hide delay, disabled.
// Parts: root, trigger, content.
// State/events: stateless delayed overlay; emits no semantic events.
// Example: Tooltip(#{ key: "help", trigger: text("?"), content: text("Help") })

define_component(#{
    metadata: #{
        id: "components/tooltip", "export": "Tooltip", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{},
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            trigger: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "top" } },
            show_delay_ms: #{ schema: #{ type: "integer", min: 0 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 500 } },
            hide_delay_ms: #{ schema: #{ type: "integer", min: 0 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 100 } },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            trigger: #{ required: true, multiple: false },
            content: #{ required: true, multiple: false },
        },
        parts: ["root", "trigger", "content"],
    },
    render: Fn("render_Tooltip")
});

fn Tooltip(props) { render_component("components/tooltip", props) }
fn render_Tooltip(ctx, props) {
    if "disabled" in props && props.disabled { return props.trigger; }
    let show_delay = if "show_delay_ms" in props && props.show_delay_ms > 0 { props.show_delay_ms } else { 0 };
    let hide_delay = if "hide_delay_ms" in props && props.hide_delay_ms > 0 { props.hide_delay_ms } else { 0 };
    let content = row([props.content]).with_style(component_style(props, "content",
        style().max_width(px(320)).padding_x(px(7)).padding_y(px(5))
            .radius(theme_radius("sm")).font_size(px(11)).line_height(px(14))
            .background(theme_color("text_primary"))
            .text_color(theme_color("surface"))
    )).accessibility_role("tooltip");
    overlay(props.trigger.with_style(component_style(props, "trigger", style())), content, #{
        id: props.key,
        kind: "tooltip",
        placement: if "placement" in props { props.placement } else { "top" },
        open: false,
        gap: 4,
        modal: false,
        dismiss_on_escape: false,
        dismiss_on_outside: false,
        show_delay_ms: show_delay,
        hide_delay_ms: hide_delay,
    }).with_key(props.key)
        .with_style(component_style(props, "root", style()))
}
