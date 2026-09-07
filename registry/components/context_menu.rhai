/* gpui-rhai
{
  "id": "components/context_menu",
  "export": "ContextMenu",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/menu"],
  "capabilities": {}
}
*/
// Controlled pointer-anchored action menu sharing Menu items and keyboard behavior.
// Props: key, trigger, open, active_value, items, placement, and Menu callbacks.
// Events: action(string), active_change(string), and open_change(bool).
// State: caller owns open/active values; component retains only the last pointer anchor.
// Parts: root, trigger, content, item, separator, shortcut, submenu.
// Example: ContextMenu(#{ key: "row-menu", trigger: row_node, open: open, active_value: "copy", items: items, on_open_change: Fn("opened") })

import "components/menu" as menu;

define_component(#{
    metadata: #{ id: "components/context_menu", "export": "ContextMenu", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: ["components/menu"], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            trigger: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            active_value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "bottom" } },
            items: #{ schema: #{ type: "array", max_items: 512, items: #{
                type: "object", allow_unknown: false, fields: #{
                    kind: #{ schema: #{ type: "string", allowed: ["item", "separator", "submenu"] }, required: true, sensitive: false },
                    value: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                    label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                    shortcut: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                    checked: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    submenu: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
                }
            } }, required: true, sensitive: false },
            on_action: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_active_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{ anchor: #{
            schema: #{ type: "object", allow_unknown: false, fields: #{
                x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                width: #{ schema: #{ type: "number", min: 0.0 }, required: true, sensitive: false },
                height: #{ schema: #{ type: "number", min: 0.0 }, required: true, sensitive: false },
            } },
            "default": #{ type: "map", value: #{
                x: #{ type: "float", value: 0.0 }, y: #{ type: "float", value: 0.0 },
                width: #{ type: "float", value: 0.0 }, height: #{ type: "float", value: 0.0 }
            } }
        } } },
        events: #{ action: #{ payload: #{ type: "string" } },
            active_change: #{ payload: #{ type: "string" } }, open_change: #{ payload: #{ type: "bool" } } },
        slots: #{ trigger: #{ required: true, multiple: false } },
        parts: ["root", "trigger", "content", "item", "separator", "shortcut", "submenu"],
    },
    render: Fn("render_ContextMenu")
});

fn ContextMenu(props) { render_component("components/context_menu", props) }
fn pointer_open(ctx, event) {
    ctx.set_state("anchor", #{ x: event.window.x, y: event.window.y, width: 0.0, height: 0.0 });
    ctx.emit("open_change", true);
}
fn forward_action(ctx, value) { ctx.emit("action", value); }
fn forward_active(ctx, value) { ctx.emit("active_change", value); }
fn forward_open(ctx, open) { ctx.emit("open_change", open); }

fn render_ContextMenu(ctx, props) {
    let trigger = props.trigger.tab_stop(false)
        .with_style(ctx.component_style("trigger", style()));
    menu::Menu(#{
        key: props.key, trigger: trigger, open: props.open, active_value: props.active_value,
        items: props.items, placement: if "placement" in props { props.placement } else { "bottom" },
        anchor: ctx.get_state("anchor"), activate_on_trigger: false,
        on_context_request: Fn("pointer_open"),
        on_action: if "on_action" in props && props.on_action != () { Fn("forward_action") } else { () },
        on_active_change: if "on_active_change" in props && props.on_active_change != () { Fn("forward_active") } else { () },
        on_open_change: if "on_open_change" in props && props.on_open_change != () { Fn("forward_open") } else { () }
    }).with_style(ctx.component_style("root", style()))
}
