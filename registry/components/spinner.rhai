/* gpui-rhai
{
  "id": "components/spinner",
  "export": "Spinner",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["metrics.spinner","spinner.speed_ms","spinner.stroke_width","spinner.track_alpha"],
  "environment": ["size", "density"]
}
*/
// Compact indeterminate activity indicator animated by the native runtime clock.
//
// Example: Spinner(#{ key: "loading", label: "Loading" })
//
// With reduced motion the spinner stops turning and settles on a visible frame.

define_component(#{
    metadata: #{ id: "components/spinner", "export": "Spinner", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["metrics.spinner","spinner.speed_ms","spinner.stroke_width","spinner.track_alpha"],
        environment: ["size", "density"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable identity of the spinner; keeps its rotation continuous across renders." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name saying what is in progress." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } }, required: false,
                sensitive: false,
                doc: "Inherits the size environment unless set. The preset's `metrics.spinner` sets the diameter: xs 12, sm/md 14 and lg 18, scaled with reading size." },
            speed_ms: #{ schema: #{ type: "optional", value: #{ type: "integer", min: 240, max: 4000 } },
                required: false, sensitive: false,
                doc: "Duration of one full turn in milliseconds; unset, `spinner.speed_ms` (Modern 750, Clean 800). Reduced/None motion holds a visible frame." },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root", "indicator"],
    },
    render: Fn("render_Spinner")
});

fn Spinner(props) { render_component("components/spinner", props) }

// Shared filled outlines for standalone and button activity. Opposite ring
// winding leaves the center empty; integrated round caps avoid overlapping
// translucent stroke/circle primitives. Native view-box projection owns scale.
fn ring_path(center, radius, width) {
    let outer = radius + width / 2.0;
    let inner = radius - width / 2.0;
    let a = outer * 0.5522847498307936;
    let b = inner * 0.5522847498307936;
    [path_move(center,center-outer),
     path_cubic(center+outer,center,center+a,center-outer,center+outer,center-a),
     path_cubic(center,center+outer,center+outer,center+a,center+a,center+outer),
     path_cubic(center-outer,center,center-a,center+outer,center-outer,center+a),
     path_cubic(center,center-outer,center-outer,center-a,center-a,center-outer),path_close(),
     path_move(center,center-inner),
     path_cubic(center-inner,center,center-b,center-inner,center-inner,center-b),
     path_cubic(center,center+inner,center-inner,center+b,center-b,center+inner),
     path_cubic(center+inner,center,center+b,center+inner,center+inner,center+b),
     path_cubic(center,center-inner,center+inner,center-b,center+b,center-inner),path_close()]
}

fn arc_path(center, radius, width) {
    let outer = radius + width / 2.0;
    let inner = radius - width / 2.0;
    let half = width / 2.0;
    let factor = 0.5522847498307936;
    let a = outer * factor;
    let b = inner * factor;
    let cap = half * factor;
    [path_move(center,center-outer),
     path_cubic(center+outer,center,center+a,center-outer,center+outer,center-a),
     path_cubic(center+radius,center+half,center+outer,center+cap,center+radius+cap,center+half),
     path_cubic(center+inner,center,center+radius-cap,center+half,center+inner,center+cap),
     path_cubic(center,center-inner,center+inner,center-b,center+b,center-inner),
     path_cubic(center-half,center-radius,center-cap,center-inner,center-half,center-radius+cap),
     path_cubic(center,center-outer,center-half,center-radius-cap,center-cap,center-outer),path_close()]
}

fn ring_scene(view_size, radius, width, track_alpha) {
    if width <= 0.0 || width >= radius * 2.0 {
        throw "Activity stroke width must be positive and smaller than the diameter";
    }
    let center = view_size / 2.0;
    canvas_scene([
        canvas_fill_path("track", ring_path(center,radius,width), current_color().alpha(track_alpha)),
        canvas_fill_path("indicator", arc_path(center,radius,width), current_color())
    ]).view_box(0,0,view_size,view_size)
}

fn setting(ctx, name, fallback) {
    // A genuinely headless render has no theme; a configured theme must supply
    // its declared tokens, rather than silently substituting another preset.
    if ctx.theme_variant() == () { fallback } else { ctx.theme_token(`spinner.${name}`) }
}

fn render_Spinner(ctx, props) {
    let scene = ring_scene(16.0,6.0,setting(ctx,"stroke_width",2.0),setting(ctx,"track_alpha",0.2));
    let node = canvas(scene).with_key(props.key)
        .with_style(ctx.component_style(["root","indicator"], style()
            .width(theme_length("metrics.spinner")).height(theme_length("metrics.spinner"))
            .flex_shrink(false)))
        .motion(motion_transition("rotate", 0.0, 360.0,
            #{ duration_ms: if "speed_ms" in props && props.speed_ms != () { props.speed_ms }
                else { setting(ctx,"speed_ms",750.0).to_int() },
                easing: "linear", iterations: "infinite", intent: "essential" }))
        .accessibility_role("progressbar").accessibility_label(props.label)
        .accessibility_value("indeterminate");
    if "size" in props && props.size != () { node.env(#{size:props.size}) } else { node }
}
