/* gpui-rhai
{
  "id": "components/collapsible",
  "export": "Collapsible",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled disclosure with trigger/content slots and Rust clip animation.
// Props: key, open, trigger, content, content_height, disabled, on_open_change.
// State/events: controlled open state; emits open_change(bool).
// Example: Collapsible(#{ key: "details", open: true, trigger: text("Details"), content: text("Body"), content_height: 40 })

define_component(#{
    metadata: #{
        id: "components/collapsible", "export": "Collapsible", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{},
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
        parts: ["root", "trigger", "content"],
    },
    render: Fn("render_Collapsible")
});

fn Collapsible(props) { render_component("components/collapsible", props) }
fn render_Collapsible(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let trigger = props.trigger
        .with_style(component_style(props, "trigger", style()
            .min_height(px(32)).padding_x(px(10)).items_center()
            .border(px(1)).border_color(rgba(0x00000000))
            .radius(theme_radius("sm"))
            .font_size(px(12)).line_height(px(16)).font_weight(700)
            .hover(style().background(theme_color("surface_hover")))
            .focus(style().border_color(theme_color("focus_ring")))
            .active(style().opacity(0.82))
            .disabled(style().text_color(theme_color("disabled")).opacity(0.72))))
        .disabled(disabled)
        .accessibility_role("button")
        .accessibility_checked(props.open);
    if !disabled && "on_open_change" in props && props.on_open_change != () {
        trigger = trigger.on_click_value(props.on_open_change, !props.open);
    }
    let height = props.content_height;
    let panel_content = props.content.with_style(component_style(props, "content", style()
        .padding(px(10)).font_size(px(12)).line_height(px(16))));
    let panel = column([panel_content])
        .with_key(`${props.key}-content`)
        .with_style(style().height(px(if props.open { height } else { 0 })).clip()
            .border_top(px(1)).border_color(theme_color("border")))
        .animate(transition("clip_height", 0.0, if props.open { height * 1.0 } else { 0.0 }, 150, "ease_out"));
    column([trigger, panel]).with_key(props.key)
        .with_style(component_style(props, "root", style()
            .border(px(1)).border_color(theme_color("border"))
            .radius(theme_radius("md")).clip()))
}
