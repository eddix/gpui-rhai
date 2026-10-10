/* gpui-rhai
{
  "id": "components/tooltip",
  "export": "Tooltip",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "text_primary", "surface", "spacing.xxs", "spacing.sm", "radius.lg", "typography.caption",
    "typography.label"
  ],
  "environment": ["corners"]
}
*/
// Native delayed tooltip using the per-window overlay scheduler.
//
// State: stateless; the overlay scheduler owns the delay.
// Example: Tooltip(#{ key: "help", label: "Help", trigger: text("?"), content: text("Help") })

define_component(#{
    metadata: #{
        id: "components/tooltip", "export": "Tooltip", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "text_primary", "surface", "spacing.xxs", "spacing.sm", "radius.lg",
            "typography.caption", "typography.label"
        ],
        environment: ["corners"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the tooltip, usually the words of `content`." },
            trigger: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "Element whose hover shows the tooltip after `show_delay_ms`." },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "Hint shown in the tooltip, in inverted colors and at most 320 px wide." },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "top" },
                doc: "Preferred side of the trigger the tooltip appears on." },
            show_delay_ms: #{ schema: #{ type: "integer", min: 0 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 500 },
                doc: "Hover time before the tooltip appears, in milliseconds." },
            hide_delay_ms: #{ schema: #{ type: "integer", min: 0 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 100 },
                doc: "Time the tooltip stays after the pointer leaves the trigger and the tooltip, in milliseconds." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Renders the trigger alone, with no tooltip." },
            action: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Registered action whose bound shortcut is shown after the hint; `shortcut` takes precedence." },
            shortcut: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Key legend shown after the hint, written like `cmd-d` and formatted for the platform." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            trigger: #{ required: true, multiple: false, doc: "Element whose hover shows the tooltip." },
            content: #{ required: true, multiple: false, doc: "Hint shown in the tooltip." },
        },
        parts: ["root", "trigger", "content", "shortcut"],
    },
    render: Fn("render_Tooltip")
});

fn Tooltip(props) { render_component("components/tooltip", props) }
fn render_Tooltip(ctx, props) {
    if "disabled" in props && props.disabled { return props.trigger; }
    let show_delay = if "show_delay_ms" in props && props.show_delay_ms > 0 { props.show_delay_ms } else { 0 };
    let hide_delay = if "hide_delay_ms" in props && props.hide_delay_ms > 0 { props.hide_delay_ms } else { 0 };
    let shortcut = if "shortcut" in props && props.shortcut != () { key_shortcut(props.shortcut).label }
        else if "action" in props && props.action != () {
            let bound = ctx.action_shortcut(props.action);
            if bound == () { "" } else { bound.label }
        } else { "" };
    let children = [props.content];
    if shortcut != "" {
        children.push(text(shortcut).with_style(ctx.component_style("shortcut",
            style().typography("label").opacity(0.7))));
    }
    // Tooltips invert ink and paper; a key legend follows the hint.
    let content = row(children).with_style(ctx.component_style("content",
        style().max_width(px(320)).padding_x(theme_spacing("sm")).padding_y(theme_spacing("xxs"))
            .gap(theme_spacing("sm")).items_center()
            .radius(theme_radius("lg")).typography("caption")
            .background(theme_color("text_primary"))
            .text_color(theme_color("surface"))
    )).accessibility_role("tooltip").accessibility_label(props.label);
    overlay(props.trigger.with_style(ctx.component_style("trigger", style())), content, #{
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
        .with_style(ctx.component_style("root", style()))
}
