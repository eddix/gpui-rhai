/* gpui-rhai
{
  "id": "components/command",
  "export": "Command",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/input", "components/kbd"],
  "capabilities": {}
}
*/
// Embeddable keyboard-first command search with deterministic fuzzy ranking.
// Props: key, label, controlled query/active_value, items, sizing, disabled, and callbacks.
// Events: query_change(string), active_change(string), and action(string).
// State: caller owns query and the roving active command.
// Parts: root, search, list, group, item, item_active, label, shortcut, empty.
// Example: Command(#{ key: "palette", label: "Commands", query: query, active_value: active, items: items, on_query_change: Fn("queried"), on_active_change: Fn("preview"), on_action: Fn("run") })

import "components/input" as input;
import "components/kbd" as kbd;

define_component(#{
    metadata: #{ id: "components/command", "export": "Command", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/input", "components/kbd"], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            query: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            active_value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            items: #{ schema: #{ type: "one_of", variants: [
                #{ type: "array", max_items: 4096, items: #{
                    type: "object", allow_unknown: false, fields: #{
                        value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                        label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                        keywords: #{ schema: #{ type: "array", max_items: 32, items: #{ type: "string" } },
                            required: false, sensitive: false, "default": #{ type: "array", value: [] } },
                        group: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                        shortcut: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                        disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                            "default": #{ type: "bool", value: false } },
                    }
                } },
                #{ type: "collection" }
            ] }, required: true, sensitive: false },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Type a command" } },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "No commands found" } },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 24 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 } },
            row_height: #{ schema: #{ type: "integer", min: 24, max: 64 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 32 } },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            autofocus: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_query_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_active_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_action: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{ query_change: #{ payload: #{ type: "string" } },
            active_change: #{ payload: #{ type: "string" } }, action: #{ payload: #{ type: "string" } } },
        slots: #{}, parts: ["root", "search", "list", "group", "item", "item_active", "label", "shortcut", "empty"],
    },
    render: Fn("render_Command")
});

fn Command(props) { render_component("components/command", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn item_disabled(item) { "disabled" in item && item.disabled }
fn item_group(item) { if "group" in item && item.group != () { item.group } else { "" } }

fn fuzzy_score(text, query) {
    let source = text.to_lower();
    let needle = query.to_lower();
    if needle == "" { return 0; }
    if source == needle { return 10000; }
    if source.starts_with(needle) { return 8000 - source.len; }
    let direct = source.index_of(needle);
    if direct >= 0 { return 6000 - direct * 8 - source.len; }
    let position = 0;
    let gaps = 0;
    let matched = true;
    for character in needle.to_chars() {
        if matched {
            let found = source.index_of(character, position);
            if found < 0 { matched = false; }
            else { gaps += found - position; position = found + 1; }
        }
    }
    if matched { 4000 - gaps * 12 - source.len } else { -1 }
}

fn item_score(item, query) {
    let score = fuzzy_score(item.label, query);
    for keyword in prop_or(item, "keywords", []) {
        let candidate = fuzzy_score(keyword, query) - 100;
        if candidate > score { score = candidate; }
    }
    score
}

fn compare_ranked(left, right) {
    if left.score > right.score { -1 }
    else if left.score < right.score { 1 }
    else if left.source_index < right.source_index { -1 }
    else if left.source_index > right.source_index { 1 }
    else { 0 }
}

fn enabled_values(items) {
    let values = [];
    for item in items { if item.kind == "item" && !item.disabled { values.push(item.value); } }
    values
}

fn includes(values, value) {
    let found = false;
    for current in values { if current == value { found = true; } }
    found
}

fn validate_array_items(items) {
    let seen = #{};
    for item in items {
        if item.value == "" { throw "Command item value must not be empty"; }
        if item.label == "" { throw `Command item ${item.value} label must not be empty`; }
        if item.value in seen { throw `duplicate Command item value ${item.value}`; }
        seen[item.value] = true;
    }
}

fn adjacent(values, active, step) {
    if values.len == 0 { return ""; }
    let index = 0;
    for i in 0..values.len { if values[i] == active { index = i; } }
    if step > 0 { values[(index + 1) % values.len] }
    else { values[(index + values.len - 1) % values.len] }
}

fn query_action(ctx, query) { ctx.emit("query_change", query); }
fn active_action(current, ctx, value) {
    if current != value { ctx.emit("active_change", value); }
    handled()
}
fn active_hover(is_active, ctx, payload) {
    if payload.hovered && !is_active { ctx.emit("active_change", payload.value); }
    handled()
}
fn action_value(ctx, value) { ctx.emit("action", value); handled() }
fn submit_action(value, ctx, query) {
    if value != "" { ctx.emit("action", value); }
}

fn render_command_row(ctx, payload) {
    let item = payload.item;
    if item.kind == "group" {
        return text(item.label).with_key(payload.key)
            .with_style(ctx.component_style("group", style()
                .height(px(item.height)).padding_start(px(9)).padding_end(px(9))
                .padding_top(px(6)).items_center()
                .typography("caption").font_weight(400).opacity(0.72)
                .text_color(theme_color("text_muted"))))
            .accessibility_role("heading").accessibility_label(item.label);
    }
    let children = [text(item.label).with_style(ctx.component_style("label", style().flex_grow()))];
    if item.shortcut != "" {
        children.push(kbd::Kbd(#{ text: item.shortcut }).with_style(ctx.component_style("shortcut", style())));
    }
    let node = row(children).with_key(payload.key)
        .with_style(ctx.component_style(if item.active { "item_active" } else { "item" }, style()
            .height(px(item.height)).padding_start(px(18)).padding_end(px(9)).gap(px(8)).items_center()
            .radius(theme_radius("sm"))
            .background(if item.active { theme_color("selection") } else { rgba(0x00000000) })
            .border_start(px(2)).border_color(if item.active { theme_color("accent") } else { rgba(0x00000000) })
            .typography("body")
            .text_color(if item.disabled { theme_color("disabled") } else { theme_color("text_primary") })
            .hover(style().background(theme_color("surface_hover")))
            .disabled(style().opacity(0.66))))
        .tab_stop(false).disabled(item.disabled)
        .accessibility_role("option").accessibility_label(item.label)
        .accessibility_checked(item.active);
    if item.disabled { node } else {
        node.on_hover_value(Fn("active_hover").curry(item.active), item.value)
            .on_click_value(Fn("action_value"), item.value)
    }
}

fn array_command_model(items, query, row_height, stored) {
    let groups = [];
    let buckets = #{};
    for index in 0..items.len {
        let item = items[index];
        let score = item_score(item, query);
        if score >= 0 {
            let group = item_group(item);
            if !(group in buckets) { groups.push(group); buckets[group] = []; }
            let bucket = buckets[group];
            bucket.push(#{ value: item.value, label: item.label, shortcut: prop_or(item, "shortcut", ""),
                disabled: item_disabled(item), score: score, source_index: index });
            buckets[group] = bucket;
        }
    }
    let data = [];
    for group in groups {
        let bucket = buckets[group];
        bucket.sort(Fn("compare_ranked"));
        if group != "" { data.push(#{ key: `group:${group}`, kind: "group", label: group, height: row_height }); }
        for item in bucket {
            data.push(#{ key: `item:${item.value}`, kind: "item", value: item.value,
                label: item.label, shortcut: item.shortcut, disabled: item.disabled,
                active: false, height: row_height });
        }
    }
    let enabled = enabled_values(data);
    let active = if includes(enabled, stored) { stored }
        else if enabled.len > 0 { enabled[0] } else { "" };
    for index in 0..data.len {
        if data[index].kind == "item" { data[index].active = data[index].value == active; }
    }
    #{ data: data, active: active, enabled: enabled.len > 0,
        previous: if enabled.len > 0 { adjacent(enabled, active, -1) } else { "" },
        next: if enabled.len > 0 { adjacent(enabled, active, 1) } else { "" },
        first: if enabled.len > 0 { enabled[0] } else { "" },
        last: if enabled.len > 0 { enabled[enabled.len - 1] } else { "" } }
}

fn native_command_model(items, query, row_height, stored) {
    let config = #{ query: query, label_field: "label", keywords_field: "keywords",
        group_field: "group", shortcut_field: "shortcut", disabled_field: "disabled",
        active: stored, row_height: row_height };
    let data = native_fuzzy_view(items, config);
    let active = native_fuzzy_adjacent(data, stored, 0);
    if active != stored {
        config.active = active;
        data = native_fuzzy_view(items, config);
    }
    #{ data: data, active: active, enabled: active != "",
        previous: native_fuzzy_adjacent(data, active, -1),
        next: native_fuzzy_adjacent(data, active, 1),
        first: native_fuzzy_edge(data, false), last: native_fuzzy_edge(data, true) }
}

fn render_Command(ctx, props) {
    let query = props.query;
    let row_height = prop_or(props, "row_height", 32);
    let stored = props.active_value;
    let model = if is_native_collection(props.items) {
        native_command_model(props.items, query, row_height, stored)
    } else {
        validate_array_items(props.items);
        array_command_model(props.items, query, row_height, stored)
    };
    let data = model.data;
    let active = model.active;
    let search = input::Input(#{ key: `${props.key}-search`, value: query,
        placeholder: prop_or(props, "placeholder", "Type a command"),
        disabled: prop_or(props, "disabled", false),
        autofocus: prop_or(props, "autofocus", false),
        on_change: Fn("query_action"), on_submit: Fn("submit_action").curry(active) })
        .with_style(ctx.component_style("search", style()));
    let body = if data.len == 0 {
        text(prop_or(props, "empty_text", "No commands found"))
            .with_style(ctx.component_style("empty", style()
                .min_height(px(88)).padding(px(16)).items_center().justify_center()
                .typography("body_small").text_color(theme_color("text_muted"))))
            .accessibility_role("status")
    } else {
        let max_visible = prop_or(props, "max_visible", 8);
        let visible_rows = if data.len < max_visible { data.len } else { max_visible };
        let reveal_key = if is_native_collection(data) { active } else { `item:${active}` };
        virtual_collection(#{ key: `${props.key}-list`, label: props.label, data: data,
            estimated_height: row_height, height: visible_rows * row_height,
            overdraw_pixels: row_height * 2, alignment: "top", follow_tail: false,
            reveal_key: reveal_key },
            Fn("render_command_row")).with_style(ctx.component_style("list", style()))
    };
    let root = column([search, body]).with_key(props.key)
        .with_style(ctx.component_style("root", style()
            .gap(px(4)).padding(px(4)).radius(theme_radius("md"))
            .border(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface"))))
        .tab_stop(false).accessibility_role("listbox").accessibility_label(props.label);
    if !prop_or(props, "disabled", false) && model.enabled {
        root.on_key_value("up", Fn("active_action").curry(active), model.previous)
            .on_key_value("down", Fn("active_action").curry(active), model.next)
            .on_key_value("home", Fn("active_action").curry(active), model.first)
            .on_key_value("end", Fn("active_action").curry(active), model.last)
            .on_key_value("enter", Fn("action_value"), active)
    } else { root }
}
