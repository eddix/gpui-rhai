/* gpui-rhai
{
  "id": "components/diff_viewer",
  "export": "DiffViewer",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["surface"],
  "environment": []
}
*/
// Neutral two-way, read-only source comparison over the native document surface.
// Example: DiffViewer(#{ key: "servers", left: #{ source: a, label: "Server A" }, right: #{ source: b, label: "Server B" }, mode: "split" })

define_component(#{
    metadata: #{ id: "components/diff_viewer", "export": "DiffViewer", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["surface"],
        environment: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable viewer identity; keeps scroll, folds, selection and search across renders." },
            left: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                source: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "document" }] }, required: true, sensitive: false,
                    doc: "Text of this side: a string, or a Host-owned `NativeTextDocument`." },
                label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Name of this side in the header, the accessible name and copied patches." },
                file_name: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                    doc: "File name whose extension picks this side's syntax when `language` is not set." },
                language: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                    doc: "Syntax to highlight this side with; takes precedence over `file_name`." },
            } }, required: true, sensitive: false,
                doc: "Document on the left in split mode; left and right carry no old or new meaning." },
            right: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                source: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "document" }] }, required: true, sensitive: false,
                    doc: "Text of this side: a string, or a Host-owned `NativeTextDocument`." },
                label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Name of this side in the header, the accessible name and copied patches." },
                file_name: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                    doc: "File name whose extension picks this side's syntax when `language` is not set." },
                language: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                    doc: "Syntax to highlight this side with; takes precedence over `file_name`." },
            } }, required: true, sensitive: false,
                doc: "Document on the right in split mode." },
            mode: #{ schema: #{ type: "string", allowed: ["unified", "split"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "unified" },
                doc: "`unified` interleaves both sides in one column; `split` aligns them side by side with one scroll position." },
            whitespace: #{ schema: #{ type: "string", allowed: ["exact", "ignore_changes", "ignore_all"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "exact" },
                doc: "`exact` compares lines as written, `ignore_changes` collapses runs of whitespace, `ignore_all` drops all whitespace." },
            context_lines: #{ schema: #{ type: "one_of", variants: [
                #{ type: "integer", min: 0, max: 100 }, #{ type: "string", allowed: ["all"] }
            ] }, required: false, sensitive: false, "default": #{ type: "integer", value: 3 },
                doc: "Unchanged lines kept around each hunk; longer equal runs fold, and `\"all\"` never folds." },
            show_line_numbers: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Shows the line-number gutters." },
            wrap: #{ schema: #{ type: "string", allowed: ["none", "viewport", "column"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "none" },
                doc: "`none` scrolls long lines sideways, `viewport` wraps them at the pane width, `column` at `wrap_column`." },
            wrap_column: #{ schema: #{ type: "integer", min: 20, max: 500 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 100 },
                doc: "Display column at which lines wrap when `wrap` is `column`." },
            tab_size: #{ schema: #{ type: "integer", min: 1, max: 16 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 4 },
                doc: "Columns between tab stops when tabs are expanded for display." },
            on_location_activate: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `#{ side, line, column }` when a side's text is double-clicked or Enter is pressed in it." },
        },
        state: #{ fields: #{} },
        events: #{ location_activate: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
            side: #{ schema: #{ type: "string", allowed: ["left", "right"] }, required: true, sensitive: false,
                doc: "Side the location belongs to." },
            line: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "One-based line in that side's source." },
            column: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "One-based column, counted in characters from the start of the line." },
        } }, doc: "Emitted on a double-click or Enter in either side's text; the payload is the side and its one-based location." } },
        slots: #{}, parts: ["root", "header", "gutter", "line", "text", "fold", "loading", "error", "search", "status"],
    },
    render: Fn("render_DiffViewer")
});

fn DiffViewer(props) { render_component("components/diff_viewer", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn side_or(side, name, fallback) { if name in side && side[name] != () { side[name] } else { fallback } }
fn location_activate(ctx, payload) { ctx.emit("location_activate", payload); }

fn render_DiffViewer(ctx, props) {
    let left = props.left;
    let right = props.right;
    let root_style = ctx.component_style("root", style()
        .width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))
        .background(theme_color("surface")));
    gpui_rhai::DiffViewPrimitive(#{
        key: props.key,
        left_source: left.source,
        left_label: left.label,
        left_file_name: side_or(left, "file_name", ()),
        left_language: side_or(left, "language", ()),
        right_source: right.source,
        right_label: right.label,
        right_file_name: side_or(right, "file_name", ()),
        right_language: side_or(right, "language", ()),
        mode: prop_or(props, "mode", "unified"),
        whitespace: prop_or(props, "whitespace", "exact"),
        context_lines: prop_or(props, "context_lines", 3),
        show_line_numbers: prop_or(props, "show_line_numbers", true),
        wrap: prop_or(props, "wrap", "none"),
        wrap_column: prop_or(props, "wrap_column", 100),
        tab_size: prop_or(props, "tab_size", 4),
        header_style: ctx.component_style("header", style()),
        gutter_style: ctx.component_style("gutter", style()),
        line_style: ctx.component_style("line", style()),
        text_style: ctx.component_style("text", style()),
        fold_style: ctx.component_style("fold", style()),
        loading_style: ctx.component_style("loading", style()),
        error_style: ctx.component_style("error", style()),
        search_style: ctx.component_style("search", style()),
        status_style: ctx.component_style("status", style()),
        on_location_activate: if "on_location_activate" in props && props.on_location_activate != () {
            Fn("location_activate")
        } else { () },
    }).with_key(props.key).with_style(root_style).accessibility_role("document")
        .accessibility_label(`${left.label} compared with ${right.label}`)
}
