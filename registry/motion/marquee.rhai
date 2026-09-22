/* gpui-rhai
{
  "id": "motion/marquee",
  "export": "Marquee",
  "version": "0.1.4",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Props: key, content node, and pixel distance. State: stateless. Emits no events.
// Example: Marquee(#{ key: "news", content: text("News"), distance: 320 })

define_component(#{
    metadata: #{ id: "motion/marquee", "export": "Marquee", version: "0.1.4",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            distance: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ content: #{ required: true, multiple: false } }, parts: ["root", "content"],
    },
    render: Fn("render_Marquee")
});

fn Marquee(props) { render_component("motion/marquee", props) }
fn render_Marquee(ctx, props) {
    row([
        props.content.with_key("content").with_style(ctx.component_style("content", style()))
            .motion(motion_transition("translate_x", 0.0, -1.0 * props.distance, #{
                duration_ms: ctx.motion_duration("ambient") * 6,
                easing: "linear", iterations: "infinite", intent: "decorative",
            }))
    ]).with_key(props.key).with_style(ctx.component_style("root", style().clip()))
}
