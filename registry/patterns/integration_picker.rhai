/* gpui-rhai
{"id":"patterns/integration_picker","export":"IntegrationPicker","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/input","components/switch","patterns/list_detail"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","surface_raised","surface_hover","text_primary","text_muted","text.success","radius.lg","radius.sm","selection","typography.body","typography.caption","radius.md","space.spacing_unit","space.unit","typography.control","typography.title","catalog.mark_amber","catalog.mark_app","catalog.mark_blue","catalog.mark_gray","catalog.mark_green","catalog.mark_violet","catalog.tile_amber","catalog.tile_app","catalog.tile_blue","catalog.tile_gray","catalog.tile_green","catalog.tile_violet","typography.catalog_initials","typography.catalog_initials_large"],"environment":["density"]}
*/
// IntegrationPicker browses application integrations and proposes changes.
// State: the Host owns connection state; selection may be controlled.
// Example: IntegrationPicker(#{key:"tools",integrations:[#{id:"notes",name:"Notes",domain:"notes.local",description:"Find shared notes."}]})
import "components/button" as button;
import "components/input" as input;
import "components/switch" as toggle;
import "patterns/list_detail" as catalog;

define_component(#{
    metadata:#{id:"patterns/integration_picker","export":"IntegrationPicker",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/input","components/switch","patterns/list_detail"],capabilities:#{},assets:[],
        tokens:["accent","border","focus_ring","surface_raised","surface_hover","text_primary","text_muted","text.success","radius.lg","radius.sm","selection","typography.body","typography.caption","radius.md","space.spacing_unit","space.unit","typography.control","typography.title","catalog.mark_amber","catalog.mark_app","catalog.mark_blue","catalog.mark_gray","catalog.mark_green","catalog.mark_violet","catalog.tile_amber","catalog.tile_app","catalog.tile_blue","catalog.tile_gray","catalog.tile_green","catalog.tile_violet","typography.catalog_initials","typography.catalog_initials_large"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable integration browser identity."},
        integrations:#{schema:#{type:"array",max_items:128,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique nonempty integration ID."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Display name and primary search term."},
            domain:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Displayed site or namespace. Never opened by this component."},
            category:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Secondary caption and search term."},
            description:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Literal details of the integration."},
            features:#{schema:#{type:"array",max_items:3,items:#{type:"string"}},required:false,sensitive:false,doc:"Optional concise capabilities."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative small logo provided by caller."},
            tone:#{schema:#{type:"optional",value:#{type:"string",allowed:["blue","violet","green","amber","gray"]}},required:false,sensitive:false,
                doc:"Optional logo tile tone."},
            disabled:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Skips the entry in selection and blocks modification."},
        }}},required:true,sensitive:false,doc:"Ordered integration metadata; does not register a service or credential."},
        connected:#{schema:#{type:"array",max_items:128,items:#{type:"string"}},"default":#{type:"array",value:[]},required:false,sensitive:false,
            doc:"Accepted connected IDs. Toggles request a change only."},
        value:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted selection; empty means none. Omit for local selection."},
        default_value:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial local selection."},
        app_name:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Application name displayed alongside the selected integration."},
        app_icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative application mark."},
        on_select:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a selected ID proposal."},
        on_toggle:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives #{id,connected}; actual connection remains Host-owned."},
        on_explore:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the ID the user wants to inspect."},
        on_back:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Back button or logical back arrow in an empty search."},
        on_close:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Escape after the search is already empty."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disable all actions, keeping accepted state visible."},
    },state:#{fields:#{
        query:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        selected:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        grouped:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        navigation:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        select:#{payload:#{type:"string"},doc:"Proposed integration ID."},
        toggle:#{payload:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false},
            connected:#{schema:#{type:"bool"},required:true,sensitive:false},
        }},doc:"Explicit request to change a connection."},
        explore:#{payload:#{type:"string"},doc:"Inspection request, with the accepted selected ID."},
        back:#{payload:#{type:"null"},doc:"User requested navigation to the previous screen."},
        close:#{payload:#{type:"null"},doc:"User requested closing the picker."},
    },slots:#{},parts:["root","tray","title","total","back","card","search","search_input","search_icon","shortcut","list","group","row","selected",
        "row_name","tile","large_tile","tile_mark","connected","detail","art","rings","bridge","detail_title","domain",
        "toggle","description","features","feature","explore","empty","icon"],
        effects:["catalog"]},
    render:Fn("render_IntegrationPicker")
});

fn IntegrationPicker(props){render_component("patterns/integration_picker",props)}
fn opt(map,key){catalog::catalog_opt(map,key)}
fn can(props,key){!props.disabled&&opt(props,"on_"+key)!=()}
fn query_changed(ctx,value){ctx.set_state("query",value);}
fn keep_state(ctx,data){
    if !data.controlled{ctx.set_state("selected",data.selected);}
    ctx.set_state("grouped",data.grouped);
    ctx.set_state("navigation",data.navigation);
}
fn cleanup(ctx,data){}
fn escape(enabled,ctx,event){
    if ctx.get_state("query")!=""{ctx.set_state("query","");return handled();}
    if enabled{ctx.emit("close",());return handled();}
    propagate()
}
fn back_key(enabled,ctx,event){
    if !enabled||ctx.get_state("query")!=""{return propagate();}
    ctx.emit("back",());handled()
}
fn focus_search(ctx,event){ctx.focus("search");handled()}
fn propose(cfg,ctx,id){
    if cfg.enabled&&id in cfg.order&&cfg.selected!=id{
        if !cfg.controlled{ctx.set_state("selected",id);}
        ctx.emit("select",id);
    }
    handled()
}
fn mouse_pick(ctx,id){
    let cfg=ctx.get_state("navigation");
    if cfg!=(){propose(cfg,ctx,id);ctx.focus("search");}handled()
}
fn move(origin,step,to_row,ctx,event){
    let cfg=ctx.get_state("navigation");if cfg==(){return propagate();}
    let id=catalog::catalog_neighbor(cfg.order,origin,step,true);
    if id==""||!cfg.enabled{return propagate();}
    propose(cfg,ctx,id);
    if to_row{ctx.focus(cfg.refs[id]);}
    ctx.scroll_into_view(cfg.refs[id]);handled()
}
fn toggle_item(id,ctx,value){ctx.emit("toggle",#{id:id,connected:value});handled()}
fn search_submit(ctx,event){
    let cfg=ctx.get_state("navigation");
    if cfg==()||cfg.term!=catalog::catalog_query(event){return handled();}
    if !(cfg.selected in cfg.order){
        if cfg.order.len>0{propose(cfg,ctx,cfg.order[0]);ctx.scroll_into_view(cfg.refs[cfg.order[0]]);}
    }else if cfg.toggle_enabled{toggle_item(cfg.selected,ctx,!cfg.connected);}
    handled()
}
fn dispatch(event,value,ctx,payload){ctx.emit(event,value);handled()}
fn index_integrations(ctx,items,term,connected){
    let all=#{};let ids=[];let usable=[];let refs=#{};let on=[];let off=[];let order=[];
    let previous=ctx.get_state("grouped");
    let grouped=if previous!=()&&previous.term==term{previous}else{#{term:term,ids:connected}};
    let total=0;
    for item in items{
        if item.id==""||item.id in all{throw "IntegrationPicker IDs must be unique and nonempty";}
        all[item.id]=item;
        refs[item.id]="integration:"+ids.len.to_string();
        ids.push(item.id);
        if opt(item,"disabled")!=true{usable.push(item.id);}
        if item.id in connected{total+=1;}
        if catalog::catalog_match(term,[item.name,item.domain,opt(item,"category")]){
            if item.id in grouped.ids{on.push(item.id);}else{off.push(item.id);}
        }
    }
    for group in [on,off]{for id in group{if opt(all[id],"disabled")!=true{order.push(id);}}}
    #{all:all,ids:ids,usable:usable,refs:refs,on:on,off:off,order:order,total:total,grouped:grouped}
}
fn glyph(ctx,id){
    let path=switch id{
        "back"=>"m14 6-6 6 6 6",
        "forward"=>"m10 6 6 6-6 6",
        "link"=>"M3 9h16m-4-4 4 4-4 4M21 15H5m4-4-4 4 4 4",
        _=>"m5 12 4 4 10-10",
    };
    svg(`<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("icon",style().width(theme_length("space.unit").times(3.75)).height(theme_length("space.unit").times(3.75)).flex_shrink(false)))
}
fn brand_tile(ctx,name,icon,tone,large){
    let paints=switch tone{
        "app"=>[theme_color("catalog.tile_app"),theme_color("catalog.mark_app")],
        "blue"=>[theme_color("catalog.tile_blue"),theme_color("catalog.mark_blue")],
        "violet"=>[theme_color("catalog.tile_violet"),theme_color("catalog.mark_violet")],
        "amber"=>[theme_color("catalog.tile_amber"),theme_color("catalog.mark_amber")],
        "green"=>[theme_color("catalog.tile_green"),theme_color("catalog.mark_green")],
        _=>[theme_color("catalog.tile_gray"),theme_color("catalog.mark_gray")],
    };
    let side=if large{52}else{24};
    let initials=if large{style().typography("catalog_initials_large")}else{style().typography("catalog_initials")};
    let leaf=if icon==(){text(text_initial(name)).with_style(initials)}
        else{icon.with_style(style().width(px(if large{29}else{15})).height(px(if large{29}else{15})))};
    row([leaf.with_style(ctx.component_style("tile_mark",style()))]).accessibility_hidden(true)
        .with_style(ctx.component_style(if large{["tile","large_tile"]}else{["tile"]},style().width(px(side)).height(px(side)).flex_shrink(false)
            .items_center().justify_center().radius(theme_radius("md")).background(paints[0]).text_color(paints[1])))
}
fn integration_row(ctx,props,item,cfg){
    let selected=cfg.selected==item.id;
    let enabled=cfg.enabled&&opt(item,"disabled")!=true;
    let labels=[brand_tile(ctx,item.name,opt(item,"icon"),opt(item,"tone"),false),
        text(item.name).with_style(ctx.component_style("row_name",style().min_width(px(0)).flex_grow()
            .whitespace_nowrap().text_ellipsis()))];
    let linked=item.id in props.connected;
    if linked{labels.push(row([]).accessibility_hidden(true).with_style(ctx.component_style("connected",style().width(px(6)).height(px(6))
        .flex_shrink(false).radius(theme_length("space.unit").times(24.75)).background(theme_color("text.success")))));}
    let paint=ctx.component_style(if selected{["row","selected"]}else{["row"]},style().min_width(px(0)).min_height(theme_length("space.unit").times(9))
        .padding_x(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit").times(1.25)).items_center().radius(theme_radius("sm")).typography("control")
        .background(if selected{theme_color("selection")}else{rgba(0x00000000)})
        .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring")));
    let row=catalog::catalog_option(item.id,item.name,selected,enabled,labels,paint)
        .audit_allow(["mixed-type-in-row"])
        .with_ref(element_ref(cfg.refs[item.id])).disabled(!enabled)
        .accessibility_description(if linked{ctx.t_or("integrations.connected","Connected")}else{ctx.t_or("integrations.available","Available")});
    if !enabled{return row;}
    row.on_click_value(Fn("mouse_pick"),item.id)
        .on_key_value("down",Fn("move").curry(item.id,1,true),()).on_key_value("up",Fn("move").curry(item.id,-1,true),())
        .on_key_value("home",Fn("move").curry(item.id,-256,true),()).on_key_value("end",Fn("move").curry(item.id,256,true),())
}
fn rings(ctx,connected,disabled){
    let layers=[];
    for i in 1..=4{
        let r=16+i*13;let k=r*0.5522847498;let x=120.0;let y=50.0;
        let path=[path_move(x,y-r),path_cubic(x+r,y,x+k,y-r,x+r,y-k),
            path_cubic(x,y+r,x+r,y+k,x+k,y+r),path_cubic(x-r,y,x-k,y+r,x-r,y+k),
            path_cubic(x,y-r,x-r,y-k,x-k,y-r),path_close()];
        layers.push(canvas_stroke_path("ring-"+i.to_string(),path,1.0,
            alpha(theme_color(if connected{"accent"}else{"text_muted"}),0.16)));
    }
    let layer=canvas(canvas_scene(layers).view_box(0,0,240,100)).with_key("rings").accessibility_hidden(true)
        .with_style(ctx.component_style("rings",style().absolute().top(px(0)).left(px(0)).width(relative(1.0)).height(relative(1.0))));
    if disabled{return layer;}
    layer.motion(motion_transition("opacity",1.0,0.45,#{duration_ms:1800,iterations:"infinite",autoreverse:true,intent:"decorative"}))
}
fn integration_detail(ctx,props,item,name){
    let linked=item.id in props.connected;
    let pair=row([rings(ctx,linked,props.disabled),brand_tile(ctx,name,opt(props,"app_icon"),"app",true),glyph(ctx,"link")
        .with_style(ctx.component_style("bridge",style())),brand_tile(ctx,item.name,opt(item,"icon"),opt(item,"tone"),true)])
        .with_key("pair").accessibility_hidden(true).with_style(ctx.component_style("art",style().relative().min_width(px(0))
            .min_height(theme_length("space.unit").times(27)).items_center().justify_center().gap(theme_length("space.spacing_unit").times(2.5))));
    let state_label=if linked{ctx.t_or("integrations.disconnect","Disconnect {name}",#{name:item.name})}
        else{ctx.t_or("integrations.connect","Connect {name}",#{name:item.name})};
    let heading=column([text(name+" + "+item.name).with_key("detail-title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("detail_title",style().typography("title").min_width(px(0))))
        .exit_motion(motion_transition("opacity",1.0,0.0,#{duration_ms:145,intent:"feedback"})),
        text(item.domain).with_style(ctx.component_style("domain",style().typography("caption").text_color(theme_color("text_muted"))))])
        .with_style(style().min_width(px(0)).flex_basis(px(0)).flex_grow().gap(theme_length("space.spacing_unit").times(0.5)));
    let state=toggle::Switch(#{checked:linked,label:state_label,size:"sm",
        disabled:!can(props,"toggle")||opt(item,"disabled")==true,on_change:Fn("toggle_item").curry(item.id),
        part_styles:#{text:style().hidden(),root:style().gap(px(0))}})
        .with_key("connection").with_style(ctx.component_style("toggle",style().flex_shrink(false)));
    let nodes=[pair,row([heading,state]).with_style(style().min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap()),
        text(item.description).with_style(ctx.component_style("description",style().typography("body").min_width(px(0))))];
    let features=opt(item,"features");if features!=()&&features.len>0{
        let list=[];for f in features{list.push(row([glyph(ctx,"check"),text(f).with_style(style().min_width(px(0)))])
            .accessibility_role("listitem").accessibility_label(f).with_style(ctx.component_style("feature",style().items_start()
                .gap(theme_length("space.spacing_unit").times(1.25)).typography("caption").min_width(px(0)))));}
        nodes.push(column(list).accessibility_role("list").with_style(ctx.component_style("features",style().min_width(px(0)).gap(theme_length("space.spacing_unit")))));
    }
    if can(props,"explore"){
        nodes.push(button::Button(#{key:"explore",text:ctx.t_or("integrations.explore","Explore integration"),size:"sm",variant:"secondary",
            suffix:glyph(ctx,if ctx.text_direction()=="rtl"{"back"}else{"forward"}),on_click:Fn("dispatch").curry("explore",item.id)})
            .with_style(ctx.component_style("explore",style())));
    }
    column(nodes).with_key(item.id).with_style(ctx.component_style("detail",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).padding(theme_length("space.spacing_unit").times(3))))
        .motion(motion_transition("opacity",0.5,1.0,#{duration_ms:235,intent:"feedback"}))
}
fn render_IntegrationPicker(ctx,props){
    let query=ctx.get_state("query");
    let term=catalog::catalog_query(query);
    let data=index_integrations(ctx,props.integrations,term,props.connected);
    let external=opt(props,"value");let local=ctx.get_state("selected");
    let seed=if local!=(){local}else{opt(props,"default_value")};
    let selected=if external!=(){external}else if seed!=()&&seed in data.all{seed}
        else if data.usable.len==0{""}else{data.usable[0]};
    let item=if selected in data.all{data.all[selected]}else{()};
    let cfg=#{selected:selected,refs:data.refs,order:data.order,controlled:external!=(),
        enabled:!props.disabled&&(external==()||opt(props,"on_select")!=()),term:term,
        toggle_enabled:can(props,"toggle")&&item!=()&&opt(item,"disabled")!=true,connected:selected in props.connected};
    effect("catalog",#{controlled:external!=(),selected:selected,grouped:data.grouped,navigation:cfg},Fn("keep_state"),Fn("cleanup"));
    let reference=element_ref("picker");let narrow=ctx.element_width_class(reference,[550])==0;
    let title=ctx.t_or("integrations.title","Integrations");
    let name=opt(props,"app_name");if name==(){name=ctx.t_or("integrations.app","Workspace");}
    let header=[];
    if can(props,"back"){
        header.push(button::Button(#{key:"back",text:ctx.t_or("integrations.back","Back"),size:"sm",variant:"ghost",
            prefix:glyph(ctx,if ctx.text_direction()=="rtl"{"forward"}else{"back"}),on_click:Fn("dispatch").curry("back",())})
            .with_style(ctx.component_style("back",style())));
    }
    header.push(text(title).with_style(ctx.component_style("title",style().typography("control").font_weight(600).min_width(px(0)).flex_grow())));
    header.push(text(ctx.t_or("integrations.count","{connected} of {total} connected",#{connected:ctx.format_number(data.total),
        total:ctx.format_number(data.ids.len)})).with_style(ctx.component_style("total",style().typography("caption").text_color(theme_color("text_muted")))));
    let head=row(header).with_style(ctx.component_style("tray",style().items_center().gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap()))
        .audit_allow(["mixed-type-in-row"]);
    let search_name=ctx.t_or("integrations.search","Search integrations");
    let editor=input::Input(#{key:"search-field",label:search_name,placeholder:search_name,value:query,appearance:"embedded",disabled:props.disabled,
        on_change:Fn("query_changed"),on_submit:Fn("search_submit")})
        .with_style(ctx.component_style("search_input",style().min_width(px(0)).flex_grow())).with_ref(element_ref("search"))
        .on_key_value("down",Fn("move").curry(selected,1,false),()).on_key_value("up",Fn("move").curry(selected,-1,false),())
        .on_key_value("escape",Fn("escape").curry(can(props,"close")),())
        .on_key_value(if ctx.text_direction()=="rtl"{"right"}else{"left"},Fn("back_key").curry(can(props,"back")),());
    let search=row([catalog::catalog_search_icon().with_style(ctx.component_style("search_icon",style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4)).flex_shrink(false))),
        editor,text(key_shortcut("cmd-/").label).accessibility_hidden(true).with_style(ctx.component_style("shortcut",
            style().typography("caption").text_color(theme_color("text_muted")).flex_shrink(false)))])
        .with_style(ctx.component_style("search",style().width(relative(1.0)).min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(1.5)).padding_x(theme_length("space.spacing_unit").times(2))));
    let rows=[];let group_names=[ctx.t_or("integrations.connected","Connected"),ctx.t_or("integrations.available","Available")];let lists=[data.on,data.off];
    for i in 0..2{
        if lists[i].len==0{continue;}
        rows.push(text(group_names[i]).with_key("group-"+i.to_string()).accessibility_role("heading").accessibility_level(4)
            .with_style(ctx.component_style("group",style().padding_x(theme_length("space.spacing_unit").times(1.5)).padding_y(theme_length("space.spacing_unit")).typography("caption").text_color(theme_color("text_muted")))));
        for id in lists[i]{rows.push(integration_row(ctx,props,data.all[id],cfg));}
    }
    if rows.len==0{rows.push(text(ctx.t_or("integrations.empty","No integrations match")).with_style(ctx.component_style("empty",style().padding(theme_length("space.spacing_unit").times(2)))));}
    let list=column(rows).with_key("catalog").accessibility_role("listbox").accessibility_label(title)
        .audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("list",style().min_width(px(0)).min_height(px(0)).max_height(px(if narrow{165}else{320})).overflow_y_scroll().padding(theme_length("space.spacing_unit"))));
    let side=if item==(){text(ctx.t_or("integrations.choose","Pick an integration to see its details"))
        .with_style(ctx.component_style("empty",style().padding(theme_length("space.spacing_unit").times(3))))}else{integration_detail(ctx,props,item,name)};
    let main=catalog::ListDetail(#{label:title,list:list,detail:side,orientation:if narrow{"vertical"}else{"horizontal"},
        list_size:px(if narrow{165}else{222})}).with_key("body");
    let card=column([search,main]).with_key("card").with_style(ctx.component_style("card",style().min_width(px(0))
        .gap(theme_length("space.spacing_unit")).background(theme_color("surface_raised")).radius(theme_radius("lg")))).audit_allow(["text-edge-misaligned"]);
    column([head,card]).with_key(props.key).with_ref(reference).with_style(ctx.component_style("root",style().width(relative(1.0))
        .min_width(px(0)).padding(theme_length("space.spacing_unit")).radius(theme_radius("lg")).gap(theme_length("space.spacing_unit")).background(theme_color("surface_hover"))))
        .audit_allow(["text-edge-misaligned"])
        .on_key_value("cmd+/",Fn("focus_search"),()).on_key_value("ctrl+/",Fn("focus_search"),())
}
