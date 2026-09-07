/* gpui-rhai
{
  "id": "components/title_bar",
  "export": "TitleBar",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Source-owned application chrome with logical start, center, and end regions.
// Native window authority remains Host-owned; this component never moves or closes a window.
// Props: accessible label, string-or-node title/subtitle, start/center/end nodes, and logical start inset.
// Events/state: stateless and emits no events. Parts: root, start, title, subtitle, center, end.
// Example: TitleBar(#{ label: "Workbench", title: breadcrumb_node, end: [settings_button] })

define_component(#{
    metadata: #{ id: "components/title_bar", "export": "TitleBar", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            title: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "node" }] },
                required: true, sensitive: false },
            subtitle: #{ schema: #{ type: "optional", value: #{ type: "one_of",
                variants: [#{ type: "string" }, #{ type: "node" }] } },
                required: false, sensitive: false },
            start: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            center: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            end: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            inset_start: #{ schema: #{ type: "integer", min: 0, max: 256 }, required: false,
                sensitive: false, "default": #{ type: "integer", value: 0 } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ start: #{ required: false, multiple: true },
            center: #{ required: false, multiple: true }, end: #{ required: false, multiple: true } },
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
        if part == "title" { defaults = defaults.font_weight(700); }
        if muted { defaults = defaults.text_color(theme_color("text_muted")); }
        text(value).with_style(ctx.component_style(part, defaults))
    } else {
        value.with_style(ctx.component_style(part, structural))
    }
}
fn render_TitleBar(ctx, props) {
    let title_children = [title_bar_content(ctx, "title", props.title, "body_small", false)];
    if "subtitle" in props && props.subtitle != () {
        title_children.push(title_bar_content(ctx, "subtitle", props.subtitle, "caption", true));
    }
    let start_children = if "start" in props { props.start } else { [] };
    start_children.push(column(title_children).with_style(style()
        .flex_grow().min_width(px(0)).gap(px(0)).overflow_hidden()));
    let inset = if "inset_start" in props { props.inset_start } else { 0 };
    let start = row(start_children).with_style(ctx.component_style("start", style()
        .width(relative(0.333333)).min_width(px(0)).height(relative(1.0))
        .padding_start(px(inset)).items_center().gap(px(8)).overflow_hidden()));
    let center = row(if "center" in props { props.center } else { [] })
        .with_style(ctx.component_style("center", style()
            .width(relative(0.333333)).min_width(px(0)).height(relative(1.0))
            .items_center().justify_center().gap(px(6)).overflow_hidden()));
    let end = row(if "end" in props { props.end } else { [] })
        .with_style(ctx.component_style("end", style()
            .width(relative(0.333333)).min_width(px(0)).height(relative(1.0))
            .items_center().justify_end().gap(px(6)).overflow_hidden()));
    row([start, center, end])
        .with_style(ctx.component_style("root", style()
            .width(relative(1.0)).height(px(34)).padding_x(px(10)).items_center()
            .border_bottom(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface_raised")).text_color(theme_color("text_primary"))))
        .accessibility_role("toolbar").accessibility_label(props.label)
}
