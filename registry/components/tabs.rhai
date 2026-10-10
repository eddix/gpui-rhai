/* gpui-rhai
{
  "id": "components/tabs",
  "export": "Tabs",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_hover", "surface_raised", "focus_ring", "text_primary", "tabs.foreground",
    "disabled", "metrics.control", "metrics.control_pad", "metrics.icon",
    "metrics.label_column", "spacing.xxs", "spacing.xs", "spacing.sm", "space.group",
    "radius.md", "radius.lg", "typography.control", "typography.body"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// Controlled tabs with one tab stop and orientation-aware arrow selection.
// State: controlled selected value.
// Example: Tabs(#{ value: "overview", label: "Sections", tabs: tabs, on_change: Fn("changed") })

define_component(#{
    metadata: #{
        id: "components/tabs", "export": "Tabs", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_hover", "surface_raised", "focus_ring", "text_primary", "tabs.foreground",
            "disabled", "metrics.control", "metrics.control_pad", "metrics.icon",
            "metrics.label_column", "spacing.xxs", "spacing.xs", "spacing.sm", "space.group",
            "radius.md", "radius.lg", "typography.control", "typography.body"
        ],
        environment: ["size", "density", "corners"],
    },
    schema: #{
        props: #{
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "The `value` of the selected tab; the caller stores the `change` payload and passes it back." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the tab list, and of the group that holds the list and the panel." },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "horizontal" },
                doc: "Direction of the list: `horizontal` is a row moved with Left/Right, `vertical` a column beside the panel moved with Up/Down." },
            layout: #{ schema: #{ type: "string", allowed: ["content", "equal"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "content" },
                doc: "Tab widths of a horizontal list: `content` fits each label, `equal` fills the width and shares it equally." },
            motion_key: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Shared-layout id, at most 128 characters, that slides the selected indicator between tabs; empty turns it off." },
            tabs: #{ schema: #{ type: "array", max_items: 128, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Identity of the tab, matched against `value` and passed to `on_change`." },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Tab text and accessible name." },
                    icon: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Node drawn at icon size before the label." },
                    label_visible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: true },
                        doc: "Whether the label text shows; a tab that hides it needs an `icon` and keeps the label as its name." },
                    content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Panel shown while this tab is selected." },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Whether the tab ignores clicks; arrow keys skip it." },
                },
            } }, required: true, sensitive: false, doc: "The tabs in list order." },
            panel: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Whether the selected tab's `content` renders; `false` renders only the tab list, for view switchers." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false, doc: "Overrides the `size` environment for the tabs." },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with a tab's `value` when it is clicked or reached with an arrow key; without it the tabs take no input." },
        },
        state: #{ fields: #{} }, events: #{ change: #{ payload: #{ type: "string" },
            doc: "Emitted when a tab is clicked or an arrow key moves the selection; the payload is the new tab's `value`." } },
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
    // A horizontal track is a capsule with round corners; a vertical one rounds like a panel.
    let corner = theme_radius(if orientation == "vertical" { "lg" } else { "md" });
    let layout = if "layout" in props { props.layout } else { "content" };
    let show_panel = if "panel" in props { props.panel } else { true };
    let motion_key = if "motion_key" in props { props.motion_key } else { "" };
    if motion_key.len > 128 { throw "Tabs motion_key must be at most 128 characters"; }
    let callback = if "on_change" in props { props.on_change } else { () };
    let headers = [];
    let panel = ();
    for tab in props.tabs {
        let selected = tab.value == props.value;
        let label_visible = if "label_visible" in tab { tab.label_visible } else { true };
        let has_icon = "icon" in tab && tab.icon != ();
        if !label_visible && !has_icon { throw `Tabs item ${tab.value} hides its label without an icon`; }
        // Slots fill the track's inner height; weight stays 400 so widths never jitter.
        let tab_style = style().relative().self_stretch()
            .items_center().justify_center().gap(theme_spacing("xs"))
            .typography("control")
            .text_color(if selected { theme_color("text_primary") } else { theme_color("tabs.foreground") })
            .disabled(style().text_color(theme_color("disabled")));
        if orientation == "vertical" {
            tab_style = tab_style.width(relative(1.0)).min_height(theme_length("metrics.control"))
                .padding_x(theme_length("metrics.control_pad")).justify_start();
        } else if layout == "equal" {
            tab_style = tab_style.flex_basis(relative(0)).flex_grow().min_width(px(0))
                .padding_x(theme_length("metrics.control_pad"));
        } else if !label_visible {
            tab_style = tab_style.padding_x(theme_spacing("sm"));
        } else {
            tab_style = tab_style.padding_x(theme_length("metrics.control_pad"));
        }
        tab_style = tab_style.hover(style().text_color(theme_color("text_primary")));
        let resolved_tab_style = ctx.component_style("tab", tab_style);
        if selected {
            resolved_tab_style = ctx.component_style("tab_selected", resolved_tab_style);
        }

        let children = [];
        if selected {
            // The thumb: a raised block inset in the track. It carries the tab list's
            // focus as the 2px ink frame.
            let thumb = style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0));
            thumb = thumb.radius(corner).background(theme_color("surface_raised"))
                .border(px(2)).border_color(theme_color("surface_raised"))
                .group_focus(style().border_color(theme_color("focus_ring")));
            let indicator = row([]).with_key("indicator")
                .with_style(ctx.component_style("indicator", thumb));
            if motion_key != "" {
                indicator = indicator.shared_layout(motion_key, "indicator")
                    .layout_motion(ctx.motion_duration("fast"), ctx.motion_easing("standard"));
            }
            children.push(indicator);
        }
        let content = [];
        if has_icon {
            content.push(tab.icon.with_style(ctx.component_style("icon",
                style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
                    .flex_shrink(false))));
        }
        if label_visible {
            content.push(text(tab.label).with_style(ctx.component_style("label",
                style().min_width(px(0)).whitespace_nowrap().text_ellipsis())));
        } else {
            content.push(text(" ").with_style(style().width(px(0)).overflow_hidden()));
        }
        children.push(row(content).with_style(style().relative().items_center().justify_center()
            .gap(theme_spacing("xs"))));

        let header = row(children).with_key(tab.value)
            .with_style(resolved_tab_style)
            .tab_stop(false)
            .disabled(tab_disabled(tab))
            .accessibility_role("tab")
            .accessibility_label(tab.label)
            .accessibility_selected(selected);
        if !tab_disabled(tab) && callback != () {
            header = header.on_click_value(callback, tab.value);
        }
        headers.push(header);
        if selected && show_panel && "content" in tab && tab.content != () {
            panel = tab.content.with_style(ctx.component_style("panel", style().typography("body")));
        }
    }
    let list = if orientation == "vertical" { column(headers) } else { row(headers) };
    // One continuous tonal track, metrics.control high, holding the slots 2px in.
    let list_style = style().relative().padding(theme_spacing("xxs")).gap(theme_spacing("xxs"))
        .background(theme_color("surface_hover")).radius(corner).min_width(px(0))
        .focus(style());
    if orientation == "vertical" {
        list_style = list_style.min_width(theme_length("metrics.label_column")).self_start();
    } else {
        list_style = list_style.min_height(theme_length("metrics.control")).overflow_x_scroll();
        if layout == "equal" { list_style = list_style.width(relative(1.0)); }
        else { list_style = list_style.width(auto()).self_start(); }
    }
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
    let size_env = "size" in props && props.size != ();
    if panel == () {
        if size_env { list = list.env(#{ size: props.size }); }
        return list;
    }
    let root = if orientation == "vertical" { row([list, panel]) } else { column([list, panel]) };
    root = root.with_style(ctx.component_style("root", style().gap(theme_length("space.group"))))
        .accessibility_role("group").accessibility_label(props.label);
    if size_env { root.env(#{ size: props.size }) } else { root }
}
