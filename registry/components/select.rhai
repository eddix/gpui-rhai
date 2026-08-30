/* gpui-rhai
{
  "id": "components/select",
  "export": "Select",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/check.svg", "icons/close.svg", "icons/disclosure_down.svg"]
}
*/
// Select is a controlled, form-oriented, scalar choice field over the native choice-list core.
//
// Props: key, options, optional value, placeholder, clearable/searchable, disabled/error,
//        size, placement, max_visible, and on_change.
// Events: change(optional<string>). State: controlled value plus keyed native transient interaction.
// Example: Select(#{ key: "country", options: countries, value: "cn", searchable: true, on_change: Fn("changed") })

define_component(#{
    metadata: #{
        id: "components/select", "export": "Select", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{},
        assets: ["icons/check.svg", "icons/close.svg", "icons/disclosure_down.svg"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            options: #{ schema: #{ type: "array", max_items: 10000, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    group: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                    keywords: #{ schema: #{ type: "array", max_items: 32, items: #{ type: "string" } }, required: false, sensitive: false,
                        "default": #{ type: "array", value: [] } },
                },
            } }, required: true, sensitive: false },
            value: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" } },
            search_placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" } },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" } },
            clearable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            searchable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            error: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            size: #{ schema: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "md" } },
            width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 32 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 } },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "bottom" } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{ change: #{ payload: #{ type: "optional", value: #{ type: "string" } } } },
        slots: #{},
        parts: ["root", "trigger", "value", "placeholder", "indicator", "clear", "panel", "search", "list", "group", "option", "option_selected", "option_disabled", "empty"],
    },
    render: Fn("render_Select")
});

fn select_height(size) {
    if size == "xs" { 24 }
    else if size == "sm" { 28 }
    else if size == "lg" { 40 }
    else { 32 }
}

fn prop_or(props, name, fallback) {
    if name in props { props[name] } else { fallback }
}

fn Select(props) { render_component("components/select", props) }
fn render_Select(ctx, props) {
    let size = prop_or(props, "size", "md");
    let height = select_height(size);
    let error = prop_or(props, "error", false);
    let empty_text = prop_or(props, "empty_text", "");
    let config = #{
        id: props.key,
        options: props.options,
        value: if "value" in props { props.value } else { () },
        searchable: prop_or(props, "searchable", false),
        clearable: prop_or(props, "clearable", false),
        placeholder: prop_or(props, "placeholder", ""),
        search_placeholder: prop_or(props, "search_placeholder", ""),
        empty_text: if empty_text == "" { ctx.t("common.no_results") } else { empty_text },
        disabled: prop_or(props, "disabled", false),
        max_visible: prop_or(props, "max_visible", 8),
        placement: prop_or(props, "placement", "bottom"),
        row_height: height * 1.0,
        trigger_height: height * 1.0,
        trigger_width: if "width" in props && props.width != () { props.width } else { px(280) },
        panel_width: 280.0,
        overlay_gap: 4.0,
        clear_asset: asset("app/icons/close"),
        indicator_asset: asset("app/icons/disclosure_down"),
        check_asset: asset("app/icons/check"),
    };
    let border = if error { theme_color("danger") } else { theme_color("border") };
    let node = select(config)
        .with_style(component_style(props, "root", style()))
        .with_part_style("trigger", component_style(props, "trigger",
            style()
                .border(px(1))
                .border_color(border)
                .radius(theme_radius("md"))
                .padding_x(theme_spacing("sm"))
                .background(theme_color("surface"))))
        .with_part_style("value", component_style(props, "value",
            style().text_color(theme_color("text_primary"))))
        .with_part_style("placeholder", component_style(props, "placeholder",
            style().text_color(theme_color("text_muted"))))
        .with_part_style("panel", component_style(props, "panel",
            style()
                .border(px(1))
                .border_color(theme_color("border"))
                .radius(theme_radius("md"))
                .padding(theme_spacing("xs"))
                .background(theme_color("surface_raised"))))
        .with_part_style("group", component_style(props, "group",
            style().padding_x(theme_spacing("sm")).text_color(theme_color("text_muted"))))
        .with_part_style("search", component_style(props, "search",
            style().border(px(1)).border_color(theme_color("border"))
                .text_color(theme_color("text_muted"))))
        .with_part_style("list", component_style(props, "list", style()))
        .with_part_style("option", component_style(props, "option",
            style().padding_x(theme_spacing("sm")).radius(theme_radius("sm"))))
        .with_part_style("option_selected", component_style(props, "option_selected",
            style().text_color(theme_color("accent"))))
        .with_part_style("option_disabled", component_style(props, "option_disabled",
            style().text_color(theme_color("disabled"))))
        .with_part_style("empty", component_style(props, "empty",
            style().padding_x(theme_spacing("sm")).text_color(theme_color("text_muted"))))
        .with_part_style("clear", component_style(props, "clear",
            style().padding_x(theme_spacing("xs")).text_color(theme_color("text_muted"))))
        .with_part_style("indicator", component_style(props, "indicator",
            style().text_color(theme_color("text_muted"))))
        .accessibility_role("combobox")
        .accessibility_label(prop_or(props, "placeholder", ""))
        .accessibility_value(if "value" in props && props.value != () { props.value } else { () })
        .accessibility_invalid(error);
    if "on_change" in props && props.on_change != () {
        node.on_change(props.on_change)
    } else { node }
}
