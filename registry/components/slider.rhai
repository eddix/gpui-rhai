/* gpui-rhai
{
  "id": "components/slider",
  "export": "Slider",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_hover", "accent", "surface", "focus_ring", "disabled", "text_primary",
    "text_muted", "metrics.mark", "metrics.control", "spacing.xs", "spacing.sm",
    "typography.body"
  ],
  "environment": ["size", "density"]
}
*/
// Controlled single-value range input with native drag preview and commit events.
// Props: key, value, min, max, step, orientation, label, show_value, disabled, and on_change.
// Events: change(number), emitted on keyboard steps and pointer commit.
// State: caller owns value; the native primitive retains only hot drag preview.
// Parts: root, header, label, value, control, track, fill, thumb.
// Example: Slider(#{ key: "volume", value: 40, min: 0, max: 100, step: 5, label: "Volume", on_change: Fn("changed") })

define_component(#{
    metadata: #{ id: "components/slider", "export": "Slider", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_hover", "accent", "surface", "focus_ring", "disabled", "text_primary",
            "text_muted", "metrics.mark", "metrics.control", "spacing.xs", "spacing.sm",
            "typography.body"
        ],
        environment: ["size", "density"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            value: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            min: #{ schema: #{ type: "number" }, required: false, sensitive: false,
                "default": #{ type: "float", value: 0.0 } },
            max: #{ schema: #{ type: "number" }, required: false, sensitive: false,
                "default": #{ type: "float", value: 100.0 } },
            step: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: false, sensitive: false,
                "default": #{ type: "float", value: 1.0 } },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "horizontal" } },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            value_label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            show_value: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ change: #{ payload: #{ type: "number" } } }, slots: #{},
        parts: ["root", "header", "label", "value", "control", "track", "fill", "thumb"],
    },
    render: Fn("render_Slider")
});

fn Slider(props) { render_component("components/slider", props) }
fn change_action(ctx, value) { ctx.emit("change", value); }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }

fn slider_cap(orientation) {
    let long = theme_length("metrics.mark") * 1.25;
    let short = theme_length("metrics.mark") * 0.75;
    style()
        .width(if orientation == "horizontal" { short } else { long })
        .height(if orientation == "horizontal" { long } else { short })
        .border(px(2)).border_color(theme_color("surface"))
        .background(theme_color("accent"))
        .focus(style().border_color(theme_color("focus_ring")))
        .disabled(style().background(theme_color("disabled")))
}

fn render_Slider(ctx, props) {
    let min = prop_or(props, "min", 0.0);
    let max = prop_or(props, "max", 100.0);
    let step = prop_or(props, "step", 1.0);
    if max <= min { throw "slider max must be greater than min"; }
    if step <= 0.0 || step > max - min { throw "slider step must be positive and no larger than max - min"; }
    if props.value < min || props.value > max { throw "slider value must be between min and max"; }
    let orientation = prop_or(props, "orientation", "horizontal");
    let disabled = prop_or(props, "disabled", false);
    let control = gpui_rhai::RangeInputPrimitive(#{
        key: props.key, value: props.value, min: min, max: max, step: step,
        orientation: orientation, disabled: disabled,
        // A 4px track, an accent fill and a fader cap: cobalt with a 2px surface
        // frame that turns ink while focused.
        track_style: ctx.component_style("track", style()
            .width(if orientation == "horizontal" { relative(1.0) } else { px(4) })
            .height(if orientation == "horizontal" { px(4) } else { relative(1.0) })
            .radius(px(0)).background(theme_color("surface_hover"))),
        fill_style: ctx.component_style("fill", style()
            .radius(px(0)).background(theme_color("accent"))),
        thumb_style: ctx.component_style("thumb", slider_cap(orientation)),
        on_change: if "on_change" in props && props.on_change != () { Fn("change_action") } else { () }
    }).with_style(ctx.component_style("control", style()
        .width(if orientation == "horizontal" { relative(1.0) } else { theme_length("metrics.control") })
        .height(if orientation == "horizontal" { theme_length("metrics.control") } else { px(180) })))
        .disabled(disabled).accessibility_role("slider").accessibility_label(props.label)
        .accessibility_value(props.value).accessibility_value_min(min).accessibility_value_max(max)
        .accessibility_orientation(orientation);
    let value_label = if "value_label" in props && props.value_label != () {
        props.value_label
    } else { `${props.value}` };
    let header = [text(props.label).with_style(ctx.component_style("label", style()
        .typography("body").text_color(theme_color("text_primary"))))];
    if prop_or(props, "show_value", true) {
        header.push(text(value_label).with_style(ctx.component_style("value", style()
            .typography("body").font_feature("tnum", 1).text_color(theme_color("text_muted")))));
    }
    // The label and value span exactly the track, so the value sits over the track's end.
    // A horizontal slider is 240 wide; give the root part a width to change it.
    let root = style().gap(theme_spacing("xs"));
    root = if orientation == "horizontal" { root.width(px(240)) } else { root.items_start() };
    column([
        row(header).with_style(ctx.component_style("header", style()
            .items_center().justify_between().gap(theme_spacing("sm")))),
        control
    ]).with_key(props.key).with_style(ctx.component_style("root", root))
}
