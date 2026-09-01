/* gpui-rhai
{
  "id": "components/popover",
  "export": "Popover",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Popover renders controlled content through the native per-window overlay layer.
//
// Props: key, trigger/content slots, open, placement, gap, dismissal policy,
//        on_open_change.
// Events: open_change(bool). State: controlled by the caller.
// Parts: root, trigger, content.
// Example: Popover(#{ key: "profile", trigger: text("Open"), content: text("Profile"), open: true })

define_component(#{
    metadata: #{
        id: "components/popover",
        "export": "Popover",
        version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [],
        capabilities: #{},
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            parent_overlay: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false,
                sensitive: false,
            },
            trigger: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            placement: #{
                schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "bottom" },
            },
            gap: #{
                schema: #{ type: "number", min: 0.0 },
                required: false,
                sensitive: false,
                "default": #{ type: "float", value: 8.0 },
            },
            dismiss_on_escape: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: true },
            },
            dismiss_on_outside: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: true },
            },
            on_open_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{ open_change: #{ payload: #{ type: "bool" } } },
        slots: #{
            trigger: #{ required: true, multiple: false },
            content: #{ required: true, multiple: false },
        },
        parts: ["root", "trigger", "content"],
    },
    render: Fn("render_Popover")
});

fn optional_bool(props, name, fallback) {
    if name in props { props[name] } else { fallback }
}

fn Popover(props) { render_component("components/popover", props) }
fn render_Popover(ctx, props) {
    let placement = if "placement" in props { props.placement } else { "bottom" };
    let gap = if "gap" in props { props.gap } else { 8.0 };
    let content = column([props.content])
        .with_style(component_style(props, "content",
            style()
                .width(px(240))
                .padding(px(10))
                .gap(px(8))
                .radius(theme_radius("md"))
                .border(px(1))
                .border_color(theme_color("border"))
                .background(theme_color("surface"))
                .font_size(px(12)).line_height(px(16))
        ))
        .accessibility_role("dialog");
    let config = #{
        id: props.key,
        kind: "popover",
        placement: placement,
        open: props.open,
        gap: gap,
        modal: false,
        dismiss_on_escape: optional_bool(props, "dismiss_on_escape", true),
        dismiss_on_outside: optional_bool(props, "dismiss_on_outside", true),
    };
    if "parent_overlay" in props && props.parent_overlay != () {
        config.parent = props.parent_overlay;
    }
    let node = overlay(
        props.trigger.with_style(component_style(props, "trigger", style())),
        content, config
    ).with_key(props.key)
        .with_style(component_style(props, "root", style()));
    if "on_open_change" in props && props.on_open_change != () {
        node.on_open_change(props.on_open_change)
    } else {
        node
    }
}
