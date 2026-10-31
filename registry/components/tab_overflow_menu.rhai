/* gpui-rhai
{
  "id": "components/tab_overflow_menu",
  "export": "TabOverflowMenu",
  "version": "0.4.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/menu"],
  "capabilities": {},
  "assets": ["icons/chevron_down.svg"],
  "tokens": ["tabs.foreground", "surface_hover", "text_primary", "metrics.control", "metrics.icon"],
  "environment": ["size", "density"]
}
*/
// A chevron button whose menu lists every tab of a strip; picking one reports its value. It
// owns the menu's open state and active row, so the tabs beside it can stay stateless: Tabs
// uses it for `overflow_menu`, and a caller may put it beside any strip of tabs.
//
// Example: TabOverflowMenu(#{ key: "files-menu", label: "All tabs", value: current,
//              items: [#{ value: "main", label: "main.rs", disabled: false }], on_pick: Fn("opened") })

import "components/menu" as menu;

define_component(#{
    metadata: #{
        id: "components/tab_overflow_menu", "export": "TabOverflowMenu", version: "0.4.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/menu"], capabilities: #{},
        tokens: ["tabs.foreground", "surface_hover", "text_primary", "metrics.control", "metrics.icon"],
        environment: ["size", "density"],
        assets: ["icons/chevron_down.svg"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable identity of the menu, which keeps its open state." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the button and its menu." },
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "The selected tab, where the menu's highlight starts each time it opens." },
            items: #{ schema: #{ type: "array", max_items: 256, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "The tab's value, reported when it is picked." },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "The tab's label, shown in the menu." },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Whether the row is skipped and cannot be picked." },
                } } }, required: true, sensitive: false, doc: "Every tab, in strip order." },
            on_pick: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the `value` of the picked tab; the menu closes." },
        },
        state: #{ fields: #{
            open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
            active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        } },
        events: #{ pick: #{ payload: #{ type: "string" },
            doc: "Emitted when a row is picked; the payload is its `value`." } },
        slots: #{}, parts: ["trigger"],
    },
    render: Fn("render_TabOverflowMenu")
});

fn TabOverflowMenu(props) { render_component("components/tab_overflow_menu", props) }
fn open_change(ctx, open) {
    ctx.set_state("open", open);
    // Each opening starts on the selected tab again.
    if !open { ctx.set_state("active", ""); }
}
fn active_change(ctx, value) { ctx.set_state("active", value); }
fn picked(ctx, value) { ctx.set_state("open", false); ctx.set_state("active", ""); ctx.emit("pick", value); }
fn render_TabOverflowMenu(ctx, props) {
    let items = [];
    for item in props.items {
        items.push(#{ kind: "item", value: item.value, label: item.label,
            disabled: if "disabled" in item { item.disabled } else { false } });
    }
    let trigger = row([image_source(asset("app/icons/chevron_down"))
        .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon")))])
        // A definite height: the Menu root around the trigger has none to fill (#128).
        .with_style(ctx.component_style("trigger", style().width(theme_length("metrics.control"))
            .height(theme_length("metrics.control")).items_center().justify_center()
            .text_color(theme_color("tabs.foreground"))
            .hover(style().background(theme_color("surface_hover")).text_color(theme_color("text_primary")))))
        .accessibility_role("button").accessibility_label(props.label);
    let active = ctx.get_state("active");
    menu::Menu(#{ key: props.key, label: props.label, trigger: trigger, open: ctx.get_state("open"),
        active_value: if active == "" { props.value } else { active }, items: items,
        on_open_change: Fn("open_change"), on_active_change: Fn("active_change"), on_action: Fn("picked") })
}
