/* gpui-rhai
{
  "id": "components/tabs",
  "export": "Tabs",
  "version": "0.1.5",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled tabs with one tab stop and orientation-aware arrow selection.
// Props: value, tabs, orientation, layout, label, motion_key, on_change.
// Parts: root, list, indicator, tab, tab_selected, label, icon, panel.
// State/events: controlled selected value; emits change(string).
// Example: Tabs(#{ value: "overview", label: "Sections", tabs: tabs, on_change: Fn("changed") })

define_component(#{
    metadata: #{
        id: "components/tabs", "export": "Tabs", version: "0.1.5",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: [], capabilities: #{},
    },
    schema: #{
        props: #{
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "horizontal" } },
            layout: #{ schema: #{ type: "string", allowed: ["content", "equal"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "content" } },
            motion_key: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" } },
            tabs: #{ schema: #{ type: "array", max_items: 128, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    icon: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
                    label_visible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: true } },
                    content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                },
            } }, required: true, sensitive: false },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ change: #{ payload: #{ type: "string" } } },
        slots: #{}, parts: ["root", "list", "indicator", "tab", "tab_selected", "label", "icon", "panel"],
    },
    render: Fn("render_Tabs")
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

fn Tabs(props) { render_component("components/tabs", props) }
fn render_Tabs(ctx, props) {
    let orientation = if "orientation" in props { props.orientation } else { "horizontal" };
    let layout = if "layout" in props { props.layout } else { "content" };
    let motion_key = if "motion_key" in props { props.motion_key } else { "" };
    if motion_key.len > 128 { throw "Tabs motion_key must be at most 128 characters"; }
    let callback = if "on_change" in props { props.on_change } else { () };
    let headers = [];
    let panel = row([]);
    for tab in props.tabs {
        let selected = tab.value == props.value;
        let label_visible = if "label_visible" in tab { tab.label_visible } else { true };
        let has_icon = "icon" in tab && tab.icon != ();
        if !label_visible && !has_icon { throw `Tabs item ${tab.value} hides its label without an icon`; }
        let tab_style = style().relative().height(px(26)).items_center().justify_center()
            .gap(px(6)).typography("body").font_weight(500)
            .border(px(1)).border_color(rgba(0x00000000))
            .text_color(if tab_disabled(tab) { theme_color("disabled") }
                else if selected { theme_color("text_primary") }
                else { theme_color("text_muted") })
            .background(rgba(0x00000000))
            .focus(style().border_color(theme_color("focus_ring")).text_color(theme_color("text_primary")))
            .disabled(style().opacity(0.68));
        if orientation == "vertical" {
            tab_style = tab_style.width(relative(1.0)).padding_x(px(10)).justify_start();
        } else if layout == "equal" {
            tab_style = tab_style.flex_basis(relative(0)).flex_grow().min_width(px(0)).padding_x(px(10));
        } else if !label_visible {
            tab_style = tab_style.width(px(26)).padding_x(px(0));
        } else {
            tab_style = tab_style.padding_x(px(10));
        }
        tab_style = tab_style.hover(style().text_color(theme_color("text_primary")));
        let resolved_tab_style = ctx.component_style("tab", tab_style);
        if selected {
            resolved_tab_style = ctx.component_style("tab_selected", resolved_tab_style
                .text_color(theme_color("text_primary")));
        }

        let children = [];
        if selected {
            let indicator = row([]).with_key("indicator").with_style(ctx.component_style("indicator",
                style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
                    .radius(theme_radius("sm")).background(theme_color("surface_raised"))));
            if motion_key != "" {
                indicator = indicator.shared_layout(motion_key, "indicator")
                    .layout_motion(ctx.motion_duration("fast"), ctx.motion_easing("standard"));
            }
            children.push(indicator);
        }
        let content = [];
        if has_icon {
            content.push(tab.icon.with_style(ctx.component_style("icon",
                style().width(px(14)).height(px(14)).flex_shrink(false))));
        }
        if label_visible {
            content.push(text(tab.label).with_style(ctx.component_style("label",
                style().min_width(px(0)).whitespace_nowrap().text_ellipsis())));
        }
        children.push(row(content).with_style(style().relative().items_center().justify_center().gap(px(6))));

        let header = row(children).with_key(tab.value)
            .with_style(resolved_tab_style)
            .tab_stop(false)
            .disabled(tab_disabled(tab))
            .accessibility_role("tab")
            .accessibility_label(tab.label)
            .accessibility_checked(selected);
        if !tab_disabled(tab) && callback != () {
            header = header.on_click_value(callback, tab.value);
        }
        headers.push(header);
        if selected { panel = tab.content.with_style(ctx.component_style("panel", style()
            .padding(px(12)).border(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface")).typography("body"))); }
    }
    let list = if orientation == "vertical" { column(headers) } else { row(headers) };
    let list_style = style().relative().padding(theme_spacing("xxs")).gap(theme_spacing("xxs"))
        .background(theme_color("surface_hover")).radius(theme_radius("sm")).min_width(px(0));
    if orientation == "vertical" {
        list_style = list_style.min_width(px(120)).self_start();
    } else {
        list_style = list_style.height(px(30)).items_center().overflow_x_scroll();
        if layout == "equal" { list_style = list_style.width(relative(1.0)); }
        else { list_style = list_style.width(auto()).self_start(); }
    }
    list_style = list_style.margin_end(auto());
    list = list
        .with_style(ctx.component_style("list", list_style))
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
    let root_style = style().gap(px(6)).margin_end(auto());
    root.with_style(ctx.component_style("root", root_style))
        .accessibility_role("group").accessibility_label(props.label)
}
