/* gpui-rhai
{
  "id": "components/context_menu",
  "export": "ContextMenu",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/menu"],
  "capabilities": {},
  "tokens": [],
  "environment": ["density"]
}
*/
// Controlled pointer-anchored action menu sharing Menu items and keyboard behavior.
//
// State: caller owns open/active values; component retains only the last pointer anchor.
// Example: ContextMenu(#{ key: "row-menu", label: "Row actions", trigger: row_node, open: open, active_value: "copy", items: items, on_open_change: Fn("opened") })

import "components/menu" as menu;

define_component(#{
    metadata: #{ id: "components/context_menu", "export": "ContextMenu", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: ["components/menu"], capabilities: #{},
        tokens: [],
        environment: ["density"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable identity of this instance; also the overlay id of its menu, which a submenu names in `parent_overlay`." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the menu panel." },
            trigger: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "Area that opens the menu at the pointer on a right-button press; it takes no focus and ignores left clicks." },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether the menu is shown; the caller stores the `open_change` payload and passes it back." },
            active_value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "`value` of the highlighted item, which Enter activates; the caller stores the `active_change` payload." },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "bottom" },
                doc: "Preferred side of the pointer position that the menu opens on." },
            items: #{ schema: #{ type: "array", max_items: 512, items: #{
                type: "object", allow_unknown: false, fields: #{
                    kind: #{ schema: #{ type: "string", allowed: ["item", "separator", "submenu", "label"] }, required: true, sensitive: false,
                        doc: "`item` is an activatable row, `separator` a divider, `label` a heading, `submenu` a nested menu row." },
                    value: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Identifier passed to `on_action` and used as `active_value`; rows without one are skipped by the keys." },
                    label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Row text, also matched by letter-key type-ahead; on a `label` row, the heading text." },
                    shortcut: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Key legend shown at the row end, written like `cmd-c`; overrides the binding of `action`." },
                    action: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Registered action the row dispatches before closing the menu; it also supplies the legend and enabled state." },
                    checked: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Shows the check mark at the row start and reports the row as checked." },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Dims the row; it cannot be clicked and keyboard navigation skips it." },
                    submenu: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Nested Menu node drawn for a `submenu` row; set its `parent_overlay` to this menu's `key`." },
                }
            } }, required: true, sensitive: false,
                doc: "Rows of the menu, in order, as in Menu." },
            on_action: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the item `value` when a row is clicked or activated with Enter; rows without an `action` leave the menu open." },
            on_active_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the item `value` to highlight on Up, Down or a letter key; store it as `active_value`." },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `true` on a right-button press on the trigger and `false` on dismissal or after an `action` row; store it as `open`." },
        },
        state: #{ fields: #{ anchor: #{
            schema: #{ type: "object", allow_unknown: false, fields: #{
                x: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Window x of the press that last opened the menu, in logical pixels." },
                y: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Window y of the press that last opened the menu, in logical pixels." },
                width: #{ schema: #{ type: "number", min: 0.0 }, required: true, sensitive: false,
                    doc: "Always 0: the menu anchors to the pointer as a point." },
                height: #{ schema: #{ type: "number", min: 0.0 }, required: true, sensitive: false,
                    doc: "Always 0: the menu anchors to the pointer as a point." },
            } },
            "default": #{ type: "map", value: #{
                x: #{ type: "float", value: 0.0 }, y: #{ type: "float", value: 0.0 },
                width: #{ type: "float", value: 0.0 }, height: #{ type: "float", value: 0.0 }
            } }
        } } },
        events: #{
            action: #{ payload: #{ type: "string" },
                doc: "Emitted when a row is clicked or activated with Enter; the payload is the item `value`." },
            active_change: #{ payload: #{ type: "string" },
                doc: "Emitted when Up, Down or a letter key moves the highlight; the payload is the new item `value`." },
            open_change: #{ payload: #{ type: "bool" },
                doc: "Emitted when a right-button press opens the menu or it closes; the payload is the requested `open`." },
        },
        slots: #{ trigger: #{ required: true, multiple: false, doc: "Area that opens the menu on a right-button press." } },
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
        key: props.key, label: props.label, trigger: trigger, open: props.open, active_value: props.active_value,
        items: props.items, placement: if "placement" in props { props.placement } else { "bottom" },
        anchor: ctx.get_state("anchor"), activate_on_trigger: false,
        on_context_request: Fn("pointer_open"),
        on_action: if "on_action" in props && props.on_action != () { Fn("forward_action") } else { () },
        on_active_change: if "on_active_change" in props && props.on_active_change != () { Fn("forward_active") } else { () },
        on_open_change: if "on_open_change" in props && props.on_open_change != () { Fn("forward_open") } else { () },
        part_styles: #{
            content: ctx.component_style("content", style()),
            item: ctx.component_style("item", style()),
            separator: ctx.component_style("separator", style()),
            shortcut: ctx.component_style("shortcut", style()),
            submenu: ctx.component_style("submenu", style()),
        }
    }).with_style(ctx.component_style("root", style()))
}
