/* gpui-rhai
{
  "id": "components/title_bar",
  "export": "TitleBar",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_raised", "border", "text_primary", "text_muted", "metrics.titlebar",
    "metrics.inset", "space.related", "typography.body"
  ],
  "environment": ["density"]
}
*/
// Source-owned application chrome with logical start, center, and end regions.
// With window_drag, the bar replaces the platform title bar: pressing its background
// moves the window and a double press zooms it. The Host decides whether the window
// draws a platform title bar; it never closes or resizes a window.
// Example: TitleBar(#{ label: "Workbench", title: breadcrumb_node, end: [settings_button] })

define_component(#{
    metadata: #{ id: "components/title_bar", "export": "TitleBar", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_raised", "border", "text_primary", "text_muted", "metrics.titlebar",
            "metrics.inset", "space.related", "typography.body"
        ],
        environment: ["density"] },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the bar, which is announced as a toolbar." },
            title: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "node" }] },
                required: true, sensitive: false,
                doc: "Title text, or a node such as a breadcrumb; a string is set semibold and the subtitle truncates before it." },
            subtitle: #{ schema: #{ type: "optional", value: #{ type: "one_of",
                variants: [#{ type: "string" }, #{ type: "node" }] } },
                required: false, sensitive: false,
                doc: "Muted text or node on the title's line; it truncates first." },
            start: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Nodes before the title, at the start edge." },
            center: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Nodes centered in the bar." },
            end: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Nodes at the end edge; they keep their width while the title truncates." },
            inset_start: #{ schema: #{ type: "integer", min: 0, max: 256 }, required: false,
                sensitive: false, "default": #{ type: "integer", value: 0 },
                doc: "Logical pixels kept free at the start edge, room for the macOS window buttons." },
            window_drag: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Whether pressing the bar's background moves the window and a double press zooms it." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ start: #{ required: false, multiple: true, doc: "Nodes before the title, at the start edge." },
            center: #{ required: false, multiple: true, doc: "Nodes centered in the bar." },
            end: #{ required: false, multiple: true, doc: "Nodes at the end edge." } },
        parts: ["root", "start", "title", "subtitle", "center", "end"],
    },
    render: Fn("render_TitleBar")
});

fn TitleBar(props) { render_component("components/title_bar", props) }
fn title_bar_content(ctx, part, value, typography, muted) {
    let structural = style();
    if type_of(value) == "string" {
        let defaults = structural.typography(typography)
            .whitespace_nowrap().text_ellipsis();
        if part == "title" { defaults = defaults.font_weight(600); }
        if muted { defaults = defaults.text_color(theme_color("text_muted")); }
        text(value).with_style(ctx.component_style(part, defaults))
    } else {
        value.with_style(ctx.component_style(part, structural))
    }
}
fn render_TitleBar(ctx, props) {
    // Title and subtitle share one line at one size, told apart by weight and color: a
    // second line does not fit metrics.titlebar, and the subtitle truncates first.
    let title_children = [title_bar_content(ctx, "title", props.title, "body", false)
        .with_style(style().flex_shrink(false))];
    if "subtitle" in props && props.subtitle != () {
        title_children.push(title_bar_content(ctx, "subtitle", props.subtitle, "body", true)
            .with_style(style().min_width(px(0))));
    }
    let start_children = if "start" in props { props.start } else { [] };
    start_children.push(row(title_children).with_style(style()
        .flex_grow().min_width(px(0)).items_center().gap(theme_length("space.related"))
        .overflow_hidden()));
    let inset = if "inset_start" in props { props.inset_start } else { 0 };
    // Start and end share the free width equally, so the center stays centered; the end
    // never shrinks below its controls, and the title truncates first.
    let start = row(start_children).with_style(ctx.component_style("start", style()
        .flex_grow().flex_basis(relative(0)).min_width(px(0)).height(relative(1.0))
        .padding_start(px(inset)).items_center().gap(theme_length("space.related")).overflow_hidden()));
    let center = row(if "center" in props { props.center } else { [] })
        .with_style(ctx.component_style("center", style()
            .flex_shrink(false).height(relative(1.0))
        .items_center().justify_center().gap(theme_length("space.related")).overflow_hidden()));
    let end = row(if "end" in props { props.end } else { [] })
        .with_style(ctx.component_style("end", style()
            .flex_grow().flex_basis(relative(0)).height(relative(1.0))
        .items_center().justify_end().gap(theme_length("space.related"))));
    // Window chrome: a raised block, metrics.titlebar high, content at metrics.inset. It
    // never shrinks: in a column beside content taller than the window it keeps its height.
    let bar = row([start, center, end])
        .with_style(ctx.component_style("root", style()
        .width(relative(1.0)).height(theme_length("metrics.titlebar")).flex_shrink(false)
            .padding_x(theme_length("metrics.inset")).items_center()
            .border_bottom(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface_raised")).text_color(theme_color("text_primary"))))
        .accessibility_role("toolbar").accessibility_label(props.label);
    if "window_drag" in props && props.window_drag { bar.window_drag_area() } else { bar }
}
