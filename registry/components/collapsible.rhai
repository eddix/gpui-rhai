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

export_component(#{
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
});

fn Collapsible(props) {
    component_render("components/collapsible", props, Fn("render_Collapsible"))
}
fn render_Collapsible(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let trigger = props.trigger
        .with_style(component_style(props, "trigger", style()))
        .disabled(disabled)
        .accessibility_role("button")
        .accessibility_checked(props.open);
    if !disabled && "on_open_change" in props && props.on_open_change != () {
        trigger = trigger.on_click_value(props.on_open_change, !props.open);
    }
    let height = props.content_height;
    let panel = column([props.content])
        .with_key(`${props.key}-content`)
        .with_style(component_style(props, "content", style().clip()))
        .animate(transition("clip_height", 0.0, if props.open { height * 1.0 } else { 0.0 }, 180, "ease_out"));
    column([trigger, panel]).with_key(props.key)
        .with_style(component_style(props, "root", style()))
}
