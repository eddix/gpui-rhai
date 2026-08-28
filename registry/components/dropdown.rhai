/* gpui-rhai
{
  "id": "components/dropdown",
  "export": "Dropdown",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
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

export_component(#{
    metadata: #{
        id: "components/dropdown",
        "export": "Dropdown",
        version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [],
        capabilities: #{},
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
                schema: #{ type: "integer" },
                required: false,
                sensitive: false,
                "default": #{ type: "integer", value: 8 },
            },
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
        parts: ["root", "trigger", "search", "list", "option", "indicator"],
    },
});

fn prop_or(props, name, fallback) {
    if name in props { props[name] } else { fallback }
}

fn Dropdown(props) {
    component_render("components/dropdown", props, Fn("render_Dropdown"))
}
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
