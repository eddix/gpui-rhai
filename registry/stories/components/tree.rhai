import "components/tree" as tree;
fn state_schema(){#{fields:#{
    expanded:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},"default":#{type:"array",value:[#{type:"string",value:"src"}]}},
    selected:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},"default":#{type:"array",value:[]}},
    active:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"string",value:"src"}},
}}}
fn set_expanded(ctx,value){ctx.set_state("expanded",value);}fn set_selected(ctx,value){ctx.set_state("selected",value);}
fn set_active(ctx,value){ctx.set_state("active",value);}
fn view(ctx){let items=[
    #{key:"src",parent:(),label:"src"},#{key:"lib",parent:"src",label:"lib.rs"},
    #{key:"main",parent:"src",label:"main.rs"},#{key:"docs",parent:(),label:"docs"},
    #{key:"guide",parent:"docs",label:"guide.md"},#{key:"target",parent:(),label:"target",disabled:true}
];column([text("VIRTUAL OUTLINE TREE").with_style(style().typography("heading")),
    tree::Tree(#{key:"files",label:"Project files",items:items,expanded:ctx.get_state("expanded"),
        selected_keys:ctx.get_state("selected"),active_key:ctx.get_state("active"),selection_mode:"multiple",height:260.0,
        on_expanded_change:Fn("set_expanded"),on_selection_change:Fn("set_selected"),on_active_change:Fn("set_active")})
]).with_style(style().padding(theme_spacing("md")).gap(theme_spacing("sm")))}
