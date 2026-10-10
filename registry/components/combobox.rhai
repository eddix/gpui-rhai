/* gpui-rhai
{
  "id": "components/combobox",
  "export": "Combobox",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/input"],
  "capabilities": {},
  "assets": ["icons/check.svg", "icons/close.svg", "icons/chevron_down.svg", "icons/chevron_up.svg"],
  "tokens": [
    "surface_raised", "border", "focus_ring", "surface_hover", "selection", "accent",
    "text.accent", "text_primary", "text_muted", "disabled", "metrics.control",
    "metrics.field_pad", "metrics.row", "metrics.inset", "metrics.icon", "spacing.xs",
    "spacing.sm", "radius.md", "radius.lg", "radius.sm", "typography.control",
    "typography.body", "typography.label"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// Public Rhai choice composition over Overlay, Input, and virtual_collection.
// State: caller owns selected/open/query; component owns only active-option navigation.
// Example: Combobox(#{ key: "country", label: "Country", options: options, selected: ["uk"], open: false, query: "", on_change: Fn("changed") })

import "components/input" as input;

define_component(#{
    metadata: #{ id: "components/combobox", "export": "Combobox", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/input"], capabilities: #{},
        tokens: [
            "surface_raised", "border", "focus_ring", "surface_hover", "selection", "accent",
            "text.accent", "text_primary", "text_muted", "disabled", "metrics.control",
            "metrics.field_pad", "metrics.row", "metrics.inset", "metrics.icon", "spacing.xs",
            "spacing.sm", "radius.md", "radius.lg", "radius.sm", "typography.control",
            "typography.body", "typography.label"
        ],
        environment: ["size", "density", "corners"],
        assets: ["icons/check.svg", "icons/close.svg", "icons/chevron_down.svg", "icons/chevron_up.svg"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Identity of the combobox; also its overlay id and the prefix of its inner keys (`<key>-trigger`, `<key>-panel`)." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the trigger, the list and the search field." },
            parent_overlay: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Key of the open overlay this one nests in, such as a Dialog; the panel stacks above it and closes with it." },
            options: #{ schema: #{ type: "array", max_items: 100000, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Value stored in `selected` when this option is picked; must be unique." },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Option text, matched by the query and joined into the trigger text when selected; must not be blank." },
                    keywords: #{ schema: #{ type: "array", max_items: 32, items: #{ type: "string" } }, required: false, sensitive: false,
                        "default": #{ type: "array", value: [] },
                        doc: "Extra terms the query also matches, case-insensitively." },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Shows the option but blocks picking it; keyboard navigation skips it." },
                    group: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Heading this option is listed under, shown in capitals; must not be blank." },
                },
            } }, required: true, sensitive: false,
                doc: "Options in display order; options that share a `group` are gathered where that group first appears." },
            mode: #{ schema: #{ type: "string", allowed: ["single", "multiple"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "single" },
                doc: "With `single`, a pick replaces the selection and closes the panel; with `multiple`, picks toggle values and it stays open." },
            selected: #{ schema: #{ type: "array", max_items: 100000, items: #{ type: "string" } }, required: true, sensitive: false,
                doc: "Selected option values, at most one in `single` mode; the caller stores the `change` payload and passes it back." },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether the panel is open; the caller stores the `open_change` payload and passes it back." },
            searchable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Shows a search field bound to `query` in the panel; without it, letter keys jump to matching options." },
            query: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Filters options by label or keyword, case-insensitively; the caller stores the `query_change` payload and passes it back." },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" }, doc: "Trigger text while nothing is selected." },
            search_placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Search" }, doc: "Placeholder of the search field." },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "No results" },
                doc: "Text shown when no option matches; the `empty` slot replaces it." },
            clear_label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Accessible name of the clear button shown with `clearable`; unset, the localized `common.clear` text." },
            trigger: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Custom trigger that replaces the default field with its clear button and chevron; the panel then takes `width`." },
            header: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Content at the top of the panel, above the search field." },
            footer: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Content at the bottom of the panel, below the options." },
            empty: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Content shown instead of `empty_text` when no option matches." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false }, doc: "Disables the trigger so the panel cannot open." },
            clearable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Shows a clear button in the default trigger while something is selected; it emits `change` with `[]`." },
            reset_query_on_close: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Emits `query_change` with `\"\"` whenever the panel closes or the selection is cleared." },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 32 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 },
                doc: "Rows the list shows before it scrolls; group headings count as rows." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false, doc: "Control size; leave unset to inherit the environment size." },
            width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false,
                doc: "Width of the default trigger, which the panel matches, or of the panel under a custom `trigger`; unset is 280 logical pixels." },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "bottom" }, doc: "Side of the trigger the panel opens on." },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the full new selection when an option is picked or the selection is cleared." },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the requested open state when the trigger is clicked, an arrow key opens it, a `single` pick closes it, or it is dismissed." },
            on_query_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the new search text as the user types, and with `\"\"` when `reset_query_on_close` resets it." },
        },
        state: #{ fields: #{
            active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        } },
        events: #{
            change: #{ payload: #{ type: "array", max_items: 100000, items: #{ type: "string" } },
                doc: "Emitted when a pick changes the selection or the selection is cleared; the payload is the full new selection." },
            open_change: #{ payload: #{ type: "bool" },
                doc: "Emitted when the panel should open or close; the payload is the requested open state." },
            query_change: #{ payload: #{ type: "string" },
                doc: "Emitted when the search text changes, or resets to `\"\"` under `reset_query_on_close`; the payload is the new query." },
        },
        slots: #{
            trigger: #{ required: false, multiple: false, doc: "Replaces the default field trigger." },
            header: #{ required: false, multiple: false, doc: "Content above the search field." },
            footer: #{ required: false, multiple: false, doc: "Content below the options." },
            empty: #{ required: false, multiple: false, doc: "Content shown when no option matches." },
        },
        parts: ["root", "trigger", "value", "placeholder", "indicator", "clear",
            "panel", "search", "list", "group", "option", "option_selected",
            "option_active", "option_disabled", "empty", "check", "indicator_bar",
            "focus_frame"],
    },
    render: Fn("render_Combobox")
});

fn Combobox(props) { render_component("components/combobox", props) }

// The 2px focus frame, drawn over the panel's edge while the panel itself holds focus: the
// panel's hairline (painted over its children) turns the focus color and this frame adds
// the inner pixel. A control inside shows its own frame instead.
fn panel_focus_frame(ctx) {
    row([]).with_style(ctx.component_style("focus_frame", style().absolute()
        .left(offset_px(-1)).right(offset_px(-1)).top(offset_px(-1)).bottom(offset_px(-1))
        .radius(theme_radius("lg")).border(px(2)).border_color(rgba(0x00000000))
        .group_focus(style().border_color(theme_color("focus_ring")))))
}
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn option_disabled(option) { prop_or(option, "disabled", false) }
fn option_group(option) {
    if "group" in option && option.group != () { option.group } else { () }
}

fn validate_combobox(props, selected) {
    let values = #{};
    for option in props.options {
        let label = option.label;
        label.trim();
        if label == "" { throw `combobox option '${option.value}' has an empty label`; }
        if option.value in values { throw `combobox option value '${option.value}' is duplicated`; }
        values[option.value] = true;
        let group = option_group(option);
        if group != () {
            let normalized_group = group;
            normalized_group.trim();
            if normalized_group == "" { throw `combobox option '${option.value}' has an empty group`; }
        }
    }
    if prop_or(props, "mode", "single") == "single" && selected.len > 1 {
        throw "single-select combobox cannot contain multiple selected values";
    }
    for value in selected {
        if !(value in values) { throw `combobox selection '${value}' does not match an option`; }
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
    ctx.emit("open_change", value);
    handled()
}
fn open_action_reset(ctx, value) {
    ctx.emit("open_change", value);
    if !value {
        ctx.set_state("active", "");
        ctx.emit("query_change", "");
    }
    handled()
}
fn query_action(ctx, value) {
    ctx.set_state("active", "");
    ctx.emit("query_change", value);
    handled()
}
fn navigate_action(ctx, payload) {
    ctx.set_state("active", payload.active);
    if payload.open {
        ctx.emit("open_change", true);
    }
    handled()
}
fn option_action(ctx, payload) {
    ctx.set_state("active", payload.active);
    if payload.changed {
        ctx.emit("change", payload.selected);
    }
    if payload.close {
        ctx.emit("open_change", false);
        if payload.reset_query {
            ctx.emit("query_change", "");
        }
    }
    handled()
}
fn clear_action(ctx, payload) {
    if payload.changed {
        ctx.emit("change", []);
    }
    if payload.reset_query {
        ctx.set_state("active", "");
        ctx.emit("query_change", "");
    }
    handled()
}

// List rows share one grammar: metrics.row high, text at metrics.inset, the cursor is
// a selection block plus a 2px accent indicator bar drawn inside the inset.
fn combobox_indicator(ctx) {
    row([]).with_style(ctx.component_style("indicator_bar", style()
        .absolute().inset_start(px(0)).top(px(0)).bottom(px(0))
        .width(px(2)).background(theme_color("accent"))))
}

fn render_combobox_row(total_rows, ctx, payload) {
    let item = payload.item;
    if item.kind == "group" {
        // Group labels align with option text, after the reserved check column.
        let column_spacer = row([]).with_style(style().width(theme_length("metrics.icon")));
        return row([column_spacer, text(item.label.to_upper())]).with_key(payload.key)
            .with_style(ctx.component_style("group",
            style().height(theme_length("metrics.row")).padding_x(theme_length("metrics.inset"))
                    .gap(theme_spacing("sm")).items_center().typography("label")
                    .text_color(theme_color("text_muted"))))
            .accessibility_role("presentation").accessibility_label(item.label);
    }
    let row_style = ctx.component_style("option",
        style().height(theme_length("metrics.row")).padding_x(theme_length("metrics.inset"))
            .items_center().gap(theme_spacing("sm")).typography("body")
            .text_color(theme_color("text_primary"))
            .hover(style().background(theme_color("surface_hover"))));
    let row_style = if item.selected {
        ctx.component_style("option_selected", row_style)
    } else { row_style };
    let row_style = if item.active {
        ctx.component_style("option_active", row_style.background(theme_color("selection"))
            .hover(style().background(theme_color("selection"))))
    } else { row_style };
    let row_style = if item.disabled {
        ctx.component_style("option_disabled", row_style.text_color(theme_color("disabled"))
            .hover(style()))
    } else { row_style };
    let children = [
        image_source(asset("app/icons/check")).with_style(ctx.component_style("check", style()
            .width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
            .text_color(theme_color("text.accent"))
            .opacity(if item.selected { 1.0 } else { 0.0 }))),
        text(item.label).with_style(style().flex_grow().min_width(px(0))
            .whitespace_nowrap().text_ellipsis())
    ];
    if item.active && !item.disabled { children.push(combobox_indicator(ctx)); }
    let node = row(children)
        .with_key(payload.key).with_style(row_style)
        .disabled(item.disabled).tab_stop(false)
        .accessibility_option(item.label, item.selected, payload.index + 1, total_rows)
        .accessibility_value(item.value);
    if item.disabled { node }
    else { node.on_click_value(Fn("option_action"), item.action) }
}

fn render_Combobox(ctx, props) {
    let selected = props.selected;
    validate_combobox(props, selected);
    let open = props.open;
    let query = props.query;
    let query_lower = query.to_lower();
    let mode = prop_or(props, "mode", "single");
    let reset_query = prop_or(props, "reset_query_on_close", false);
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
            data.push(#{ key: `group:${group.key}`, kind: "group", label: group.label });
        }
        for option in buckets[group.key] {
            let next = selection_for(selected, option.value, mode);
            data.push(#{ key: `option:${option.value}`, kind: "option", value: option.value,
                label: option.label, disabled: option_disabled(option),
                selected: selected_contains(selected, option.value), active: option.value == active,
                action: #{ selected: next, active: option.value,
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
    if !custom_trigger {
        trigger_children.push(text(if selected_label == "" { placeholder } else { selected_label })
            .with_key(`${props.key}-trigger-value`)
            .with_style(ctx.component_style(
                if selected_label == "" { "placeholder" } else { "value" },
                style().flex_grow().text_color(if selected_label == "" { theme_color("text_muted") }
                    else { theme_color("text_primary") }))));
        let trigger_actions = [];
        if prop_or(props, "clearable", false) && selected.len > 0 {
            let clear_label = if "clear_label" in props && props.clear_label != () {
                props.clear_label
            } else { ctx.t("common.clear") };
            let clear_style = ctx.component_style("clear", style()
                .width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
                .items_center().justify_center()
                .text_color(theme_color("text_muted")).radius(theme_radius("sm"))
                .hover(style().background(theme_color("surface_hover"))));
            let clear = image_source(asset("app/icons/close"))
                .with_key(`${props.key}-clear`).with_style(clear_style)
                .accessibility_role("button").accessibility_label(clear_label)
                .on_click_value(Fn("clear_action"),
                    #{ changed: true, reset_query: reset_query });
            trigger_actions.push(clear);
        }
        trigger_actions.push(image_source(asset(if open {
                "app/icons/chevron_up"
            } else { "app/icons/chevron_down" }))
            .with_key(`${props.key}-indicator`)
            .with_style(ctx.component_style("indicator",
                style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
                    .items_center().justify_center()
                    .text_color(theme_color("text_muted")))));
        trigger_children.push(row(trigger_actions).with_style(style().gap(theme_spacing("xs")).items_center()));
    }
    let trigger_style = if custom_trigger {
        style()
            .focus(style().border_color(theme_color("focus_ring")))
            .disabled(style().text_color(theme_color("disabled")).opacity(0.72))
    } else {
        // The trigger is a field: a framed well that takes the focus color while
        // focused or open.
        let field = style().width(relative(1.0)).min_width(px(0))
            .min_height(theme_length("metrics.control"))
            .padding_x(theme_length("metrics.field_pad")).gap(theme_spacing("sm")).items_center();
        field
            .border(px(2)).border_color(if open { theme_color("focus_ring") } else { theme_color("border") })
            .radius(theme_radius("md")).background(theme_color("surface_raised"))
            .typography("control")
            .focus(style().border_color(theme_color("focus_ring")))
            .disabled(style().text_color(theme_color("disabled")).opacity(0.72))
    };
    let trigger = if custom_trigger { props.trigger } else { row(trigger_children) };
    trigger = trigger.with_key(`${props.key}-trigger`)
        .with_style(ctx.component_style("trigger", trigger_style))
        .disabled(prop_or(props, "disabled", false)).tab_stop(true)
        .accessibility_role("combobox").accessibility_expanded(open)
        .accessibility_label(props.label).accessibility_value(selected_label);
    if enabled.len > 0 {
        trigger = trigger
            .on_key_value("up", Fn("navigate_action"),
                #{ active: if open { adjacent_value(enabled, active, -1) }
                    else { edge_value(enabled, true) }, open: !open })
            .on_key_value("down", Fn("navigate_action"),
                #{ active: if open { adjacent_value(enabled, active, 1) }
                    else { edge_value(enabled, false) }, open: !open })
            .on_key_value("home", Fn("navigate_action"),
                #{ active: edge_value(enabled, false), open: false })
            .on_key_value("end", Fn("navigate_action"),
                #{ active: edge_value(enabled, true), open: false });
        if open {
            for item in data {
                if item.kind == "option" && item.value == active {
                    trigger = trigger.on_key_value("enter", Fn("option_action"), item.action);
                }
            }
        }
    }

    let content = [];
    let header = "header" in props && props.header != ();
    let searchable = prop_or(props, "searchable", false);
    if header { content.push(props.header); }
    if searchable {
        // The search spans the panel on the rows' inset; without a header it heads the panel.
        content.push(input::Input(#{
            key: `${props.key}-search`, label: props.label, value: query,
            placeholder: prop_or(props, "search_placeholder", "Search"),
            appearance: "embedded",
            on_change: Fn("query_action")
        }).with_style(ctx.component_style("search", style())));
    }
    if ordered.len == 0 {
        content.push(if "empty" in props && props.empty != () { props.empty }
            else { row([text(prop_or(props, "empty_text", "No results"))])
            .with_style(ctx.component_style("empty", style().min_height(theme_length("metrics.row"))
                    .padding_x(theme_length("metrics.inset")).items_center()
                    .typography("body").text_color(theme_color("text_muted")))) });
    } else {
        // Rows are metrics.row high, so the viewport is a whole number of rows in
        // either density. The estimate only seeds the first frame.
        let visible = prop_or(props, "max_visible", 8);
        if data.len < visible { visible = data.len; }
        content.push(column([virtual_collection(#{
            key: `${props.key}-options`, label: props.label,
            data: data, estimated_height: 32, fill_height: true,
            overdraw_pixels: 64, alignment: "top", follow_tail: false
        }, Fn("render_combobox_row").curry(data.len))])
            .with_style(ctx.component_style("list", style()
                .height(theme_length("metrics.row") * visible))));
    }
    if "footer" in props && props.footer != () { content.push(props.footer); }
    content.push(panel_focus_frame(ctx));
    let panel = column(content).with_key(`${props.key}-panel`)
        .with_style(ctx.component_style("panel",
            style().relative().width(if custom_trigger { width } else { relative(1.0) })
            .min_width(px(0)).gap(theme_spacing("xs")).padding_y(theme_spacing("xs"))
                .padding_top(if searchable && !header { px(0) } else { theme_spacing("xs") })
                .border(px(1)).border_color(theme_color("border"))
                .radius(theme_radius("lg")).background(theme_color("surface_raised"))
                .focus(style().border_color(theme_color("focus_ring")))))
        .tab_stop(true).accessibility_role("listbox").accessibility_label(props.label);

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
        open: open, align: "start", gap: 4, modal: false, dismiss_on_escape: true, dismiss_on_outside: true,
        match_trigger_width: !custom_trigger
    };
    if "parent_overlay" in props && props.parent_overlay != () {
        overlay_config.parent = props.parent_overlay;
    }
    let root_style = if custom_trigger { style() }
        else { style().width(width).min_width(px(0)) };
    let node = overlay(trigger, panel, overlay_config)
        .with_key(props.key).with_style(ctx.component_style("root", root_style))
        .on_open_change(if reset_query { Fn("open_action_reset") } else { Fn("open_action") });
    if "size" in props && props.size != () { node = node.env(#{ size: props.size }); }
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
