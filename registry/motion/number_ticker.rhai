/* gpui-rhai
{
  "id": "motion/number_ticker",
  "export": "NumberTicker",
  "version": "0.1.5",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Props: key and integer value. State: stateless. Emits no events.
// Example: NumberTicker(#{ key: "count", value: 42 })

define_component(#{
    metadata: #{ id: "motion/number_ticker", "export": "NumberTicker", version: "0.1.5",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            value: #{ schema: #{ type: "integer" }, required: true, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"],
    },
    render: Fn("render_NumberTicker")
});

fn NumberTicker(props) { render_component("motion/number_ticker", props) }
fn render_NumberTicker(ctx, props) {
    text(`${props.value}`)
        .with_key(props.key)
        .motion_replay_key(`${props.value}`)
        .with_style(ctx.component_style("root", theme_typography("display")))
        .enter_motion(motion_transition("opacity", 0.0, 1.0, #{
            duration_ms: ctx.motion_duration("fast"), easing: ctx.motion_easing("entrance"),
            intent: "feedback",
        }))
        .exit_motion(motion_transition("opacity", 1.0, 0.0, #{
            duration_ms: ctx.motion_duration("fast"), easing: ctx.motion_easing("exit"),
            intent: "feedback",
        }))
}
