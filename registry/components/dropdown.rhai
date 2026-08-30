/* gpui-rhai
{
  "id": "components/dropdown",
  "export": "Dropdown",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/check.svg", "icons/close.svg", "icons/disclosure_down.svg"]
}
*/
// Dropdown wraps the keyed native dropdown entity and overlay mechanism.
//
// Props: key, options, mode, selected/open/query (optional controlled values),
// searchable, labels, placement, max_visible, and semantic callbacks.
// Events: change(array<string>), open_change(bool), query_change(string).
// Parts: root, trigger, search, list, option, indicator.
// State: controlled props or keyed native state.
// Example: Dropdown(#{ key: "country", options: options, selected: ["uk"], on_change: Fn("changed") })

define_component(#{
    metadata: #{
        id: "components/dropdown",
        "export": "Dropdown",
        version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [],
        capabilities: #{},
        assets: ["icons/check.svg", "icons/close.svg", "icons/disclosure_down.svg"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            parent_overlay: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false,
                sensitive: false,
            },
            options: #{
                schema: #{
                    type: "array",
                    max_items: 10000,
                    items: #{
                        type: "object",
                        allow_unknown: false,
                        fields: #{
                            value: #{
                                schema: #{ type: "string" },
                                required: true,
                                sensitive: false,
                            },
                            label: #{
                                schema: #{ type: "string" },
                                required: true,
                                sensitive: false,
                            },
                            keywords: #{
                                schema: #{
                                    type: "array",
                                    max_items: 32,
                                    items: #{ type: "string" }
                                },
                                required: false,
                                sensitive: false,
                                "default": #{ type: "array", value: [] },
                            },
                            disabled: #{
                                schema: #{ type: "bool" },
                                required: false,
                                sensitive: false,
                                "default": #{ type: "bool", value: false },
                            },
                            group: #{
                                schema: #{ type: "optional", value: #{ type: "string" } },
                                required: false,
                                sensitive: false,
                            },
                        },
                    },
                },
                required: true,
                sensitive: false,
            },
            mode: #{
                schema: #{ type: "string", allowed: ["single", "multiple"] },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "single" },
            },
            selected: #{
                schema: #{
                    type: "optional",
                    value: #{ type: "array", max_items: 10000, items: #{ type: "string" } }
                },
                required: false,
                sensitive: false,
            },
            open: #{
                schema: #{ type: "optional", value: #{ type: "bool" } },
                required: false,
                sensitive: false,
            },
            searchable: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            query: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false,
                sensitive: false,
            },
            placeholder: #{
                schema: #{ type: "string" },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "" },
            },
            search_placeholder: #{
                schema: #{ type: "string" },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "" },
            },
            empty_text: #{
                schema: #{ type: "string" },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "" },
            },
            trigger: #{
                schema: #{ type: "optional", value: #{ type: "node" } },
                required: false,
                sensitive: false,
            },
            header: #{
                schema: #{ type: "optional", value: #{ type: "node" } },
                required: false,
                sensitive: false,
            },
            footer: #{
                schema: #{ type: "optional", value: #{ type: "node" } },
                required: false,
                sensitive: false,
            },
            empty: #{
                schema: #{ type: "optional", value: #{ type: "node" } },
                required: false,
                sensitive: false,
            },
            disabled: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            max_visible: #{
                schema: #{ type: "integer", min: 1, max: 32 },
                required: false,
                sensitive: false,
                "default": #{ type: "integer", value: 8 },
            },
            width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false },
            placement: #{
                schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "bottom" },
            },
            on_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
            },
            on_open_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
            },
            on_query_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{
            change: #{
                payload: #{ type: "array", max_items: 10000, items: #{ type: "string" } }
            },
            open_change: #{ payload: #{ type: "bool" } },
            query_change: #{ payload: #{ type: "string" } },
        },
        slots: #{
            trigger: #{ required: false, multiple: false },
            header: #{ required: false, multiple: false },
            footer: #{ required: false, multiple: false },
            empty: #{ required: false, multiple: false },
        },
        parts: ["root", "trigger", "value", "placeholder", "indicator", "clear",
            "panel", "search", "list", "group", "option", "option_selected",
            "option_disabled", "empty"],
    },
    render: Fn("render_Dropdown")
});

fn prop_or(props, name, fallback) {
    if name in props { props[name] } else { fallback }
}

fn Dropdown(props) { render_component("components/dropdown", props) }
fn render_Dropdown(ctx, props) {
    let config = #{
        id: props.key,
        options: props.options,
        mode: prop_or(props, "mode", "single"),
        searchable: prop_or(props, "searchable", false),
        placeholder: prop_or(props, "placeholder", ""),
        search_placeholder: prop_or(props, "search_placeholder", ""),
        empty_text: prop_or(props, "empty_text", ""),
        disabled: prop_or(props, "disabled", false),
        max_visible: prop_or(props, "max_visible", 8),
        placement: prop_or(props, "placement", "bottom"),
        row_height: 32.0,
        trigger_height: 32.0,
        trigger_width: if "width" in props && props.width != () { props.width } else { px(280) },
        panel_width: 280.0,
        panel_extra_height: 104.0,
        overlay_gap: 4.0,
        clear_asset: asset("app/icons/close"),
        indicator_asset: asset("app/icons/disclosure_down"),
        check_asset: asset("app/icons/check"),
    };
    if "selected" in props && props.selected != () { config.selected = props.selected; }
    if "parent_overlay" in props && props.parent_overlay != () {
        config.parent_overlay = props.parent_overlay;
    }
    if "open" in props && props.open != () { config.open = props.open; }
    if "query" in props && props.query != () { config.query = props.query; }
    if "trigger" in props && props.trigger != () { config.trigger = props.trigger; }
    if "header" in props && props.header != () { config.header = props.header; }
    if "footer" in props && props.footer != () { config.footer = props.footer; }
    if "empty" in props && props.empty != () { config.empty = props.empty; }

    let node = dropdown(config).with_style(component_style(props, "root", style()));
    let node = node
        .with_part_style("trigger", component_style(props, "trigger",
            style().border(px(1)).border_color(theme_color("border"))
                .radius(theme_radius("md")).padding_x(theme_spacing("sm"))
                .background(theme_color("surface"))))
        .with_part_style("value", component_style(props, "value",
            style().text_color(theme_color("text_primary"))))
        .with_part_style("placeholder", component_style(props, "placeholder",
            style().text_color(theme_color("text_muted"))))
        .with_part_style("indicator", component_style(props, "indicator",
            style().text_color(theme_color("text_muted"))));
    let node = node
        .with_part_style("clear", component_style(props, "clear",
            style().text_color(theme_color("text_muted"))))
        .with_part_style("panel", component_style(props, "panel",
            style().border(px(1)).border_color(theme_color("border"))
                .radius(theme_radius("md")).padding(theme_spacing("xs"))
                .background(theme_color("surface_raised"))))
        .with_part_style("search", component_style(props, "search",
            style().border(px(1)).border_color(theme_color("border"))
                .text_color(theme_color("text_muted"))))
        .with_part_style("list", component_style(props, "list", style()))
        .with_part_style("group", component_style(props, "group",
            style().padding_x(theme_spacing("sm")).text_color(theme_color("text_muted"))));
    let node = node
        .with_part_style("option", component_style(props, "option",
            style().padding_x(theme_spacing("sm")).radius(theme_radius("sm"))))
        .with_part_style("option_selected", component_style(props, "option_selected",
            style().text_color(theme_color("accent"))))
        .with_part_style("option_disabled", component_style(props, "option_disabled",
            style().text_color(theme_color("disabled"))))
        .with_part_style("empty", component_style(props, "empty",
            style().padding_x(theme_spacing("sm")).text_color(theme_color("text_muted"))));
    if "on_change" in props && props.on_change != () {
        node = node.on_change(props.on_change);
    }
    if "on_open_change" in props && props.on_open_change != () {
        node = node.on_open_change(props.on_open_change);
    }
    if "on_query_change" in props && props.on_query_change != () {
        node = node.on_query_change(props.on_query_change);
    }
    node
        .accessibility_role("combobox")
        .accessibility_label(prop_or(props, "placeholder", ""))
}
