/* gpui-rhai
{
  "id": "components/accordion",
  "export": "Accordion",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/minus.svg", "icons/plus.svg"],
  "tokens": [
    "border", "surface_hover", "focus_ring", "text_primary", "text_muted", "disabled",
    "metrics.row", "metrics.inset", "metrics.icon", "typography.body"
  ],
  "environment": ["density"]
}
*/
// Controlled single/multiple accordion with Rust-side clip motion.
// Props: key, items, expanded, mode, disabled, on_change.
// State/events: controlled expanded values; emits change(array<string>).
// Example: Accordion(#{ key: "faq", items: items, expanded: [], on_change: Fn("changed") })

define_component(#{
    metadata: #{
        id: "components/accordion", "export": "Accordion", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "border", "surface_hover", "focus_ring", "text_primary", "text_muted", "disabled",
            "metrics.row", "metrics.inset", "metrics.icon", "typography.body"
        ],
        environment: ["density"],
        assets: ["icons/minus.svg", "icons/plus.svg"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            expanded: #{ schema: #{ type: "array", max_items: 256, items: #{ type: "string" } }, required: true, sensitive: false },
            mode: #{ schema: #{ type: "string", allowed: ["single", "multiple"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "single" } },
            items: #{
                schema: #{ type: "array", max_items: 256, items: #{
                    type: "object", allow_unknown: false, fields: #{
                        key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                        title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                        content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
                        content_height: #{ schema: #{ type: "integer", min: 0 }, required: true, sensitive: false },
                        disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                            "default": #{ type: "bool", value: false } },
                    },
                } }, required: true, sensitive: false,
            },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ change: #{ payload: #{ type: "array", max_items: 256, items: #{ type: "string" } } } },
        slots: #{}, parts: ["root", "item", "trigger", "content", "focus_frame"],
    },
    render: Fn("render_Accordion")
});

fn has_value(values, key) { key in values }

fn toggled(values, key, mode) {
    if mode == "single" {
        if has_value(values, key) { [] } else { [key] }
    } else {
        let next = [];
        for value in values { if value != key { next.push(value); } }
        if !has_value(values, key) { next.push(key); }
        next
    }
}

fn Accordion(props) { render_component("components/accordion", props) }
fn render_Accordion(ctx, props) {
    let mode = if "mode" in props { props.mode } else { "single" };
    let all_disabled = if "disabled" in props { props.disabled } else { false };
    let callback = if "on_change" in props { props.on_change } else { () };
    let items = [];
    for item in props.items {
        let open = has_value(props.expanded, item.key);
        let item_disabled = all_disabled || ("disabled" in item && item.disabled);
        // Headers are list rows: metrics.row high, text at metrics.inset. Focus is a 2px
        // frame drawn over the row so it never moves the title.
        let trigger_style = style().height(theme_length("metrics.row"))
            .padding_x(theme_length("metrics.inset"))
            .justify_between().items_center().typography("body").font_weight(600)
            .text_color(theme_color("text_primary"));
        trigger_style = trigger_style
            .hover(style().background(theme_color("surface_hover")))
            .focus(style())
            .disabled(style().text_color(theme_color("disabled")));
        let focus_frame = row([]).with_style(ctx.component_style("focus_frame", style()
            .absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
            .border(px(2)).border_color(rgba(0x00000000))
            .group_focus(style().border_color(theme_color("focus_ring")))));
        let trigger = row([
            text(item.title),
            image_source(asset(if open { "app/icons/minus" } else { "app/icons/plus" }))
                .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
                    .text_color(if open { theme_color("text_primary") } else { theme_color("text_muted") })),
            focus_frame,
        ])
            .with_key(`${item.key}-trigger`)
            .with_style(ctx.component_style("trigger", trigger_style))
            .disabled(item_disabled)
            .accessibility_role("button")
            .accessibility_expanded(open);
        if !item_disabled && callback != () {
            trigger = trigger.on_click_value(callback, toggled(props.expanded, item.key, mode));
        }
        let height = item.content_height;
        let panel_content = item.content.with_style(ctx.component_style("content", style()
                .padding_x(theme_length("metrics.inset")).padding_bottom(theme_length("metrics.inset"))
                .typography("body")));
        let panel = column([panel_content])
            .with_key(`${item.key}-content`)
            .with_style(style().height(px(if open { height } else { 0 })).clip())
            .motion(motion_transition("clip_height", 0.0, if open { height * 1.0 } else { 0.0 },
                #{ duration_ms: ctx.motion_duration("fast"), easing: ctx.motion_easing("entrance"),
                    intent: "feedback" }));
        items.push(column([trigger, panel]).with_key(item.key)
            .with_style(ctx.component_style("item", style()
                .border_bottom(px(1)).border_color(theme_color("border")))));
    }
    // A Swiss list: hairlines between sections, no enclosing box.
    column(items).with_key(props.key)
        .with_style(ctx.component_style("root", style()
            .border_top(px(1)).border_color(theme_color("border"))))
}
