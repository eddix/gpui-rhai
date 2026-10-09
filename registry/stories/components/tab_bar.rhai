import "components/tab_bar" as tab_bar;
fn state_schema(){#{fields:#{
    value:#{schema:#{type:"string"},"default":#{type:"string",value:"main"}},
    open:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},"default":#{type:"array",value:[
        #{type:"string",value:"main"},#{type:"string",value:"lib"},#{type:"string",value:"theme"},
        #{type:"string",value:"readme"}]}},
}}}
fn set_value(ctx,value){ctx.set_state("value",value);}
fn closed(ctx,value){let open=[];for key in ctx.get_state("open"){if key!=value{open.push(key);}}
    ctx.set_state("open",open);if ctx.get_state("value")==value&&open.len>0{ctx.set_state("value",open[0]);}}
fn label_for(key){if key=="main"{"main.rs"}else if key=="lib"{"lib.rs"}else if key=="theme"{"theme.rhai"}else{"README.md"}}
fn view(ctx){let tabs=[];for key in ctx.get_state("open"){
    tabs.push(#{value:key,label:label_for(key),closable:true,dirty:key=="lib"});}
column([
    tab_bar::TabBar(#{key:"files",label:"Open files",value:ctx.get_state("value"),tabs:tabs,overflow_menu:true,
        on_change:Fn("set_value"),on_close:Fn("closed")}),
    box([text(`Editing ${label_for(ctx.get_state("value"))}`)]).with_style(style().padding(theme_spacing("md"))
        .height(px(120)).background(theme_color("tabbar.active")))
]).with_style(style().width(px(520)))}
