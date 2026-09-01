/* gpui-rhai
{
  "id": "components/select",
  "export": "Select",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/dropdown"],
  "capabilities": {},
  "assets": []
}
*/
// Scalar controlled Select composed from the public Rhai Dropdown component.
// Props: key/options/value/placeholder/searchable/disabled/error/size/width and on_change.
// Events: change(optional<string>).
// State: Dropdown owns keyed open/query interaction; value remains controlled by caller.
// Example: Select(#{ key: "country", options: countries, value: "cn", on_change: Fn("changed") })

import "components/dropdown" as dropdown;

define_component(#{
    metadata: #{ id: "components/select", "export": "Select", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/dropdown"], capabilities: #{}, assets: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            options: #{ schema: #{ type: "array", max_items: 100000, items: #{
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
            clear_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Clear" } },
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
        parts: ["root", "trigger", "value", "placeholder", "indicator", "clear",
            "panel", "search", "list", "group", "option", "option_selected",
            "option_active", "option_disabled", "empty"],
    },
    render: Fn("render_Select")
});

fn Select(props) { render_component("components/select", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn select_values(ctx, values) { ctx.emit("change", if values.len == 0 { () } else { values[0] }); }
fn select_height(size) {
    if size == "xs" { 24 }
    else if size == "sm" { 28 }
    else if size == "lg" { 36 }
    else { 32 }
}

fn render_Select(ctx, props) {
    let selected = if "value" in props && props.value != () { [props.value] } else { [] };
    let height = select_height(prop_or(props, "size", "md"));
    let border = if prop_or(props, "error", false) {
        theme_color("danger")
    } else { theme_color("border") };
    let empty_text = prop_or(props, "empty_text", "");
    let node = dropdown::Dropdown(#{
        key: `${props.key}-dropdown`, options: props.options, mode: "single",
        selected: selected, searchable: prop_or(props, "searchable", false),
        placeholder: prop_or(props, "placeholder", ""),
        search_placeholder: prop_or(props, "search_placeholder", ""),
        empty_text: if empty_text == "" { ctx.t("common.no_results") } else { empty_text },
        clear_label: prop_or(props, "clear_label", "Clear"),
        clearable: prop_or(props, "clearable", false), reset_query_on_close: true,
        disabled: prop_or(props, "disabled", false),
        max_visible: prop_or(props, "max_visible", 8),
        placement: prop_or(props, "placement", "bottom"),
        row_height: 28, trigger_height: height,
        width: if "width" in props { props.width } else { px(280) },
        on_change: Fn("select_values"),
        style: component_style(props, "root", style()),
        part_styles: #{
            trigger: component_style(props, "trigger", style().border_color(border)),
            value: component_style(props, "value", style()),
            placeholder: component_style(props, "placeholder", style()),
            indicator: component_style(props, "indicator", style()),
            clear: component_style(props, "clear", style()),
            panel: component_style(props, "panel", style()),
            search: component_style(props, "search", style()),
            list: component_style(props, "list", style()),
            group: component_style(props, "group", style()),
            option: component_style(props, "option", style()),
            option_selected: component_style(props, "option_selected", style()),
            option_active: component_style(props, "option_active", style()),
            option_disabled: component_style(props, "option_disabled", style()),
            empty: component_style(props, "empty", style()),
        }
    }).with_key(props.key);
    node.accessibility_invalid(prop_or(props, "error", false))
        .accessibility_value(if selected.len == 0 { () } else { selected[0] })
}
