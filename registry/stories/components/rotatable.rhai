import "components/rotatable" as rotatable;
fn state_schema(){#{fields:#{angle:#{schema:#{type:"number"},"default":#{type:"float",value:0.0}}}}}
fn changed(ctx,value){ctx.set_state("angle",value);}
fn view(ctx){
    let scene=canvas(canvas_scene([
        canvas_rect("arm",100.0,86.0,82.0,28.0,theme_color("accent")),
        canvas_circle("pivot",100.0,100.0,7.0,theme_color("warning"))
    ])).with_key("rotation-canvas");
    column([
        text("EXPLICIT-PIVOT ROTATION").with_style(style().typography("heading")),
        text(`Accepted angle ${ctx.get_state("angle")}`).accessibility_role("status"),
        rotatable::Rotatable(#{key:"arm",label:"Rotate arm",angle:ctx.get_state("angle"),
            pivot:#{x:100.0,y:100.0},snap:15.0,content:scene,
            handle:text("↻").with_style(style().items_center().justify_center()),on_rotate:Fn("changed")})
            .with_style(style().width(px(300)).height(px(220)))
    ]).with_style(style().padding(theme_spacing("md")).gap(theme_spacing("sm")))
}
