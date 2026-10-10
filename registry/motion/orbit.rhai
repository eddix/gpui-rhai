/* gpui-rhai
{
  "id": "motion/orbit",
  "export": "Orbit",
  "version": "0.1.6",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Orbit spins a satellite dot around a center dot in an endless loop.
// State: stateless. Emits no events.
// Example: Orbit(#{ key: "orbit", size: 96 })

define_component(#{
    metadata: #{ id: "motion/orbit", "export": "Orbit", version: "0.1.6",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Motion identity of the orbit; keep it stable so the loop keeps its progress across renders." },
            size: #{ schema: #{ type: "integer", min: 8 }, required: true, sensitive: false,
                doc: "Side of the square canvas in logical pixels; the dots scale with it." },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"],
    },
    render: Fn("render_Orbit")
});

fn Orbit(props) { render_component("motion/orbit", props) }
fn render_Orbit(ctx, props) {
    let center = props.size / 2.0;
    let scene = canvas_scene([
        canvas_circle("center", center, center, props.size / 8.0, theme_color("accent")),
        canvas_circle("satellite", center, props.size / 8.0, props.size / 16.0, theme_color("warning")),
    ]);
    canvas(scene).with_key(props.key)
        .with_style(ctx.component_style("root", style().width(px(props.size)).height(px(props.size))))
        .motion(motion_transition("rotate", 0.0, 360.0, #{
            duration_ms: ctx.motion_duration("ambient") * 2, easing: "linear",
            iterations: "infinite", intent: "decorative",
        }))
}
