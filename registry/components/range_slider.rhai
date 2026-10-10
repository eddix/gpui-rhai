/* gpui-rhai
{
  "id": "components/range_slider",
  "export": "RangeSlider",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/slider"],
  "capabilities": {},
  "tokens": [
    "surface_hover", "accent", "text_primary", "text_muted", "metrics.control", "spacing.xs",
    "spacing.sm", "typography.body"
  ],
  "environment": ["size", "density"]
}
*/
// Controlled two-thumb range selection with native pointer preview and keyboard thumbs.
// State: caller owns accepted values.
// Example: RangeSlider(#{ key:"price", label:"Price", values:#{low:20,high:80}, on_change:Fn("changed") })

import "components/slider" as slider;

define_component(#{
    metadata: #{ id:"components/range_slider", "export":"RangeSlider", version: "0.2.0",
        runtime_api:#{min_inclusive: 3,max_exclusive: 4}, dependencies:["components/slider"], capabilities:#{},
        tokens: [
            "surface_hover", "accent", "text_primary", "text_muted", "metrics.control",
            "spacing.xs", "spacing.sm", "typography.body"
        ],
        environment: ["size", "density"] },
    schema: #{ props: #{
        key: #{schema:#{type:"string"},required:true,sensitive:false,
            doc:"Names the native slider, which holds the drag preview across renders; it also keys the root."},
        label: #{schema:#{type:"string"},required:true,sensitive:false,
            doc:"Visible label at the start of the header row; also the accessible name of the slider."},
        low_label: #{schema:#{type:"string"},required:false,sensitive:false,
            "default":#{type:"string",value:"Minimum"},doc:"Accessible name of the low thumb."},
        high_label: #{schema:#{type:"string"},required:false,sensitive:false,
            "default":#{type:"string",value:"Maximum"},doc:"Accessible name of the high thumb."},
        values: #{schema:#{type:"object",allow_unknown:false,fields:#{
            low:#{schema:#{type:"number"},required:true,sensitive:false,
                doc:"Value of the low thumb; at least `min`."},
            high:#{schema:#{type:"number"},required:true,sensitive:false,
                doc:"Value of the high thumb; at most `max` and at least `low + minimum_gap`."},
        }},required:true,sensitive:false,
            doc:"Current range; the caller stores the `change` payload and passes it back."},
        min: #{schema:#{type:"number"},required:false,sensitive:false,"default":#{type:"float",value:0.0},
            doc:"Lowest value, at the start of the track; must be below `max`."},
        max: #{schema:#{type:"number"},required:false,sensitive:false,"default":#{type:"float",value:100.0},
            doc:"Highest value, at the end of the track; must be above `min`."},
        step: #{schema:#{type:"number",exclusive_min:0.0},required:false,sensitive:false,"default":#{type:"float",value:1.0},
            doc:"Distance between accepted values, counted from `min`; one arrow key moves a thumb one step. At most `max - min`; when `max - min` is not a multiple of it, the last step below `max` is the highest value."},
        minimum_gap: #{schema:#{type:"number",min:0.0},required:false,sensitive:false,"default":#{type:"float",value:0.0},
            doc:"Smallest distance kept between `low` and `high`; a thumb stops there. At most `max - min`."},
        orientation: #{schema:#{type:"string",allowed:["horizontal","vertical"]},required:false,
            sensitive:false,"default":#{type:"string",value:"horizontal"},
            doc:"`horizontal` is 240 logical pixels wide (set a width on `root` to change it); `vertical` has a 180-pixel track."},
        show_value: #{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:true},
            doc:"Shows `low – high` at the end of the header row."},
        disabled: #{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},
            doc:"Blocks pointer and keyboard input and greys the thumbs."},
        on_change: #{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,
            doc:"Called with `#{ low, high }` when an arrow, Home or End key or a drag moves a thumb."},
    }, state:#{fields:#{}}, events:#{change:#{payload:#{type:"object",allow_unknown:false,fields:#{
        low:#{schema:#{type:"number"},required:true,sensitive:false,doc:"New value of the low thumb."},
        high:#{schema:#{type:"number"},required:true,sensitive:false,doc:"New value of the high thumb."},
    }},doc:"Emitted when a key or a finished drag moves a thumb, not during the drag; the payload is the new range, snapped to `step`."}},
    slots:#{}, parts:["root","header","label","value","control","track","fill","thumb"] },
    render:Fn("render_RangeSlider")
});

fn RangeSlider(props) { render_component("components/range_slider", props) }
fn range_slider_changed(ctx, value) { ctx.emit("change", value); }

fn render_RangeSlider(ctx, props) {
    if props.max <= props.min { throw "RangeSlider max must be greater than min"; }
    if props.step <= 0.0 || props.step > props.max - props.min { throw "RangeSlider step is invalid"; }
    if props.minimum_gap < 0.0 || props.minimum_gap > props.max - props.min {
        throw "RangeSlider minimum_gap is invalid";
    }
    if props.values.low < props.min || props.values.high > props.max
        || props.values.low + props.minimum_gap > props.values.high {
        throw "RangeSlider values are outside the accepted ordered range";
    }
    let horizontal = props.orientation == "horizontal";
    let control = gpui_rhai::RangeSliderPrimitive(#{key:props.key,
        low:props.values.low,high:props.values.high,min:props.min,max:props.max,
        step:props.step,minimum_gap:props.minimum_gap,orientation:props.orientation,
        disabled:props.disabled,low_label:props.low_label,high_label:props.high_label,
        track_style:ctx.component_style("track",style()
            .width(if horizontal{relative(1.0)}else{px(4)})
            .height(if horizontal{px(4)}else{relative(1.0)})
            .radius(px(0)).background(theme_color("surface_hover"))),
        fill_style:ctx.component_style("fill",style().radius(px(0)).background(theme_color("accent"))),
        thumb_style:ctx.component_style("thumb",slider::slider_cap(props.orientation)),
        on_change:if "on_change" in props && props.on_change != (){Fn("range_slider_changed")}else{()}
    }).with_key(`${props.key}-range`)
        .with_style(ctx.component_style("control",style()
            .width(if horizontal{relative(1.0)}else{theme_length("metrics.control")})
            .height(if horizontal{theme_length("metrics.control")}else{px(180)})))
        .disabled(props.disabled).accessibility_role("group").accessibility_label(props.label)
        .accessibility_value(`${props.values.low} – ${props.values.high}`)
        .accessibility_orientation(props.orientation);
    let header = [text(props.label).with_style(ctx.component_style("label",style()
        .typography("body").text_color(theme_color("text_primary"))))];
    if props.show_value {
        header.push(text(`${props.values.low} – ${props.values.high}`).with_style(
            ctx.component_style("value",style().typography("body").font_feature("tnum", 1)
                .text_color(theme_color("text_muted")))));
    }
    // Like Slider: the label and value span exactly the 240-wide track.
    let root = style().gap(theme_spacing("xs"));
    root = if horizontal { root.width(px(240)) } else { root.items_start() };
    column([
        row(header).with_style(ctx.component_style("header",style().items_center().justify_between().gap(theme_spacing("sm")))),
        control
    ]).with_key(props.key).with_style(ctx.component_style("root",root))
}
