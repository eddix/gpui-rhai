import "components/tabs" as tabs;
fn state_schema(){#{fields:#{
    value:#{schema:#{type:"string"},"default":#{type:"string",value:"main"}},
    open:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},"default":#{type:"array",value:[
        #{type:"string",value:"main"},#{type:"string",value:"lib"},#{type:"string",value:"theme"},
        #{type:"string",value:"readme"}]}},
    added:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
}}}
fn set_value(ctx,value){ctx.set_state("value",value);}
fn closed(ctx,request){let open=[];for key in ctx.get_state("open"){if key!=request.value{open.push(key);}}
    ctx.set_state("open",open);if ctx.get_state("value")==request.value{ctx.set_state("value",request.next);}}
fn added(ctx,payload){let count=ctx.get_state("added")+1;let key=`new-${count}`;let open=ctx.get_state("open");
    open.push(key);ctx.set_state("open",open);ctx.set_state("added",count);ctx.set_state("value",key);}
fn label_for(key){if key=="main"{"main.rs"}else if key=="lib"{"lib.rs"}else if key=="theme"{"theme.rhai"}
    else if key=="readme"{"README.md"}else{`${key}.rs`}}
fn view(ctx){let items=[];for key in ctx.get_state("open"){
    items.push(#{value:key,label:label_for(key),closable:true,dirty:key=="lib",
        count:if key=="theme"{3}else{()},tone:if key=="theme"{"accent"}else{()},tone_label:"changed"});}
column([
    tabs::Tabs(#{key:"files",label:"Open files",value:ctx.get_state("value"),tabs:items,variant:"bar",
        overflow_menu:true,add_label:"New file",panel:false,
        on_change:Fn("set_value"),on_close:Fn("closed"),on_add:Fn("added")}),
    box([text(`Editing ${label_for(ctx.get_state("value"))}`)]).with_style(style().padding(theme_spacing("md"))
        .height(px(120)).background(theme_color("tabbar.active")))
]).with_style(style().width(px(520)))}
