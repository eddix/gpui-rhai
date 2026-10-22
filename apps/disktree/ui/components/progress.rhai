/* gpui-rhai
{
  "id": "components/progress",
  "export": "Progress",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["accent", "surface_hover"],
  "environment": []
}
*/
// Determinate or indeterminate progress indicator animated entirely in Rust.
//
// Example: Progress(#{ key: "upload", value: 60, max: 100, label: "Upload" })

define_component(#{
    metadata: #{
        id: "components/progress", "export": "Progress", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["accent", "surface_hover"],
        environment: [],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable identity of the bar; its indicator is keyed `<key>-indicator` to keep its animation." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name saying what is in progress." },
            value: #{ schema: #{ type: "number", min: 0.0 }, required: false, sensitive: false,
                "default": #{ type: "float", value: 0.0 },
                doc: "Amount done, in the units of `max`; a value above `max` shows a full bar." },
            max: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: false, sensitive: false,
                "default": #{ type: "float", value: 100.0 },
                doc: "The value of a complete bar." },
            indeterminate: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Ignores `value` and loops a short indicator across the track, for work of unknown length." },
            width: #{ schema: #{ type: "integer", min: 24, max: 4096 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 200 },
                doc: "Track width in logical pixels; the track is 4px tall." },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root", "track", "indicator"],
    },
    render: Fn("render_Progress")
});

fn Progress(props) { render_component("components/progress", props) }
fn render_Progress(ctx, props) {
    let max = if "max" in props { props.max } else { 100.0 };
    let value = if "value" in props { props.value } else { 0.0 };
    let ratio = if value > max { 1.0 } else { (value * 1.0) / (max * 1.0) };
    let indeterminate = if "indeterminate" in props { props.indeterminate } else { false };
    let width = if "width" in props { props.width * 1.0 } else { 200.0 };
    let indeterminate_width = width * 0.36;
    let indicator = row([])
        .with_key(`${props.key}-indicator`)
        .with_style(ctx.component_style("indicator",
            style().width(px(if indeterminate { indeterminate_width } else { ratio * width }))
                .height(px(4)).radius(px(0)).background(theme_color("accent"))
        ));
    indicator = if indeterminate {
        indicator.motion(motion_transition("translate_x", -indeterminate_width, width,
            #{ duration_ms: ctx.motion_duration("ambient"), easing: ctx.motion_easing("standard"),
                iterations: "infinite", intent: "essential" }))
    } else {
        indicator.motion(motion_transition("width", 0.0, ratio * width,
            #{ duration_ms: ctx.motion_duration("normal"), easing: ctx.motion_easing("entrance"),
                intent: "feedback" }))
    };
    row([indicator])
        .with_key(props.key)
        .with_style(ctx.component_style("root", ctx.component_style("track",
            style().width(px(width)).height(px(4)).radius(px(0)).clip()
                .background(theme_color("surface_hover"))
        )))
        .accessibility_role("progressbar")
        .accessibility_label(props.label)
        .accessibility_value(if indeterminate { "indeterminate" } else { value })
        .accessibility_value_min(0.0)
        .accessibility_value_max(max)
}
