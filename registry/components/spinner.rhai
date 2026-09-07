/* gpui-rhai
{
  "id": "components/spinner",
  "export": "Spinner",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Compact indeterminate activity indicator animated by the native runtime clock.
// Props: key, label, size, and speed_ms.
// Events/state: stateless and emits no events; reduced motion settles to a visible frame.
// Parts: root, indicator.
// Example: Spinner(#{ key: "loading", label: "Loading" })

define_component(#{
    metadata: #{ id: "components/spinner", "export": "Spinner", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            size: #{ schema: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "md" } },
            speed_ms: #{ schema: #{ type: "integer", min: 240, max: 4000 }, required: false,
                sensitive: false, "default": #{ type: "integer", value: 850 } },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root", "indicator"],
    },
    render: Fn("render_Spinner")
});

fn Spinner(props) { render_component("components/spinner", props) }
fn spinner_size(size) {
    if size == "xs" { 12 } else if size == "sm" { 14 }
    else if size == "lg" { 22 } else { 18 }
}

fn render_Spinner(ctx, props) {
    let size = spinner_size(if "size" in props { props.size } else { "md" });
    let scale = size / 16.0;
    let width = 1.7 * scale;
    let color = theme_color("accent");
    let scene = canvas_scene([
        canvas_line("a", 8.0 * scale, 1.0 * scale, 8.0 * scale, 4.0 * scale, width, color),
        canvas_line("b", 12.95 * scale, 3.05 * scale, 10.8 * scale, 5.2 * scale, width, color),
        canvas_line("c", 15.0 * scale, 8.0 * scale, 12.0 * scale, 8.0 * scale, width, color),
        canvas_line("d", 12.95 * scale, 12.95 * scale, 10.8 * scale, 10.8 * scale, width, color),
        canvas_line("e", 8.0 * scale, 15.0 * scale, 8.0 * scale, 12.0 * scale, width, color)
    ]);
    canvas(scene).with_key(props.key)
        .with_style(ctx.component_style("root", ctx.component_style("indicator", style()
            .width(px(size)).height(px(size)))))
        .animate(loop_transition("rotate", 0.0, 360.0,
            if "speed_ms" in props { props.speed_ms } else { 850 }, "linear"))
        .accessibility_role("progressbar").accessibility_label(props.label)
        .accessibility_value("indeterminate")
}
