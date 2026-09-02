/* gpui-rhai
{
  "id": "components/table",
  "export": "Table",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {},
  "assets": []
}
*/
// Data-backed Table composed entirely from public Box/Text/virtual_collection APIs.
// Use either a positive logical-pixel height or fill_height for flex layouts.
// Props: key, label, row_key, Array-or-NativeCollection rows, columns,
// height, loading/empty, sort and selection.
// Events: sort_change, selection_change, and row_click.
// State: controlled values only; realized item component state is keyed by row.
// Example: Table(#{ key: "users", label: "Users", row_key: "id", rows: rows,
//     columns: columns, height: 420 })

define_component(#{
    metadata: #{ id: "components/table", "export": "Table", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}, assets: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            row_key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            rows: #{ schema: #{ type: "one_of", variants: [
                #{ type: "array", max_items: 100000,
                    items: #{ type: "map", values: #{ type: "ui_value" } } },
                #{ type: "collection" },
            ] }, required: true, sensitive: false },
            columns: #{ schema: #{ type: "array", max_items: 256, items: #{
                type: "object", allow_unknown: false, fields: #{
                    key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    width: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                        kind: #{ schema: #{ type: "string", allowed: ["fixed", "percent", "flex"] }, required: true, sensitive: false },
                        value: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
                    } }, required: true, sensitive: false },
                    align: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["start", "center", "end"] } }, required: false, sensitive: false },
                    sortable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                },
            } }, required: true, sensitive: false },
            height: #{ schema: #{ type: "optional", value: #{ type: "number", exclusive_min: 0.0 } }, required: false, sensitive: false },
            fill_height: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            estimated_row_height: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: false, sensitive: false,
                "default": #{ type: "float", value: 30.0 } },
            loading: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            loading_content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            empty: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "No results" } },
            striped: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            selection_mode: #{ schema: #{ type: "string", allowed: ["none", "single", "multiple"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "none" } },
            selected_keys: #{ schema: #{ type: "array", max_items: 100000, items: #{ type: "string" } }, required: false, sensitive: false,
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
            selection_change: #{ payload: #{ type: "array", max_items: 100000, items: #{ type: "string" } } },
            row_click: #{ payload: #{ type: "string" } },
        },
        slots: #{
            loading_content: #{ required: false, multiple: false },
            empty: #{ required: false, multiple: false },
        },
        parts: ["root", "header", "header_cell", "body", "loading", "empty"],
    },
    render: Fn("render_Table")
});

fn Table(props) { render_component("components/table", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }

fn column_style(column) {
    let style = style().padding_x(theme_spacing("sm"));
    if column.width.kind == "fixed" { style.width(px(column.width.value)) }
    else if column.width.kind == "percent" { style.width(relative(column.width.value / 100.0)) }
    else { style.width(px(column.width.value)) }
}

fn next_sort(current, key) {
    if current != () && current.key == key {
        if current.direction == "ascending" { #{ key: key, direction: "descending" } }
        else { () }
    } else { #{ key: key, direction: "ascending" } }
}

fn sort_action(ctx, payload) { ctx.emit("sort_change", payload); }
fn row_action(ctx, payload) {
    ctx.emit("row_click", payload.key);
    if payload.selection != () { ctx.emit("selection_change", payload.selection); }
}

fn render_table_row(ctx, payload) {
    let item = payload.item;
    let cells = [];
    for cell in item.cells {
        cells.push(text(cell.text).with_key(cell.key).with_style(
            column_style(cell).min_width(px(0)).whitespace_nowrap().text_ellipsis()
                .font_size(px(12)).line_height(px(16))
        ));
    }
    let base = style().height(px(item.height)).items_center()
        .overflow_hidden()
        .border_bottom(px(1)).border_color(theme_color("border"))
        .background(if item.selected { theme_color("selection") }
            else if item.striped { theme_color("surface_raised") }
            else { rgba(0x00000000) });
    if !item.selected {
        base = base.hover(style().background(theme_color("surface_hover")));
    }
    let node = row(cells)
        .with_key(payload.key)
        .with_style(base)
        .accessibility_role("row")
        .accessibility_label(item.label);
    node.on_click_value(Fn("row_action"), #{ key: payload.key, selection: item.selection })
}

fn render_Table(ctx, props) {
    let fill_height = prop_or(props, "fill_height", false);
    if fill_height && "height" in props {
        throw "Table accepts either height or fill_height, not both";
    }
    if !fill_height && !("height" in props) {
        throw "Table requires height or fill_height: true";
    }
    let current_sort = if "sort" in props { props.sort } else { () };
    let headers = [];
    for column in props.columns {
        let title = if current_sort != () && current_sort.key == column.key {
            `${column.title} ${if current_sort.direction == "ascending" { "↑" } else { "↓" }}`
        } else { column.title };
        let header = text(title).with_key(column.key)
            .with_style(component_style(props, "header_cell",
                column_style(column).min_width(px(0)).height(px(30)).items_center()
                    .whitespace_nowrap().text_ellipsis()
                    .font_size(px(11)).line_height(px(14)).font_weight(700)
                    .hover(style().background(theme_color("surface_hover")))))
            .accessibility_role("columnheader");
        headers.push(if prop_or(column, "sortable", false) {
            header.on_click_value(Fn("sort_action"), next_sort(current_sort, column.key))
        } else { header });
    }
    let header = row(headers).with_key(`${props.key}-header`)
        .with_style(component_style(props, "header",
            style().height(px(30)).items_center()
                .border_bottom(px(1)).border_color(theme_color("border"))
                .background(theme_color("surface_raised"))))
        .accessibility_role("row");

    if prop_or(props, "loading", false) {
        let loading = if "loading_content" in props && props.loading_content != () {
            props.loading_content
        } else { text("Loading…") };
        return box([header, loading.with_style(component_style(props, "loading", style()
            .padding(px(12)).font_size(px(11)).text_color(theme_color("text_muted"))))])
            .with_key(props.key).with_style(component_style(props, "root", style().flex_col()
                .border(px(1)).border_color(theme_color("border"))))
            .accessibility_role("table").accessibility_label(props.label);
    }
    if props.rows.len == 0 {
        let empty = if "empty" in props && props.empty != () { props.empty }
            else { text(prop_or(props, "empty_text", "No results")) };
        return box([header, empty.with_style(component_style(props, "empty", style()
            .padding(px(12)).font_size(px(11)).text_color(theme_color("text_muted"))))])
            .with_key(props.key).with_style(component_style(props, "root", style().flex_col()
                .border(px(1)).border_color(theme_color("border"))))
            .accessibility_role("table").accessibility_label(props.label);
    }

    let selected = prop_or(props, "selected_keys", []);
    let selection_mode = prop_or(props, "selection_mode", "none");
    let row_height = prop_or(props, "estimated_row_height", 30.0);
    let striped = prop_or(props, "striped", false);
    let data;
    if is_native_collection(props.rows) {
        data = native_table_view(props.rows, #{
            row_key: props.row_key, label: props.label, columns: props.columns,
            selected_keys: selected, selection_mode: selection_mode,
            striped: striped, row_height: row_height, sort: current_sort,
        });
    } else {
        data = [];
        for index in 0..props.rows.len {
            let source = props.rows[index];
            let key = `${source[props.row_key]}`;
            let is_selected = key in selected;
            let next_selection = if selection_mode == "none" { () }
                else if selection_mode == "single" { if is_selected { [] } else { [key] } }
                else {
                    let result = [];
                    for candidate in selected { if candidate != key { result.push(candidate); } }
                    if !is_selected { result.push(key); }
                    result
                };
            let cells = [];
            for column in props.columns {
                cells.push(#{
                    key: column.key, text: if column.key in source { `${source[column.key]}` } else { "" },
                    width: column.width, align: prop_or(column, "align", "start")
                });
            }
            data.push(#{ key: key, label: `${props.label} row ${index + 1}`, cells: cells,
                selected: is_selected, selection: next_selection,
                striped: striped && index % 2 == 1, height: row_height });
        }
    }
    let collection = #{
        key: `${props.key}-body`, label: props.label, data: data,
        estimated_height: row_height,
        overdraw_pixels: row_height * 4.0,
        alignment: "top", follow_tail: false
    };
    if fill_height { collection.fill_height = true; }
    else { collection.height = props.height; }
    let body_style = if fill_height {
        style().flex_grow().min_width(px(0)).min_height(px(0))
    } else { style().min_width(px(0)) };
    let body = virtual_collection(collection, Fn("render_table_row"))
        .with_style(component_style(props, "body", body_style));
    let root_style = if fill_height {
        style().flex_col().flex_grow().min_width(px(0)).min_height(px(0))
    } else { style().flex_col().min_width(px(0)) };
    box([header, body]).with_key(props.key)
        .with_style(component_style(props, "root", root_style.overflow_x_scroll()
            .border(px(1)).border_color(theme_color("border"))))
        .accessibility_role("table").accessibility_label(props.label)
}
