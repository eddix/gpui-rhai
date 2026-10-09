import "components/list" as list;
fn state_schema(){#{fields:#{
    selected:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},"default":#{type:"array",value:[#{type:"string",value:"t2"}]}},
    opened:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
}}}
fn set_selected(ctx,value){ctx.set_state("selected",value);}
fn set_opened(ctx,value){ctx.set_state("opened",value);}
fn view(ctx){let items=[
    #{key:"t1",title:"Deploy api-gateway",secondary:"#4812 · ada",meta:"09:41",badge:#{text:"Running",variant:"accent"}},
    #{key:"t2",title:"Roll back worker",secondary:"#4809 · grace",meta:"09:12",badge:#{text:"Failed",variant:"danger"}},
    #{key:"t3",title:"Rotate certificates",secondary:"#4801 · linus",meta:"Yesterday",badge:#{text:"Done",variant:"success"}},
    #{key:"t4",title:"Archived ticket",secondary:"#4700",meta:"Mon",disabled:true},
];column([text("KEYED LIST").with_style(style().typography("heading")),
    list::List(#{key:"tickets",label:"Tickets",items:items,selection_mode:"single",
        selected_keys:ctx.get_state("selected"),height:200.0,
        on_selection_change:Fn("set_selected"),on_row_click:Fn("set_opened")}),
    text(`Opened: ${ctx.get_state("opened")}`)
]).with_style(style().padding(theme_spacing("md")).gap(theme_spacing("sm")).width(px(480)))}
