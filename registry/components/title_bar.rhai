/* gpui-rhai
{
  "id": "components/title_bar",
  "export": "TitleBar",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Source-owned application chrome with logical start, center, and end regions.
// Native window authority remains Host-owned; this component never moves or closes a window.
// Props: title, optional subtitle, start/center/end nodes, and logical start inset.
// Events/state: stateless and emits no events. Parts: root, start, title, subtitle, center, end.
// Example: TitleBar(#{ title: "Workbench", end: [settings_button] })

define_component(#{
    metadata: #{ id: "components/title_bar", "export": "TitleBar", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            subtitle: #{ schema: #{ type: "optional", value: #{ type: "string" } },
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
fn render_TitleBar(ctx, props) {
    let title_children = [text(props.title).with_style(component_style(props, "title",
        style().typography("body_small").font_weight(700)
            .whitespace_nowrap().text_ellipsis()))];
    if "subtitle" in props && props.subtitle != () {
        title_children.push(text(props.subtitle).with_style(component_style(props, "subtitle",
            style().typography("caption").text_color(theme_color("text_muted"))
                .whitespace_nowrap().text_ellipsis())));
    }
    let start_children = if "start" in props { props.start } else { [] };
    start_children.push(column(title_children).with_style(style().min_width(px(0)).gap(px(0))));
    let inset = if "inset_start" in props { props.inset_start } else { 0 };
    let start = row(start_children).with_style(component_style(props, "start", style()
        .width(relative(0.333333)).min_width(px(0)).height(relative(1.0))
        .padding_start(px(inset)).items_center().gap(px(8)).overflow_hidden()));
    let center = row(if "center" in props { props.center } else { [] })
        .with_style(component_style(props, "center", style()
            .width(relative(0.333333)).min_width(px(0)).height(relative(1.0))
            .items_center().justify_center().gap(px(6)).overflow_hidden()));
    let end = row(if "end" in props { props.end } else { [] })
        .with_style(component_style(props, "end", style()
            .width(relative(0.333333)).min_width(px(0)).height(relative(1.0))
            .items_center().justify_end().gap(px(6)).overflow_hidden()));
    row([start, center, end])
        .with_style(component_style(props, "root", style()
            .width(relative(1.0)).height(px(34)).padding_x(px(10)).items_center()
            .border_bottom(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface_raised")).text_color(theme_color("text_primary"))))
        .accessibility_role("toolbar").accessibility_label(props.title)
}
