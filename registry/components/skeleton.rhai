/* gpui-rhai
{
  "id": "components/skeleton",
  "export": "Skeleton",
  "version": "0.1.5",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Content placeholder with optional Rust-side pulse animation.
// Props: key, width, height, radius, animated. Parts: root.
// State/events: stateless; Rust owns pulse frames and no events are emitted.
// Example: Skeleton(#{ key: "avatar", width: 40, height: 40, animated: true })

define_component(#{
    metadata: #{
        id: "components/skeleton", "export": "Skeleton", version: "0.1.5",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: [], capabilities: #{},
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            width: #{ schema: #{ type: "integer", min: 0 }, required: true, sensitive: false },
            height: #{ schema: #{ type: "integer", min: 0 }, required: true, sensitive: false },
            radius: #{ schema: #{ type: "integer", min: 0 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 0 } },
            animated: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"],
    },
    render: Fn("render_Skeleton")
});

fn Skeleton(props) { render_component("components/skeleton", props) }
fn render_Skeleton(ctx, props) {
    let width = props.width;
    let height = props.height;
    let requested_radius = if "radius" in props { props.radius } else { 0 };
    let radius = requested_radius;
    let node = row([])
        .with_key(props.key)
        .with_style(ctx.component_style("root",
            style().width(px(width)).height(px(height))
                .radius(px(radius))
                .background(theme_color("surface_hover"))
        ))
        .accessibility_role("presentation");
    if !("animated" in props) || props.animated {
        node.motion(motion_transition("opacity", 0.42, 0.72,
            #{ duration_ms: ctx.motion_duration("ambient"), easing: ctx.motion_easing("standard"),
                iterations: "infinite",
                autoreverse: true, intent: "decorative" }))
    } else { node }
}
