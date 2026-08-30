/* gpui-rhai
{
  "id": "components/table",
  "export": "Table",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/skeleton"],
  "capabilities": {},
  "assets": ["icons/check.svg", "icons/sort_ascending.svg", "icons/sort_descending.svg"]
}
*/
// Table is a controlled fixed-row-height data table with native row virtualization.
//
// Props: key/label, columns, rows, row_key, height, size, loading/empty states,
//        stripes, controlled sort/selection, and semantic callbacks.
// Events: sort_change(optional<sort>), selection_change(array<string>), row_click(string).
// Example: Table(#{ key: "users", label: "Users", columns: columns, rows: rows, row_key: "id", height: px(420) })

import "components/skeleton" as skeleton;

export_component(#{
    metadata: #{ id: "components/table", "export": "Table", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/skeleton"], capabilities: #{},
        assets: ["icons/check.svg", "icons/sort_ascending.svg", "icons/sort_descending.svg"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            row_key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            rows: #{ schema: #{ type: "array", max_items: 10000, items: #{ type: "map", values: #{ type: "ui_value" } } }, required: true, sensitive: false },
            columns: #{ schema: #{ type: "array", max_items: 256, items: #{
                type: "object", allow_unknown: false, fields: #{
                    key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    width: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                        kind: #{ schema: #{ type: "string", allowed: ["fixed", "percent", "flex"] }, required: true, sensitive: false },
                        value: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
                    } }, required: true, sensitive: false },
                    align: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["start", "center", "end"] } }, required: false, sensitive: false },
                    format: #{ schema: #{ type: "optional", value: #{ type: "map", values: #{ type: "ui_value" } } }, required: false, sensitive: false },
                    sortable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    cell_renderer: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
                },
            } }, required: true, sensitive: false },
            height: #{ schema: #{ type: "length" }, required: true, sensitive: false },
            size: #{ schema: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "md" } },
            loading: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            loading_content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            empty: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" } },
            striped: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            selection_mode: #{ schema: #{ type: "string", allowed: ["none", "single", "multiple"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "none" } },
            selected_keys: #{ schema: #{ type: "array", max_items: 10000, items: #{ type: "string" } }, required: false, sensitive: false,
                "default": #{ type: "array", value: [] } },
            sort: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                direction: #{ schema: #{ type: "string", allowed: ["ascending", "descending"] }, required: true, sensitive: false },
            } } }, required: false, sensitive: false },
            on_sort_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_selection_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_row_click: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{
            sort_change: #{ payload: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                direction: #{ schema: #{ type: "string", allowed: ["ascending", "descending"] }, required: true, sensitive: false },
            } } } },
            selection_change: #{ payload: #{ type: "array", max_items: 10000, items: #{ type: "string" } } },
            row_click: #{ payload: #{ type: "string" } },
        },
        slots: #{
            loading_content: #{ required: false, multiple: false },
            empty: #{ required: false, multiple: false },
        },
        parts: ["root", "header", "header_cell", "body", "row", "cell", "selection_header", "selection_cell", "selection_indicator", "horizontal_scrollbar", "horizontal_scrollbar_thumb", "loading", "empty"],
    },
});

fn table_row_height(size) {
    if size == "xs" { 24 }
    else if size == "sm" { 28 }
    else if size == "lg" { 40 }
    else { 32 }
}

fn prop_or(props, name, fallback) {
    if name in props { props[name] } else { fallback }
}

fn default_loading(key, row_height) {
    let rows = [];
    for index in 0..100 {
        rows.push(skeleton::Skeleton(#{
            key: `${key}-loading-${index}`, width: 1, height: row_height - 4,
            animated: false
        }).with_style(style().width(relative(1.0))));
    }
    rows
}

fn Table(props) {
    component_render("components/table", props, Fn("render_Table"))
}
fn render_Table(ctx, props) {
    let row_height = table_row_height(prop_or(props, "size", "md"));
    let has_loading_content = "loading_content" in props && props.loading_content != ();
    let loading = prop_or(props, "loading", false);
    let config = #{
        key: props.key,
        label: props.label,
        columns: props.columns,
        rows: props.rows,
        row_key: props.row_key,
        height: props.height,
        row_height: row_height * 1.0,
        flex_min_width: 80.0,
        selection_width: row_height * 1.0,
        selection_size: row_height * 0.5,
        horizontal_scrollbar_height: row_height * 0.375,
        horizontal_scrollbar_thumb_min_width: row_height * 2.0,
        horizontal_scrollbar_inset: row_height * 0.125,
        overscan: 2,
        loading: loading,
        loading_rows: if loading && !has_loading_content {
            default_loading(props.key, row_height)
        } else { [] },
        empty_text: if prop_or(props, "empty_text", "") == "" {
            ctx.t("common.no_results")
        } else { props.empty_text },
        striped: prop_or(props, "striped", false),
        selection_mode: prop_or(props, "selection_mode", "none"),
        selected_keys: prop_or(props, "selected_keys", []),
        sort: if "sort" in props { props.sort } else { () },
        calendar: ctx.calendar(),
        number: ctx.number(),
        check_asset: asset("app/icons/check"),
        sort_ascending_asset: asset("app/icons/sort_ascending"),
        sort_descending_asset: asset("app/icons/sort_descending"),
    };
    if has_loading_content { config.loading_slot = props.loading_content; }
    if "empty" in props && props.empty != () { config.empty_slot = props.empty; }
    let node = table(config)
        .with_style(component_style(props, "root", style()))
        .with_part_style("header", component_style(props, "header",
            style().background(theme_color("surface_raised")).border_color(theme_color("border"))))
        .with_part_style("header_cell", component_style(props, "header_cell",
            style().padding_x(theme_spacing("sm"))))
        .with_part_style("body", component_style(props, "body", style()))
        .with_part_style("row", component_style(props, "row",
            style().border_color(theme_color("border"))))
        .with_part_style("cell", component_style(props, "cell",
            style().padding_x(theme_spacing("sm"))))
        .with_part_style("selection_header", component_style(props, "selection_header", style()))
        .with_part_style("selection_cell", component_style(props, "selection_cell", style()))
        .with_part_style("selection_indicator", component_style(props, "selection_indicator",
            style().radius(theme_radius("sm"))))
        .with_part_style("horizontal_scrollbar", component_style(props, "horizontal_scrollbar",
            style().background(theme_color("surface_raised"))))
        .with_part_style("horizontal_scrollbar_thumb", component_style(props,
            "horizontal_scrollbar_thumb",
            style().background(theme_color("text_muted")).radius(px(row_height))))
        .with_part_style("loading", component_style(props, "loading",
            style().flex_col().gap(theme_spacing("xs"))
                .text_color(theme_color("text_muted"))))
        .with_part_style("empty", component_style(props, "empty",
            style().text_color(theme_color("text_muted"))))
        .accessibility_role("table")
        .accessibility_label(props.label);
    let node = if "on_sort_change" in props && props.on_sort_change != () {
        node.on_sort_change(props.on_sort_change)
    } else { node };
    let node = if "on_selection_change" in props && props.on_selection_change != () {
        node.on_selection_change(props.on_selection_change)
    } else { node };
    if "on_row_click" in props && props.on_row_click != () {
        node.on_row_click(props.on_row_click)
    } else { node }
}
