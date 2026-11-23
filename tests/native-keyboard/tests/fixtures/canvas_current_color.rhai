fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn outside(ctx,value){ctx.set_state("count",ctx.get_state("count")+1);}
fn noop(ctx,value){}
fn view(ctx){
    let segment=[path_move(88,8),path_line(104,8)];
    let morph=[path_move(112,8),path_line(128,8)];
    let scene=canvas_scene([
        canvas_rect("rect",0,0,16,16,current_color()),
        canvas_circle("circle",28,8,8,current_color()),
        canvas_line("line",40,8,56,8,8,current_color()),
        canvas_fill_path("fill",[path_move(64,0),path_line(80,0),path_line(80,16),path_line(64,16),path_close()],current_color()),
        canvas_stroke_path("stroke",segment,8,current_color()),
        canvas_morph_stroke_path("morph",morph,morph,8,current_color()),
        canvas_rect("alpha",136,0,16,16,current_color().alpha(0.5)),
        canvas_rect("fixed",160,0,16,16,rgb(0xff0000)),
        canvas_rect("ref-red",0,32,16,16,rgba(0xff000080)),
        canvas_rect("ref-red-half",24,32,16,16,rgba(0xff000040)),
        canvas_rect("ref-blue",48,32,16,16,rgba(0x0000ff60)),
        canvas_rect("ref-blue-half",72,32,16,16,rgba(0x0000ff30)),
        canvas_rect("ref-green",96,32,16,16,rgb(0x00ff00)),
        canvas_rect("ref-green-half",120,32,16,16,rgba(0x00ff0080)),
        canvas_rect("ref-clear",144,32,16,16,rgba(0xabcdef00))
    ]);
    let cells=[];
    for i in 0..40 {cells.push(box([]).with_key(`cell-${i}`).with_style(style().width(px(1)).height(px(1)).background(rgb(0x555555))));}
    let children=[canvas(scene).with_key("canvas")
        .accessibility_role("button").accessibility_label("Canvas").on_click(Fn("noop"))
        .with_style(style().width(px(180)).height(px(80))
            .hover(style().text_color(rgb(0x00ff00)))
            .focus(style().text_color(rgba(0x12345600))))];
    column([
        column([column(children).with_key("cold").with_style(style().width(px(180)).height(px(120)))])
            .with_key("parent").accessibility_role("group").accessibility_label("Parent")
            .with_style(style().width(px(420)).height(px(160)).padding(px(10))
                .text_color(rgba(0xff000080)).hover(style().text_color(rgba(0x0000ff60)))),
        text(`Outside ${ctx.get_state("count")}`).with_key("outside")
            .accessibility_role("button").accessibility_label("Outside").on_click(Fn("outside"))
            .with_style(style().text_color(rgb(0x000000))),
        column(cells).with_key("cacheable-sibling").with_style(style().width(relative(1.0)))
    ]).with_style(style().width(px(480)).height(px(260)).background(rgb(0xffffff)))
}
