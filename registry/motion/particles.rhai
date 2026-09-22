/* gpui-rhai
{
  "id": "motion/particles",
  "export": "Particles",
  "version": "0.1.4",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Bounded deterministic particle field. Geometry is built once; Canvas paints it in one batch.
// Props: key, width, height, and bounded count. State: stateless. Emits no events.
// Example: Particles(#{ key: "stars", width: 640, height: 240, count: 96 })

define_component(#{
    metadata: #{ id: "motion/particles", "export": "Particles", version: "0.1.4",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            width: #{ schema: #{ type: "integer", min: 8 }, required: true, sensitive: false },
            height: #{ schema: #{ type: "integer", min: 8 }, required: true, sensitive: false },
            count: #{ schema: #{ type: "integer", min: 1, max: 512 }, required: true, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"],
    },
    render: Fn("render_Particles")
});

fn Particles(props) { render_component("motion/particles", props) }
fn render_Particles(ctx, props) {
    let limit = if ctx.motion_quality() == "low" { 64 }
        else if ctx.motion_quality() == "medium" { 256 } else { 512 };
    let count = if props.count < limit { props.count } else { limit };
    let commands = [];
    for i in 0..count {
        let x = ((i * 47) % props.width) * 1.0;
        let y = ((i * 83) % props.height) * 1.0;
        let radius = 1.0 + ((i * 13) % 3);
        commands.push(canvas_circle(`particle-${i}`, x, y, radius, theme_color("accent")));
    }
    canvas(canvas_scene(commands)).with_key(props.key).motion_particle_count(count)
        .with_style(ctx.component_style("root", style().width(px(props.width)).height(px(props.height))))
        .motion(motion_transition("rotate", 0.0, 360.0, #{
            duration_ms: ctx.motion_duration("ambient") * 8, easing: "linear",
            iterations: "infinite", intent: "decorative",
        }))
}
