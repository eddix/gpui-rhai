/* gpui-rhai
{
  "id": "components/collapsible",
  "export": "Collapsible",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/chevron_down.svg", "icons/chevron_right.svg"],
  "tokens": [
    "surface_hover", "focus_ring", "text_primary", "text_muted", "disabled", "metrics.row",
    "metrics.inset", "metrics.icon", "spacing.xs", "typography.body"
  ],
  "environment": ["density"]
}
*/
// Controlled disclosure with trigger/content slots and Rust clip animation.
// Props: key, open, trigger, content, content_height, disabled, on_open_change.
// State/events: controlled open state; emits open_change(bool).
// Example: Collapsible(#{ key: "details", open: true, trigger: text("Details"), content: text("Body"), content_height: 40 })

define_component(#{
    metadata: #{
        id: "components/collapsible", "export": "Collapsible", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        assets: ["icons/chevron_down.svg", "icons/chevron_right.svg"],
        tokens: [
            "surface_hover", "focus_ring", "text_primary", "text_muted", "disabled",
            "metrics.row", "metrics.inset", "metrics.icon", "spacing.xs", "typography.body"
        ],
        environment: ["density"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            trigger: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            content_height: #{ schema: #{ type: "integer", min: 0 }, required: true, sensitive: false },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ open_change: #{ payload: #{ type: "bool" } } },
        slots: #{
            trigger: #{ required: true, multiple: false },
            content: #{ required: true, multiple: false },
        },
        parts: ["root", "trigger", "content", "indicator", "focus_frame"],
    },
    render: Fn("render_Collapsible")
});

fn Collapsible(props) { render_component("components/collapsible", props) }
fn render_Collapsible(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    // The header is a list row around the caller's trigger content; focus is a 2px
    // frame drawn over it.
    let focus_frame = row([]).with_style(ctx.component_style("focus_frame", style()
        .absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
        .border(px(2)).border_color(rgba(0x00000000))
        .group_focus(style().border_color(theme_color("focus_ring")))));
    let indicator = image_source(asset(if props.open { "app/icons/chevron_down" } else { "app/icons/chevron_right" }))
        .with_style(ctx.component_style("indicator", style()
            .width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
            .text_color(theme_color("text_muted"))));
    let header_style = style().min_height(theme_length("metrics.row"))
        .padding_x(theme_length("metrics.inset")).gap(theme_spacing("xs")).items_center()
        .typography("body").font_weight(600).text_color(theme_color("text_primary"));
    header_style = header_style
        .hover(style().background(theme_color("surface_hover")))
        .focus(style())
        .disabled(style().text_color(theme_color("disabled")));
    let trigger = row([indicator, props.trigger.with_style(style().flex_grow()), focus_frame])
        .with_style(ctx.component_style("trigger", header_style))
        .disabled(disabled)
        .accessibility_role("button")
        .accessibility_expanded(props.open);
    if !disabled && "on_open_change" in props && props.on_open_change != () {
        trigger = trigger.on_click_value(props.on_open_change, !props.open);
    }
    let height = props.content_height;
    let panel_content = props.content.with_style(ctx.component_style("content", style()
            .padding_x(theme_length("metrics.inset")).padding_bottom(theme_length("metrics.inset"))
            .typography("body")));
    let panel = column([panel_content])
        .with_key(`${props.key}-content`)
        .with_style(style().height(px(if props.open { height } else { 0 })).clip())
        .motion(motion_transition("clip_height", 0.0, if props.open { height * 1.0 } else { 0.0 },
            #{ duration_ms: ctx.motion_duration("fast"), easing: ctx.motion_easing("entrance"),
                intent: "feedback" }));
    column([trigger, panel]).with_key(props.key)
        .with_style(ctx.component_style("root", style()))
}
