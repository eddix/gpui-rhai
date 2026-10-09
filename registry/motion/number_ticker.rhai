/* gpui-rhai
{
  "id": "motion/number_ticker",
  "export": "NumberTicker",
  "version": "0.1.6",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {}
}
*/
// NumberTicker shows an integer and fades each new value in.
// State: stateless. Emits no events.
// Example: NumberTicker(#{ key: "count", value: 42 })

define_component(#{
    metadata: #{ id: "motion/number_ticker", "export": "NumberTicker", version: "0.1.6",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Motion identity of the ticker; keep it stable so only a changed `value` replays the fade." },
            value: #{ schema: #{ type: "integer" }, required: true, sensitive: false,
                doc: "Number shown in `display` typography; a new value fades in while the old one fades out." },
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
