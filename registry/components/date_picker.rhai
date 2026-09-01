/* gpui-rhai
{
  "id": "components/date_picker",
  "export": "DatePicker",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/calendar.svg", "icons/date_previous.svg", "icons/date_next.svg", "icons/close.svg"]
}
*/
// Controlled single-date field composed from public date data helpers, atoms, and Overlay.
// Props: key, optional strict ISO value/min/max, localized presentation, presets, and on_change.
// Events: change(optional<string>). State: keyed open/visible-month/focused-date transients.
// Example: DatePicker(#{ key: "appointment", value: "2026-09-01", on_change: Fn("changed") })

define_component(#{
    metadata: #{
        id: "components/date_picker", "export": "DatePicker", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{},
        assets: ["icons/calendar.svg", "icons/date_previous.svg", "icons/date_next.svg", "icons/close.svg"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            parent_overlay: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            value: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            min_date: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            max_date: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" } },
            display_style: #{ schema: #{ type: "string", allowed: ["short", "medium", "long"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "short" } },
            clearable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            presets: #{ schema: #{ type: "array", max_items: 32, items: #{
                type: "object", allow_unknown: false, fields: #{
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                },
            } }, required: false, sensitive: false,
                "default": #{ type: "array", value: [] } },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            error: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            size: #{ schema: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "md" } },
            width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "bottom" } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{
            open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
            visible_month: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
            focused_date: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
            anchor_date: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        } },
        events: #{ change: #{ payload: #{ type: "optional", value: #{ type: "string" } } } },
        slots: #{},
        parts: ["root", "trigger", "trigger_value", "trigger_icon", "panel", "header",
            "month_title", "previous", "next", "weekdays", "weekday", "grid", "week",
            "day", "day_outside", "day_today", "day_selected", "day_focused",
            "day_disabled", "footer", "preset", "clear"],
        effects: ["controlled_anchor"],
    },
    render: Fn("render_DatePicker")
});

fn DatePicker(props) { render_component("components/date_picker", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn optional_prop(props, name) { if name in props && props[name] != () { props[name] } else { () } }
fn date_picker_size(size) {
    if size == "xs" { 24 }
    else if size == "sm" { 28 }
    else if size == "lg" { 36 }
    else { 32 }
}
fn weekday_index(name) {
    if name == "monday" { 1 }
    else if name == "tuesday" { 2 }
    else if name == "wednesday" { 3 }
    else if name == "thursday" { 4 }
    else if name == "friday" { 5 }
    else if name == "saturday" { 6 }
    else { 0 }
}
fn date_enabled(date, min_date, max_date) {
    date != () && (min_date == () || date >= min_date) && (max_date == () || date <= max_date)
}
fn checked_or(value, fallback) { if value == () { fallback } else { value } }

fn validate_date_picker(props, value, min_date, max_date, today) {
    date_info(today);
    if min_date != () { date_info(min_date); }
    if max_date != () { date_info(max_date); }
    if min_date != () && max_date != () && min_date > max_date {
        throw "DatePicker min_date cannot be after max_date";
    }
    if value != () {
        date_info(value);
        if !date_enabled(value, min_date, max_date) {
            throw "DatePicker controlled value is outside min_date/max_date";
        }
    }
    let preset_values = #{};
    for preset in prop_or(props, "presets", []) {
        date_info(preset.value);
        let label = preset.label;
        label.trim();
        if label == "" { throw "DatePicker preset labels cannot be empty"; }
        if preset.value in preset_values {
            throw `DatePicker preset date '${preset.value}' is duplicated`;
        }
        preset_values[preset.value] = true;
    }
}

fn sync_anchor(ctx, payload) {
    ctx.set_state("anchor_date", payload.anchor);
    ctx.set_state("visible_month", date_month_start(payload.anchor));
    ctx.set_state("focused_date", payload.anchor);
}
fn cleanup_anchor(ctx, payload) { () }
fn set_open(ctx, open) {
    ctx.set_state("open", open);
    if !open {
        ctx.set_state("visible_month", "");
        ctx.set_state("focused_date", "");
    }
    handled()
}
fn navigate_date(ctx, payload) {
    ctx.set_state("focused_date", payload.date);
    ctx.set_state("visible_month", date_month_start(payload.date));
    handled()
}
fn move_month(ctx, payload) {
    ctx.set_state("focused_date", payload.date);
    ctx.set_state("visible_month", date_month_start(payload.date));
    handled()
}
fn choose_date(ctx, payload) {
    if payload.changed { ctx.emit("change", payload.date); }
    ctx.set_state("open", false);
    ctx.set_state("visible_month", "");
    ctx.set_state("focused_date", "");
    handled()
}
fn clear_date(ctx, payload) {
    if payload.changed { ctx.emit("change", ()); }
    ctx.set_state("open", false);
    ctx.set_state("visible_month", "");
    ctx.set_state("focused_date", "");
    handled()
}

fn date_target(value, days, min_date, max_date) {
    let target = date_checked_add_days(value, days);
    if target == () { value } else { date_clamp(target, min_date, max_date) }
}
fn month_target(value, months, min_date, max_date) {
    let target = date_checked_add_months(value, months);
    if target == () || !date_month_intersects(target, min_date, max_date) { value }
    else { date_clamp(target, min_date, max_date) }
}
fn week_target(value, first_weekday, end, min_date, max_date) {
    let target = date_week_edge(value, first_weekday, end);
    if target == () { value } else { date_clamp(target, min_date, max_date) }
}

fn icon_node(source, rtl_source, label, size, part_style) {
    directional_image_source(asset(source), asset(rtl_source))
        .with_style(part_style.width(px(size)).height(px(size)))
        .accessibility_role("image").accessibility_label(label)
}

fn render_DatePicker(ctx, props) {
    let value = optional_prop(props, "value");
    let min_date = optional_prop(props, "min_date");
    let max_date = optional_prop(props, "max_date");
    let today = ctx.today();
    validate_date_picker(props, value, min_date, max_date, today);
    let anchor = date_clamp(if value == () { today } else { value }, min_date, max_date);
    let open = ctx.get_state("open");
    effect("controlled_anchor", #{ anchor: anchor }, Fn("sync_anchor"), Fn("cleanup_anchor"));

    let stored_month = ctx.get_state("visible_month");
    let stored_focus = ctx.get_state("focused_date");
    let anchor_changed = ctx.get_state("anchor_date") != anchor;
    let visible_month = if stored_month == "" || anchor_changed {
        date_month_start(anchor)
    } else { stored_month };
    let focused_date = if stored_focus == "" || anchor_changed { anchor } else { stored_focus };
    let calendar = ctx.calendar();
    let size = date_picker_size(prop_or(props, "size", "md"));
    let width = if "width" in props && props.width != () { props.width } else { px(280) };
    let panel_width = size * 7 + 32;
    let placeholder = prop_or(props, "placeholder", "");
    let display = if value == () { placeholder }
        else { ctx.format_date(value, prop_or(props, "display_style", "short")) };
    let error = prop_or(props, "error", false);

    let trigger_value = text(display).with_style(component_style(props, "trigger_value",
        style().flex_grow().text_color(if value == () { theme_color("text_muted") }
            else { theme_color("text_primary") })));
    let trigger_actions = [];
    if prop_or(props, "clearable", false) && value != () {
        trigger_actions.push(row([image_source(asset("app/icons/close"))
            .with_style(style().width(px(12)).height(px(12))
                .text_color(theme_color("text_primary")))])
            .with_style(component_style(props, "clear", style()
                .width(px(20)).height(px(20)).items_center().justify_center()
                .radius(theme_radius("sm"))
                .hover(style().background(theme_color("surface_hover")))))
            .accessibility_role("button").accessibility_label(ctx.t("calendar.clear"))
            .on_click_value(Fn("clear_date"), #{ changed: true }));
    }
    trigger_actions.push(row([image_source(asset("app/icons/calendar"))
        .with_style(style().width(px(14)).height(px(14))
            .text_color(theme_color("text_muted")))])
        .with_style(component_style(props, "trigger_icon", style()
            .width(px(20)).height(px(20)).items_center().justify_center()
            .text_color(theme_color("text_muted"))))
        .accessibility_role("image").accessibility_label(ctx.t("calendar.open")));
    let trigger_children = [trigger_value, row(trigger_actions)
        .with_style(style().gap(px(4)).items_center())];
    let trigger_border = if error { theme_color("danger") }
        else if open { theme_color("focus_ring") } else { theme_color("border") };
    let trigger_style = style().width(width).height(px(size)).padding_x(px(10)).gap(px(8))
        .items_center().border(px(1)).border_color(trigger_border)
        .radius(theme_radius("md"))
        .background(if open { theme_color("surface_raised") } else { theme_color("surface") })
        .font_size(px(12)).line_height(px(16))
        .hover(style().background(theme_color("surface_raised")))
        .focus(style().border_color(theme_color("focus_ring")))
        .disabled(style().background(theme_color("surface_raised"))
            .text_color(theme_color("disabled")).opacity(0.72));
    let trigger = row(trigger_children).with_key(`${props.key}-trigger`)
        .with_style(component_style(props, "trigger", trigger_style))
        .disabled(prop_or(props, "disabled", false)).tab_stop(true)
        .accessibility_role("combobox").accessibility_checked(open)
        .accessibility_label(placeholder).accessibility_value(value)
        .accessibility_invalid(error);

    let previous = date_checked_add_months(focused_date, -1);
    let next = date_checked_add_months(focused_date, 1);
    let can_previous = previous != () && date_month_intersects(previous, min_date, max_date);
    let can_next = next != () && date_month_intersects(next, min_date, max_date);
    let previous_node = row([icon_node(
        "app/icons/date_previous", "app/icons/date_next", ctx.t("calendar.previous_month"), 16,
        component_style(props, "previous", style().text_color(theme_color("text_muted")))
    )]).with_key(`${props.key}-previous`).with_style(style()
        .width(px(28)).height(px(28)).items_center().justify_center()
        .border(px(1)).border_color(rgba(0x00000000)).radius(theme_radius("sm"))
        .hover(style().background(theme_color("surface_hover")))
        .focus(style().border_color(theme_color("focus_ring")))
        .disabled(style().opacity(0.55))).disabled(!can_previous)
        .accessibility_role("button").accessibility_label(ctx.t("calendar.previous_month"));
    if can_previous {
        previous_node = previous_node.on_click_value(Fn("move_month"),
            #{ date: month_target(focused_date, -1, min_date, max_date) });
    }
    let next_node = row([icon_node(
        "app/icons/date_next", "app/icons/date_previous", ctx.t("calendar.next_month"), 16,
        component_style(props, "next", style().text_color(theme_color("text_muted")))
    )]).with_key(`${props.key}-next`).with_style(style()
        .width(px(28)).height(px(28)).items_center().justify_center()
        .border(px(1)).border_color(rgba(0x00000000)).radius(theme_radius("sm"))
        .hover(style().background(theme_color("surface_hover")))
        .focus(style().border_color(theme_color("focus_ring")))
        .disabled(style().opacity(0.55))).disabled(!can_next)
        .accessibility_role("button").accessibility_label(ctx.t("calendar.next_month"));
    if can_next {
        next_node = next_node.on_click_value(Fn("move_month"),
            #{ date: month_target(focused_date, 1, min_date, max_date) });
    }
    let header = row([
        previous_node,
        text(ctx.format_month_year(visible_month))
            .with_style(component_style(props, "month_title",
                style().font_size(px(12)).line_height(px(16)).font_weight(700))),
        next_node,
    ]).with_style(component_style(props, "header",
        style().items_center().justify_between().padding_x(theme_spacing("xs"))));

    let weekday_nodes = [];
    let first_weekday = weekday_index(calendar.first_weekday);
    for offset in 0..7 {
        let label = calendar.weekdays.short[(first_weekday + offset) % 7];
        weekday_nodes.push(text(label)
            .with_style(component_style(props, "weekday",
                style().width(px(size)).height(px(24)).items_center().justify_center()
                    .font_size(px(10)).font_weight(700)
                    .text_color(theme_color("text_muted"))))
            .accessibility_role("columnheader").accessibility_label(label));
    }
    let weekdays = row(weekday_nodes)
        .with_style(component_style(props, "weekdays", style()));

    let cells = date_month_grid(visible_month, calendar.first_weekday);
    let weeks = [];
    for week_index in 0..6 {
        let days = [];
        for day_index in 0..7 {
            let cell = cells[week_index * 7 + day_index];
            let disabled = !date_enabled(cell.date, min_date, max_date);
            let selected = cell.date != () && cell.date == value;
            let is_today = cell.date != () && cell.date == today;
            let is_focused = cell.date != () && cell.date == focused_date;
            let day_style = component_style(props, "day",
                style().width(px(size)).height(px(size)).items_center().justify_center()
                    .radius(theme_radius("sm")).font_size(px(11)).line_height(px(14))
                    .border(px(1)).border_color(rgba(0x00000000))
                    .hover(style().background(theme_color("surface_hover")))
                    .focus(style().border_color(theme_color("focus_ring"))));
            let day_style = if cell.outside {
                component_style(props, "day_outside",
                    day_style.text_color(theme_color("text_muted")).opacity(0.62))
            } else { day_style };
            let day_style = if is_today {
                component_style(props, "day_today",
                    day_style.border(px(1)).border_color(theme_color("accent")))
            } else { day_style };
            let day_style = if is_focused {
                component_style(props, "day_focused",
                    day_style.border(px(1)).border_color(theme_color("focus_ring")))
            } else { day_style };
            let day_style = if selected {
                component_style(props, "day_selected",
                    day_style.background(theme_color("accent")).border_color(theme_color("accent"))
                        .text_color(theme_color("on_accent"))
                        .hover(style().background(theme_color("accent_hover"))))
            } else { day_style };
            let day_style = if disabled {
                component_style(props, "day_disabled", day_style.text_color(theme_color("disabled")))
            } else { day_style };
            let label = if cell.date == () { "" } else { ctx.format_date(cell.date, "long") };
            let day = row([text(if cell.day == () { "" }
                else { ctx.format_number(cell.day, #{ grouping: false }) })])
                .with_key(if cell.date == () { `blank-${week_index}-${day_index}` } else { cell.date })
                .with_style(day_style).disabled(disabled).tab_stop(false)
                .accessibility_role("gridcell").accessibility_label(label)
                .accessibility_checked(selected).accessibility_value(cell.date);
            if !disabled {
                day = day.on_click_value(Fn("choose_date"),
                    #{ date: cell.date, changed: cell.date != value });
            }
            days.push(day);
        }
        weeks.push(row(days).with_key(`week-${week_index}`)
            .with_style(component_style(props, "week", style())));
    }
    let grid = column(weeks).with_style(component_style(props, "grid", style()))
        .accessibility_role("grid").accessibility_label(ctx.format_month_year(visible_month));

    let footer_nodes = [];
    for preset in prop_or(props, "presets", []) {
        let preset_disabled = prop_or(preset, "disabled", false)
            || !date_enabled(preset.value, min_date, max_date);
        let preset_node = text(preset.label).with_key(`preset-${preset.value}`)
            .with_style(component_style(props, "preset", style()
                .height(px(28)).padding_x(px(8)).items_center()
                .font_size(px(11)).border(px(1)).border_color(theme_color("border"))
                .radius(theme_radius("sm")).hover(style().background(theme_color("surface_hover")))
                .disabled(style().opacity(0.55))))
            .disabled(preset_disabled).accessibility_role("button")
            .accessibility_label(preset.label).accessibility_value(preset.value);
        if !preset_disabled {
            preset_node = preset_node.on_click_value(Fn("choose_date"),
                #{ date: preset.value, changed: preset.value != value });
        }
        footer_nodes.push(preset_node);
    }
    let panel_children = [header, weekdays, grid];
    if footer_nodes.len > 0 {
        panel_children.push(row(footer_nodes)
            .with_style(component_style(props, "footer", style().gap(px(4))
                .padding_top(px(6)).border_top(px(1)).border_color(theme_color("border")))));
    }
    let panel = column(panel_children).with_key(`${props.key}-panel`)
        .with_style(component_style(props, "panel",
            style().width(px(panel_width)).gap(px(4)).padding(theme_spacing("sm"))
                .border(px(1)).border_color(theme_color("border"))
                .radius(theme_radius("md")).background(theme_color("surface"))
                .font_size(px(12)).line_height(px(16))
                .focus(style().border_color(theme_color("focus_ring")))))
        .tab_stop(true).accessibility_role("dialog")
        .accessibility_label(ctx.format_month_year(visible_month));

    panel = panel
        .on_key_value("left", Fn("navigate_date"),
            #{ date: date_target(focused_date, -1, min_date, max_date) })
        .on_key_value("right", Fn("navigate_date"),
            #{ date: date_target(focused_date, 1, min_date, max_date) })
        .on_key_value("up", Fn("navigate_date"),
            #{ date: date_target(focused_date, -7, min_date, max_date) })
        .on_key_value("down", Fn("navigate_date"),
            #{ date: date_target(focused_date, 7, min_date, max_date) })
        .on_key_value("home", Fn("navigate_date"),
            #{ date: week_target(focused_date, calendar.first_weekday, false, min_date, max_date) })
        .on_key_value("end", Fn("navigate_date"),
            #{ date: week_target(focused_date, calendar.first_weekday, true, min_date, max_date) })
        .on_key_value("pageup", Fn("navigate_date"),
            #{ date: month_target(focused_date, -1, min_date, max_date) })
        .on_key_value("pagedown", Fn("navigate_date"),
            #{ date: month_target(focused_date, 1, min_date, max_date) })
        .on_key_value("enter", Fn("choose_date"),
            #{ date: focused_date, changed: focused_date != value })
        .on_key_value("space", Fn("choose_date"),
            #{ date: focused_date, changed: focused_date != value });

    let config = #{ id: props.key, kind: "popover", open: open,
        placement: prop_or(props, "placement", "bottom"), gap: 4,
        modal: false, dismiss_on_escape: true, dismiss_on_outside: true };
    if "parent_overlay" in props && props.parent_overlay != () { config.parent = props.parent_overlay; }
    overlay(trigger, panel, config).with_key(props.key)
        .with_style(component_style(props, "root", style()))
        .on_open_change(Fn("set_open"))
}
