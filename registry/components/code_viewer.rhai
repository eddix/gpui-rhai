/* gpui-rhai
{
  "id": "components/code_viewer",
  "export": "CodeViewer",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["surface", "surface_raised", "scrollbar.thumb", "scrollbar.thumb_hover"],
  "environment": []
}
*/
// Native, virtualized, selectable read-only source document surface.
// Example: CodeViewer(#{ key: "source", source: rhai_source, label: "Rhai source", language: "rhai" })

define_component(#{
    metadata: #{ id: "components/code_viewer", "export": "CodeViewer", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["surface", "surface_raised", "scrollbar.thumb", "scrollbar.thumb_hover"],
        environment: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable viewer identity; keeps scroll, selection and search across renders, also when an inline `source` changes." },
            source: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "document" }] },
                required: true, sensitive: false,
                doc: "Text to show: a string, or a Host-owned `NativeTextDocument` from `ctx.get_native_text_document`." },
            label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Accessible name of the document surface. Empty, the localized `code.label` message, `Code` without one." },
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
            scrollbar: #{ schema: #{ type: "string", allowed: ["auto", "always", "hidden"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "auto" }, doc: "Shared themed scrollbar visibility; the viewport keeps keyboard focus." },
            height: #{ schema: #{ type: "string", allowed: ["fill", "content"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "fill" },
                doc: "`fill` fills the viewer's box; `content` makes it as tall as its rows at the current width, wrapped rows included, up to `max_height`." },
            max_height: #{ schema: #{ type: "optional", value: #{ type: "one_of", variants: [
                #{ type: "integer", min: 0 }, #{ type: "length" }] } }, required: false, sensitive: false,
                doc: "Most a `content` viewer grows: logical pixels, or a Length in pixels, rems or a token; longer documents scroll inside. Ignored with `fill`." },
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
        slots: #{}, parts: ["root", "gutter", "line", "text", "loading", "error", "search",
            "scrollbar_track", "scrollbar_thumb", "scrollbar_thumb_hover"],
    },
    render: Fn("render_CodeViewer")
});

// The text prop `name`, or the localized `key` message (`fallback` without one) while it
// is empty or unset.
fn localized_prop(ctx, props, name, key, fallback) {
    let value = if name in props && props[name] != () { props[name] } else { "" };
    if value == "" { ctx.t_or(key, fallback) } else { value }
}

fn CodeViewer(props) { render_component("components/code_viewer", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn location_activate(ctx, payload) { ctx.emit("location_activate", payload); }

fn render_CodeViewer(ctx, props) {
    let height = prop_or(props, "height", "fill");
    let max_height = prop_or(props, "max_height", ());
    let root_base = style().width(relative(1.0)).min_width(px(0)).background(theme_color("surface"));
    // A content-height viewer measures its own rows; it is not shrunk below them.
    if height != "content" { root_base = root_base.height(relative(1.0)).min_height(px(0)); }
    let root_style = ctx.component_style("root", root_base);
    gpui_rhai::CodeViewPrimitive(#{
        key: props.key,
        messages: #{
            find: ctx.t_or("document.find", "Find in document"),
            no_matches: ctx.t_or("document.no_matches", "No matches"),
            invalid_regex: ctx.t_or("document.invalid_regex", "Invalid regex: {error}"),
            loading: ctx.t_or("document.loading", "Loading document…"),
            updating: ctx.t_or("document.updating", "Updating document…"),
            highlight_unavailable: ctx.t_or("document.highlight_unavailable", "Syntax highlighting unavailable: {error}"),
        },
        source: props.source,
        label: localized_prop(ctx, props, "label", "code.label", "Code"),
        file_name: prop_or(props, "file_name", ()),
        language: prop_or(props, "language", ()),
        show_line_numbers: prop_or(props, "show_line_numbers", true),
        wrap: prop_or(props, "wrap", "none"),
        wrap_column: prop_or(props, "wrap_column", 100),
        tab_size: prop_or(props, "tab_size", 4),
        scrollbar: prop_or(props, "scrollbar", "auto"),
        scrollbar_track_style: ctx.component_style("scrollbar_track", style().background(rgba(0x00000000))),
        scrollbar_thumb_style: ctx.component_style("scrollbar_thumb", style().background(theme_color("scrollbar.thumb"))),
        scrollbar_thumb_hover_style: ctx.component_style("scrollbar_thumb_hover", style().background(theme_color("scrollbar.thumb_hover"))),
        height: height,
        max_height: if type_of(max_height) == "i64" { px(max_height) } else { max_height },
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
        .accessibility_label(localized_prop(ctx, props, "label", "code.label", "Code"))
}
