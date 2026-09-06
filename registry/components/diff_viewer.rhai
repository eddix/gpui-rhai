/* gpui-rhai
{
  "id": "components/diff_viewer",
  "export": "DiffViewer",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Neutral two-way, read-only source comparison over the native document surface.
// Props: key, left/right descriptors, unified/split mode, comparison and display policy.
// Events: location_activate(#{ side: "left" | "right", line, column }).
// Parts: root, header, gutter, line, text, fold, loading, error, search, status.
// Example: DiffViewer(#{ key: "servers", left: #{ source: a, label: "Server A" }, right: #{ source: b, label: "Server B" }, mode: "split" })

define_component(#{
    metadata: #{ id: "components/diff_viewer", "export": "DiffViewer", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            left: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                source: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "document" }] }, required: true, sensitive: false },
                label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                file_name: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                language: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            } }, required: true, sensitive: false },
            right: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                source: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "document" }] }, required: true, sensitive: false },
                label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                file_name: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                language: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            } }, required: true, sensitive: false },
            mode: #{ schema: #{ type: "string", allowed: ["unified", "split"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "unified" } },
            whitespace: #{ schema: #{ type: "string", allowed: ["exact", "ignore_changes", "ignore_all"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "exact" } },
            context_lines: #{ schema: #{ type: "one_of", variants: [
                #{ type: "integer", min: 0, max: 100 }, #{ type: "string", allowed: ["all"] }
            ] }, required: false, sensitive: false, "default": #{ type: "integer", value: 3 } },
            show_line_numbers: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            wrap: #{ schema: #{ type: "string", allowed: ["none", "viewport", "column"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "none" } },
            wrap_column: #{ schema: #{ type: "integer", min: 20, max: 500 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 100 } },
            tab_size: #{ schema: #{ type: "integer", min: 1, max: 16 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 4 } },
            on_location_activate: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{ location_activate: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
            side: #{ schema: #{ type: "string", allowed: ["left", "right"] }, required: true, sensitive: false },
            line: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false },
            column: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false },
        } } } },
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
    let root_style = component_style(props, "root", style()
        .width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))
        .background(theme_color("surface")).border(px(1)).border_color(theme_color("border")));
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
        header_style: component_style(props, "header", style()),
        gutter_style: component_style(props, "gutter", style()),
        line_style: component_style(props, "line", style()),
        text_style: component_style(props, "text", style()),
        fold_style: component_style(props, "fold", style()),
        loading_style: component_style(props, "loading", style()),
        error_style: component_style(props, "error", style()),
        search_style: component_style(props, "search", style()),
        status_style: component_style(props, "status", style()),
        on_location_activate: if "on_location_activate" in props && props.on_location_activate != () {
            Fn("location_activate")
        } else { () },
    }).with_key(props.key).with_style(root_style).accessibility_role("document")
        .accessibility_label(`${left.label} compared with ${right.label}`)
}
