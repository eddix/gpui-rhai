/* gpui-rhai
{
  "id": "components/combobox",
  "export": "Combobox",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/input"],
  "capabilities": {},
  "assets": ["icons/check.svg", "icons/close.svg", "icons/chevron_down.svg", "icons/chevron_up.svg"],
  "tokens": [
    "surface_raised", "border", "control.border", "focus_ring", "surface_hover", "selection",
    "text.accent", "text_primary", "text_muted", "disabled", "metrics.control",
    "metrics.field_pad", "metrics.row", "metrics.inset", "metrics.icon", "spacing.xs",
    "spacing.sm", "radius.xs", "radius.md", "radius.lg", "typography.control",
    "typography.body", "typography.label", "scrollbar.thumb", "scrollbar.thumb_hover"
  ],
  "environment": ["size", "density"]
}
*/
// Public Rhai choice composition over Overlay, Input, and virtual_collection.
// State: caller owns selected/open/query; component owns only active-option navigation.
// Example: Combobox(#{ key: "country", label: "Country", options: options, selected: ["uk"], open: false, query: "", on_change: Fn("changed") })

import "components/input" as input;

define_component(#{
    metadata: #{ id: "components/combobox", "export": "Combobox", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/input"], capabilities: #{},
        tokens: [
            "surface_raised", "border", "control.border", "focus_ring", "surface_hover", "selection",
            "text.accent", "text_primary", "text_muted", "disabled", "metrics.control",
            "metrics.field_pad", "metrics.row", "metrics.inset", "metrics.icon", "spacing.xs",
            "spacing.sm", "radius.xs", "radius.md", "radius.lg", "typography.control",
            "typography.body", "typography.label", "scrollbar.thumb", "scrollbar.thumb_hover"
        ],
        environment: ["size", "density"],
        assets: ["icons/check.svg", "icons/close.svg", "icons/chevron_down.svg", "icons/chevron_up.svg"] },
    schema: #{
        props: #{
            scrollbar: #{ schema: #{ type: "string", allowed: ["auto", "always", "hidden"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "auto" }, doc: "Shared option-list scrollbar visibility; the combobox keeps keyboard ownership." },
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
                doc: "Selected option values, at most one in `single` mode, kept when no option has them (the trigger then shows the value itself); the caller stores the `change` payload and passes it back." },
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
                doc: "Custom trigger that replaces the default field with its clear button and chevron; the panel then takes `width`, or its content's width with `\"content\"`." },
            header: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Content at the top of the panel, above the search field." },
            footer: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Content at the bottom of the panel, below the options." },
            empty: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Content shown instead of `empty_text` when no option matches." },
            option_trailing: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Node factory bound with ctx.node_factory(Fn(...)), called as Fn(ctx, option) for content after an option's label, such as a Tag; option is #{ value, label, selected, active, disabled, place }, `place` being `option` in the list and `trigger` beside the value of a single-mode default trigger. Curry the caller's data into the factory to look an option up by value. The label gives way first; a content width measures it too." },
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
            width: #{ schema: #{ type: "optional", value: #{ type: "one_of", variants: [
                #{ type: "length" }, #{ type: "string", allowed: ["content"] }] } }, required: false, sensitive: false,
                doc: "Width of the default trigger, or of the panel under a custom `trigger`; unset is 280 logical pixels. `\"content\"` fits the widest option (and the placeholder), at most 360 logical pixels; past 64 options it measures the 16 longest labels." },
            panel_width: #{ schema: #{ type: "string", allowed: ["trigger", "content"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "trigger" },
                doc: "With the default trigger: `trigger` makes the panel exactly as wide as the trigger; `content` makes it at least that wide and wider for longer options, whose labels it measures up to 480 logical pixels." },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "bottom" }, doc: "Side of the trigger the panel opens on." },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the full new selection when an option is picked or the selection is cleared." },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the requested open state when the trigger is clicked, an arrow key opens it, a `single` pick closes it, or it is dismissed." },
            on_query_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the new search text as the user types, and with `\"\"` when `reset_query_on_close` resets it." },
            on_submit: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the query when Enter is pressed in the search field and no enabled option matches it, so typed free text can be taken; while an option is active Enter picks it instead." },
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
            submit: #{ payload: #{ type: "string" },
                doc: "Emitted when Enter is pressed in the search field while no enabled option matches the query, only with `on_submit` set; the payload is the query. The panel stays open." },
        },
        slots: #{
            trigger: #{ required: false, multiple: false, doc: "Replaces the default field trigger." },
            header: #{ required: false, multiple: false, doc: "Content above the search field." },
            footer: #{ required: false, multiple: false, doc: "Content below the options." },
            empty: #{ required: false, multiple: false, doc: "Content shown when no option matches." },
        },
        parts: ["root", "trigger", "value", "placeholder", "indicator", "clear",
            "panel", "search", "list", "group", "option", "option_selected",
            "option_active", "option_disabled", "empty", "check", "trailing",
            "focus_frame", "scrollbar_track", "scrollbar_thumb", "scrollbar_thumb_hover"],
    },
    render: Fn("render_Combobox")
});

fn Combobox(props) { render_component("components/combobox", props) }

// The options a content width is measured from: every option of a short list, else the
// 16 with the longest labels by character count, so a long list does not lay every one out.
fn width_probe_options(options) {
    if options.len <= 64 { return options; }
    let longest = [];
    for option in options {
        let chars = option.label.len;
        if longest.len < 16 || chars > longest[longest.len - 1].label.len {
            let at = 0;
            while at < longest.len && longest[at].label.len >= chars { at += 1; }
            longest.insert(at, option);
            if longest.len > 16 { longest.pop(); }
        }
    }
    longest
}

// The caller's content after an option's label (`option_trailing`), or () without it.
fn option_trailing(ctx, factory, value, label, flags) {
    if factory == () { return (); }
    let detail = #{ value: value, label: label, selected: flags.selected, active: flags.active,
        disabled: flags.disabled, place: flags.place };
    row([ctx.render_node(factory, detail)]).with_style(ctx.component_style("trailing",
        style().flex_shrink(false).items_center()))
}

// Lays rows out without showing them, so its width is the widest row's: what a content-sized
// trigger or panel measures. Invisible, it is not painted or exposed to accessibility.
fn width_probe(rows) {
    column(rows).with_style(style().height(px(0)).overflow_hidden().invisible())
}

// The panel's own width: under a custom trigger, `width` unless it is "content"; with the
// default trigger, the trigger's (the overlay sets it), or its content's at least that wide.
fn panel_width_style(custom_trigger, width, content_width, panel_content) {
    if custom_trigger {
        if content_width { style().relative() } else { style().relative().width(width) }
    } else if panel_content {
        style().relative()
    } else {
        style().relative().width(relative(1.0))
    }
}

// Square option rows clear the panel's rounded corner: every side pads by half of
// radius.lg (clipping is rectangular). Menu and Command pad their panels the same way.
fn list_panel_padding() { theme_radius("lg") * 0.5 }

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
fn collection_scrollbars(ctx, props, node) {
    node.with_part_style("scrollbar_track", ctx.component_style("scrollbar_track", style().background(rgba(0x00000000))))
        .with_part_style("scrollbar_thumb", ctx.component_style("scrollbar_thumb", style().background(theme_color("scrollbar.thumb"))))
        .with_part_style("scrollbar_thumb_hover", ctx.component_style("scrollbar_thumb_hover", style().background(theme_color("scrollbar.thumb_hover"))))
        .scrollbars("hidden", prop_or(props, "scrollbar", "auto"))
}
fn option_disabled(option) { prop_or(option, "disabled", false) }
fn option_group(option) {
    if "group" in option && option.group != () { option.group } else { () }
}

// Returns the option values as a map from value to true. A selected value no option has is
// not an error: options often arrive after the first render, or are cleared while they reload.
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
    values
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
fn submit_action(ctx, value) {
    ctx.emit("submit", value);
    handled()
}
fn navigate_action(ctx, payload) {
    ctx.set_state("active", payload.active);
    if payload.open {
        ctx.emit("open_change", true);
    }
    handled()
}
// A pick builds the next selection for the picked option alone, from the selection the
// combobox was rendered with, so the option rows carry only their selected flag.
fn option_action(mode, selected, reset_query, ctx, value) {
    let next = selection_for(selected, value, mode);
    ctx.set_state("active", value);
    if selection_changed(selected, next) {
        ctx.emit("change", next);
    }
    if mode == "single" {
        ctx.emit("open_change", false);
        if reset_query {
            ctx.emit("query_change", "");
        }
    }
    handled()
}
fn pick_action(mode, selected, reset_query) {
    Fn("option_action").curry(mode).curry(selected).curry(reset_query)
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
// a selection block with a control's corners, inside the panel's padding.

fn render_combobox_row(total_rows, mode, selected, reset_query, trailing, ctx, payload) {
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
            .radius(theme_radius("md")).items_center().gap(theme_spacing("sm")).typography("body")
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
    let extra = option_trailing(ctx, trailing, item.value, item.label,
        #{ selected: item.selected, active: item.active, disabled: item.disabled, place: "option" });
    if extra != () { children.push(extra); }
    let node = row(children)
        .with_key(payload.key).with_style(row_style)
        .disabled(item.disabled).tab_stop(false)
        .accessibility_option(item.label, item.selected, payload.index + 1, total_rows)
        .accessibility_value(item.value);
    if item.disabled { node }
    else { node.on_click_value(pick_action(mode, selected, reset_query), item.value) }
}

fn render_Combobox(ctx, props) {
    let selected = props.selected;
    let values = validate_combobox(props, selected);
    let open = props.open;
    let query = props.query;
    let query_lower = query.to_lower();
    let mode = prop_or(props, "mode", "single");
    let reset_query = prop_or(props, "reset_query_on_close", false);
    let width = if "width" in props && props.width != () { props.width } else { px(280) };
    let content_width = type_of(width) == "string";
    let panel_content = prop_or(props, "panel_width", "trigger") == "content";
    let probe_options = if content_width || panel_content {
        width_probe_options(props.options)
    } else { [] };
    let trailing = if "option_trailing" in props { props.option_trailing } else { () };

    let chosen = #{};
    for value in selected { chosen[value] = true; }
    let group_order = [];
    let buckets = #{};
    let labels = [];
    for option in props.options {
        if option.value in chosen { labels.push(option.label); }
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
    // A selected value no option has stays selected and shows as its own text, after the
    // known labels, so the user sees the stored selection; the list has no row for it.
    for value in selected { if !(value in values) { labels.push(value); } }
    let ordered = [];
    for group in group_order { for option in buckets[group.key] { ordered.push(option); } }
    let active = active_value(ordered, selected, ctx.get_state("active"));
    let enabled = enabled_options(ordered);

    // The rows stay linear in the options: each carries its selected flag, and the selection
    // reaches the row renderer and the Enter handlers once, so the data does not grow with
    // options times selected values.
    let data = [];
    for group in group_order {
        if group.label != () {
            data.push(#{ key: `group:${group.key}`, kind: "group", label: group.label });
        }
        for option in buckets[group.key] {
            data.push(#{ key: `option:${option.value}`, kind: "option", value: option.value,
                label: option.label, disabled: option_disabled(option),
                selected: option.value in chosen, active: option.value == active });
        }
    }

    let selected_label = "";
    for label in labels {
        selected_label += if selected_label == "" { label } else { `, ${label}` };
    }
    let placeholder = prop_or(props, "placeholder", "");
    let custom_trigger = "trigger" in props && props.trigger != ();
    let trigger_trailing = if trailing != () && mode == "single" && selected.len == 1
        && selected[0] in values && labels.len == 1 {
        option_trailing(ctx, trailing, selected[0], labels[0],
            #{ selected: true, active: false, disabled: false, place: "trigger" })
    } else { () };
    let trigger_children = [];
    if !custom_trigger {
        // One line that gives way first: a long label or placeholder ends in an ellipsis so the
        // clear button and the chevron stay inside a fixed-width trigger.
        let value = text(if selected_label == "" { placeholder } else { selected_label })
            .with_key(`${props.key}-trigger-value`)
            .with_style(ctx.component_style(
                if selected_label == "" { "placeholder" } else { "value" },
                style().flex_grow().min_width(px(0)).whitespace_nowrap().text_ellipsis()
                    .text_color(if selected_label == "" { theme_color("text_muted") }
                    else { theme_color("text_primary") })));
        if content_width {
            // The value sits over the probe, whose width is the widest option's, its
            // trailing content included.
            let rows = [text(placeholder).with_style(style().whitespace_nowrap())];
            for option in probe_options {
                let cells = [text(option.label).with_style(style().whitespace_nowrap())];
                let extra = option_trailing(ctx, trailing, option.value, option.label,
                    #{ selected: true, active: false, disabled: false, place: "trigger" });
                if extra != () { cells.push(extra); }
                rows.push(row(cells).with_style(style().gap(theme_spacing("sm"))));
            }
            value = column([value, width_probe(rows)])
                .with_style(style().flex_grow().min_width(px(0)));
        }
        trigger_children.push(value);
        if trigger_trailing != () { trigger_children.push(trigger_trailing); }
        let trigger_actions = [];
        if prop_or(props, "clearable", false) && selected.len > 0 {
            let clear_label = if "clear_label" in props && props.clear_label != () {
                props.clear_label
            } else { ctx.t("common.clear") };
            let clear_style = ctx.component_style("clear", style()
                .width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
                .items_center().justify_center()
                .text_color(theme_color("text_muted")).radius(theme_radius("xs"))
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
            .border(px(2)).border_color(if open { theme_color("focus_ring") } else { theme_color("control.border") })
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
                    trigger = trigger.on_key_value("enter",
                        pick_action(mode, selected, reset_query), item.value);
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
        let search = #{
            key: `${props.key}-search`, label: props.label, value: query,
            placeholder: prop_or(props, "search_placeholder", "Search"),
            appearance: "embedded",
            on_change: Fn("query_action")
        };
        // With nothing to pick, Enter in the field submits the typed text; otherwise it reaches
        // the panel, which picks the active option.
        if enabled.len == 0 && "on_submit" in props && props.on_submit != () {
            search.on_submit = Fn("submit_action");
        }
        content.push(input::Input(search).with_style(ctx.component_style("search", style())));
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
        let list = [collection_scrollbars(ctx, props, virtual_collection(#{
            key: `${props.key}-options`, label: props.label,
            data: data, estimated_height: 32, fill_height: true,
            overdraw_pixels: 64, alignment: "top", follow_tail: false
        }, Fn("render_combobox_row").curry(data.len).curry(mode).curry(selected)
            .curry(reset_query).curry(trailing)))];
        if panel_content || (custom_trigger && content_width) {
            // Option rows as render_combobox_row lays them out: the check column, the label.
            let rows = [];
            for option in probe_options {
                let cells = [
                    row([]).with_style(style().width(theme_length("metrics.icon"))),
                    text(option.label).with_style(style().whitespace_nowrap().max_width(px(480)))
                ];
                let extra = option_trailing(ctx, trailing, option.value, option.label,
                    #{ selected: false, active: false, disabled: false, place: "option" });
                if extra != () { cells.push(extra); }
                rows.push(row(cells).with_style(style().padding_x(theme_length("metrics.inset"))
                    .gap(theme_spacing("sm")).typography("body")));
            }
            list.push(width_probe(rows));
        }
        content.push(column(list)
            .with_style(ctx.component_style("list", style()
                .height(theme_length("metrics.row") * visible))));
    }
    if "footer" in props && props.footer != () { content.push(props.footer); }
    content.push(panel_focus_frame(ctx));
    let panel = column(content).with_key(`${props.key}-panel`)
        .with_style(ctx.component_style("panel",
            panel_width_style(custom_trigger, width, content_width, panel_content)
            .min_width(px(0)).gap(theme_spacing("xs")).padding(list_panel_padding())
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
                panel = panel.on_key_value("enter",
                    pick_action(mode, selected, reset_query), item.value);
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
        match_trigger_width: !custom_trigger && !panel_content,
        min_trigger_width: !custom_trigger && panel_content
    };
    if "parent_overlay" in props && props.parent_overlay != () {
        overlay_config.parent = props.parent_overlay;
    }
    let root_style = if custom_trigger { style() }
        else if content_width { style().max_width(px(360)).min_width(px(0)) }
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
