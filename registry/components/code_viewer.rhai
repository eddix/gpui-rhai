/* gpui-rhai
{
  "id": "components/code_viewer",
  "export": "CodeViewer",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["surface", "surface_raised"],
  "environment": []
}
*/
// Native, virtualized, selectable read-only source document surface.
// Example: CodeViewer(#{ key: "source", source: rhai_source, label: "Rhai source", language: "rhai" })

define_component(#{
    metadata: #{ id: "components/code_viewer", "export": "CodeViewer", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["surface", "surface_raised"],
        environment: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable viewer identity; keeps scroll, selection and search across renders, also when an inline `source` changes." },
            source: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "document" }] },
                required: true, sensitive: false,
                doc: "Text to show: a string, or a Host-owned `NativeTextDocument` from `ctx.get_native_text_document`." },
            label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Code" },
                doc: "Accessible name of the document surface." },
            file_name: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "File name whose extension picks the syntax when `language` is not set." },
            language: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Syntax to highlight, such as `\"rhai\"`; takes precedence over `file_name`, and unknown names show plain text." },
            show_line_numbers: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Shows 1-based line numbers in the gutter; with `false` the gutter stays, empty." },
            wrap: #{ schema: #{ type: "string", allowed: ["none", "viewport", "column"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "none" },
                doc: "`none` scrolls long lines sideways, `viewport` wraps them at the viewer width, `column` at `wrap_column`." },
            wrap_column: #{ schema: #{ type: "integer", min: 20, max: 500 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 100 },
                doc: "Display column at which lines wrap when `wrap` is `column`." },
            tab_size: #{ schema: #{ type: "integer", min: 1, max: 16 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 4 },
                doc: "Columns between tab stops when tabs are expanded for display." },
            on_location_activate: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `#{ line, column }` when the text is double-clicked or Enter is pressed in it." },
        },
        state: #{ fields: #{} },
        events: #{ location_activate: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
            line: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "One-based source line." },
            column: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "One-based column, counted in characters from the start of the line." },
        } }, doc: "Emitted on a double-click or Enter in the text; the payload is the one-based source location." } },
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
        .background(theme_color("surface")));
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
