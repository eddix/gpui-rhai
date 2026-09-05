/* gpui-rhai
{
  "id": "components/status_bar",
  "export": "StatusBar",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Compact application footer with stable logical start, center, and end regions.
// Props: accessible label and optional start/center/end node arrays.
// Events/state: stateless and emits no events. Parts: root, start, center, end.
// Example: StatusBar(#{ label: "Editor status", start: [text("Ready")], end: [text("UTF-8")] })

define_component(#{
    metadata: #{ id: "components/status_bar", "export": "StatusBar", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            start: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            center: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            end: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ start: #{ required: false, multiple: true },
            center: #{ required: false, multiple: true }, end: #{ required: false, multiple: true } },
        parts: ["root", "start", "center", "end"],
    },
    render: Fn("render_StatusBar")
});

fn StatusBar(props) { render_component("components/status_bar", props) }
fn status_region(props, part, children, alignment) {
    let base = style().width(relative(0.333333)).min_width(px(0))
        .height(relative(1.0)).items_center().gap(px(6)).overflow_hidden()
        .whitespace_nowrap().text_ellipsis();
    if alignment == "center" { base = base.justify_center(); }
    else if alignment == "end" { base = base.justify_end(); }
    row(children).with_style(component_style(props, part, base))
}
fn render_StatusBar(ctx, props) {
    row([
        status_region(props, "start", if "start" in props { props.start } else { [] }, "start"),
        status_region(props, "center", if "center" in props { props.center } else { [] }, "center"),
        status_region(props, "end", if "end" in props { props.end } else { [] }, "end"),
    ]).with_style(component_style(props, "root", style()
        .width(relative(1.0)).height(px(24)).padding_x(px(8)).items_center()
        .border_top(px(1)).border_color(theme_color("border"))
        .background(theme_color("surface_raised"))
        .typography("caption").text_color(theme_color("text_muted"))))
        .accessibility_role("statusbar").accessibility_label(props.label)
}
