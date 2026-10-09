/* gpui-rhai
{
  "id": "components/command",
  "export": "Command",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/input", "components/kbd"],
  "capabilities": {},
  "tokens": [
    "surface_raised", "border", "surface_hover", "selection", "accent", "text_primary",
    "text_muted", "disabled", "metrics.row", "metrics.inset", "spacing.xs", "spacing.sm",
    "radius.lg", "typography.body", "typography.label"
  ],
  "environment": ["density", "corners"]
}
*/
// Embeddable keyboard-first command search with deterministic fuzzy ranking.
//
// State: caller owns query and the roving active command.
// Example: Command(#{ key: "palette", label: "Commands", query: query, active_value: active, items: items, on_query_change: Fn("queried"), on_active_change: Fn("preview"), on_action: Fn("run") })

import "components/input" as input;
import "components/kbd" as kbd;

define_component(#{
    metadata: #{ id: "components/command", "export": "Command", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/input", "components/kbd"], capabilities: #{},
        tokens: [
            "surface_raised", "border", "surface_hover", "selection", "accent", "text_primary",
            "text_muted", "disabled", "metrics.row", "metrics.inset", "spacing.xs", "spacing.sm",
            "radius.lg", "typography.body", "typography.label"
        ],
        environment: ["density", "corners"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the search field and the command list." },
            query: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Search text; the caller stores the `query_change` payload and passes it back." },
            active_value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Highlighted command value; the caller stores the `active_change` payload. Other values show the first enabled match." },
            items: #{ schema: #{ type: "one_of", variants: [
                #{ type: "array", max_items: 4096, items: #{
                    type: "object", allow_unknown: false, fields: #{
                        value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                            doc: "Unique, non-empty id reported by `active_change` and `action`." },
                        label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                            doc: "Non-empty text ranked against the query; the row shows it unless `content` is set." },
                        content: #{ schema: #{ type: "optional", value: #{ type: "node" } },
                            required: false, sensitive: false,
                            doc: "Rich node shown in place of the label text; ranking and the accessible name still use `label`." },
                        keywords: #{ schema: #{ type: "array", max_items: 32, items: #{ type: "string" } },
                            required: false, sensitive: false, "default": #{ type: "array", value: [] },
                            doc: "Extra terms matched against the query; a keyword match ranks just below the same label match." },
                        group: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                            doc: "Heading the command is listed under; groups keep the source order of their first match." },
                        shortcut: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                            doc: "Key legend shown at the row end, written like `cmd-k`; overrides the binding of `action`." },
                        action: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                            doc: "Registered action dispatched on activation; it also supplies the shortcut legend and can disable the row." },
                        disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                            "default": #{ type: "bool", value: false },
                            doc: "Dims the command; it cannot be activated and keyboard navigation skips it." },
                    }
                } },
                #{ type: "collection" }
            ] }, required: true, sensitive: false,
                doc: "Commands to rank: item maps, or a native collection keyed by value with the same fields but no `content` or `action`." },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Type a command" },
                doc: "Hint shown in the empty search field." },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "No commands found" },
                doc: "Message shown in place of the list when no command matches." },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 24 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 },
                doc: "Rows the list shows before it scrolls; group headings count as rows." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Disables the search field and the list's arrow, Home, End and Enter keys." },
            autofocus: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Focuses the search field when the command first mounts." },
            on_query_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the new search text on each edit; store it as `query`." },
            on_active_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the command value to highlight on Up, Down, Home, End or hover; store it as `active_value`." },
            on_action: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the command value when a row is clicked or Enter activates the highlighted command." },
        },
        state: #{ fields: #{} },
        events: #{
            query_change: #{ payload: #{ type: "string" },
                doc: "Emitted on each edit of the search field; the payload is the new query." },
            active_change: #{ payload: #{ type: "string" },
                doc: "Emitted when a key or hover moves the highlight; the payload is the newly highlighted value." },
            action: #{ payload: #{ type: "string" },
                doc: "Emitted when a command is clicked or activated with Enter, after its bound action; the payload is its value." },
        },
        slots: #{}, parts: ["root", "search", "list", "group", "item", "item_active", "label", "shortcut", "empty",
            "indicator_bar"],
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
// Items bound to a registered action dispatch it before reporting the activation.
fn dispatch_value(action, ctx, value) {
    ctx.dispatch_action(action, ());
    ctx.emit("action", value);
    handled()
}
fn submit_action(spec, ctx, query) {
    if spec.value == "" { return; }
    if spec.action != () { ctx.dispatch_action(spec.action, ()); }
    ctx.emit("action", spec.value);
}
fn activation_handler(action) {
    if action == () { Fn("action_value") } else { Fn("dispatch_value").curry(action) }
}

// List rows: metrics.row high, text at metrics.inset; the cursor is a selection block
// plus the 2px indicator bar inside the inset.
fn command_indicator(ctx) {
    row([]).with_style(ctx.component_style("indicator_bar", style()
        .absolute().inset_start(px(0)).top(px(0)).bottom(px(0))
        .width(px(2)).background(theme_color("accent"))))
}

fn render_command_row(content, total_rows, ctx, payload) {
    let item = payload.item;
    if item.kind == "group" {
        return row([text(item.label.to_upper())]).with_key(payload.key)
            .with_style(ctx.component_style("group", style()
                .height(theme_length("metrics.row")).padding_x(theme_length("metrics.inset"))
                .items_center().typography("label")
                .text_color(theme_color("text_muted"))))
            .accessibility_role("heading").accessibility_label(item.label)
            .accessibility_level(2);
    }
    let action = if "action" in item { item.action } else { () };
    let disabled = item.disabled || (action != () && !ctx.action_enabled(action));
    let shortcut = if item.shortcut != "" { key_shortcut(item.shortcut).label }
        else if action != () {
            let bound = ctx.action_shortcut(action);
            if bound == () { "" } else { bound.label }
        } else { "" };
    let label = if item.value in content { content[item.value] } else { text(item.label) };
    let children = [label.with_style(ctx.component_style("label", style().flex_grow()))];
    if shortcut != "" {
        children.push(kbd::Kbd(#{ key: `${payload.key}-shortcut`, text: shortcut, appearance: "inline" })
            .with_style(ctx.component_style("shortcut", style())));
    }
    if item.active && !disabled { children.push(command_indicator(ctx)); }
    let row_style = style()
        .height(theme_length("metrics.row")).padding_x(theme_length("metrics.inset"))
        .gap(theme_spacing("sm")).items_center()
        .typography("body")
        .text_color(theme_color("text_primary"))
        .disabled(style().text_color(theme_color("disabled")));
    row_style = if item.active {
        row_style.background(theme_color("selection")).hover(style().background(theme_color("selection")))
    } else { row_style.hover(style().background(theme_color("surface_hover"))) };
    let node = row(children).with_key(payload.key)
        .with_style(ctx.component_style(if item.active { "item_active" } else { "item" }, row_style))
        .tab_stop(false).disabled(disabled)
        .accessibility_option(item.label, item.active, payload.index + 1, total_rows);
    if shortcut != "" { node = node.accessibility_key_shortcuts(shortcut); }
    if disabled { node } else {
        node.on_hover_value(Fn("active_hover").curry(item.active), item.value)
            .on_click_value(activation_handler(action), item.value)
    }
}

fn array_command_model(items, query, row_height, stored) {
    let groups = [];
    let buckets = #{};
    let content = #{};
    for index in 0..items.len {
        let item = items[index];
        let score = item_score(item, query);
        if score >= 0 {
            let group = item_group(item);
            if !(group in buckets) { groups.push(group); buckets[group] = []; }
            let bucket = buckets[group];
            let ranked = #{ value: item.value, label: item.label, shortcut: prop_or(item, "shortcut", ""),
                action: prop_or(item, "action", ()),
                disabled: item_disabled(item), score: score, source_index: index };
            if "content" in item && item.content != () { content[item.value] = item.content; }
            bucket.push(ranked);
            buckets[group] = bucket;
        }
    }
    let data = [];
    for group in groups {
        let bucket = buckets[group];
        bucket.sort(Fn("compare_ranked"));
        if group != "" { data.push(#{ key: `group:${group}`, kind: "group", label: group, height: row_height }); }
        for item in bucket {
            let row = #{ key: `item:${item.value}`, kind: "item", value: item.value,
                label: item.label, shortcut: item.shortcut, action: item.action, disabled: item.disabled,
                active: false, height: row_height };
            data.push(row);
        }
    }
    let enabled = enabled_values(data);
    let active = if includes(enabled, stored) { stored }
        else if enabled.len > 0 { enabled[0] } else { "" };
    let active_action = ();
    for index in 0..data.len {
        if data[index].kind == "item" {
            data[index].active = data[index].value == active;
            if data[index].active { active_action = data[index].action; }
        }
    }
    #{ data: data, content: content, active: active, active_action: active_action,
        enabled: enabled.len > 0,
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
    #{ data: data, content: #{}, active: active, active_action: (), enabled: active != "",
        previous: native_fuzzy_adjacent(data, active, -1),
        next: native_fuzzy_adjacent(data, active, 1),
        first: native_fuzzy_edge(data, false), last: native_fuzzy_edge(data, true) }
}

fn render_Command(ctx, props) {
    let query = props.query;
    // The native model estimates with the comfortable row; rows render at metrics.row.
    let row_height = 32;
    let stored = props.active_value;
    let model = if is_native_collection(props.items) {
        native_command_model(props.items, query, row_height, stored)
    } else {
        validate_array_items(props.items);
        array_command_model(props.items, query, row_height, stored)
    };
    let data = model.data;
    let active = model.active;
    // The search heads the panel full width, one size above the rows, on their inset.
    let search = input::Input(#{ key: `${props.key}-search`, label: props.label, value: query,
        placeholder: prop_or(props, "placeholder", "Type a command"),
        appearance: "embedded", size: "lg",
        disabled: prop_or(props, "disabled", false),
        autofocus: prop_or(props, "autofocus", false),
        on_change: Fn("query_action"),
        on_submit: Fn("submit_action").curry(#{ value: active, action: model.active_action }) })
        .with_style(ctx.component_style("search", style()));
    let body = if data.len == 0 {
        row([text(prop_or(props, "empty_text", "No commands found"))])
            .with_style(ctx.component_style("empty", style()
                .min_height(theme_length("metrics.row") * 3).padding_x(theme_length("metrics.inset"))
                .items_center().justify_center()
                .typography("body").text_color(theme_color("text_muted"))))
            .accessibility_role("status")
    } else {
        let max_visible = prop_or(props, "max_visible", 8);
        let visible_rows = if data.len < max_visible { data.len } else { max_visible };
        let collection = #{ key: `${props.key}-list`, label: props.label, data: data,
            estimated_height: row_height, fill_height: true,
            overdraw_pixels: row_height * 2, alignment: "top", follow_tail: false,
        };
        if active != "" {
            collection.reveal_key = if is_native_collection(data) { active } else { `item:${active}` };
        }
        // A whole number of metrics.row rows in either density.
        column([virtual_collection(collection,
            Fn("render_command_row").curry(model.content).curry(data.len))])
            .with_style(ctx.component_style("list", style()
                .height(theme_length("metrics.row") * visible_rows)))
    };
    let root = column([search, body]).with_key(props.key)
        .with_style(ctx.component_style("root", style()
            .gap(theme_spacing("xs")).padding_bottom(theme_spacing("xs")).radius(theme_radius("lg"))
            .border(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface_raised"))))
        .tab_stop(false).accessibility_role("listbox").accessibility_label(props.label);
    if !prop_or(props, "disabled", false) && model.enabled {
        root.on_key_value("up", Fn("active_action").curry(active), model.previous)
            .on_key_value("down", Fn("active_action").curry(active), model.next)
            .on_key_value("home", Fn("active_action").curry(active), model.first)
            .on_key_value("end", Fn("active_action").curry(active), model.last)
            .on_key_value("enter", activation_handler(model.active_action), active)
    } else { root }
}
