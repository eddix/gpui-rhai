/* gpui-rhai
{
  "id": "components/code_viewer",
  "export": "CodeViewer",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Native, virtualized, selectable read-only source document surface.
// Props: key, source (string or NativeTextDocument), label, language/file hints,
// wrapping, line-number policy, tab size, styles, and location activation.
// Events: location_activate(#{ line, column }).
// Parts: root, gutter, line, text, loading, error, search.
// Example: CodeViewer(#{ key: "source", source: rhai_source, label: "Rhai source", language: "rhai" })

define_component(#{
    metadata: #{ id: "components/code_viewer", "export": "CodeViewer", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            source: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "document" }] },
                required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Code" } },
            file_name: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            language: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
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
            line: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false },
            column: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false },
        } } } },
        slots: #{}, parts: ["root", "gutter", "line", "text", "loading", "error", "search"],
    },
    render: Fn("render_CodeViewer")
});

fn CodeViewer(props) { render_component("components/code_viewer", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn location_activate(ctx, payload) { ctx.emit("location_activate", payload); }

fn render_CodeViewer(ctx, props) {
    let root_style = ctx.component_style("root", style()
        .width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))
        .background(theme_color("surface")).border(px(1)).border_color(theme_color("border")));
    gpui_rhai::CodeViewPrimitive(#{
        key: props.key,
        source: props.source,
        label: prop_or(props, "label", "Code"),
        file_name: prop_or(props, "file_name", ()),
        language: prop_or(props, "language", ()),
        show_line_numbers: prop_or(props, "show_line_numbers", true),
        wrap: prop_or(props, "wrap", "none"),
        wrap_column: prop_or(props, "wrap_column", 100),
        tab_size: prop_or(props, "tab_size", 4),
        gutter_style: ctx.component_style("gutter", style()
            .background(theme_color("surface_raised"))),
        line_style: ctx.component_style("line", style()),
        text_style: ctx.component_style("text", style()),
        loading_style: ctx.component_style("loading", style()),
        error_style: ctx.component_style("error", style()),
        search_style: ctx.component_style("search", style()),
        on_location_activate: if "on_location_activate" in props && props.on_location_activate != () {
            Fn("location_activate")
        } else { () },
    }).with_key(props.key).with_style(root_style).accessibility_role("document")
        .accessibility_label(prop_or(props, "label", "Code"))
}
