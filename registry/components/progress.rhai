/* gpui-rhai
{
  "id": "components/progress",
  "export": "Progress",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Determinate or indeterminate progress indicator animated entirely in Rust.
// Props: key, value, max, indeterminate, label. Parts: root, track, indicator.
// State/events: stateless; Rust owns animation frames and no events are emitted.
// Example: Progress(#{ key: "upload", value: 60, max: 100, label: "Upload" })

export_component(#{
    metadata: #{
        id: "components/progress", "export": "Progress", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{},
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            value: #{ schema: #{ type: "number" }, required: false, sensitive: false,
                "default": #{ type: "float", value: 0.0 } },
            max: #{ schema: #{ type: "number" }, required: false, sensitive: false,
                "default": #{ type: "float", value: 100.0 } },
            indeterminate: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" } },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root", "track", "indicator"],
    },
});

fn Progress(props) {
    component_render("components/progress", props, Fn("render_Progress"))
}
fn render_Progress(ctx, props) {
    let max = if "max" in props && props.max > 0 { props.max } else { 100.0 };
    let value = if "value" in props { props.value } else { 0.0 };
    let ratio = if value < 0 { 0.0 } else if value > max { 1.0 } else { (value * 1.0) / (max * 1.0) };
    let indeterminate = if "indeterminate" in props { props.indeterminate } else { false };
    let indicator = row([])
        .with_key(`${props.key}-indicator`)
        .with_style(component_style(props, "indicator",
            style().width(px(if indeterminate { 72 } else { ratio * 200 }))
                .height(px(8)).radius(px(4)).background(theme_color("accent"))
        ));
    indicator = if indeterminate {
        indicator.animate(loop_transition("translate_x", -72.0, 200.0, 900, "ease_in_out"))
    } else {
        indicator.animate(transition("width", 0.0, ratio * 200.0, 220, "ease_out"))
    };
    row([indicator])
        .with_key(props.key)
        .with_style(component_style(props, "root", component_style(props, "track",
            style().width(px(200)).height(px(8)).radius(px(4)).clip()
                .background(theme_color("surface_hover"))
        )))
        .accessibility_role("progressbar")
        .accessibility_label(if "label" in props { props.label } else { "" })
        .accessibility_value(if indeterminate { "indeterminate" } else { value })
}
