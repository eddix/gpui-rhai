/* gpui-rhai
{
  "id": "components/tabs",
  "export": "Tabs",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled tabs with one tab stop and orientation-aware arrow selection.
// Props: value, tabs, orientation, label, on_change. Parts: root, list, tab, panel.
// State/events: controlled selected value; emits change(string).
// Example: Tabs(#{ value: "overview", label: "Sections", tabs: tabs, on_change: Fn("changed") })

export_component(#{
    metadata: #{
        id: "components/tabs", "export": "Tabs", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{},
    },
    schema: #{
        props: #{
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "horizontal" } },
            tabs: #{ schema: #{ type: "array", max_items: 128, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                },
            } }, required: true, sensitive: false },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ change: #{ payload: #{ type: "string" } } },
        slots: #{}, parts: ["root", "list", "tab", "panel"],
    },
});

fn tab_disabled(tab) { if "disabled" in tab { tab.disabled } else { false } }
fn tab_index(tabs, value) {
    for index in 0..tabs.len { if tabs[index].value == value { return index; } }
    0
}
fn next_tab(tabs, value, step) {
    if tabs.len == 0 { return value; }
    let current = tab_index(tabs, value);
    for offset in 1..(tabs.len + 1) {
        let index = if step > 0 { (current + offset) % tabs.len }
            else { (current + tabs.len - (offset % tabs.len)) % tabs.len };
        if !tab_disabled(tabs[index]) { return tabs[index].value; }
    }
    value
}

fn Tabs(props) {
    component_render("components/tabs", props, Fn("render_Tabs"))
}
fn render_Tabs(ctx, props) {
    let orientation = if "orientation" in props { props.orientation } else { "horizontal" };
    let callback = if "on_change" in props { props.on_change } else { () };
    let headers = [];
    let panel = row([]);
    for tab in props.tabs {
        let selected = tab.value == props.value;
        let header = text(tab.label)
            .with_key(tab.value)
            .with_style(component_style(props, "tab",
                style().padding(px(8)).radius(px(6))
                    .background(if selected { theme_color("accent") } else { rgba(0x00000000) })
                    .text_color(if tab_disabled(tab) {
                        theme_color("disabled")
                    } else if selected {
                        theme_color("on_accent")
                    } else { theme_color("text_primary") })
            ))
            .tab_stop(false)
            .disabled(tab_disabled(tab))
            .accessibility_role("tab")
            .accessibility_checked(selected);
        if !tab_disabled(tab) && callback != () {
            header = header.on_click_value(callback, tab.value);
        }
        headers.push(header);
        if selected { panel = tab.content.with_style(component_style(props, "panel", style())); }
    }
    let list = if orientation == "vertical" { column(headers) } else { row(headers) };
    list = list
        .with_style(component_style(props, "list", style().gap(px(4))))
        .accessibility_role("tablist")
        .accessibility_label(props.label)
        .accessibility_value(props.value);
    if callback != () {
        let previous = next_tab(props.tabs, props.value, -1);
        let next = next_tab(props.tabs, props.value, 1);
        list = list.tab_stop(true);
        if orientation == "vertical" {
            list = list.on_key_value("up", callback, previous).on_key_value("down", callback, next);
        } else {
            list = list.on_key_value("left", callback, previous).on_key_value("right", callback, next);
        }
    }
    let root = if orientation == "vertical" { row([list, panel]) } else { column([list, panel]) };
    root.with_style(component_style(props, "root", style().gap(px(12))))
        .accessibility_role("group").accessibility_label(props.label)
}
