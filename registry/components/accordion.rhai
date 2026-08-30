/* gpui-rhai
{
  "id": "components/accordion",
  "export": "Accordion",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled single/multiple accordion with Rust-side clip animations.
// Props: key, items, expanded, mode, disabled, on_change.
// State/events: controlled expanded values; emits change(array<string>).
// Example: Accordion(#{ key: "faq", items: items, expanded: [], on_change: Fn("changed") })

export_component(#{
    metadata: #{
        id: "components/accordion", "export": "Accordion", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{},
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
        slots: #{}, parts: ["root", "item", "trigger", "content"],
    },
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

fn Accordion(props) {
    component_render("components/accordion", props, Fn("render_Accordion"))
}
fn render_Accordion(ctx, props) {
    let mode = if "mode" in props { props.mode } else { "single" };
    let all_disabled = if "disabled" in props { props.disabled } else { false };
    let callback = if "on_change" in props { props.on_change } else { () };
    let items = [];
    for item in props.items {
        let open = has_value(props.expanded, item.key);
        let item_disabled = all_disabled || ("disabled" in item && item.disabled);
        let trigger = row([
            text(item.title),
            text(if open { "−" } else { "+" })
        ])
            .with_key(`${item.key}-trigger`)
            .with_style(component_style(props, "trigger",
                style().padding(px(8)).justify_between().items_center()))
            .disabled(item_disabled)
            .accessibility_role("button")
            .accessibility_checked(open);
        if !item_disabled && callback != () {
            trigger = trigger.on_click_value(callback, toggled(props.expanded, item.key, mode));
        }
        let height = item.content_height;
        let panel = column([item.content])
            .with_key(`${item.key}-content`)
            .with_style(component_style(props, "content", style().padding_x(px(8)).clip()))
            .animate(transition("clip_height", 0.0, if open { height * 1.0 } else { 0.0 }, 180, "ease_out"));
        items.push(column([trigger, panel]).with_key(item.key)
            .with_style(component_style(props, "item", style())));
    }
    column(items).with_key(props.key)
        .with_style(component_style(props, "root", style().gap(px(2))))
}
