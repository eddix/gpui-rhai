/* gpui-rhai
{
  "id": "components/markdown_viewer",
  "export": "MarkdownViewer",
  "version": "0.2.2",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["surface", "surface_raised", "border", "spacing.xs", "spacing.sm", "typography.title", "typography.subtitle", "typography.body", "typography.body_small", "typography.caption"],
  "environment": []
}
*/
// Native CommonMark/GFM, not an editor or a WebView. Host documents keep source
// and parse models out of Rhai. Links emit; images require explicit Host assets.
//
// Example: MarkdownViewer(#{key:"article",label:"Article",source:ctx.get_native_text_document("article")})
define_component(#{
    metadata: #{ id: "components/markdown_viewer", "export": "MarkdownViewer", version: "0.2.2",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["surface", "surface_raised", "border", "spacing.xs", "spacing.sm", "typography.title", "typography.subtitle", "typography.body", "typography.body_small", "typography.caption"], environment: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable viewer identity; owns scroll, selection and background-job cancellation." },
            source: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "document" }] }, required: true, sensitive: false,
                doc: "Markdown string or Host-owned NativeTextDocument from ctx.get_native_text_document." },
            label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Markdown" }, doc: "Accessible document name." },
            height: #{ schema: #{ type: "string", allowed: ["fill", "content"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "fill" }, doc: "Fill available space or grow to measured content height." },
            max_height: #{ schema: #{ type: "optional", value: #{ type: "one_of", variants: [#{ type: "integer", min: 0 }, #{ type: "length" }] } }, required: false, sensitive: false,
                doc: "Maximum content height as a logical-pixel integer or absolute-resolvable Length (px/rem); relative lengths are not supported. Overflow scrolls natively." },
            code_block_actions: #{ schema: #{ type: "array", max_items: 1, items: #{ type: "string", allowed: ["copy"] } }, required: false, sensitive: false,
                "default": #{ type: "array", value: [#{ type: "string", value: "copy" }] }, doc: "Code block actions; copy writes code without fences." },
            image_assets: #{ schema: #{ type: "map", values: #{ type: "string" } }, required: false, sensitive: false,
                "default": #{ type: "map", value: #{} }, doc: "Explicit Markdown destination to Host asset ID map. The viewer never fetches URLs; missing assets show alt text." },
            on_link: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Link activation callback with href and visible text. Host validates and decides what to open." },
        },
        state: #{ fields: #{} },
        events: #{ link: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
            href: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Untrusted destination; never opened by the viewer." },
            text: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Visible link text." },
        } }, doc: "User activated a link by pointer or keyboard." } },
        slots: #{}, parts: ["root", "heading", "paragraph", "code_block", "inline_code", "quote", "table", "link", "rule", "loading", "error"],
    },
    render: Fn("render_MarkdownViewer")
});

fn MarkdownViewer(props) { render_component("components/markdown_viewer", props) }
fn prop_or(props, key, fallback) { if key in props { props[key] } else { fallback } }
fn link(ctx, payload) { ctx.emit("link", payload); }

fn render_MarkdownViewer(ctx, props) {
    let height = prop_or(props, "height", "fill");
    let max_height = prop_or(props, "max_height", ());
    let root = style().width(relative(1.0)).min_width(px(0)).background(theme_color("surface"));
    if height != "content" { root = root.height(relative(1.0)).min_height(px(0)); }
    gpui_rhai::MarkdownViewPrimitive(#{
        key: props.key,
        source: props.source, label: prop_or(props, "label", "Markdown"), height: height,
        max_height: if type_of(max_height) == "i64" { px(max_height) } else { max_height },
        code_block_actions: prop_or(props, "code_block_actions", ["copy"]),
        image_assets: prop_or(props, "image_assets", #{}),
        heading_roles: ["title", "subtitle", "subtitle", "body", "body_small", "caption"],
        heading_style: ctx.component_style("heading", style().padding_y(theme_spacing("xs"))),
        paragraph_style: ctx.component_style("paragraph", style().typography("body").padding_y(theme_spacing("xs"))),
        code_block_style: ctx.component_style("code_block", style().background(theme_color("surface_raised")).padding(theme_spacing("sm"))),
        inline_code_style: ctx.component_style("inline_code", style().background(theme_color("surface_raised"))),
        quote_style: ctx.component_style("quote", style().typography("body").padding_y(theme_spacing("xs")).border_left(px(2)).border_color(theme_color("border"))),
        table_style: ctx.component_style("table", style().typography("body_small")),
        link_style: ctx.component_style("link", style()),
        rule_style: ctx.component_style("rule", style()),
        loading_style: ctx.component_style("loading", style().typography("caption")),
        error_style: ctx.component_style("error", style().typography("body")),
        on_link: if "on_link" in props && props.on_link != () { Fn("link") } else { () },
    }).with_key(props.key).with_style(ctx.component_style("root", root))
        .accessibility_role("document").accessibility_label(prop_or(props, "label", "Markdown"))
}
