/* gpui-rhai
{
  "id": "components/dropdown",
  "export": "Dropdown",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/input"],
  "capabilities": {},
  "assets": []
}
*/
// Public Rhai choice composition over Overlay, Input, and virtual_collection.
// Props: key/options/mode, optional controlled selected/open/query, search and slots.
// Events: change(array<string>), open_change(bool), query_change(string).
// State: keyed selection, open, query, and active-option state when not controlled.
// Example: Dropdown(#{ key: "country", options: options, selected: ["uk"], on_change: Fn("changed") })

import "components/input" as input;

define_component(#{
    metadata: #{ id: "components/dropdown", "export": "Dropdown", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/input"], capabilities: #{}, assets: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            parent_overlay: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            options: #{ schema: #{ type: "array", max_items: 100000, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    keywords: #{ schema: #{ type: "array", max_items: 32, items: #{ type: "string" } }, required: false, sensitive: false,
                        "default": #{ type: "array", value: [] } },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    group: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                },
            } }, required: true, sensitive: false },
            mode: #{ schema: #{ type: "string", allowed: ["single", "multiple"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "single" } },
            selected: #{ schema: #{ type: "optional", value: #{ type: "array", max_items: 100000, items: #{ type: "string" } } }, required: false, sensitive: false },
            open: #{ schema: #{ type: "optional", value: #{ type: "bool" } }, required: false, sensitive: false },
            searchable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            query: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" } },
            search_placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Search" } },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "No results" } },
            clear_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Clear selection" } },
            trigger: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            header: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            footer: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            empty: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            clearable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            reset_query_on_close: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 32 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 } },
            row_height: #{ schema: #{ type: "integer", min: 20, max: 96 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 32 } },
            trigger_height: #{ schema: #{ type: "integer", min: 20, max: 96 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 32 } },
            width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "bottom" } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_query_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{
            selected: #{ schema: #{ type: "array", max_items: 100000, items: #{ type: "string" } },
                "default": #{ type: "array", value: [] } },
            open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
            query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
            active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        } },
        events: #{
            change: #{ payload: #{ type: "array", max_items: 100000, items: #{ type: "string" } } },
            open_change: #{ payload: #{ type: "bool" } },
            query_change: #{ payload: #{ type: "string" } },
        },
        slots: #{
            trigger: #{ required: false, multiple: false }, header: #{ required: false, multiple: false },
            footer: #{ required: false, multiple: false }, empty: #{ required: false, multiple: false },
        },
        parts: ["root", "trigger", "value", "placeholder", "indicator", "clear",
            "panel", "search", "list", "group", "option", "option_selected",
            "option_active", "option_disabled", "empty"],
    },
    render: Fn("render_Dropdown")
});

fn Dropdown(props) { render_component("components/dropdown", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn option_disabled(option) { prop_or(option, "disabled", false) }
fn option_group(option) {
    if "group" in option && option.group != () { option.group } else { () }
}

fn validate_dropdown(props, selected) {
    let values = #{};
    for option in props.options {
        let label = option.label;
        label.trim();
        if label == "" { throw `dropdown option '${option.value}' has an empty label`; }
        if option.value in values { throw `dropdown option value '${option.value}' is duplicated`; }
        values[option.value] = true;
        let group = option_group(option);
        if group != () {
            let normalized_group = group;
            normalized_group.trim();
            if normalized_group == "" { throw `dropdown option '${option.value}' has an empty group`; }
        }
    }
    if prop_or(props, "mode", "single") == "single" && selected.len > 1 {
        throw "single-select dropdown cannot contain multiple selected values";
    }
    for value in selected {
        if !(value in values) { throw `dropdown selection '${value}' does not match an option`; }
    }
}

fn option_matches(option, query) {
    if query == "" || option.label.to_lower().contains(query) { return true; }
    for keyword in prop_or(option, "keywords", []) {
        if keyword.to_lower().contains(query) { return true; }
    }
    false
}

fn selected_contains(selected, value) { value in selected }
fn selection_for(selected, value, mode) {
    if mode == "single" { return [value]; }
    let next = [];
    for current in selected { if current != value { next.push(current); } }
    if !selected_contains(selected, value) { next.push(value); }
    next
}
fn selection_changed(selected, next) {
    if selected.len != next.len { return true; }
    for value in selected { if !(value in next) { return true; } }
    false
}

fn active_value(options, selected, current) {
    for option in options {
        if !option_disabled(option) && option.value == current { return current; }
    }
    for value in selected {
        for option in options {
            if !option_disabled(option) && option.value == value { return value; }
        }
    }
    for option in options { if !option_disabled(option) { return option.value; } }
    ""
}
fn enabled_options(options) {
    let result = [];
    for option in options { if !option_disabled(option) { result.push(option); } }
    result
}
fn adjacent_value(options, active, step) {
    if options.len == 0 { return ""; }
    let current = -1;
    for index in 0..options.len { if options[index].value == active { current = index; } }
    if current < 0 { return if step < 0 { options[options.len - 1].value } else { options[0].value }; }
    let index = if step < 0 { (current + options.len - 1) % options.len }
        else { (current + 1) % options.len };
    options[index].value
}
fn edge_value(options, end) {
    if options.len == 0 { "" }
    else if end { options[options.len - 1].value }
    else { options[0].value }
}
fn typeahead_value(options, active, key) {
    if options.len == 0 { return ""; }
    let current = -1;
    for index in 0..options.len { if options[index].value == active { current = index; } }
    let start = if current < 0 { 0 } else { current + 1 };
    for offset in 0..options.len {
        let option = options[(start + offset) % options.len];
        if option.label.to_lower().starts_with(key) { return option.value; }
    }
    active
}

fn open_action(ctx, value) {
    ctx.set_state("open", value);
    ctx.emit("open_change", value);
    handled()
}
fn open_action_reset(ctx, value) {
    ctx.set_state("open", value);
    ctx.emit("open_change", value);
    if !value {
        ctx.set_state("query", "");
        ctx.set_state("active", "");
        ctx.emit("query_change", "");
    }
    handled()
}
fn query_action(ctx, value) {
    ctx.set_state("query", value);
    ctx.set_state("active", "");
    ctx.emit("query_change", value);
    handled()
}
fn navigate_action(ctx, payload) {
    ctx.set_state("active", payload.active);
    if payload.open {
        ctx.set_state("open", true);
        ctx.emit("open_change", true);
    }
    handled()
}
fn option_action(ctx, payload) {
    ctx.set_state("active", payload.active);
    if payload.changed {
        ctx.set_state("selected", payload.selected);
        ctx.emit("change", payload.selected);
    }
    if payload.close {
        ctx.set_state("open", false);
        ctx.emit("open_change", false);
        if payload.reset_query {
            ctx.set_state("query", "");
            ctx.emit("query_change", "");
        }
    }
    handled()
}
fn clear_action(ctx, payload) {
    if payload.changed {
        ctx.set_state("selected", []);
        ctx.emit("change", []);
    }
    if payload.reset_query {
        ctx.set_state("query", "");
        ctx.set_state("active", "");
        ctx.emit("query_change", "");
    }
    handled()
}

fn render_dropdown_row(ctx, payload) {
    let item = payload.item;
    if item.kind == "group" {
        return text(item.label).with_key(payload.key)
            .with_style(ctx.component_style("group",
                style().height(px(item.height)).padding_x(theme_spacing("sm"))
                    .text_color(theme_color("text_muted"))))
            .accessibility_role("presentation").accessibility_label(item.label);
    }
    let row_style = ctx.component_style("option",
        style().height(px(item.height)).padding_x(theme_spacing("sm"))
            .radius(theme_radius("sm")).items_center().gap(px(6)));
    let row_style = if item.selected {
        ctx.component_style("option_selected", row_style.text_color(theme_color("accent")))
    } else { row_style.text_color(theme_color("text_primary")) };
    let row_style = if item.active {
        ctx.component_style("option_active", row_style.background(theme_color("surface_hover")))
    } else { row_style };
    let row_style = if item.disabled {
        ctx.component_style("option_disabled", row_style.text_color(theme_color("disabled")))
    } else { row_style };
    let node = row([text(if item.selected { "✓" } else { "" }), text(item.label)])
        .with_key(payload.key).with_style(row_style)
        .disabled(item.disabled).tab_stop(false)
        .accessibility_role("option")
        .accessibility_checked(item.selected)
        .accessibility_label(item.label)
        .accessibility_value(item.value);
    if item.disabled { node }
    else { node.on_click_value(Fn("option_action"), item.action) }
}

fn render_Dropdown(ctx, props) {
    let selected = if "selected" in props && props.selected != () {
        props.selected
    } else { ctx.get_state("selected") };
    validate_dropdown(props, selected);
    let open = if "open" in props && props.open != () { props.open } else { ctx.get_state("open") };
    let query = if "query" in props && props.query != () { props.query } else { ctx.get_state("query") };
    let query_lower = query.to_lower();
    let mode = prop_or(props, "mode", "single");
    let reset_query = prop_or(props, "reset_query_on_close", false);
    let row_height = prop_or(props, "row_height", 32);
    let width = if "width" in props && props.width != () { props.width } else { px(280) };

    let group_order = [];
    let buckets = #{};
    let labels = [];
    for option in props.options {
        if selected_contains(selected, option.value) { labels.push(option.label); }
        if option_matches(option, query_lower) {
            let group = option_group(option);
            let group_key = if group == () { "u:" } else { `g:${group}` };
            if !(group_key in buckets) {
                group_order.push(#{ key: group_key, label: group });
                buckets[group_key] = [];
            }
            let bucket = buckets[group_key];
            bucket.push(option);
            buckets[group_key] = bucket;
        }
    }
    let ordered = [];
    for group in group_order { for option in buckets[group.key] { ordered.push(option); } }
    let active = active_value(ordered, selected, ctx.get_state("active"));
    let enabled = enabled_options(ordered);

    let data = [];
    for group in group_order {
        if group.label != () {
            data.push(#{ key: `group:${group.key}`, kind: "group", label: group.label,
                height: row_height });
        }
        for option in buckets[group.key] {
            let next = selection_for(selected, option.value, mode);
            data.push(#{ key: `option:${option.value}`, kind: "option", value: option.value,
                label: option.label, disabled: option_disabled(option),
                selected: selected_contains(selected, option.value), active: option.value == active,
                height: row_height, action: #{ selected: next, active: option.value,
                    changed: selection_changed(selected, next), close: mode == "single",
                    reset_query: reset_query } });
        }
    }

    let selected_label = "";
    for label in labels {
        selected_label += if selected_label == "" { label } else { `, ${label}` };
    }
    let placeholder = prop_or(props, "placeholder", "");
    let custom_trigger = "trigger" in props && props.trigger != ();
    let trigger_children = [];
    if custom_trigger {
        trigger_children.push(props.trigger);
    } else {
        trigger_children.push(text(if selected_label == "" { placeholder } else { selected_label })
            .with_key(`${props.key}-trigger-value`)
            .with_style(component_style(props,
                if selected_label == "" { "placeholder" } else { "value" },
                style().text_color(if selected_label == "" { theme_color("text_muted") }
                    else { theme_color("text_primary") }))));
        if prop_or(props, "clearable", false) && selected.len > 0 {
            trigger_children.push(text("×").with_key(`${props.key}-clear`)
                .with_style(component_style(props, "clear", style().padding_x(px(4))))
                .accessibility_role("button").accessibility_label(prop_or(props, "clear_label", "Clear selection"))
                .on_click_value(Fn("clear_action"),
                    #{ changed: true, reset_query: reset_query }));
        }
        trigger_children.push(text(if open { "▴" } else { "▾" })
            .with_key(`${props.key}-indicator`)
            .with_style(component_style(props, "indicator",
                style().text_color(theme_color("text_muted")))));
    }
    let trigger = row(trigger_children).with_key(`${props.key}-trigger`)
        .with_style(component_style(props, "trigger",
            style().width(width).height(px(prop_or(props, "trigger_height", 32)))
                .padding_x(theme_spacing("sm")).justify_between().items_center()
                .border(px(1)).border_color(theme_color("border"))
                .radius(theme_radius("md")).background(theme_color("surface"))))
        .disabled(prop_or(props, "disabled", false)).tab_stop(true)
        .accessibility_role("combobox").accessibility_checked(open)
        .accessibility_label(placeholder).accessibility_value(selected_label);

    let content = [];
    if "header" in props && props.header != () { content.push(props.header); }
    if prop_or(props, "searchable", false) {
        content.push(input::Input(#{
            key: `${props.key}-search`, value: query,
            placeholder: prop_or(props, "search_placeholder", "Search"),
            on_change: Fn("query_action")
        }).with_style(component_style(props, "search", style())));
    }
    if ordered.len == 0 {
        content.push(if "empty" in props && props.empty != () { props.empty }
            else { text(prop_or(props, "empty_text", "No results"))
                .with_style(component_style(props, "empty", style().padding(px(8)))) });
    } else {
        content.push(virtual_collection(#{
            key: `${props.key}-options`, label: placeholder,
            data: data, estimated_height: row_height,
            height: prop_or(props, "max_visible", 8) * row_height,
            overdraw_pixels: row_height * 2, alignment: "top", follow_tail: false
        }, Fn("render_dropdown_row")).with_style(component_style(props, "list", style())));
    }
    if "footer" in props && props.footer != () { content.push(props.footer); }
    let panel = column(content).with_key(`${props.key}-panel`)
        .with_style(component_style(props, "panel",
            style().width(width).gap(px(4)).padding(px(4))
                .border(px(1)).border_color(theme_color("border"))
                .radius(theme_radius("md")).background(theme_color("surface_raised"))))
        .tab_stop(true).accessibility_role("listbox").accessibility_label(placeholder);

    if enabled.len > 0 {
        panel = panel
            .on_key_value("up", Fn("navigate_action"),
                #{ active: adjacent_value(enabled, active, -1), open: false })
            .on_key_value("down", Fn("navigate_action"),
                #{ active: adjacent_value(enabled, active, 1), open: false })
            .on_key_value("home", Fn("navigate_action"),
                #{ active: edge_value(enabled, false), open: false })
            .on_key_value("end", Fn("navigate_action"),
                #{ active: edge_value(enabled, true), open: false });
        for item in data {
            if item.kind == "option" && item.value == active {
                panel = panel.on_key_value("enter", Fn("option_action"), item.action);
            }
        }
        if !prop_or(props, "searchable", false) {
            for key in [
                "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
                "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z"
            ] {
                panel = panel.on_key_value(key, Fn("navigate_action"),
                    #{ active: typeahead_value(enabled, active, key), open: false });
            }
        }
    }

    let overlay_config = #{
        id: props.key, kind: "popover", placement: prop_or(props, "placement", "bottom"),
        open: open, gap: 4, modal: false, dismiss_on_escape: true, dismiss_on_outside: true
    };
    if "parent_overlay" in props && props.parent_overlay != () {
        overlay_config.parent = props.parent_overlay;
    }
    let node = overlay(trigger, panel, overlay_config)
        .with_key(props.key).with_style(component_style(props, "root", style()))
        .on_open_change(if reset_query { Fn("open_action_reset") } else { Fn("open_action") });
    if enabled.len > 0 {
        node = node
            .on_key_value("up", Fn("navigate_action"),
                #{ active: if open { adjacent_value(enabled, active, -1) }
                    else { edge_value(enabled, true) }, open: !open })
            .on_key_value("down", Fn("navigate_action"),
                #{ active: if open { adjacent_value(enabled, active, 1) }
                    else { edge_value(enabled, false) }, open: !open });
    }
    node
}
