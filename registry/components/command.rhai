/* gpui-rhai
{
  "id": "components/command",
  "export": "Command",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/input", "components/kbd"],
  "capabilities": {},
  "assets": ["icons/command_search_modern.svg", "icons/command_search_clean.svg", "icons/command_enter_modern.svg", "icons/command_enter_clean.svg"],
  "tokens": [
    "surface_raised", "border", "surface_hover", "selection", "text_primary",
    "text_muted", "disabled", "metrics.command_row", "metrics.command_narrow_row", "metrics.inset", "spacing.xs", "spacing.sm",
    "radius.md", "radius.lg", "typography.body", "typography.label", "command.search_asset", "command.enter_asset", "command.enter_label", "command.group_case", "command.narrow_footer", "scrollbar.thumb", "scrollbar.thumb_hover"
  ],
  "environment": ["size", "density"]
}
*/
// Embeddable keyboard-first command search with deterministic fuzzy ranking.
//
// State: caller owns query and the roving active command.
// Example: Command(#{ key: "palette", label: "Commands", query: query, active_value: active, items: items, on_query_change: Fn("queried"), on_active_change: Fn("preview"), on_action: Fn("run") })

import "components/input" as input;
import "components/kbd" as kbd;

define_component(#{
    metadata: #{ id: "components/command", "export": "Command", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/input", "components/kbd"], capabilities: #{},
        assets: ["icons/command_search_modern.svg", "icons/command_search_clean.svg", "icons/command_enter_modern.svg", "icons/command_enter_clean.svg"],
        tokens: [
            "surface_raised", "border", "surface_hover", "selection", "text_primary",
            "text_muted", "disabled", "metrics.command_row", "metrics.command_narrow_row", "metrics.inset", "spacing.xs", "spacing.sm",
            "radius.md", "radius.lg", "typography.body", "typography.label", "command.search_asset", "command.enter_asset", "command.enter_label", "command.group_case", "command.narrow_footer", "scrollbar.thumb", "scrollbar.thumb_hover"
        ],
        environment: ["size", "density"] },
    schema: #{
        props: #{
            appearance: #{ schema: #{type:"string",allowed:["panel","embedded"]},required:false,sensitive:false,
                "default":#{type:"string",value:"panel"},doc:"panel owns its tray/card chrome; embedded lets a surrounding Dialog or host panel own those edges, without changing the input or row identity." },
            footer_visible: #{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:true},
                doc:"Shows the keyboard-hint footer, or the caller's footer node. Hiding it does not change navigation or bindings."},
            footer: #{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,
                doc:"Optional footer content in place of the default navigation/Enter hints; no shortcut is registered."},
            scrollbar: #{ schema: #{ type: "string", allowed: ["auto", "always", "hidden"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "auto" }, doc: "Shared command-list scrollbar visibility; the search field keeps keyboard focus." },
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the search field and the command list." },
            query: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Search text; the caller stores the `query_change` payload and passes it back." },
            active_value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Highlighted command value; the caller stores the `active_change` payload. Other values show the first enabled match." },
            items: #{ schema: /* @contract command-items:start */ #{"type": "one_of", "variants": [#{"type": "array", "max_items": 4096, "items": #{"type": "object", "allow_unknown": false, "fields": #{"value": #{"schema": #{"type": "string"}, "required": true, "sensitive": false, "doc": "Unique non-empty value reported by active_change and action."}, "label": #{"schema": #{"type": "string"}, "required": true, "sensitive": false, "doc": "Required searchable/accessibility text; content may replace its visual label."}, "content": #{"schema": #{"type": "optional", "value": #{"type": "node"}}, "required": false, "sensitive": false, "doc": "Rich visual label; ranking and accessible name still use label."}, "hint": #{"schema": #{"type": "optional", "value": #{"type": "string"}}, "required": false, "sensitive": false, "doc": "Inline secondary text, also searched with keyword precedence."}, "icon": #{"schema": #{"type": "optional", "value": #{"type": "one_of", "variants": [#{"type": "node"}, #{"type": "string"}]}}, "required": false, "sensitive": false, "doc": "Non-interactive glyph node or logical asset ID; the leading slot is reserved when empty."}, "logo": #{"schema": #{"type": "optional", "value": #{"type": "one_of", "variants": [#{"type": "node"}, #{"type": "string"}]}}, "required": false, "sensitive": false, "doc": "Non-interactive logo node or logical asset ID on a tile; takes precedence over icon."}, "keywords": #{"schema": #{"type": "array", "max_items": 32, "items": #{"type": "string"}}, "required": false, "sensitive": false, "default": #{"type": "array", "value": []}, "doc": "Additional search terms with keyword precedence."}, "group": #{"schema": #{"type": "optional", "value": #{"type": "string"}}, "required": false, "sensitive": false, "doc": "Heading; groups preserve first matching source order."}, "shortcut": #{"schema": #{"type": "optional", "value": #{"type": "string"}}, "required": false, "sensitive": false, "doc": "Row-end shortcut legend overriding a registered action binding."}, "action": #{"schema": #{"type": "optional", "value": #{"type": "string"}}, "required": false, "sensitive": false, "doc": "Registered action dispatched on activation; its enabled state and shortcut are inherited."}, "disabled": #{"schema": #{"type": "bool"}, "required": false, "sensitive": false, "default": #{"type": "bool", "value": false}, "doc": "Dimmed and excluded from activation and navigation."}}}}, #{"type": "collection"}]} /* @contract command-items:end */, required: true, sensitive: false,
                doc: "Commands to rank: item maps, or a native collection keyed by value with the same textual fields and logical asset IDs for icon/logo, but no `content`, UI nodes or per-row `action`." },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Hint shown in the empty search field. Empty, the localized `command.placeholder` message, `Type a command` without one." },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Message shown in place of the list when no command matches. Empty, the localized `command.empty` message, `No commands found` without one." },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 24 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 },
                doc: "Maximum viewport capacity in command-row units; group headings use their natural measured height." },
            narrow_at: #{schema:#{type:"number",min:0.0,max:16384.0},required:false,sensitive:false,
                "default":#{type:"float",value:480.0},
                doc:"At or below this committed card width in logical pixels, use taller rows and hide visual hints/keycaps. This observes the actual card, not the window; initial unmeasured layout uses wide presentation."},
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Disables the whole command: the search field, the rows and the list's keys." },
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
        slots: #{footer:#{required:false,multiple:false,doc:"Optional keyboard hints or application-owned status."}},
        parts: ["root", "root_embedded", "card", "card_embedded", "search", "search_input", "search_placeholder", "search_icon", "search_hint", "search_hint_icon", "search_hint_text", "footer", "footer_hint", "list", "list_viewport", "group", "item", "item_narrow", "item_active", "icon_slot", "icon_slot_active", "logo_slot", "icon", "logo", "image_fallback", "text", "label", "hint", "shortcut", "shortcut_active", "empty",
            "scrollbar_track", "scrollbar_thumb", "scrollbar_thumb_hover"],
    },
    render: Fn("render_Command")
});

// The text prop `name`, or the localized `key` message (`fallback` without one) while it
// is empty or unset.
fn localized_prop(ctx, props, name, key, fallback) {
    let value = if name in props && props[name] != () { props[name] } else { "" };
    if value == "" { ctx.t_or(key, fallback) } else { value }
}

fn Command(props) { render_component("components/command", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn command_setting(ctx,name,fallback) {
    if ctx.theme_variant()==() {fallback}else{ctx.theme_token(`command.${name}`)}
}
fn collection_scrollbars(ctx, props, node) {
    node.with_part_style("scrollbar_track", ctx.component_style("scrollbar_track", style().background(rgba(0x00000000))))
        .with_part_style("scrollbar_thumb", ctx.component_style("scrollbar_thumb", style().background(theme_color("scrollbar.thumb"))))
        .with_part_style("scrollbar_thumb_hover", ctx.component_style("scrollbar_thumb_hover", style().background(theme_color("scrollbar.thumb_hover"))))
        .scrollbars("hidden", prop_or(props, "scrollbar", "auto"))
}
fn item_disabled(item) { "disabled" in item && item.disabled }
fn item_group(item) { if "group" in item && item.group != () { item.group } else { "" } }
fn item_hint(item) { if "hint" in item && item.hint != () {item.hint}else{""} }
fn item_asset(item,name) { if name in item && type_of(item[name])=="string" {item[name]}else{""} }
fn glyph_present(value) {value!=() && (type_of(value)!="string" || value!="")}
fn glyph_node(ctx,value,label) {
    if type_of(value)=="string" {
        image_source(asset(value)).image_fallback_text(text_initial(label))
            .with_part_style("image_fallback",ctx.component_style("image_fallback",style()))
    }else{value}
}

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
    let hint = item_hint(item);
    if hint!="" {let candidate=fuzzy_score(hint,query)-100;if candidate>score {score=candidate;}}
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
        for name in ["icon","logo"] {
            let id=item_asset(item,name);
            if id!="" {asset(id);}
        }
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
fn focus_search(ctx, reference_key) { ctx.focus(reference_key); handled() }
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

// Items have a stable command-row height; headings have their own natural
// text/padding extent. Both belong to the same measured native collection.

fn render_command_row(content, total_rows, narrow, ctx, payload) {
    let item = payload.item;
    if item.kind == "group" {
        let label = if command_setting(ctx,"group_case","preserve")=="upper" {item.label.to_upper()}else{item.label};
        return row([text(label)]).with_key(payload.key)
            .with_style(ctx.component_style("group", style()
                .padding_x(theme_length("metrics.inset"))
                .items_center().typography("label")
                .text_color(theme_color("text_muted"))))
            .accessibility_role("heading").accessibility_label(item.label)
            .accessibility_level(2);
    }
    let action = if "action" in item { item.action } else { () };
    let disabled = item.disabled || (action != () && !ctx.action_enabled(action));
    let shortcut = if item.shortcut != "" { key_shortcut(item.shortcut) }
        else if action != () {
            let bound = ctx.action_shortcut(action);
            if bound == () { key_shortcut("") } else { bound }
        } else { key_shortcut("") };
    let visual = if item.value in content {content[item.value]}else{#{}};
    let label = if "content" in visual {visual.content}else{text(item.label)};
    let logo = prop_or(visual,"logo",prop_or(item,"logo",""));
    let icon = prop_or(visual,"icon",prop_or(item,"icon",""));
    let has_logo = glyph_present(logo);
    let glyph = if has_logo {logo}else{icon};
    let leading = if glyph_present(glyph) {
        [glyph_node(ctx,glyph,item.label).with_style(ctx.component_style(if has_logo {"logo"}else{"icon"},style()))]
    }else{[]};
    let slot_parts = ["icon_slot"];
    if has_logo {slot_parts.push("logo_slot");}
    else if item.active {slot_parts.push("icon_slot_active");}
    let leading = row(leading).with_key("leading").with_style(ctx.component_style(slot_parts,
        style().items_center().justify_center().flex_shrink(false))).accessibility_hidden(true).tab_stop(false);
    let text_nodes = [label.with_style(ctx.component_style("label",style().min_width(px(0))
        .whitespace_nowrap().text_ellipsis()))];
    let hint = item_hint(item);
    if hint!="" {text_nodes.push(text(hint).with_key("hint").with_style(ctx.component_style("hint",style()
        .min_width(px(0)).whitespace_nowrap().text_ellipsis())).retained_page(!narrow));}
    // Keycaps are centered controls beside the label, not inline text whose
    // baseline or width is inferred from a concatenated legend.
    let children = [leading,row(text_nodes).with_key("text").with_style(ctx.component_style("text",style()
        .flex_grow().min_width(px(0)).items_baseline().gap(theme_spacing("sm"))))];
    if shortcut.label != "" {
        let caps = [];
        if shortcut.chords.is_empty() || shortcut.chords.len > 8 {
            // Literal legends are opaque text. Long valid sequences also stay
            // one display value rather than constructing an unbounded keycap
            // grid. Their complete binding/semantic metadata remains intact.
            caps.push(kbd::Kbd(#{key:`${payload.key}-shortcut-literal`,text:shortcut.label,appearance:"menu"}));
        }else{
            for chord_index in 0..shortcut.chords.len {
                let chord = shortcut.chords[chord_index];
                for key_index in 0..chord.keys.len {
                    let key = chord.keys[key_index];
                    caps.push(kbd::Kbd(#{key:`${payload.key}-shortcut-${chord_index}-${key_index}`,
                        text:key.label,label:key.spoken,appearance:"menu"}));
                }
            }
        }
        // Fixed grid columns retain binding order in RTL as well. Chords remain
        // structured in the SDK; the spoken full shortcut lives on the option.
        children.push(box(caps).with_key("shortcut").with_style(ctx.component_style(
            if item.active {["shortcut","shortcut_active"]}else{["shortcut"]},
            style().grid_cols_max_content(caps.len).items_center()))
            .accessibility_hidden(true).retained_page(!narrow));
    }
    let row_style = style()
        .height(theme_length(if narrow {"metrics.command_narrow_row"}else{"metrics.command_row"})).padding_x(theme_length("metrics.inset"))
        .radius(theme_radius("md")).gap(theme_spacing("sm")).items_center()
        .typography("body")
        .text_color(theme_color("text_primary"))
        .disabled(style().text_color(theme_color("disabled")));
    row_style = if item.active {
        row_style.background(theme_color("selection")).hover(style().background(theme_color("selection")))
    } else { row_style.hover(style().background(theme_color("surface_hover"))) };
    let item_parts=["item"];
    if narrow {item_parts.push("item_narrow");}
    if item.active {item_parts.push("item_active");}
    let node = row(children).with_key(payload.key)
        .with_style(ctx.component_style(item_parts, row_style))
        .tab_stop(false).disabled(disabled)
        .accessibility_option(item.label, item.active, payload.index + 1, total_rows);
    if hint!="" {node=node.accessibility_description(hint);}
    if shortcut.label != "" { node = node.accessibility_key_shortcuts(shortcut.label); }
    if disabled { node } else {
        node.on_hover_value(Fn("active_hover").curry(item.active), item.value)
            .on_click_value(activation_handler(action), item.value)
    }
}

fn array_command_model(ctx, items, query, row_height, stored) {
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
            let action = prop_or(item, "action", ());
            // A row whose action is disabled is disabled for the keys too.
            let ranked = #{ value: item.value, label: item.label,
                hint:item_hint(item),icon:item_asset(item,"icon"),logo:item_asset(item,"logo"),
                shortcut: if "shortcut" in item && item.shortcut != () { item.shortcut } else { "" },
                action: action,
                disabled: item_disabled(item) || (action != () && !ctx.action_enabled(action)),
                score: score, source_index: index };
            let visual = #{};
            for name in ["content","icon","logo"] {
                if name in item && item[name]!=() && type_of(item[name])!="string" {visual[name]=item[name];}
            }
            if len(visual)>0 {content[item.value]=visual;}
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
                label: item.label, hint:item.hint,icon:item.icon,logo:item.logo,
                shortcut: item.shortcut, action: item.action, disabled: item.disabled,
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
        hint_field:"hint",icon_field:"icon",logo_field:"logo",
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
    let container_ref=element_ref("command-container");
    let width_class=ctx.element_width_class(container_ref,[props.narrow_at]);
    let narrow=width_class!=() && width_class==0;
    let row_metric=theme_length(if narrow {"metrics.command_narrow_row"}else{"metrics.command_row"});
    let query = props.query;
    // Native data carries an initial estimate; layout and the viewport's row
    // capacity share the late-bound command-row metric, including density/scale.
    let row_height = 32;
    let stored = props.active_value;
    let model = if is_native_collection(props.items) {
        native_command_model(props.items, query, row_height, stored)
    } else {
        validate_array_items(props.items);
        array_command_model(ctx, props.items, query, row_height, stored)
    };
    let data = model.data;
    let active = model.active;
    let search_ref = element_ref("command-search-input");
    let search = input::Input(#{ key: `${props.key}-search`, label: props.label, value: query,
        placeholder: localized_prop(ctx, props, "placeholder", "command.placeholder", "Type a command"),
        appearance: "embedded", size: "lg",
        disabled: prop_or(props, "disabled", false),
        autofocus: prop_or(props, "autofocus", false),
        on_change: Fn("query_action"),
        on_submit: Fn("submit_action").curry(#{ value: active, action: model.active_action }),
        part_styles:#{placeholder:ctx.component_style("search_placeholder",style())} })
        .with_ref(search_ref)
        .with_style(ctx.component_style("search_input", style().flex_grow().flex_basis(relative(0))
            .min_width(px(0)).min_height(px(0)).padding(px(0)).border(px(0)).radius(px(0))
            .background(rgba(0x00000000)).shadows([]).focus(style().shadows([]))));
    let enter_label = command_setting(ctx,"enter_label","ENTER");
    let search = row([
        image_source(asset(command_setting(ctx,"search_asset","app/icons/command_search_modern"))).with_key("search-icon")
            .with_style(ctx.component_style("search_icon",style().flex_shrink(false))).accessibility_hidden(true),
        search,
        row([
            image_source(asset(command_setting(ctx,"enter_asset","app/icons/command_enter_modern"))).with_key("enter-icon")
                .with_style(ctx.component_style("search_hint_icon",style().flex_shrink(false))),
            text(if enter_label=="" {""}else{ctx.t_or("command.enter_hint",enter_label)})
                .with_style(ctx.component_style("search_hint_text",style().whitespace_nowrap())),
        ]).with_key("enter-hint").with_style(ctx.component_style("search_hint",style().items_center().flex_shrink(false)))
            .accessibility_hidden(true).retained_page(!narrow),
    ]).with_key("search").with_style(ctx.component_style("search",style().items_center().cursor_text()))
        .on_click_value(Fn("focus_search"),"command-search-input").tab_stop(false).accessibility_role("group");
    let body = if data.len == 0 {
        row([text(localized_prop(ctx, props, "empty_text", "command.empty", "No commands found"))])
            .with_style(ctx.component_style("empty", style()
                .min_height(row_metric * 3).padding_x(theme_length("metrics.inset"))
                .items_center().justify_center()
                .typography("body").text_color(theme_color("text_muted"))))
            .accessibility_role("status")
    } else {
        let max_visible = prop_or(props, "max_visible", 8);
        let collection = #{ key: `${props.key}-list`, label: props.label, data: data,
            estimated_height: row_height, max_height: row_metric * max_visible,
            overdraw_pixels: row_height * 2, alignment: "top", follow_tail: false,
        };
        if active != "" {
            collection.reveal_key = if is_native_collection(data) { active } else { `item:${active}` };
        }
        // Short results fit actual measured rows, including natural headings;
        // large results keep one bounded viewport without eager measurement.
        let viewport = column([collection_scrollbars(ctx, props, virtual_collection(collection,
            Fn("render_command_row").curry(model.content).curry(data.len).curry(narrow))
            .accessibility_role("listbox").accessibility_label(props.label).tab_stop(false))])
            .with_key("viewport").with_style(ctx.component_style("list_viewport", style()
                .min_height(px(0))));
        column([viewport]).with_key("list").with_style(ctx.component_style("list",style().min_width(px(0)).min_height(px(0))))
    };
    let embedded = props.appearance=="embedded";
    let card = column([search,body]).with_key("card").with_ref(container_ref)
        .with_style(ctx.component_style(if embedded {["card","card_embedded"]}else{["card"]},
            style().min_width(px(0)).min_height(px(0)).gap(px(0))));
    // Only search/results own these keys. An application footer may contain
    // real controls; Enter or arrows there must not activate a command as well.
    if model.enabled {
        card = card.on_key_value("up", Fn("active_action").curry(active), model.previous)
            .on_key_value("down", Fn("active_action").curry(active), model.next)
            .on_key_value("home", Fn("active_action").curry(active), model.first)
            .on_key_value("end", Fn("active_action").curry(active), model.last)
            .on_key_value("enter", activation_handler(model.active_action), active);
    }
    let children = [card];
    if props.footer_visible {
        let custom_footer="footer" in props && props.footer!=();
        let footer = if custom_footer {props.footer}else{row([
            row([kbd::Kbd(#{key:"navigate-hint",keys:["up","down"],appearance:"menu"}),
                text(ctx.t_or("command.navigate_hint","Navigate"))])
                .with_style(ctx.component_style("footer_hint",style().items_center())),
            row([kbd::Kbd(#{key:"activate-hint",keys:"enter",appearance:"menu"}),
                text(ctx.t_or("command.run_hint","Run"))])
                .with_key("run-hint").with_style(ctx.component_style("footer_hint",style().items_center())).retained_page(!narrow),
        ]).with_style(style().width(relative(1.0)).min_width(px(0)).items_center().justify_between())};
        let hide_footer=narrow && !custom_footer && command_setting(ctx,"narrow_footer","visible")=="hidden";
        children.push(row([footer]).with_key("footer").with_style(ctx.component_style("footer",style().min_width(px(0)).items_center().flex_shrink(false))).retained_page(!hide_footer));
    }
    let root = column(children).with_key(props.key)
        .with_style(ctx.component_style(if embedded {["root","root_embedded"]}else{["root"]}, style()
            .gap(px(0)).min_width(px(0)).min_height(px(0)).padding(theme_radius("lg") * 0.5).radius(theme_radius("lg"))
            .border(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface_raised"))))
        .tab_stop(false).accessibility_role("group").accessibility_label(props.label);
    // A disabled container disables every row natively.
    if prop_or(props, "disabled", false) { return root.disabled(true); }
    root
}
