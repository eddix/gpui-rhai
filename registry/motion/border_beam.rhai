/* gpui-rhai
{
  "id": "motion/border_beam",
  "export": "BorderBeam",
  "version": "0.1.6",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {}
}
*/
// BorderBeam traces an accent stroke around a rectangle in an endless loop.
// State: stateless. Emits no events.
// Example: BorderBeam(#{ key: "beam", width: 220, height: 72 })

define_component(#{
    metadata: #{ id: "motion/border_beam", "export": "BorderBeam", version: "0.1.6",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Motion identity of the beam; keep it stable so the loop keeps its progress across renders." },
            width: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "Width of the traced rectangle in logical pixels." },
            height: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "Height of the traced rectangle in logical pixels." },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"],
    },
    render: Fn("render_BorderBeam")
});

fn BorderBeam(props) { render_component("motion/border_beam", props) }
fn render_BorderBeam(ctx, props) {
    let path = [path_move(1.0, 1.0), path_line(props.width - 1.0, 1.0),
        path_line(props.width - 1.0, props.height - 1.0), path_line(1.0, props.height - 1.0),
        path_close()];
    canvas(canvas_scene([canvas_stroke_path("beam", path, 2.0, theme_color("accent"))]))
        .with_key(props.key)
        .with_style(ctx.component_style("root", style().width(px(props.width)).height(px(props.height))))
        .motion(motion_transition("path_progress", 0.0, 1.0, #{
            duration_ms: ctx.motion_duration("ambient"), easing: "linear",
            iterations: "infinite", intent: "decorative",
        }))
}
