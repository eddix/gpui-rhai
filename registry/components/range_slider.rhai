/* gpui-rhai
{
  "id": "components/range_slider",
  "export": "RangeSlider",
  "version": "0.1.8",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled two-thumb range selection with native pointer preview and keyboard thumbs.
// Props: key, values{low,high}, min, max, step, minimum_gap, orientation,
// label/low_label/high_label, show_value, disabled, on_change.
// Events: change({low,high}). State: caller owns accepted values.
// Parts: root, header, label, value, control, track, fill, thumb.
// Example: RangeSlider(#{ key:"price", label:"Price", values:#{low:20,high:80}, on_change:Fn("changed") })

define_component(#{
    metadata: #{ id:"components/range_slider", "export":"RangeSlider", version:"0.1.8",
        runtime_api:#{min_inclusive: 3,max_exclusive: 4}, dependencies:[], capabilities:#{} },
    schema: #{ props: #{
        key: #{schema:#{type:"string"},required:true,sensitive:false},
        label: #{schema:#{type:"string"},required:true,sensitive:false},
        low_label: #{schema:#{type:"string"},required:false,sensitive:false,
            "default":#{type:"string",value:"Minimum"}},
        high_label: #{schema:#{type:"string"},required:false,sensitive:false,
            "default":#{type:"string",value:"Maximum"}},
        values: #{schema:#{type:"object",allow_unknown:false,fields:#{
            low:#{schema:#{type:"number"},required:true,sensitive:false},
            high:#{schema:#{type:"number"},required:true,sensitive:false},
        }},required:true,sensitive:false},
        min: #{schema:#{type:"number"},required:false,sensitive:false,"default":#{type:"float",value:0.0}},
        max: #{schema:#{type:"number"},required:false,sensitive:false,"default":#{type:"float",value:100.0}},
        step: #{schema:#{type:"number",exclusive_min:0.0},required:false,sensitive:false,"default":#{type:"float",value:1.0}},
        minimum_gap: #{schema:#{type:"number",min:0.0},required:false,sensitive:false,"default":#{type:"float",value:0.0}},
        orientation: #{schema:#{type:"string",allowed:["horizontal","vertical"]},required:false,
            sensitive:false,"default":#{type:"string",value:"horizontal"}},
        show_value: #{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:true}},
        disabled: #{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false}},
        on_change: #{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false},
    }, state:#{fields:#{}}, events:#{change:#{payload:#{type:"object",allow_unknown:false,fields:#{
        low:#{schema:#{type:"number"},required:true,sensitive:false},
        high:#{schema:#{type:"number"},required:true,sensitive:false},
    }}}}, slots:#{}, parts:["root","header","label","value","control","track","fill","thumb"] },
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
        thumb_style:ctx.component_style("thumb",style().width(px(14)).height(px(14)).radius(px(7))
            .border(px(2)).border_color(theme_color("accent")).background(theme_color("surface"))),
        on_change:if "on_change" in props && props.on_change != (){Fn("range_slider_changed")}else{()}
    }).with_key(`${props.key}-range`)
        .with_style(ctx.component_style("control",style()
            .width(if horizontal{px(240)}else{px(24)})
            .height(if horizontal{px(24)}else{px(180)})))
        .disabled(props.disabled).accessibility_role("group").accessibility_label(props.label)
        .accessibility_value(`${props.values.low} – ${props.values.high}`)
        .accessibility_orientation(props.orientation);
    let header = [text(props.label).with_style(ctx.component_style("label",style()
        .typography("body").font_weight(700)))];
    if props.show_value {
        header.push(text(`${props.values.low} – ${props.values.high}`).with_style(
            ctx.component_style("value",style().typography("body_small").text_color(theme_color("text_muted")))));
    }
    column([
        row(header).with_style(ctx.component_style("header",style().items_center().justify_between().gap(theme_spacing("sm")))),
        control
    ]).with_key(props.key).with_style(ctx.component_style("root",style().gap(theme_spacing("xs"))))
}
