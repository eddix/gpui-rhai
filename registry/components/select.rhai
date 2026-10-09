/* gpui-rhai
{
  "id": "components/select",
  "export": "Select",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/combobox"],
  "capabilities": {},
  "assets": [],
  "tokens": ["border", "danger"],
  "environment": ["size", "density"]
}
*/
// Scalar controlled Select composed from the public Rhai Combobox component.
// State: caller owns value/open/query; Combobox owns only active-option navigation.
// Example: Select(#{ key: "country", label: "Country", options: countries, value: "cn", open: false, query: "", on_change: Fn("changed") })

import "components/combobox" as combobox;

define_component(#{
    metadata: #{ id: "components/select", "export": "Select", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/combobox"], capabilities: #{},
        tokens: ["border", "danger"],
        environment: ["size", "density"], assets: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Identity of the select; its inner Combobox, and so the list's overlay id, is keyed `<key>-combobox`." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the trigger and the list." },
            options: #{ schema: #{ type: "array", max_items: 100000, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Value that `change` reports when this option is picked; must be unique." },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Option text, also matched by the search query; must not be blank." },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Shows the option but blocks picking it; keyboard navigation skips it." },
                    group: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Heading this option is listed under; must not be blank." },
                    keywords: #{ schema: #{ type: "array", max_items: 32, items: #{ type: "string" } }, required: false, sensitive: false,
                        "default": #{ type: "array", value: [] },
                        doc: "Extra terms the search query also matches, case-insensitively." },
                },
            } }, required: true, sensitive: false,
                doc: "Options in display order; options that share a `group` are gathered where that group first appears." },
            value: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Selected option value, or `()` for none; the caller stores the `change` payload and passes it back." },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether the list is open; the caller stores the `open_change` payload and passes it back." },
            query: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Search text that filters the options; the caller stores the `query_change` payload and passes it back." },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" }, doc: "Trigger text while nothing is selected." },
            search_placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" }, doc: "Placeholder of the search field shown with `searchable`." },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Text shown when no option matches; empty uses the localized `common.no_results` message." },
            clear_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Clear" },
                doc: "Accessible name of the clear button shown with `clearable`." },
            clearable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Shows a clear button in the trigger while a value is selected; clicking it emits `change` with `()`." },
            searchable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Shows a search field at the top of the list; without it, letter keys jump to matching options." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false }, doc: "Disables the trigger so the list cannot open." },
            error: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Marks the value invalid: the trigger border turns `danger` and accessibility reports it invalid." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false, doc: "Control size; leave unset to inherit the environment size." },
            width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false,
                doc: "Width of the trigger, which the list matches; unset means 280 logical pixels." },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 32 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 },
                doc: "Rows the list shows before it scrolls; group headings count as rows." },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "bottom" }, doc: "Side of the trigger the list opens on." },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the picked value, or `()` when the value is cleared." },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the requested open state when the trigger is clicked, an arrow key opens the list, an option is picked, or it is dismissed." },
            on_query_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the new search text as the user types, and with `\"\"` when the list closes or the value is cleared." },
        },
        state: #{ fields: #{} },
        events: #{ change: #{ payload: #{ type: "optional", value: #{ type: "string" } },
                doc: "Emitted when the user picks a different option or clears the value; the payload is the new value or `()`." },
            open_change: #{ payload: #{ type: "bool" },
                doc: "Emitted when the list should open or close; the payload is the requested open state." },
            query_change: #{ payload: #{ type: "string" },
                doc: "Emitted when the search text changes or is reset on close; the payload is the new query." } },
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
fn select_open(ctx, open) { ctx.emit("open_change", open); }
fn select_query(ctx, query) { ctx.emit("query_change", query); }
fn render_Select(ctx, props) {
    let selected = if "value" in props && props.value != () { [props.value] } else { [] };
    let trigger_style = if prop_or(props, "error", false) {
        style().border_color(theme_color("danger"))
    } else { style() };
    let empty_text = prop_or(props, "empty_text", "");
    let node = combobox::Combobox(#{
        key: `${props.key}-combobox`, label: props.label, options: props.options, mode: "single",
        selected: selected, open: props.open, query: props.query,
        searchable: prop_or(props, "searchable", false),
        placeholder: prop_or(props, "placeholder", ""),
        search_placeholder: prop_or(props, "search_placeholder", ""),
        empty_text: if empty_text == "" { ctx.t("common.no_results") } else { empty_text },
        clear_label: prop_or(props, "clear_label", "Clear"),
        clearable: prop_or(props, "clearable", false), reset_query_on_close: true,
        disabled: prop_or(props, "disabled", false),
        max_visible: prop_or(props, "max_visible", 8),
        placement: prop_or(props, "placement", "bottom"),
        size: prop_or(props, "size", ()),
        width: if "width" in props { props.width } else { px(280) },
        on_change: Fn("select_values"),
        on_open_change: Fn("select_open"), on_query_change: Fn("select_query"),
        style: ctx.component_style("root", style()),
        part_styles: #{
            trigger: ctx.component_style("trigger", trigger_style),
            value: ctx.component_style("value", style()),
            placeholder: ctx.component_style("placeholder", style()),
            indicator: ctx.component_style("indicator", style()),
            clear: ctx.component_style("clear", style()),
            panel: ctx.component_style("panel", style()),
            search: ctx.component_style("search", style()),
            list: ctx.component_style("list", style()),
            group: ctx.component_style("group", style()),
            option: ctx.component_style("option", style()),
            option_selected: ctx.component_style("option_selected", style()),
            option_active: ctx.component_style("option_active", style()),
            option_disabled: ctx.component_style("option_disabled", style()),
            empty: ctx.component_style("empty", style()),
        }
    }).with_key(props.key);
    node.accessibility_invalid(prop_or(props, "error", false))
        .accessibility_value(if selected.len == 0 { () } else { selected[0] })
}
