/* gpui-rhai
{
  "id": "components/table",
  "export": "Table",
  "version": "0.1.2",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/disclosure_down.svg", "icons/chevron_right.svg", "icons/sort_ascending.svg", "icons/sort_descending.svg"]
}
*/
// Data-backed Table composed entirely from public Box/Text/virtual_collection APIs.
// Use either a positive logical-pixel height or fill_height for flex layouts.
// Props: key, label, row_key, Array-or-NativeCollection rows, columns,
// height, loading/empty, sort, selection, optional grouping, and native column resizing.
// Events: sort_change, selection_change, row_click, group_toggle, and column_resize.
// State: application values are controlled; keyed native width overrides retain drag previews.
// Example: Table(#{ key: "users", label: "Users", row_key: "id", rows: rows,
//     columns: columns, height: 420 })

define_component(#{
    metadata: #{ id: "components/table", "export": "Table", version: "0.1.2",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{},
        assets: ["icons/disclosure_down.svg", "icons/chevron_right.svg", "icons/sort_ascending.svg", "icons/sort_descending.svg"] },
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
                    resizable: #{ schema: #{ type: "optional", value: #{ type: "bool" } },
                        required: false, sensitive: false },
                    min_width: #{ schema: #{ type: "optional", value: #{ type: "number", exclusive_min: 0.0, max: 16384.0 } },
                        required: false, sensitive: false },
                    max_width: #{ schema: #{ type: "optional", value: #{ type: "number", exclusive_min: 0.0, max: 16384.0 } },
                        required: false, sensitive: false },
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
            group_by: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            collapsed_groups: #{ schema: #{ type: "array", max_items: 100000, items: #{ type: "string" } }, required: false, sensitive: false,
                "default": #{ type: "array", value: [] } },
            sticky_group_headers: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            resizable_columns: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_sort_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_selection_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_row_click: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_group_toggle: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_column_resize: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{
            sort_change: #{ payload: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                direction: #{ schema: #{ type: "string", allowed: ["ascending", "descending"] }, required: true, sensitive: false },
            } } } },
            selection_change: #{ payload: #{ type: "array", max_items: 100000, items: #{ type: "string" } } },
            row_click: #{ payload: #{ type: "string" } },
            group_toggle: #{ payload: #{ type: "string" } },
            column_resize: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                width: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                    kind: #{ schema: #{ type: "string", allowed: ["fixed"] }, required: true, sensitive: false },
                    value: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
                } }, required: true, sensitive: false },
            } } },
        },
        slots: #{
            loading_content: #{ required: false, multiple: false },
            empty: #{ required: false, multiple: false },
        },
        parts: ["root", "header", "header_cell", "resize_handle", "body", "group_header",
            "group_indicator", "group_label", "group_count", "loading", "empty"],
    },
    render: Fn("render_Table")
});

fn Table(props) { render_component("components/table", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn optional_or(props, name, fallback) {
    if name in props && props[name] != () { props[name] } else { fallback }
}

fn column_style(column) {
    let style = style().padding_x(theme_spacing("sm"));
    if column.width.kind == "fixed" {
        style.width(px(column.width.value)).flex_shrink(false)
    } else if column.width.kind == "percent" {
        style.width(relative(column.width.value / 100.0)).flex_shrink(false)
    } else {
        style.flex_basis(relative(0)).flex_grow(column.width.value).min_width(px(0))
    }
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
fn group_action(ctx, group) { ctx.emit("group_toggle", group); }
fn column_resize_action(ctx, payload) { ctx.emit("column_resize", payload); }

fn clamp_column_width(value, min_width, max_width) {
    let result = if value < min_width { min_width } else { value };
    if max_width != () && result > max_width { max_width } else { result }
}

fn column_resize_key(ctx, payload) {
    let current = ctx.get_signal(payload.signal_key);
    if current == () {
        let bounds = ctx.element_bounds(payload.ref_key);
        if bounds == () { return; }
        current = bounds.layout.width;
    }
    let width = clamp_column_width(current + payload.delta, payload.min_width, payload.max_width);
    ctx.set_signal(payload.signal_key, width);
    ctx.emit("column_resize", #{ key: payload.column_key,
        width: #{ kind: "fixed", value: width } });
}

fn render_table_row(ctx, payload) {
    let item = payload.item;
    if item.kind == "group" {
        let indicator = image_source(asset(if item.collapsed {
                "app/icons/chevron_right"
            } else { "app/icons/disclosure_down" }))
            .with_key(`${payload.key}-indicator`)
            .with_style(ctx.component_style("group_indicator",
                style().width(px(14)).height(px(14)).text_color(theme_color("text_muted"))));
        let label = text(item.group).with_key(`${payload.key}-label`)
            .with_style(ctx.component_style("group_label",
                style().flex_grow().typography("body_small").font_weight(700)));
        let count = text(`${item.count}`).with_key(`${payload.key}-count`)
            .with_style(ctx.component_style("group_count",
                style().typography("caption").text_color(theme_color("text_muted"))));
        let node = row([indicator, label, count])
            .with_key(payload.key)
            .with_style(ctx.component_style("group_header",
                style().height(px(item.height)).padding_x(theme_spacing("sm")).gap(px(6))
                    .items_center().border_bottom(px(1)).border_color(theme_color("border"))
                    .background(theme_color("surface_raised")).occlude_except_scroll()))
            .accessibility_role("rowheader")
            .accessibility_label(`${item.group}, ${item.count}`)
            .accessibility_checked(item.collapsed);
        return if item.toggle {
            node.on_click_value(Fn("group_action"), item.group)
        } else { node.tab_stop(false) };
    }
    let cells = [];
    for cell in item.cells {
        let node = text(cell.text).with_key(cell.key).with_style(
            column_style(cell).min_width(px(0)).whitespace_nowrap().text_ellipsis()
                .typography("body")
        );
        if "resize_signal_key" in cell {
            node = node.bind_parent_signal(ctx, "width_override", cell.resize_signal_key);
        }
        cells.push(node);
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

fn table_row_item(source, source_index, columns, row_key, label, selected,
        selection_mode, striped, row_height, display_index) {
    let key = `${source[row_key]}`;
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
    for column in columns {
        let cell = #{
            key: column.key, text: if column.key in source { `${source[column.key]}` } else { "" },
            width: column.width, align: prop_or(column, "align", "start"),
        };
        if "resize_signal_key" in column {
            cell.resize_signal_key = column.resize_signal_key;
        }
        cells.push(cell);
    }
    #{ kind: "row", key: key, label: `${label} row ${source_index + 1}`, cells: cells,
        selected: is_selected, selection: next_selection,
        striped: striped && display_index % 2 == 1, height: row_height }
}

fn unique_group_header_key(value, row_keys, header_keys) {
    let key = `__gpui_rhai_group__:${value}`;
    while key in row_keys || key in header_keys { key = `_${key}`; }
    key
}

fn table_column_model(props) {
    let table_resizable = prop_or(props, "resizable_columns", false);
    let requested = false;
    for index in 0..props.columns.len {
        if index + 1 < props.columns.len
                && optional_or(props.columns[index], "resizable", table_resizable) {
            requested = true;
        }
    }
    if !requested { return #{ columns: props.columns, signals: [] }; }
    let column_keys = [];
    let columns = [];
    let resize_signals = [];
    for index in 0..props.columns.len {
        let source = props.columns[index];
        if source.key in column_keys { throw `Table column key ${source.key} is duplicated`; }
        column_keys.push(source.key);
        let enabled = index + 1 < props.columns.len
            && optional_or(source, "resizable", table_resizable);
        let column = #{
            key: source.key, title: source.title, width: source.width,
            align: prop_or(source, "align", "start"), sortable: prop_or(source, "sortable", false),
            resize_enabled: enabled,
        };
        if enabled {
            let min_width = optional_or(source, "min_width", 48.0);
            let max_width = if "max_width" in source { source.max_width } else { () };
            if max_width != () && max_width < min_width {
                throw `Table column ${source.key} max_width must be at least min_width`;
            }
            let signal_key = `column-width-${index}`;
            column.resize_signal_key = signal_key;
            column.resize_ref_key = `column-header-${index}`;
            column.min_width = min_width;
            column.max_width = max_width;
            resize_signals.push(optional_float_signal(signal_key));
        } else { resize_signals.push(()); }
        columns.push(column);
    }
    #{ columns: columns, signals: resize_signals }
}

fn table_resize_handle(ctx, table_key, column, index, signal, header_ref) {
    let base = style().width(px(8)).height(relative(1.0)).flex_shrink(false)
        .margin_start(offset_px(-4)).margin_end(offset_px(-4)).cursor_resize_x()
        .focus(style().background(theme_color("selection")));
    let key_payload = #{ column_key: column.key,
        signal_key: column.resize_signal_key, ref_key: column.resize_ref_key,
        min_width: column.min_width, max_width: column.max_width, delta: -8.0 };
    let resize_props = #{ key: `${table_key}-column-resize-${index}`,
        column_key: column.key, source_kind: column.width.kind,
        source_value: column.width.value, min_width: column.min_width,
        signal: signal, column_ref: header_ref, on_resize: Fn("column_resize_action") };
    if column.max_width != () { resize_props.max_width = column.max_width; }
    let handle = gpui_rhai::ColumnResizePrimitive(resize_props)
        .with_style(ctx.component_style("resize_handle", base))
        .tab_stop(true).accessibility_role("separator")
        .accessibility_label(`Resize ${column.title} column`)
        .accessibility_orientation("vertical")
        .on_key_value("left", Fn("column_resize_key"), key_payload);
    key_payload.delta = 8.0;
    handle.on_key_value("right", Fn("column_resize_key"), key_payload)
}

fn table_header(ctx, props, columns, resize_signals, current_sort) {
    let headers = [];
    for index in 0..columns.len {
        let column = columns[index];
        let resize_enabled = prop_or(column, "resize_enabled", false);
        let resize_signal = if resize_enabled { resize_signals[index] } else { () };
        let content = [text(column.title).with_style(style()
            .min_width(px(0)).whitespace_nowrap().text_ellipsis())];
        if current_sort != () && current_sort.key == column.key {
            content.push(image_source(asset(if current_sort.direction == "ascending" {
                    "app/icons/sort_ascending"
                } else { "app/icons/sort_descending" }))
                .with_style(style().width(px(14)).height(px(14))
                    .text_color(theme_color("text_primary"))));
        }
        let children = [row(content).with_style(
            style().flex_grow().min_width(px(0)).height(relative(1.0)).items_center().gap(px(5)))];
        let header_ref = if resize_enabled { element_ref(column.resize_ref_key) } else { () };
        if resize_enabled {
            children.push(table_resize_handle(
                ctx, props.key, column, index, resize_signal, header_ref));
        }
        let cell = row(children).with_key(column.key)
            .with_style(ctx.component_style("header_cell",
                column_style(column).relative().min_width(px(0)).height(px(30)).items_center()
                    .typography("body_small").font_weight(700)
                    .hover(style().background(theme_color("surface_hover")))))
            .accessibility_role("columnheader").accessibility_label(column.title);
        if resize_enabled {
            cell = cell.bind_signal("width_override", resize_signal).with_ref(header_ref);
        }
        headers.push(if optional_or(column, "sortable", false) {
            cell.on_click_value(Fn("sort_action"), next_sort(current_sort, column.key))
        } else { cell });
    }
    row(headers).with_key(`${props.key}-header`)
        .with_style(ctx.component_style("header", style().height(px(30)).items_center()
            .border_bottom(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface_raised"))))
        .accessibility_role("row")
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
    let model = table_column_model(props);
    let columns = model.columns;
    let header = table_header(ctx, props, columns, model.signals, current_sort);

    if prop_or(props, "loading", false) {
        let loading = if "loading_content" in props && props.loading_content != () {
            props.loading_content
        } else { text("Loading…") };
        return box([header, loading.with_style(ctx.component_style("loading", style()
            .padding(px(12)).typography("body_small").text_color(theme_color("text_muted"))))])
            .with_key(props.key).with_style(ctx.component_style("root", style().flex_col()
                .border(px(1)).border_color(theme_color("border"))))
            .accessibility_role("table").accessibility_label(props.label);
    }
    if props.rows.len == 0 {
        let empty = if "empty" in props && props.empty != () { props.empty }
            else { text(prop_or(props, "empty_text", "No results")) };
        return box([header, empty.with_style(ctx.component_style("empty", style()
            .padding(px(12)).typography("body_small").text_color(theme_color("text_muted"))))])
            .with_key(props.key).with_style(ctx.component_style("root", style().flex_col()
                .border(px(1)).border_color(theme_color("border"))))
            .accessibility_role("table").accessibility_label(props.label);
    }

    let selected = prop_or(props, "selected_keys", []);
    let selection_mode = prop_or(props, "selection_mode", "none");
    let row_height = prop_or(props, "estimated_row_height", 30.0);
    let striped = prop_or(props, "striped", false);
    let group_by = if "group_by" in props { props.group_by } else { () };
    let collapsed = prop_or(props, "collapsed_groups", []);
    if group_by == () && collapsed.len > 0 {
        throw "Table collapsed_groups requires group_by";
    }
    if group_by != () {
        let found = false;
        for column in columns { if column.key == group_by { found = true; } }
        if !found { throw `Table group_by ${group_by} is not a declared column`; }
    }
    let group_toggle = "on_group_toggle" in props && props.on_group_toggle != ();
    let sticky_groups = prop_or(props, "sticky_group_headers", true);
    let native_data = is_native_collection(props.rows);
    let data;
    let sticky_headers = [];
    if native_data {
        data = native_table_view(props.rows, #{
            row_key: props.row_key, label: props.label, columns: columns,
            selected_keys: selected, selection_mode: selection_mode,
            striped: striped, row_height: row_height, sort: current_sort,
            group_by: group_by, collapsed_groups: collapsed, group_toggle: group_toggle,
        });
    } else if group_by == () {
        data = [];
        for index in 0..props.rows.len {
            data.push(table_row_item(props.rows[index], index, columns, props.row_key,
                props.label, selected, selection_mode, striped, row_height, index));
        }
    } else {
        data = [];
        let group_order = [];
        let buckets = #{};
        let row_keys = [];
        for index in 0..props.rows.len {
            let source = props.rows[index];
            let group = if group_by in source { source[group_by] } else { () };
            if type_of(group) != "string" || group == "" {
                throw `Table row ${index} group field ${group_by} must be a non-empty string`;
            }
            let key = `${source[props.row_key]}`;
            row_keys.push(key);
            if !(group in buckets) {
                group_order.push(group);
                buckets[group] = [];
            }
            let bucket = buckets[group];
            bucket.push(#{ source: source, index: index });
            buckets[group] = bucket;
        }
        let header_keys = [];
        let display_index = 0;
        for group in group_order {
            let rows = buckets[group];
            let header_key = unique_group_header_key(group, row_keys, header_keys);
            header_keys.push(header_key);
            sticky_headers.push(data.len);
            let is_collapsed = group in collapsed;
            data.push(#{ kind: "group", key: header_key, group: group, count: rows.len,
                collapsed: is_collapsed, toggle: group_toggle, height: row_height });
            if !is_collapsed {
                for row in rows {
                    data.push(table_row_item(row.source, row.index, columns, props.row_key,
                        props.label, selected, selection_mode, striped, row_height, display_index));
                    display_index += 1;
                }
            }
        }
    }
    let collection = #{
        key: `${props.key}-body`, label: props.label, data: data,
        estimated_height: row_height,
        overdraw_pixels: row_height * 4.0,
        alignment: "top", follow_tail: false
    };
    if !native_data || !sticky_groups { collection.sticky_headers = if sticky_groups { sticky_headers } else { [] }; }
    if fill_height { collection.fill_height = true; }
    else { collection.height = props.height; }
    let body_style = if fill_height {
        style().flex_grow().min_width(px(0)).min_height(px(0))
    } else { style().min_width(px(0)) };
    let body = virtual_collection(collection, Fn("render_table_row"))
        .with_style(ctx.component_style("body", body_style));
    let root_style = if fill_height {
        style().flex_col().flex_grow().min_width(px(0)).min_height(px(0))
    } else { style().flex_col().min_width(px(0)) };
    box([header, body]).with_key(props.key)
        .with_style(ctx.component_style("root", root_style.overflow_x_scroll()
            .border(px(1)).border_color(theme_color("border"))))
        .accessibility_role("table").accessibility_label(props.label)
}
