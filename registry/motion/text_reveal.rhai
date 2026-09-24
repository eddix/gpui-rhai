/* gpui-rhai
{
  "id": "motion/text_reveal",
  "export": "TextReveal",
  "version": "0.1.5",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Grapheme-safe native text reveal. Rhai builds spans once; Rust samples opacity.
// Props: key and text. State: stateless. Emits no events.
// Example: TextReveal(#{ key: "hero", text: "Hello" })

define_component(#{
    metadata: #{ id: "motion/text_reveal", "export": "TextReveal", version: "0.1.5",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            text: #{ schema: #{ type: "string" }, required: true, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"],
    },
    render: Fn("render_TextReveal")
});

fn TextReveal(props) { render_component("motion/text_reveal", props) }
fn render_TextReveal(ctx, props) {
    text(motion_text_spans(props.text, #{
        duration_ms: ctx.motion_duration("normal"),
        stagger_ms: ctx.motion_stagger("tight"),
        easing: ctx.motion_easing("entrance"),
        intent: "decorative",
    }))
        .with_key(props.key)
        .with_style(ctx.component_style("root", theme_typography("title")))
}
