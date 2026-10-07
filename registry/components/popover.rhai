/* gpui-rhai
{
  "id": "components/popover",
  "export": "Popover",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["surface_raised", "border", "focus_ring", "metrics.inset", "space.related", "radius.lg", "typography.body"],
  "environment": ["density", "corners"]
}
*/
// Popover renders controlled content through the native per-window overlay layer.
//
// Props: key, label, trigger/content slots, open, placement, align (start|center|end,
//        default start), gap, dismissal policy, on_open_change.
// Events: open_change(bool). State: controlled by the caller.
// Parts: root, trigger, content, focus_frame.
// Example: Popover(#{ key: "profile", label: "Profile", trigger: text("Open"), content: text("Profile"), open: true })

define_component(#{
    metadata: #{
        id: "components/popover",
        "export": "Popover",
        version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [],
        capabilities: #{},
        tokens: [
            "surface_raised", "border", "focus_ring", "metrics.inset", "space.related", "radius.lg",
            "typography.body"
        ],
        environment: ["density", "corners"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
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
            align: #{
                schema: #{ type: "string", allowed: ["start", "center", "end"] },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "start" },
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
        parts: ["root", "trigger", "content", "focus_frame"],
    },
    render: Fn("render_Popover")
});

fn optional_bool(props, name, fallback) {
    if name in props { props[name] } else { fallback }
}

fn Popover(props) { render_component("components/popover", props) }

// The 2px focus frame, drawn over the panel's edge while the panel itself holds focus: the
// panel's hairline (painted over its children) turns the focus color and this frame adds
// the inner pixel. A control inside shows its own frame instead.
fn panel_focus_frame(ctx) {
    row([]).with_style(ctx.component_style("focus_frame", style().absolute()
        .left(offset_px(-1)).right(offset_px(-1)).top(offset_px(-1)).bottom(offset_px(-1))
        .radius(theme_radius("lg")).border(px(2)).border_color(rgba(0x00000000))
        .group_focus(style().border_color(theme_color("focus_ring")))))
}
fn render_Popover(ctx, props) {
    let placement = if "placement" in props { props.placement } else { "bottom" };
    let gap = if "gap" in props { props.gap } else { 8.0 };
    let content = column([props.content, panel_focus_frame(ctx)])
        .with_style(ctx.component_style("content",
            style()
                .relative()
                .min_width(px(240))
                .max_width(px(400))
                .padding(theme_length("metrics.inset"))
                .gap(theme_length("space.related"))
                .radius(theme_radius("lg"))
                .border(px(1))
                .border_color(theme_color("border"))
                .group_focus(style().border_color(theme_color("focus_ring")))
                .background(theme_color("surface_raised"))
                .typography("body")
        ))
        .accessibility_role("dialog")
        .accessibility_label(props.label);
    let config = #{
        id: props.key,
        kind: "popover",
        placement: placement,
        align: if "align" in props { props.align } else { "start" },
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
        props.trigger.with_style(ctx.component_style("trigger", style())),
        content, config
    ).with_key(props.key)
        .with_style(ctx.component_style("root", style()));
    if "on_open_change" in props && props.on_open_change != () {
        node.on_open_change(props.on_open_change)
    } else {
        node
    }
}
