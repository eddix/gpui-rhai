import "components/selection_area" as selection_area;
fn state_schema(){#{fields:#{
    selected:#{schema:#{type:"array",max_items:8,items:#{type:"string"}},"default":#{type:"array",value:[]}},
    active:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"null"}},
    anchor:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"null"}},
}}}
fn changed(ctx,value){ctx.set_state("selected",value.selected_keys);ctx.set_state("active",value.active_key);ctx.set_state("anchor",value.anchor_key);}
fn is_selected(values,key){key in values}
fn view(ctx){
    let targets=[
        #{key:"alpha",x:30.0,y:30.0,width:70.0,height:50.0},
        #{key:"bravo",x:130.0,y:45.0,width:74.0,height:54.0},
        #{key:"charlie",x:68.0,y:130.0,width:82.0,height:58.0},
        #{key:"delta",x:210.0,y:126.0,width:66.0,height:50.0},
    ];
    let commands=[];for target in targets{commands.push(canvas_rect(target.key,target.x,target.y,target.width,target.height,
        if is_selected(ctx.get_state("selected"),target.key){theme_color("accent")}else{theme_color("surface_hover")}));}
    column([
        text("CANVAS SELECTION").with_style(style().typography("heading")),
        text(`${ctx.get_state("selected").len} selected`).accessibility_role("status"),
        selection_area::SelectionArea(#{key:"nodes",label:"Selectable nodes",targets:targets,
            selected_keys:ctx.get_state("selected"),active_key:ctx.get_state("active"),anchor_key:ctx.get_state("anchor"),
            content:canvas(canvas_scene(commands)).with_key("nodes-canvas"),on_selection_change:Fn("changed")})
            .with_style(style().width(px(320)).height(px(220)))
    ]).with_style(style().padding(theme_spacing("md")).gap(theme_spacing("sm")))
}
