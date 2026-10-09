/* gpui-rhai
{
  "id": "motion/shimmer",
  "export": "Shimmer",
  "version": "0.1.6",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Shimmer is a loading placeholder that a beam sweeps across in an endless loop.
// State: stateless. Emits no events.
// Example: Shimmer(#{ key: "loading", width: 220, height: 72 })

define_component(#{
    metadata: #{ id: "motion/shimmer", "export": "Shimmer", version: "0.1.6",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Motion identity of the shimmer; keep it stable so the sweep keeps its progress across renders." },
            width: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "Width of the placeholder in logical pixels; the beam is a third as wide." },
            height: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "Height of the placeholder and the beam in logical pixels." },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root", "beam"],
    },
    render: Fn("render_Shimmer")
});

fn Shimmer(props) { render_component("motion/shimmer", props) }
fn render_Shimmer(ctx, props) {
    row([
        row([]).with_key("beam").with_style(ctx.component_style("beam", style()
            .width(px(props.width / 3)).height(px(props.height))
            .background(theme_color("surface_hover"))))
            .motion(motion_transition("translate_x", -1.0 * props.width, props.width * 1.0, #{
                duration_ms: ctx.motion_duration("ambient"), easing: "linear",
                iterations: "infinite", intent: "decorative",
            }))
    ]).with_key(props.key).with_style(ctx.component_style("root", style()
        .width(px(props.width)).height(px(props.height)).clip()
        .background(theme_color("surface_raised"))))
}
