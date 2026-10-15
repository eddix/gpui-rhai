/* gpui-rhai
{
  "id": "components/column_browser",
  "export": "ColumnBrowser",
  "version": "0.2.2",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/list", "components/button"],
  "capabilities": {},
  "tokens": ["border", "text_primary", "text_muted", "focus_ring", "metrics.row", "metrics.inset", "spacing.xs", "spacing.sm", "typography.label", "typography.body"],
  "environment": ["density"]
}
*/
// Controlled Miller columns. Data loading, cancellation, request revisions and
// dropping descendants belong to the caller, not this presentational component.
import "components/list" as list;
import "components/button" as button;

define_component(#{
    metadata: #{ id: "components/column_browser", "export": "ColumnBrowser", version: "0.2.2",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/list", "components/button"], capabilities: #{},
        tokens: ["border", "text_primary", "text_muted", "focus_ring", "metrics.row", "metrics.inset", "spacing.xs", "spacing.sm", "typography.label", "typography.body"],
        environment: ["density"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Accessible name of the browser." },
            columns: #{ schema: #{ type: "array", max_items: 128, items: #{ type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Stable unique column identity; every event names this key." },
                title: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Column heading and listbox name." },
                items: #{ schema: #{ type: "one_of", variants: [#{ type: "collection" }, #{ type: "array", max_items: 10000,
                    items: #{ type: "object", allow_unknown: true, fields: #{} } }] }, required: true, sensitive: false,
                    doc: "List row grammar plus has_children; a NativeCollection stays native, including keyboard target lookup." },
                selected: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                    doc: "Controlled selected key. A disappeared selection is not replaced or emitted during render." },
                width: #{ schema: #{ type: "optional", value: #{ type: "one_of", variants: [#{ type: "number", exclusive_min: 0.0 }, #{ type: "length" }] } }, required: false, sensitive: false,
                    doc: "Column width in pixels or Length, overriding column_width; columns never shrink to fit." },
                loading: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: false }, doc: "Caller-owned loading state for this column alone." },
                empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false, "default": #{ type: "string", value: "No items" }, doc: "Text for a loaded empty column." },
                error: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false, doc: "Caller-owned error; loading takes precedence, then error, then items." },
            } } }, required: true, sensitive: false, doc: "Controlled column chain. After selection the caller replaces descendants and guards late loading results by path/request revision." },
            path: #{ schema: #{ type: "optional", value: #{ type: "array", max_items: 128, items: #{ type: "string" } } }, required: false, sensitive: false,
                doc: "Optional controlled selected path; when present it supplies each column's selected key instead of selected. No longer than columns." },
            height: #{ schema: #{ type: "optional", value: #{ type: "one_of", variants: [#{ type: "number", exclusive_min: 0.0 }, #{ type: "length" }] } }, required: false, sensitive: false, doc: "Total browser height; give height or fill_height, not both." },
            fill_height: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: false }, doc: "Fill the bounded flex parent's remaining height." },
            column_width: #{ schema: #{ type: "one_of", variants: [#{ type: "number", exclusive_min: 0.0 }, #{ type: "length" }] }, required: false, sensitive: false,
                "default": #{ type: "float", value: 240.0 }, doc: "Default nonshrinking column width; the strip overflows horizontally in narrow windows." },
            detail: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false, doc: "Optional caller-owned trailing detail pane, using column_width." },
            show_path: #{ schema: #{ type: "bool" }, required: false, sensitive: false, "default": #{ type: "bool", value: true }, doc: "Show the selectable path, level breadcrumbs and Copy path action." },
            path_separator: #{ schema: #{ type: "string" }, required: false, sensitive: false, "default": #{ type: "string", value: "/" }, doc: "Separator used for display and clipboard text." },
            on_select: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false, doc: "Called with #{ column, key, path }; the caller owns updating the chain." },
            on_activate: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false, doc: "Called with #{ column, key, path } on Enter or double click of a leaf." },
            on_retry: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false, doc: "Called with the failed column key; retry and loading remain caller-owned." },
        },
        state: #{ fields: #{ active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } } } },
        events: #{
            select: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                column: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                path: #{ schema: #{ type: "array", max_items: 128, items: #{ type: "string" } }, required: true, sensitive: false },
            } }, doc: "Proposal selecting a row and truncating the path at its level." },
            activate: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                column: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                path: #{ schema: #{ type: "array", max_items: 128, items: #{ type: "string" } }, required: true, sensitive: false },
            } }, doc: "Leaf activation, distinct from selection." },
            retry: #{ payload: #{ type: "string" }, doc: "Retry requested for a column; the browser starts no work." },
        },
        slots: #{ detail: #{ required: false, multiple: false, doc: "Caller-owned detail pane after the last column." } },
        parts: ["root", "path", "path_text", "strip", "column", "heading", "status", "detail", "focus_frame"],
        effects: ["reveal"],
    },
    render: Fn("render_ColumnBrowser")
});

fn ColumnBrowser(props) { render_component("components/column_browser", props) }
fn value_or(value, name, fallback) { if name in value && value[name] != () { value[name] } else { fallback } }
fn length(value) { if type_of(value) == "Length" { value } else { px(value) } }
fn payload(target, key) {
    let path = target.ancestors;
    path.push(key);
    #{ column: target.column, key: key, path: path }
}
fn select_row(target, ctx, key) {
    ctx.set_state("active", target.column);
    ctx.emit("select", payload(target, key));
    ctx.focus("browser-focus");
    handled()
}
fn activate_row(target, ctx, item) {
    if !item.has_children { ctx.emit("activate", payload(target, item.key)); }
    ctx.focus("browser-focus");
    handled()
}
fn key_activate(target, ctx, key) { ctx.emit("activate", payload(target, key)); handled() }
fn move_column(target, ctx, key) {
    ctx.set_state("active", target.column);
    if key != () { ctx.emit("select", payload(target, key)); }
    ctx.scroll_into_view(target.reference);
    ctx.focus("browser-focus");
    handled()
}
fn breadcrumb(target, key, ctx, event) { move_column(target, ctx, key) }
fn retry(key, ctx, event) { ctx.emit("retry", key); ctx.focus("browser-focus"); }
fn retry_key(ctx, key) { ctx.emit("retry", key); handled() }
fn copy_path(path, ctx, event) { ctx.copy_text(path); ctx.focus("browser-focus"); }
fn copy_key(ctx, path) { ctx.copy_text(path); handled() }
fn reveal(ctx, deps) { ctx.scroll_into_view("newest-column"); }
fn cleanup(ctx, deps) {}

fn render_ColumnBrowser(ctx, props) {
    let fill = value_or(props, "fill_height", false);
    let height = value_or(props, "height", ());
    if fill == (height != ()) { throw "ColumnBrowser requires exactly one of height or fill_height"; }
    let path = [];
    let has_path = value_or(props, "path", ()) != ();
    if has_path && props.path.len > props.columns.len { throw "ColumnBrowser path exceeds columns"; }
    let seen = #{};
    let active = 0;
    let wanted = ctx.get_state("active");
    for index in 0..props.columns.len {
        let column = props.columns[index];
        if column.key in seen { throw `ColumnBrowser duplicate column key ${column.key}`; }
        seen[column.key] = true;
        let selected = if has_path { if index < props.path.len { props.path[index] } else { () } }
            else { value_or(column, "selected", ()) };
        if selected != () && path.len == index { path.push(selected); }
        if column.key == wanted { active = index; }
    }
    if wanted != "" && !(wanted in seen) { active = if props.columns.len == 0 { 0 } else { props.columns.len - 1 }; }
    let children = [];
    let parts = [];
    let targets = [];
    let neighbors = [];
    let order = [];
    for index in 0..props.columns.len {
        let column = props.columns[index];
        order.push(column.key);
        let selected = if index < path.len { path[index] } else { () };
        let ancestors = [];
        for parent in 0..index { if parent < path.len { ancestors.push(path[parent]); } }
        let reference = if index == props.columns.len - 1 && value_or(props, "detail", ()) == () {
            "newest-column"
        } else { `column-${index}` };
        let target = #{ column: column.key, ancestors: ancestors, reference: reference };
        targets.push(target);
        let available = !value_or(column, "loading", false) && value_or(column, "error", ()) == ();
        let nearby = if available {
            let data = native_list_view(column.items, if selected == () { [] } else { [selected] });
            native_list_neighbors(data, if selected == () { [] } else { [selected] })
        } else { #{ first: (), last: (), previous: (), next: (), current: (), current_index: (), has_children: false } };
        neighbors.push(nearby);
        let body;
        if value_or(column, "error", ()) != () && !value_or(column, "loading", false) {
            let status = [text(column.error).selectable(true)];
            if value_or(props, "on_retry", ()) != () {
                status.push(button::Button(#{ key: `retry-${index}`, text: "Retry", variant: "ghost", on_click: Fn("retry").curry(column.key) }).tab_stop(false));
            }
            body = column(status).with_style(ctx.component_style("status", style().flex_grow().min_height(px(0))
                .padding_x(theme_length("metrics.inset")).padding_y(theme_spacing("sm"))
                .gap(theme_spacing("sm")).text_color(theme_color("text_muted"))));
        } else {
            body = list::List(#{ key: `list-${column.key}`, label: `${column.title}, level ${index + 1} of ${props.columns.len}`,
                items: column.items, fill_height: true, selection_mode: "single",
                selected_keys: if selected == () { [] } else { [selected] }, keyboard_navigation: false, active: index == active,
                loading: value_or(column, "loading", false), empty_text: value_or(column, "empty_text", "No items"),
                on_row_click: Fn("select_row").curry(target),
                on_row_activate: Fn("activate_row").curry(target) });
        }
        let heading = text(column.title.to_upper()).with_style(ctx.component_style("heading", style()
            .min_height(theme_length("metrics.row")).padding_x(theme_length("metrics.inset"))
            .typography("label").text_color(theme_color("text_muted"))));
        parts.push(column([heading, body]).with_key(`column-${column.key}`).with_ref(element_ref(reference))
            .with_style(ctx.component_style("column", style().width(length(value_or(column, "width", value_or(props, "column_width", 240.0))))
                .min_width(px(0)).min_height(px(0)).height(relative(1.0)).flex_shrink(false)
                .border_end(px(1)).border_color(theme_color("border")))));
    }
    if value_or(props, "detail", ()) != () {
        parts.push(column([props.detail]).with_key("detail").with_ref(element_ref("newest-column"))
            .with_style(ctx.component_style("detail", style().width(length(value_or(props, "column_width", 240.0)))
                .min_width(px(0)).min_height(px(0)).height(relative(1.0)).flex_shrink(false))));
    }
    let joined = "";
    for index in 0..path.len {
        if index > 0 { joined += props.path_separator; }
        joined += path[index];
    }
    if value_or(props, "show_path", true) {
        let crumbs = [];
        for index in 0..path.len {
            crumbs.push(button::Button(#{ key: `crumb-${index}`, text: path[index], variant: "ghost",
                on_click: Fn("breadcrumb").curry(targets[index]).curry(path[index]) }).tab_stop(false));
        }
        crumbs.push(text(joined).selectable(true).with_style(ctx.component_style("path_text", style()
            .min_width(px(0)).whitespace_nowrap().text_ellipsis().typography("body"))));
        crumbs.push(button::Button(#{ key: "copy-path", text: "Copy path", variant: "ghost", disabled: path.len == 0,
            on_click: Fn("copy_path").curry(joined) }).tab_stop(false));
        children.push(row(crumbs).with_key("path").with_style(ctx.component_style("path", style()
            .flex_shrink(false).min_width(px(0)).items_center().gap(theme_spacing("xs")).overflow_x_scroll())));
    }
    children.push(row(parts).with_key("strip").with_style(ctx.component_style("strip", style()
        .flex_grow().min_width(px(0)).min_height(px(0)).overflow_x_scroll())));
    if targets.len == 0 || neighbors[active].current == () {
        let frame = style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
            .border(px(2)).border_color(rgba(0x00000000)).group_focus(style().border_color(theme_color("focus_ring")));
        children.push(row([]).with_style(ctx.component_style("focus_frame", frame)));
    }
    let root = column(children).with_key(props.key).with_ref(element_ref("browser-focus")).tab_stop(true)
        .accessibility_role("group").accessibility_label(props.label)
        .with_style(ctx.component_style("root", (if fill { style().flex_grow() } else { style().height(length(height)) })
            .relative().min_width(px(0)).min_height(px(0)).focus(style())));
    if targets.len > 0 {
        let nearby = neighbors[active];
        let target = targets[active];
        if nearby.first != () {
            root = root.on_key_value("up", Fn("select_row").curry(target), nearby.previous)
                .on_key_value("down", Fn("select_row").curry(target), nearby.next)
                .on_key_value("home", Fn("select_row").curry(target), nearby.first)
                .on_key_value("end", Fn("select_row").curry(target), nearby.last);
        }
        if nearby.current != () && !nearby.has_children {
            root = root.on_key_value("enter", Fn("key_activate").curry(target), nearby.current);
        }
        if value_or(props.columns[active], "error", ()) != () && value_or(props, "on_retry", ()) != ()
            && !value_or(props.columns[active], "loading", false) {
            root = root.on_key_value("enter", Fn("retry_key"), target.column);
        }
        // on_key_value already receives logical Left/Right from the public
        // runtime; a second locale reversal here would break RTL navigation.
        if active + 1 < targets.len && nearby.current != () && nearby.has_children {
            root = root.on_key_value("right", Fn("move_column").curry(targets[active + 1]), neighbors[active + 1].first);
        }
        if active > 0 {
            root = root.on_key_value("left", Fn("move_column").curry(targets[active - 1]), ());
        }
        effect("reveal", #{ order: order, detail: value_or(props, "detail", ()) != () }, Fn("reveal"), Fn("cleanup"));
    }
    if path.len > 0 {
        root = root.on_key_value("cmd+shift+c", Fn("copy_key"), joined)
            .on_key_value("ctrl+shift+c", Fn("copy_key"), joined);
    }
    root
}
