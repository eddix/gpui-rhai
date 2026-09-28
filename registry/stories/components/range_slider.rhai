import "components/range_slider" as range_slider;

fn state_schema(){#{fields:#{range:#{schema:#{type:"object",allow_unknown:false,fields:#{
    low:#{schema:#{type:"number"},required:true,sensitive:false},
    high:#{schema:#{type:"number"},required:true,sensitive:false},
}},"default":#{type:"map",value:#{low:#{type:"float",value:20.0},high:#{type:"float",value:80.0}}}}
}}}
fn changed(ctx,value){ctx.set_state("range",value);}
fn view(ctx){column([
    text("TWO-THUMB RANGE").with_style(style().typography("heading")),
    range_slider::RangeSlider(#{key:"price",label:"Price interval",low_label:"Minimum price",
        high_label:"Maximum price",values:ctx.get_state("range"),min:0.0,max:100.0,step:5.0,
        minimum_gap:10.0,on_change:Fn("changed")}),
    text(`Accepted ${ctx.get_state("range").low} – ${ctx.get_state("range").high}`)
        .accessibility_role("status")
]).with_style(style().padding(theme_spacing("md")).gap(theme_spacing("md")))}
