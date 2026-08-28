/* gpui-rhai
{
  "id": "components/toast",
  "export": "Toast",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled toast host backed by the native per-window timed region queue.
// Props: key, items, max_visible, on_dismiss. Events: dismiss(string).
// Parts: root, region, toast, title, message, close.
// State: controlled item list; native queue owns timing and pause state.
// Example: Toast(#{ key: "toasts", items: items, on_dismiss: Fn("dismissed") })

export_component(#{
    metadata: #{
        id: "components/toast", "export": "Toast", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{},
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            max_visible: #{ schema: #{ type: "integer" }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 3 } },
            items: #{ schema: #{ type: "array", max_items: 100, items: #{
                type: "object", allow_unknown: false, fields: #{
                    id: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    message: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                        "default": #{ type: "string", value: "" } },
                    variant: #{ schema: #{ type: "string", allowed: ["neutral", "success", "warning", "danger"] }, required: false, sensitive: false,
                        "default": #{ type: "string", value: "neutral" } },
                    region: #{ schema: #{ type: "string", allowed: ["top_left", "top_right", "bottom_left", "bottom_right"] }, required: false, sensitive: false,
                        "default": #{ type: "string", value: "top_right" } },
                    duration_ms: #{ schema: #{ type: "integer" }, required: false, sensitive: false,
                        "default": #{ type: "integer", value: 5000 } },
                    paused: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    dismissible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: true } },
                },
            } }, required: true, sensitive: false },
            on_dismiss: #{ schema: #{ type: "callback" }, required: true, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ dismiss: #{ payload: #{ type: "string" } } },
        slots: #{}, parts: ["root", "region", "toast", "title", "message", "close"],
    },
});

fn Toast(props) {
    component_render("components/toast", props, Fn("render_Toast"))
}
fn render_Toast(ctx, props) {
    toast_host(#{
        key: props.key,
        items: props.items,
        max_visible: if "max_visible" in props { props.max_visible } else { 3 },
    }).with_style(component_style(props, "root", style()))
        .on_dismiss(props.on_dismiss)
}
