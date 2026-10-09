/* gpui-rhai
{
  "id": "components/spinner",
  "export": "Spinner",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["accent"],
  "environment": []
}
*/
// Compact indeterminate activity indicator animated by the native runtime clock.
//
// Example: Spinner(#{ key: "loading", label: "Loading" })
//
// With reduced motion the spinner stops turning and settles on a visible frame.

define_component(#{
    metadata: #{ id: "components/spinner", "export": "Spinner", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["accent"],
        environment: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable identity of the spinner; keeps its rotation continuous across renders." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name saying what is in progress." },
            size: #{ schema: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "md" },
                doc: "Diameter: `xs` 12, `sm` 14, `md` 18 and `lg` 22 logical pixels." },
            speed_ms: #{ schema: #{ type: "integer", min: 240, max: 4000 }, required: false,
                sensitive: false, "default": #{ type: "integer", value: 850 },
                doc: "Duration of one full turn in milliseconds." },
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
        .motion(motion_transition("rotate", 0.0, 360.0,
            #{ duration_ms: if "speed_ms" in props { props.speed_ms } else { ctx.motion_duration("ambient") },
                easing: "linear", iterations: "infinite", intent: "essential" }))
        .accessibility_role("progressbar").accessibility_label(props.label)
        .accessibility_value("indeterminate")
}
