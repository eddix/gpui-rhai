/* gpui-rhai
{
  "id": "components/date_picker",
  "export": "DatePicker",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/calendar"],
  "capabilities": {},
  "assets": ["icons/calendar.svg", "icons/date_previous.svg", "icons/date_next.svg", "icons/close.svg"],
  "tokens": [
    "surface_raised", "border", "control.border", "danger", "focus_ring", "surface_hover", "control.hover",
    "accent", "accent_hover", "on_accent", "text.accent", "text_primary", "text_muted",
    "disabled", "metrics.control", "metrics.control_pad", "metrics.field_pad", "metrics.icon", "metrics.marker",
    "metrics.inset", "spacing.xs", "spacing.sm", "radius.md", "radius.lg",
    "typography.control", "typography.body", "typography.label"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// Controlled single-date field composed from public date data helpers, atoms, and Overlay.
// State: keyed overlay open; Calendar owns visible-month and focused-date transients.
// Example: DatePicker(#{ key: "appointment", label: "Appointment date", value: "2026-09-01", on_change: Fn("changed") })

import "components/calendar" as calendar;

define_component(#{
    metadata: #{
        id: "components/date_picker", "export": "DatePicker", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/calendar"], capabilities: #{},
        tokens: [
            "surface_raised", "border", "control.border", "danger", "focus_ring", "surface_hover", "control.hover",
            "accent", "accent_hover", "on_accent", "text.accent", "text_primary", "text_muted",
            "disabled", "metrics.control", "metrics.control_pad", "metrics.field_pad", "metrics.icon",
            "metrics.marker", "metrics.inset", "spacing.xs", "spacing.sm", "radius.md",
            "radius.lg", "typography.control", "typography.body", "typography.label"
        ],
        environment: ["size", "density", "corners"],
        assets: ["icons/calendar.svg", "icons/date_previous.svg", "icons/date_next.svg", "icons/close.svg"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Identity of the picker; also its overlay id and the prefix of its inner keys (`<key>-trigger`, `<key>-panel`)." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the trigger." },
            parent_overlay: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Key of the open overlay this one nests in, such as a Dialog; the calendar stacks above it and closes with it." },
            value: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Selected date as strict ISO `YYYY-MM-DD`, or `()` for none; the caller stores the `change` payload and passes it back." },
            min_date: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Earliest selectable date, strict ISO `YYYY-MM-DD`, inclusive; earlier days are disabled and `value` must not be earlier." },
            max_date: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Latest selectable date, strict ISO `YYYY-MM-DD`, inclusive; not before `min_date`, and `value` must not be later." },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" }, doc: "Trigger text while no date is selected." },
            display_style: #{ schema: #{ type: "string", allowed: ["short", "medium", "long"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "short" },
                doc: "Length of the trigger's date text, formatted for the current locale." },
            clearable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Shows a clear button in the trigger while a date is set; it emits `change` with `()`." },
            presets: #{ schema: #{ type: "array", max_items: 32, items: #{
                type: "object", allow_unknown: false, fields: #{
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Button text; must not be blank." },
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Strict ISO `YYYY-MM-DD` date the preset picks; must be unique among the presets." },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false }, doc: "Shows the preset but blocks picking it." },
                },
            } }, required: false, sensitive: false,
                "default": #{ type: "array", value: [] },
                doc: "Quick-pick buttons in the calendar footer; a preset outside `min_date` to `max_date` is disabled." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false }, doc: "Disables the trigger so the calendar cannot open." },
            error: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Marks the value invalid: the trigger border turns `danger` and accessibility reports it invalid." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false,
                doc: "Control size; leave unset to inherit the environment size. The preset footer is always `xs`." },
            width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false,
                doc: "Width of the trigger; unset is 280 logical pixels." },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "bottom" }, doc: "Side of the trigger the calendar opens on." },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the picked ISO date, or `()` when the date is cleared." },
        },
        state: #{ fields: #{
            open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        } },
        events: #{ change: #{ payload: #{ type: "optional", value: #{ type: "string" } },
            doc: "Emitted when a day or preset other than `value` is picked, or the date is cleared; the payload is the ISO date or `()`." } },
        slots: #{},
        parts: ["root", "trigger", "trigger_value", "trigger_icon", "panel", "header",
            "month_title", "previous", "next", "weekdays", "weekday", "grid", "week",
            "day", "day_outside", "day_today", "day_selected", "day_focused",
            "day_disabled", "footer", "preset", "clear", "focus_frame"],
    },
    render: Fn("render_DatePicker")
});

fn DatePicker(props) { render_component("components/date_picker", props) }

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
fn optional_prop(props, name) { if name in props && props[name] != () { props[name] } else { () } }
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

fn set_open(ctx, open) {
    ctx.set_state("open", open);
    handled()
}
fn choose_date(ctx, payload) {
    if payload.changed { ctx.emit("change", payload.date); }
    ctx.set_state("open", false);
    handled()
}
fn calendar_chosen(value, ctx, date) { choose_date(ctx, #{date:date, changed:date!=value}); }
fn clear_date(ctx, payload) {
    if payload.changed { ctx.emit("change", ()); }
    ctx.set_state("open", false);
    handled()
}

// Presets are xs secondary blocks.
fn preset_style() {
    let preset = style()
        .min_height(theme_length("metrics.control")).padding_x(theme_length("metrics.control_pad"))
        .items_center().typography("control");
    preset
        .border(px(2)).border_color(theme_color("surface_hover"))
        .background(theme_color("surface_hover")).radius(theme_radius("md"))
        .hover(style().background(theme_color("control.hover")))
        .focus(style().border_color(theme_color("focus_ring")))
        .disabled(style().text_color(theme_color("disabled")))
}

fn render_DatePicker(ctx, props) {
    let value = optional_prop(props, "value");
    let min_date = optional_prop(props, "min_date");
    let max_date = optional_prop(props, "max_date");
    let today = ctx.today();
    validate_date_picker(props, value, min_date, max_date, today);
    let open = ctx.get_state("open");
    let visible_month = date_month_start(date_clamp(if value == () { today } else { value }, min_date, max_date));
    let width = if "width" in props && props.width != () { props.width } else { px(280) };
    let placeholder = prop_or(props, "placeholder", "");
    let display = if value == () { placeholder }
        else { ctx.format_date(value, prop_or(props, "display_style", "short")) };
    let error = prop_or(props, "error", false);

    let trigger_value = text(display).with_style(ctx.component_style("trigger_value",
        style().flex_grow().text_color(if value == () { theme_color("text_muted") }
            else { theme_color("text_primary") })));
    let trigger_actions = [];
    if prop_or(props, "clearable", false) && value != () {
        trigger_actions.push(row([image_source(asset("app/icons/close"))
            .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
                .text_color(theme_color("text_muted")))])
            .with_style(ctx.component_style("clear", style()
                .items_center().justify_center()
                .hover(style().background(theme_color("surface_hover")))))
            .accessibility_role("button").accessibility_label(ctx.t("calendar.clear"))
            .on_click_value(Fn("clear_date"), #{ changed: true }));
    }
    trigger_actions.push(row([image_source(asset("app/icons/calendar"))
        .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
            .text_color(theme_color("text_muted")))])
        .with_style(ctx.component_style("trigger_icon", style()
            .items_center().justify_center()
            .text_color(theme_color("text_muted"))))
        .accessibility_role("image").accessibility_label(ctx.t("calendar.open")));
    let trigger_children = [trigger_value, row(trigger_actions)
            .with_style(style().gap(theme_spacing("xs")).items_center())];
    // The trigger is a field; while focused or open the focus color owns the frame.
    let trigger_border = if open { theme_color("focus_ring") }
        else if error { theme_color("danger") } else { theme_color("control.border") };
    let trigger_style = style().width(width).min_height(theme_length("metrics.control"))
        .padding_x(theme_length("metrics.field_pad")).gap(theme_spacing("sm")).items_center();
    trigger_style = trigger_style
        .border(px(2)).border_color(trigger_border)
        .radius(theme_radius("md")).background(theme_color("surface_raised"))
        .typography("control")
        .focus(style().border_color(theme_color("focus_ring")))
        .disabled(style().text_color(theme_color("disabled")).opacity(0.72));
    let trigger = row(trigger_children).with_key(`${props.key}-trigger`)
        .with_style(ctx.component_style("trigger", trigger_style))
        .disabled(prop_or(props, "disabled", false)).tab_stop(true)
        .accessibility_role("combobox").accessibility_expanded(open)
        .accessibility_label(props.label).accessibility_value(value)
        .accessibility_invalid(error);

    let calendar_parts = #{};
    for part in ["header","month_title","previous","next","weekdays","weekday","grid","week","day","day_outside","day_today","day_selected","day_focused","day_disabled"] {
        calendar_parts[part] = ctx.component_style(part, style());
    }
    let calendar_node = calendar::Calendar(#{ key: `${props.key}-calendar-${open}`, label: props.label,
        value: value, min_date: min_date, max_date: max_date,
        part_styles: calendar_parts, on_change: Fn("calendar_chosen").curry(value) });

    let footer_nodes = [];
    for preset in prop_or(props, "presets", []) {
        let preset_disabled = prop_or(preset, "disabled", false)
            || !date_enabled(preset.value, min_date, max_date);
        let preset_node = row([text(preset.label)]).with_key(`preset-${preset.value}`)
            .with_style(ctx.component_style("preset", preset_style()))
            .disabled(preset_disabled).accessibility_role("button")
            .accessibility_label(preset.label).accessibility_value(preset.value);
        if !preset_disabled {
            preset_node = preset_node.on_click_value(Fn("choose_date"),
                #{ date: preset.value, changed: preset.value != value });
        }
        footer_nodes.push(preset_node);
    }
    let panel_children = [calendar_node];
    if footer_nodes.len > 0 {
        panel_children.push(row(footer_nodes)
            .with_style(ctx.component_style("footer", style().gap(theme_spacing("xs"))
                .padding_top(theme_spacing("sm")).border_top(px(1)).border_color(theme_color("border"))))
            .env(#{ size: "xs" }));
    }
    panel_children.push(panel_focus_frame(ctx));
    let panel = column(panel_children).with_key(`${props.key}-panel`)
        .with_style(ctx.component_style("panel",
            style().relative().gap(theme_spacing("sm")).padding(theme_length("metrics.inset"))
                .border(px(1)).border_color(theme_color("border"))
                .radius(theme_radius("lg")).background(theme_color("surface_raised"))
                .typography("body")
                .focus(style().border_color(theme_color("focus_ring")))))
        .tab_stop(false).accessibility_role("dialog")
        .accessibility_label(ctx.format_month_year(visible_month));


    let config = #{ id: props.key, kind: "popover", open: open,
        placement: prop_or(props, "placement", "bottom"), align: "start", gap: 4,
        modal: false, initial_focus: "first", dismiss_on_escape: true, dismiss_on_outside: true };
    if "parent_overlay" in props && props.parent_overlay != () { config.parent = props.parent_overlay; }
    let root = overlay(trigger, panel, config).with_key(props.key)
        .with_style(ctx.component_style("root", style()))
        .on_open_change(Fn("set_open"));
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
