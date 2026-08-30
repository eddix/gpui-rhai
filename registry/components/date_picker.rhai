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
// DatePicker is a controlled single-date field with a localized native Gregorian month grid.
//
// Props: key, optional ISO value/min/max, placeholder/display_style, presets, clearable,
//        disabled/error, size, placement, and on_change.
// Events: change(optional<string>). State: controlled value plus keyed native calendar transients.
// Example: DatePicker(#{ key: "appointment", value: "2026-09-01", min_date: "2026-08-30", on_change: Fn("changed") })

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
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "bottom" } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{ change: #{ payload: #{ type: "optional", value: #{ type: "string" } } } },
        slots: #{},
        parts: ["root", "trigger", "trigger_value", "trigger_icon", "panel", "header", "previous", "next", "weekdays", "weekday", "grid", "day", "day_outside", "day_today", "day_selected", "day_disabled", "footer", "preset", "clear"],
    },
    render: Fn("render_DatePicker")
});

fn date_picker_size(size) {
    if size == "xs" { 24 }
    else if size == "sm" { 28 }
    else if size == "lg" { 40 }
    else { 32 }
}

fn prop_or(props, name, fallback) {
    if name in props { props[name] } else { fallback }
}

fn DatePicker(props) { render_component("components/date_picker", props) }
fn render_DatePicker(ctx, props) {
    let value = if "value" in props { props.value } else { () };
    let size = prop_or(props, "size", "md");
    let height = date_picker_size(size);
    let error = prop_or(props, "error", false);
    let config = #{
        id: props.key,
        value: value,
        today: ctx.today(),
        display_value: if value == () { "" } else {
            ctx.format_date(value, prop_or(props, "display_style", "short"))
        },
        placeholder: prop_or(props, "placeholder", ""),
        open_label: ctx.t("calendar.open"),
        previous_label: ctx.t("calendar.previous_month"),
        next_label: ctx.t("calendar.next_month"),
        clear_label: ctx.t("calendar.clear"),
        calendar: ctx.calendar(),
        number: ctx.number(),
        presets: prop_or(props, "presets", []),
        clearable: prop_or(props, "clearable", false),
        disabled: prop_or(props, "disabled", false),
        placement: prop_or(props, "placement", "bottom"),
        cell_size: height * 1.0,
        trigger_height: height * 1.0,
        panel_width: height * 7.0 + 32.0,
        overlay_gap: 4.0,
        previous_asset: asset("app/icons/date_previous"),
        next_asset: asset("app/icons/date_next"),
        trigger_asset: asset("app/icons/calendar"),
        clear_asset: asset("app/icons/close"),
    };
    if "min_date" in props && props.min_date != () { config.min_date = props.min_date; }
    if "max_date" in props && props.max_date != () { config.max_date = props.max_date; }
    let border = if error { theme_color("danger") } else { theme_color("border") };
    let node = date_picker(config).with_style(component_style(props, "root", style()));
    let node = node.with_part_style("trigger", component_style(props, "trigger",
        style().border(px(1)).border_color(border).radius(theme_radius("md"))
            .padding_x(theme_spacing("sm")).background(theme_color("surface"))));
    let node = node
        .with_part_style("trigger_value", component_style(props, "trigger_value", style()))
        .with_part_style("trigger_icon", component_style(props, "trigger_icon",
            style().text_color(theme_color("text_muted"))))
        .with_part_style("panel", component_style(props, "panel",
            style().border(px(1)).border_color(theme_color("border"))
                .radius(theme_radius("md")).padding(theme_spacing("sm"))
                .background(theme_color("surface_raised"))
                .text_color(theme_color("text_primary"))));
    let node = node
        .with_part_style("header", component_style(props, "header",
            style().padding_x(theme_spacing("xs"))))
        .with_part_style("previous", component_style(props, "previous", style()))
        .with_part_style("next", component_style(props, "next", style()))
        .with_part_style("weekdays", component_style(props, "weekdays", style()))
        .with_part_style("weekday", component_style(props, "weekday",
            style().text_color(theme_color("text_muted"))));
    let node = node
        .with_part_style("grid", component_style(props, "grid", style()))
        .with_part_style("day", component_style(props, "day",
            style().radius(theme_radius("sm"))))
        .with_part_style("day_selected", component_style(props, "day_selected",
            style().radius(theme_radius("sm")).background(theme_color("accent"))
                .text_color(theme_color("on_accent"))))
        .with_part_style("day_today", component_style(props, "day_today",
            style().border(px(1)).border_color(theme_color("accent"))));
    let node = node
        .with_part_style("day_disabled", component_style(props, "day_disabled",
            style().text_color(theme_color("disabled"))))
        .with_part_style("day_outside", component_style(props, "day_outside",
            style().text_color(theme_color("text_muted"))))
        .with_part_style("footer", component_style(props, "footer", style()))
        .with_part_style("preset", component_style(props, "preset", style()))
        .with_part_style("clear", component_style(props, "clear",
            style().text_color(theme_color("text_muted"))))
        .accessibility_role("combobox")
        .accessibility_label(prop_or(props, "placeholder", ""))
        .accessibility_value(value)
        .accessibility_invalid(error);
    if "on_change" in props && props.on_change != () {
        node.on_change(props.on_change)
    } else { node }
}
