/* gpui-rhai
{
  "id": "layouts/region",
  "export": "Region",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "space.group", "space.related", "metrics.inset", "metrics.control", "metrics.statusbar",
    "border", "text_primary", "text_muted", "typography.title",
    "typography.caption"
  ],
  "environment": ["density"]
}
*/
// Region is one working area: a header with the region's title and actions, an optional
// toolbar, a body that fills the remaining height, and a footer pinned to the bottom.
//
// Props: label (required), body (required), title (string or node), actions, toolbar,
//        footer (status items), fill (default true), inset (default true), bleed,
//        scroll (default false), external_title (default false).
// Events/state: none. Parts: root, header, title, actions, body, footer.
// Example: Region(#{ label: "Hosts", title: "Hosts", toolbar: toolbar_node, body: table_node,
//              footer: [text("128 hosts")] })
//
// Content starts at metrics.inset; the header, toolbar and body are `group` apart, so
// anything nested inside them must use a smaller relationship. `bleed` lets the body
// reach the region's sides, for tables and lists whose rows carry their own inset: their
// text then starts on the same edge as the title.
//
// `scroll` makes the body scroll vertically when it is taller than the region; the header,
// toolbar and footer stay put. Leave it off for bodies that scroll themselves (Table,
// VirtualList). `external_title` says the region's title is drawn elsewhere, in a TitleBar
// or a tab: the region takes no `title`, and its parts are not held to a missing title.

define_component(#{
    metadata: #{
        id: "layouts/region", "export": "Region", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "space.group", "space.related", "metrics.inset", "metrics.control",
            "metrics.statusbar", "border", "text_primary", "text_muted",
            "typography.title", "typography.caption"
        ],
        environment: ["density"],
    },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            title: #{ schema: #{ type: "optional", value: #{ type: "one_of",
                variants: [#{ type: "string" }, #{ type: "node" }] } }, required: false, sensitive: false },
            actions: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            toolbar: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            body: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            footer: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            fill: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            inset: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            bleed: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            scroll: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            external_title: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            actions: #{ required: false, multiple: true }, toolbar: #{ required: false, multiple: false },
            body: #{ required: true, multiple: false }, footer: #{ required: false, multiple: true },
        },
        parts: ["root", "header", "title", "actions", "body", "footer"],
    },
    render: Fn("render_Region")
});

fn Region(props) { render_component("layouts/region", props) }
fn title_style() {
    let title = style().typography("title").text_color(theme_color("text_primary"));
    title.whitespace_nowrap().text_ellipsis().min_width(px(0))
}
fn header_style() {
    let header = style().flex_row().items_center().justify_between();
    header.gap(theme_length("space.group")).min_height(theme_length("metrics.control"))
}
fn footer_style() {
    // Footer fields are groups of related items, so they sit `group` apart.
    let footer = style().flex_row().items_center().gap(theme_length("space.group"));
    footer = footer.min_height(theme_length("metrics.statusbar"));
    footer.border_top(px(1)).border_color(theme_color("border"))
        .typography("caption").text_color(theme_color("text_muted"))
}
fn region_title(ctx, title) {
    if type_of(title) == "string" {
        let node = text(title).with_style(ctx.component_style("title", title_style()));
        node.accessibility_role("heading").accessibility_level(1)
    } else { title.with_style(ctx.component_style("title", style())) }
}

fn render_Region(ctx, props) {
    let fill = if "fill" in props { props.fill } else { true };
    let inset = if "inset" in props { props.inset } else { true };
    let bleed = "bleed" in props && props.bleed;
    let actions = if "actions" in props { props.actions } else { [] };
    let has_title = "title" in props && props.title != ();
    let external_title = "external_title" in props && props.external_title;
    if external_title && has_title { throw "Region takes title or external_title, not both"; }
    // Horizontal inset is applied per part, so a bled body can reach the sides.
    let side = if inset { theme_length("metrics.inset") } else { px(0) };
    let children = [];
    if has_title || actions.len > 0 {
        let header = [];
        if has_title { header.push(region_title(ctx, props.title)); }
        let action_style = style().flex_row().items_center().gap(theme_length("space.related"));
        header.push(row(actions).with_style(ctx.component_style("actions", action_style)));
        children.push(row(header).with_style(ctx.component_style("header", header_style().padding_x(side))));
    }
    if "toolbar" in props && props.toolbar != () {
        children.push(column([props.toolbar]).with_style(style().padding_x(side)));
    }
    let body_style = style().flex_col().flex_grow().min_height(px(0)).min_width(px(0));
    if !bleed { body_style = body_style.padding_x(side); }
    if "scroll" in props && props.scroll { body_style = body_style.overflow_y_scroll(); }
    children.push(column([props.body]).with_style(ctx.component_style("body", body_style)));
    let footer = if "footer" in props { props.footer } else { [] };
    if footer.len > 0 {
        let footer_row = row(footer).with_style(ctx.component_style("footer", footer_style()));
        children.push(column([footer_row]).with_style(style().padding_x(side)));
    }
    let root = style().flex_col().gap(theme_length("space.group")).min_width(px(0));
    if inset { root = root.padding_y(theme_length("metrics.inset")); }
    if fill { root = root.flex_grow().min_height(px(0)); }
    let node = column(children).with_style(ctx.component_style("root", root))
        .accessibility_role("region").accessibility_label(props.label);
    if external_title { node.heading_elsewhere() } else { node }
}
