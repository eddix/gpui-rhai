import "components/split_pane" as split_pane;
import "components/resizable" as resizable;
import "components/draggable" as draggable;
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
import "components/sortable" as sortable;
import "components/pan_zoom" as pan_zoom;
import "components/rotatable" as rotatable;
import "components/selection_area" as selection_area;
import "components/range_slider" as range_slider;
import "components/tree" as tree;
import "components/scroll_area" as scroll_area;

fn state_schema(){#{fields:#{
    split:#{schema:#{type:"number",min:0.2,max:0.7},"default":#{type:"float",value:0.34}},
    rect:#{schema:#{type:"object",allow_unknown:false,fields:#{x:#{schema:#{type:"number"},required:true,sensitive:false},
        y:#{schema:#{type:"number"},required:true,sensitive:false},width:#{schema:#{type:"number"},required:true,sensitive:false},
        height:#{schema:#{type:"number"},required:true,sensitive:false}}},"default":#{type:"map",value:#{
        x:#{type:"float",value:12.0},y:#{type:"float",value:12.0},width:#{type:"float",value:220.0},height:#{type:"float",value:120.0}}}},
    position:#{schema:#{type:"object",allow_unknown:false,fields:#{x:#{schema:#{type:"number"},required:true,sensitive:false},
        y:#{schema:#{type:"number"},required:true,sensitive:false}}},"default":#{type:"map",value:#{
        x:#{type:"float",value:18.0},y:#{type:"float",value:18.0}}}},
    transform:#{schema:#{type:"object",allow_unknown:false,fields:#{x:#{schema:#{type:"number"},required:true,sensitive:false},
        y:#{schema:#{type:"number"},required:true,sensitive:false},scale:#{schema:#{type:"number",exclusive_min:0.0},required:true,sensitive:false}}},
        "default":#{type:"map",value:#{x:#{type:"float",value:0.0},y:#{type:"float",value:0.0},scale:#{type:"float",value:1.0}}}},
    angle:#{schema:#{type:"number"},"default":#{type:"float",value:0.0}},
    range:#{schema:#{type:"object",allow_unknown:false,fields:#{low:#{schema:#{type:"number"},required:true,sensitive:false},
        high:#{schema:#{type:"number"},required:true,sensitive:false}}},"default":#{type:"map",value:#{
        low:#{type:"float",value:20.0},high:#{type:"float",value:80.0}}}},
    selected:#{schema:#{type:"array",max_items:8,items:#{type:"string"}},"default":#{type:"array",value:[]}},
    selection_active:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"null"}},
    selection_anchor:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"null"}},
    expanded:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},"default":#{type:"array",value:[#{type:"string",value:"workspace"}]}},
    tree_selected:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},"default":#{type:"array",value:[]}},
    tree_active:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"string",value:"workspace"}},
    order:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},"default":#{type:"array",value:[
        #{type:"string",value:"build"},#{type:"string",value:"test"},#{type:"string",value:"ship"},#{type:"string",value:"observe"}]}},
    artifact_lane:#{schema:#{type:"string",allowed:["queue","archive"]},"default":#{type:"string",value:"queue"}},
    status:#{schema:#{type:"string"},"default":#{type:"string",value:"Interaction lab ready"}},
}}}

fn status(label,ctx,value){ctx.set_state("status",label);}
fn set_split(ctx,value){ctx.set_state("split",value);}fn set_rect(ctx,value){ctx.set_state("rect",value);}
fn set_position(ctx,value){ctx.set_state("position",value);}fn set_transform(ctx,value){ctx.set_state("transform",value);}
fn set_angle(ctx,value){ctx.set_state("angle",value);}fn set_range(ctx,value){ctx.set_state("range",value);}
fn set_selection(ctx,value){ctx.set_state("selected",value.selected_keys);ctx.set_state("selection_active",value.active_key);ctx.set_state("selection_anchor",value.anchor_key);}
fn set_expanded(ctx,value){ctx.set_state("expanded",value);}fn set_tree_selected(ctx,value){ctx.set_state("tree_selected",value);}
fn set_tree_active(ctx,value){ctx.set_state("tree_active",value);}
fn index_of(values,key){for index in 0..values.len{if values[index]==key{return index;}}-1}
fn set_order(ctx,value){let order=ctx.get_state("order");let source=index_of(order,value.source_key);if source<0{return;}
    let moved=order.remove(source);let anchor=index_of(order,value.anchor_key);if anchor<0{return;}
    order.insert(if value.placement=="after"{anchor+1}else{anchor},moved);ctx.set_state("order",order);ctx.set_state("status","Pipeline reordered");}
fn dropped(ctx,value){ctx.set_state("artifact_lane",value.target_id);ctx.set_state("status",`Artifact moved to ${value.target_id}`);}

fn navigator(ctx){
    let items=[#{key:"workspace",parent:(),label:"workspace"},#{key:"canvas",parent:"workspace",label:"canvas.rhai"},
        #{key:"pipeline",parent:"workspace",label:"pipeline.rhai"},#{key:"archive",parent:(),label:"archive"}];
    column([
        text("INTERACTION WORKBENCH").with_style(style().typography("heading")),
        tree::Tree(#{key:"lab-tree",label:"Workbench files",items:items,expanded:ctx.get_state("expanded"),
            selected_keys:ctx.get_state("tree_selected"),active_key:ctx.get_state("tree_active"),height:170.0,
            on_expanded_change:Fn("set_expanded"),on_selection_change:Fn("set_tree_selected"),on_active_change:Fn("set_tree_active")}),
        range_slider::RangeSlider(#{key:"lab-range",label:"Active window",values:ctx.get_state("range"),
            min:0.0,max:100.0,step:5.0,minimum_gap:10.0,on_change:Fn("set_range")}),
        text(ctx.get_state("status")).accessibility_role("status")
    ]).with_style(style().height(relative(1.0)).padding(theme_spacing("sm")).gap(theme_spacing("sm")))
}

fn board(ctx){
    let targets=[#{key:"one",x:28.0,y:28.0,width:58.0,height:42.0},#{key:"two",x:118.0,y:72.0,width:64.0,height:48.0},
        #{key:"three",x:210.0,y:34.0,width:60.0,height:44.0}];
    let commands=[];for target in targets{commands.push(canvas_rect(target.key,target.x,target.y,target.width,target.height,
        if target.key in ctx.get_state("selected"){theme_color("accent")}else{theme_color("surface_hover")}));}
    let selection=selection_area::SelectionArea(#{key:"lab-selection",label:"Canvas nodes",targets:targets,
        selected_keys:ctx.get_state("selected"),active_key:ctx.get_state("selection_active"),anchor_key:ctx.get_state("selection_anchor"),
        content:canvas(canvas_scene(commands)).with_key("lab-selection-canvas"),on_selection_change:Fn("set_selection")})
        .with_style(style().width(relative(1.0)).height(px(150)));
    let pan=pan_zoom::PanZoom(#{key:"lab-pan",label:"Pan zoom map",transform:ctx.get_state("transform"),
        min_scale:0.5,max_scale:3.0,pan_button:"middle",content:canvas(canvas_scene([
            canvas_circle("center",140.0,70.0,18.0,theme_color("accent")),canvas_rect("marker",44.0,38.0,40.0,28.0,theme_color("success"))
        ])).with_key("lab-pan-canvas"),on_transform_change:Fn("set_transform")})
        .with_style(style().width(relative(1.0)).height(px(150)));
    let rotate=rotatable::Rotatable(#{key:"lab-rotate",label:"Rotate arm",angle:ctx.get_state("angle"),
        pivot:#{x:90.0,y:60.0},snap:15.0,handle:text("↻"),content:canvas(canvas_scene([
            canvas_rect("arm",90.0,52.0,66.0,16.0,theme_color("warning")),canvas_circle("pivot",90.0,60.0,5.0,theme_color("accent"))
        ])).with_key("lab-rotate-canvas"),on_rotate:Fn("set_angle")}).with_style(style().width(px(200)).height(px(130)));
    column([row([selection,pan]).with_style(style().gap(theme_spacing("sm"))),rotate])
        .with_style(style().padding(theme_spacing("sm")).gap(theme_spacing("sm")))
}

fn pipeline(ctx){
    let data=[];for key in ctx.get_state("order"){data.push(#{key:key,label:key.to_upper()});}
    let card=drag_source::DragSource(#{key:"artifact",label:"Move artifact",source_id:"artifact",payload_type:"artifact",
        payload:#{id:"artifact"},operation:"move",keyboard_target:if ctx.get_state("artifact_lane")=="queue"{"archive"}else{"queue"},
        content:text("ARTIFACT").with_style(style().height(px(38)).padding(theme_spacing("sm")).items_center()
            .border(px(1)).border_color(theme_color("border")).background(theme_color("surface_raised"))) });
    let queue=drop_zone::DropZone(#{key:"queue",label:"Queue",target_id:"queue",payload_types:["artifact"],operations:["move"],
        content:column([text("QUEUE"),if ctx.get_state("artifact_lane")=="queue"{card}else{text("Drop here")}])
            .with_style(style().height(px(84)).padding(theme_spacing("sm")).gap(theme_spacing("xs"))),on_drop:Fn("dropped")});
    let archive=drop_zone::DropZone(#{key:"archive",label:"Archive",target_id:"archive",payload_types:["artifact"],operations:["move"],
        content:column([text("ARCHIVE"),if ctx.get_state("artifact_lane")=="archive"{card}else{text("Drop here")}])
            .with_style(style().height(px(84)).padding(theme_spacing("sm")).gap(theme_spacing("xs"))),on_drop:Fn("dropped")});
    let virtual=sortable::Sortable(#{key:"pipeline-order",label:"Pipeline order",virtual_data:data,direction:"vertical",
        height:180.0,on_reorder:Fn("set_order")});
    let movable=draggable::Draggable(#{key:"lab-drag",label:"Move floating card",position:ctx.get_state("position"),contain:true,
        handle:text("MOVE").with_style(style().height(px(24)).padding_x(theme_spacing("sm")).items_center()),
        content:text("Floating card").with_style(style().width(px(130)).height(px(42)).padding(theme_spacing("sm"))
            .background(theme_color("surface_raised"))),on_move:Fn("set_position")});
    let sized=resizable::Resizable(#{key:"lab-resize",label:"Resize floating region",rect:ctx.get_state("rect"),
        min_width:160.0,min_height:90.0,max_width:360.0,max_height:220.0,content:movable,on_resize:Fn("set_rect")})
        .with_style(style().width(relative(1.0)).height(px(240)));
    column([row([queue,archive]).with_style(style().gap(theme_spacing("sm"))),virtual,sized])
        .with_style(style().padding(theme_spacing("sm")).gap(theme_spacing("sm")))
}

fn view(ctx){
    let main=scroll_area::ScrollArea(#{key:"lab-main",label:"Interaction workbench",height:px(620),axis:"vertical",
        content:column([board(ctx),pipeline(ctx)]).with_style(style().gap(theme_spacing("md")))});
    split_pane::SplitPane(#{key:"lab-split",label:"Resize workbench navigation",size:ctx.get_state("split"),
        min_start:220.0,min_end:520.0,start:navigator(ctx),end:main,on_resize:Fn("set_split")})
        .with_style(style().width(relative(1.0)).height(relative(1.0)))
}
