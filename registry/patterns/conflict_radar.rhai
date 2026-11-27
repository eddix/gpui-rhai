/* gpui-rhai
{"id":"patterns/conflict_radar","export":"ConflictRadar","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/tooltip"],"capabilities":{},"assets":[],"tokens":["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","typography.body","typography.caption","typography.initials","radar.tray","radar.card","radar.ink","radar.text","radar.muted","radar.faint","radar.line","radar.fill","radar.hover","radar.stripe","radar.edit","radar.edit_edge","radar.hit","radar.hit_edge","radar.hit_ink","radar.hit_row","radar.kept","radar.kept_edge","radar.kept_ink","radar.accent","radar.chip","radar.inverse","radar.sheen"],"environment":[]}
*/
// A branch activity matrix, exact line overlap, and Host-owned merge proposals.
// Example: ConflictRadar(#{key:"radar",session:"sync",agents:[],files:[]})
import "components/tooltip" as tip;
define_component(#{
    metadata:#{id:"patterns/conflict_radar","export":"ConflictRadar",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/tooltip"],capabilities:#{},assets:[],
        tokens:["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","typography.body","typography.caption","typography.initials","radar.tray","radar.card","radar.ink","radar.text","radar.muted","radar.faint","radar.line","radar.fill","radar.hover","radar.stripe","radar.edit","radar.edit_edge","radar.hit","radar.hit_edge","radar.hit_ink","radar.hit_row","radar.kept","radar.kept_edge","radar.kept_ink","radar.accent","radar.chip","radar.inverse","radar.sheen"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable component key."},
        session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of the collaboration run; changing it resets local conflict choices."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted Host snapshot revision carried by proposals."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Visible and accessible heading."},
        agents:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable unique branch owner ID."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Agent display name."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional model or branch name."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decoration replacing the initials."},
            status:#{schema:#{type:"string",allowed:["running","done"]},required:false,sensitive:false,doc:"Unset counts as running; every agent must finish before merge is offered."}
        }}},required:true,sensitive:false,doc:"Agent columns in display order."},
        files:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable unique file ID."},
            path:#{schema:#{type:"string"},required:true,sensitive:true,doc:"File label; the SDK does not open the path."},
            edits:#{schema:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable unique edit ID within the file."},
                agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Owner ID from agents."},
                start:#{schema:#{type:"integer",min:1,max:100000000},required:true,sensitive:false,doc:"First affected line, inclusive and one-based."},
                end:#{schema:#{type:"integer",min:1,max:100000000},required:true,sensitive:false,doc:"Last affected line, inclusive and not smaller than start."},
                revision:#{schema:#{type:"integer",min:0,max:4294967295},required:true,sensitive:false,doc:"Change when this edit changes, even if its range stays unchanged."},
                summary:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short description of this branch's approach."},
                active:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Whether the branch is currently writing this edit."}
            }}},required:true,sensitive:true,doc:"Edits on this file. At most 256 edits across all files."}
        }}},required:true,sensitive:true,doc:"Accepted file edits, listed in display order."},
        resolved:#{schema:#{type:"optional",value:#{type:"map",values:#{type:"object",allow_unknown:false,fields:#{
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted agent for this exact overlap."},
            witness:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Opaque witness from the accepted resolve proposal."}
        }}}},required:false,sensitive:true,doc:"Optional controlled resolution receipts by overlap ID. Omission allows local choices."},
        pending:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Block proposing another resolution or merge while the Host decides."},
        on_resolve:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives an explicit overlap resolution proposal."},
        on_merge:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives an explicit merge proposal when all guards pass."}
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        resolve:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current collaboration identity."},
            revision:#{schema:#{type:"integer"},required:true,sensitive:false,doc:"Accepted Host snapshot revision."},
            file:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable file ID."},
            overlap:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Opaque identity of the exact overlapping line band."},
            witness:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Canonical input identity for accepting or invalidating the receipt."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Suggested agent to keep for this overlap."}
        }},doc:"Propose which branch to keep in one exact overlapping band."},
        merge:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current collaboration identity."},
            revision:#{schema:#{type:"integer"},required:true,sensitive:false,doc:"Accepted Host snapshot revision; revalidate before doing work."}
        }},doc:"Ask the Host to merge; the component never executes a merge."}
    },slots:#{},parts:["root","bar","title","merge","merge_ready","card","viewport","head","header_file","agent","avatar","row","row_alt","row_open","row_selected",
        "file","path","directory","cell","bar_mark","bar_edit","bar_conflict","bar_kept","bar_mixed","connection","legend","decision","decision_head","decision_title","where","pager","way","kept","left","summary","calm","indicator"],
        effects:["prune"]},render:Fn("render_radar")
});
fn ConflictRadar(p){render_component("patterns/conflict_radar",p)}
fn get(m,k,d){if m!=()&&k in m&&m[k]!=(){m[k]}else{d}}
fn tr(c,k,v){c.t_or("conflict_radar."+k,v)}
fn color(name){let token="radar."+name;theme_color(token)}
fn u(n){theme_length("space.unit").times(n)}
fn s(n){theme_length("space.spacing_unit").times(n)}
fn ink(){style().min_width(px(0)).typography("caption").text_color(color("text"))}
fn ellip(){ink().whitespace_nowrap().text_ellipsis()}
fn ring(){style().focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn paint(c,part,n,st){n.with_style(c.component_style(part,st))}
fn press(n,f){n.on_click(f).on_key_value("plain+enter",f,()).on_key_value("plain+space",f,())}
fn control(c,part,label,n,st,f){
    let node=paint(c,part,n.accessibility_role("button").accessibility_label(label).tab_stop(f!=()).disabled(f==()),ring().merge(st));
    if f==(){node}else{press(node,f)}
}
fn tone(t){switch t{"conflict"=>["hit","hit_edge","hit_ink"],"kept"=>["kept","kept_edge","kept_ink"],
    "mixed"=>["kept","edit_edge","kept_ink"],"edit"=>["edit","edit_edge","accent"],_=>["fill","fill","muted"]}}
fn icon(name){
    let d=switch name{
        "mark"=>"M8 1.8a6.2 6.2 0 1 0 6.2 6.2M8 4.9A3.1 3.1 0 1 0 11.1 8M8 8l4.4-4.4M11 10.5h1",
        "check"=>"m3 8 3 3 7-7",
        "previous"=>"m10 4-4 4 4 4",
        _=>"m6 4 4 4-4 4"
    };
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.3' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
        .accessibility_hidden(true).with_style(style().width(u(4)).height(u(4)).flex_shrink(false))
}
fn local(c,session){
    let d=c.get_state("ui");
    if d!=()&&d.session==session{d}else{#{session:session,focus:"",recent:"",receipt:#{}}}
}
fn focus(session,id,c,e){let d=local(c,session);d.focus=id;d.recent="";c.set_state("ui",d);handled()}
fn propose(ticket,band,agent,c,e){
    let d=local(c,ticket.session);d.focus=band.id;d.recent=band.id;
    if !ticket.controlled{d.receipt[band.id]=#{agent:agent,witness:band.witness};}
    c.set_state("ui",d);
    c.emit("resolve",#{session:ticket.session,revision:ticket.revision,file:band.file,overlap:band.id,witness:band.witness,agent:agent});
    handled()
}
fn merge(ticket,c,e){c.emit("merge",#{session:ticket.session,revision:ticket.revision});handled()}
fn advance(c,ticket){
    let d=local(c,ticket.session);if d.recent==ticket.band&&d.focus==ticket.band{d.focus="";d.recent="";c.set_state("ui",d);}
}
fn prune(c,check){
    let d=local(c,check.session);let next=#{};
    for id in keys(d.receipt){if id in check.ids&&d.receipt[id].witness==check.ids[id]{next[id]=d.receipt[id];}}
    if d.receipt!=next{d.receipt=next;c.set_state("ui",d);}
}
fn noop(c,data){}
fn read(c,p,d){
    let thin=[];let people=#{};let names=[];
    for source in p.agents{
        let a=source;
        if a.name==""||utf8_byte_count(a.name)>256{throw "ConflictRadar needs a bounded agent display name";}
        a.status=get(a,"status","running");thin.push(#{id:a.id,status:a.status});people[a.id]=a;names.push(#{id:a.id,name:a.name});
    }
    let files=[];
    for source in p.files{
        let file=source;let edits=[];
        for raw in file.edits{let e=raw;e.active=get(e,"active",false);edits.push(e);}
        file.edits=edits;files.push(file);
    }
    let pmodel=conflict_radar_project(thin,files,get(p,"resolved",d.receipt));
    let indexed=#{};let ordered=[];let first="";let witness=#{};
    for f in pmodel.files{
        for b in f.bands{
            b.file=f.id;b.path=f.path;indexed[b.id]=b;ordered.push(b.id);witness[b.id]=b.witness;
            if first==""&&b.accepted==(){first=b.id;}
        }
    }
    if d.receipt.len>0{effect("prune",#{session:p.session,ids:witness},Fn("prune"),Fn("noop"));}
    let selected=if d.focus in indexed{d.focus}else if first!=""{first}else if ordered.len>0{ordered[ordered.len-1]}else{""};
    if selected!=""&&d.recent==selected&&indexed[selected].accepted!=()&&first!=""&&first!=selected{
        timeout("advance",700,false,Fn("advance"),#{session:p.session,band:selected});
    }
    #{data:pmodel,people:people,names:names,indexed:indexed,ordered:ordered,selected:selected}
}
fn info(c,start,end){
    let left=c.format_number(start);
    if start==end{c.t_or("conflict_radar.line","Line {start}",#{start:left})}
    else{c.t_or("conflict_radar.lines","Lines {start}–{end}",#{start:left,end:c.format_number(end)})}
}
fn file_name(path){let bits=path.split("/");bits[bits.len-1]}
fn avatar(c,a){
    let n=get(a,"icon",());
    let child=if n!=(){n}else{text(text_initial(a.name)).with_style(style().typography("initials").font_weight(500))};
    paint(c,"avatar",row([child]).accessibility_hidden(true),
        style().width(u(4.5)).height(u(4.5)).justify_center().items_center().radius(theme_radius("sm")).clip().flex_shrink(false)
            .background(color("fill")).text_color(color("text")).typography("initials"))
}
fn title_bar(c,p,m,ticket){
    let d=m.data;let title=get(p,"title",tr(c,"title","Parallel work"));
    let label=if d.open==1{tr(c,"one_overlap","1 overlap to settle")}
        else if d.open>1{c.t_or("conflict_radar.to_settle","{n} overlaps to settle",#{n:c.format_number(d.open)})}
        else if d.running==1{tr(c,"one_running","Waiting for 1 agent")}
        else if d.running>1{c.t_or("conflict_radar.waiting","Waiting for {n} agents",#{n:c.format_number(d.running)})}
        else if d.branches==1{tr(c,"one_branch","Merge 1 branch")}
        else if d.branches>1{c.t_or("conflict_radar.branches","Merge {n} branches",#{n:c.format_number(d.branches)})}
        else{tr(c,"no_merge","Nothing to merge")};
    let ready=d.ready&&!p.pending&&get(p,"on_merge",())!=();
    let action=if ready{Fn("merge").curry(ticket)}else{()};
    let button=control(c,if ready{["merge","merge_ready"]}else{["merge"]},label,row([text(label).with_style(ellip()
        .text_color(color(if ready{"inverse"}else{"muted"})).font_weight(600))]),style().min_height(u(6.5)).padding_x(s(2.5))
        .items_center().flex_shrink(false).radius(theme_radius("sm")).background(color(if ready{"chip"}else{"card"})),action)
        .with_key("merge").paint_transition(240,"ease_out");
    let mark=row([icon("mark")]).with_style(style().text_color(color("accent")));
    let heading=paint(c,"title",text(title).accessibility_role("heading").accessibility_level(2),
        ellip().typography("body").text_color(color("ink")).font_weight(600).flex_grow());
    paint(c,"bar",row([mark,heading,button]).with_key("header"),style().min_width(px(0)).items_center()
        .min_height(u(11)).padding_x(s(3)).gap(s(2.5)))
}
fn head(c,m,l){
    let cells=[paint(c,"header_file",row([text(tr(c,"files","Files")).with_style(ellip().font_weight(500))]).accessibility_role("columnheader"),
        style().items_center().width(px(l.file)).flex_shrink(false).padding_x(s(3)))];
    for i in 0..m.names.len{
        let a=m.people[m.names[i].id];let counts=m.data.stats[i];
        let label=c.t_or("conflict_radar.agent_summary","{name}: {files} changed, {open} open, {kept} kept",#{name:a.name,
            files:c.format_number(counts.files),open:c.format_number(counts.conflicts),kept:c.format_number(counts.kept)});
        cells.push(paint(c,"agent",row([avatar(c,a),text(a.name).with_style(ellip())]).with_key(a.id).accessibility_role("columnheader")
            .accessibility_label(a.name).accessibility_description(label+" · "+get(a,"model","")),
            style().width(px(l.cell)).min_width(px(0)).flex_shrink(false).padding_x(s(1.25)).gap(s(1.25)).items_center()
                .justify_center().border_start(px(1)).border_color(color("line"))));
    }
    paint(c,"head",row(cells).with_key("columns").accessibility_role("row"),style().height(u(10)).width(px(l.width)))
}
fn flat_fill(ink){#{stops:[#{offset:0.0,color:ink},#{offset:1.0,color:ink}]}}
fn bar_mark(c,state){
    let colors=tone(state.role);
    let width=switch state.role{"none"=>4.0,"lost"=>12.0,_=>32.0};
    let height=if state.role=="none"{4.0}else{8.0};
    let parts=["bar_mark"];let edge=state.role!="none"&&state.role!="lost";
    if edge{parts.push("bar_"+state.role);}
    let layers=[flat_fill(color(colors[0])),flat_fill(color("card"))];
    let writing=state.active&&edge;
    if writing{
        let clear=rgba(0x00000000);
        // A 14px image on a 32px bar has 18px of positioning space.
        // The authored travel is -14px to 40px over 1300ms.
        layers.insert(0,#{angle:90.0,size:[0.4375,1.0],position:[-7.0/9.0,0.0],
            stops:[#{offset:0.0,color:clear},#{offset:0.5,color:color("sheen")},#{offset:1.0,color:clear}]});
    }
    let shape=style().width(px(width)).height(px(height)).radius(px(if state.role=="none"{2}else{4}))
        .layered_gradient(layered_gradient(layers));
    if edge{shape=shape.border(px(0.75)).border_color(color(colors[1]));}
    let touched=state.role!="none";
    if touched{shape=shape.paint_reveal_x(4,1);}
    // Keep a touched mark stable across status changes. An untouched
    // placeholder has a different visual identity so the first edit enters.
    let n=paint(c,parts,box([]).with_key(if touched{"bar"}else{"untouched-bar"}).accessibility_hidden(true),shape);
    if touched{n=n.enter_motion(motion_transition("paint_reveal_x",0.0,1.0,
        #{duration_ms:520,easing:ease(),intent:"decorative"}));}
    if writing{n.motion(motion_transition("background_position_x",-7.0/9.0,20.0/9.0,
        #{duration_ms:1300,easing:"linear",iterations:"infinite",intent:"decorative"}))}
    else{n}
}
fn connector(c,row,l){
    if row.link.len<2{return ();}
    let start=l.file+(row.link[0].to_float()+0.5)*l.cell;
    let stop=l.file+(row.link[row.link.len-1].to_float()+0.5)*l.cell;
    let stroke=canvas_stroke_path("connection",[path_move(start,20.0),path_line(stop,20.0)],1.25,color(if row.link_open{"hit_edge"}else{"kept_edge"}));
    let node=paint(c,"connection",canvas(canvas_scene([stroke]).view_box(0,0,l.width,40.0)).with_key("connect").accessibility_hidden(true),
        style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)));
    node.motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:350,easing:ease(),intent:"decorative"})).motion_replay_key(row.link_id)
}
fn matrix_cell(c,f,l,i){
    let state=f.cells[i];let a=f.names[i];
    let role=switch state.role{"none"=>tr(c,"unchanged","Unchanged"),"edit"=>tr(c,"changed","Changed"),"conflict"=>tr(c,"conflict","Conflict"),
        "kept"=>tr(c,"kept","Kept"),"lost"=>tr(c,"left","Left out"),_=>tr(c,"mixed","Mixed")};
    let description="";for range in state.lines{if description!=""{description+=" · ";}description+=info(c,range[0],range[1]);}
    let label=a.name+": "+role;
    let cell=paint(c,"cell",row([bar_mark(c,state)]).with_key("cell").accessibility_role("gridcell").accessibility_label(label)
        .accessibility_description(description),style().width(px(l.cell)).height(u(10)).flex_shrink(false).items_center().justify_center()
            .border_start(px(1)).border_color(color("line")));
    if state.role=="none"{return cell.with_key(a.id);}
    let hint=label+" · "+description;
    let id=l.scope+":"+utf8_byte_count(f.id).to_string()+":"+f.id+":"+a.id;
    tip::Tooltip(#{key:id,label:hint,trigger:cell,content:text(hint).with_style(style().line_clamp(3)),placement:"top",show_delay_ms:360})
        .with_key(a.id).with_style(style().width(px(l.cell)).height(u(10)).flex_shrink(false))
}
fn matrix_row(l,c,item){
    let f=item.item;
    if f.id==""{return paint(c,"calm",row([text(f.path).with_style(ellip())]).with_key(item.key),style().height(u(10)).items_center().padding(s(3)));}
    let name=file_name(f.path);
    let flag=paint(c,"indicator",box([]).accessibility_hidden(true),style().width(u(1.5)).height(u(1.5)).radius(u(0.75))
        .background(color(if f.open>0{"hit_edge"}else if f.choice!=""{"kept_edge"}else{"fill"})).flex_shrink(false));
    let trail=f.path.sub_string(0,f.path.len-name.len);
    let name_ink=color(if f.choice==""{"text"}else{"ink"});
    let path=row([
        paint(c,"directory",text(trail).with_key("directory"),ellip().text_color(color("faint")).flex_shrink_weight(1000)),
        // Reserve the whole name before the directory. A fraction of a pixel
        // lost to proportional shrink can make GPUI ellipsize a fitting name.
        // The max still lets it truncate when the name itself is too long.
        paint(c,"path",text(name).with_key("name"),ellip().text_color(name_ink)
            .max_width(relative(1.0)).flex_shrink(false)),
    ]).with_style(style().min_width(px(0)).flex_grow().clip());
    let bits=[paint(c,"file",row([flag,path]).accessibility_role("rowheader").accessibility_label(f.path),style()
        .width(px(l.file)).min_width(px(0)).flex_shrink(false).padding_x(s(3)).items_center().gap(s(1.5)))];
    for i in 0..f.cells.len{
        bits.push(matrix_cell(c,f,l,i));
    }
    let line=connector(c,f,l);if line!=(){bits.insert(0,line);}
    let parts=["row"];if f.alt{parts.push("row_alt");}if f.open>0{parts.push("row_open");}if f.selected{parts.push("row_selected");}
    let fill=if f.selected&&f.open>0{"hit_row"}else if f.selected{"hover"}else if f.alt{"stripe"}else{"card"};
    let row_style=style().relative().height(u(10)).width(px(l.width)).background(color(fill)).border_top(px(1)).border_color(color("line"));
    let row=row(bits).with_key(item.key);
    if f.choice==""{return paint(c,parts,row.accessibility_role("row"),row_style);}
    let action=Fn("focus").curry(f.session,f.choice);
    control(c,parts,f.path,row,row_style.hover(style().background(color(if f.open>0{"hit_row"}else{"hover"}))),action)
        .accessibility_pressed(f.selected).accessibility_description(if f.open>0{tr(c,"unsettled","Unsettled overlaps")}else{tr(c,"settled","Settled")})
}
fn row_items(c,p,m){
    let data=[];let i=0;
    for file in m.data.files{
        let chosen="";let first="";
        for band in file.bands{if first==""&&band.accepted==(){first=band.id;}if band.id==m.selected{chosen=band.id;}}
        let selected=chosen!="";
        if chosen==""{chosen=if first!=""{first}else if file.bands.len>0{file.bands[0].id}else{""};}
        let line=[];let open=false;let segment=if chosen==""{()}else{m.indexed[chosen]};
        let cells=file.cells;
        if segment!=(){
            open=segment.accepted==();let set=#{};for o in segment.options{set[o.agent]=true;}
            for col in 0..m.names.len{
                let id=m.names[col].id;
                if !(id in set){continue;}
                line.push(col);
                if selected&&!open{
                    let cell=cells[col];
                    cell.role=if id==segment.accepted{"kept"}else{"lost"};cell.active=false;cells[col]=cell;
                }
            }
        }
        data.push(#{key:"file:"+file.id,id:file.id,path:file.path,session:p.session,open:file.open,choice:chosen,cells:cells,names:m.names,
            link:line,link_open:open,link_id:chosen,selected:selected,alt:i%2==1});
        i+=1;
    }
    if data.len==0{data.push(#{key:"empty",id:"",path:tr(c,"no_files","No changes yet")});}
    data
}
fn matrix(c,p,m,l,ref){
    let data=row_items(c,p,m);
    let table=virtual_collection(#{key:"files:"+p.session,label:tr(c,"file_matrix","Files by agent"),data:data,
        estimated_height:40,item_height:u(10),max_height:u(76)},Fn("matrix_row").curry(l)).with_key("rows");
    let content=column([head(c,m,l),table]).with_style(style().width(px(l.width)).min_width(px(l.width)).flex_shrink(false));
    let horizontal=paint(c,"viewport",column([content]).with_key("viewport").with_ref(ref).scrollbars("auto","hidden").accessibility_role("grid")
        .accessibility_label(tr(c,"file_matrix","Files by agent")),style().min_width(px(0)).width(relative(1.0)).overflow_x_scroll());
    paint(c,"card",column([horizontal]).with_key("card"),style().min_width(px(0)).radius(theme_radius("lg")).background(color("card")))
}
fn legends(c){
    let items=[];
    for state in [["edit",tr(c,"changed","Changed")],["conflict",tr(c,"conflict","Conflict")],["kept",tr(c,"kept","Kept")],["lost",tr(c,"left","Left out")]]{
        let dot=box([]).accessibility_hidden(true).with_style(style().width(u(1.5)).height(u(1.5)).radius(u(0.75))
            .background(color(tone(state[0])[1])));
        items.push(row([dot,text(state[1]).with_style(ellip())]).with_style(style().items_center().gap(s(1))));
    }
    paint(c,"legend",row(items).with_key("legend").accessibility_role("group").accessibility_label(tr(c,"legend","Edit legend")),
        style().min_width(px(0)).flex_wrap().gap(s(2.5)).padding_x(s(3)).padding_y(s(1.5)))
}
fn pager(c,p,m,b,l){
    if m.ordered.len<2{return ();}
    let i=m.ordered.index_of(b.id);
    let prev=if c.text_direction()=="rtl"{"next"}else{"previous"};
    let next=if c.text_direction()=="rtl"{"previous"}else{"next"};
    let center=style().width(u(6)).height(u(6)).justify_center().items_center().radius(theme_radius("sm")).text_color(color("text"))
        .hover(style().background(color("fill")));
    let back=control(c,"pager",tr(c,"previous_overlap","Previous overlap"),row([icon(prev)]),center,
        if i>0{Fn("focus").curry(p.session,m.ordered[i-1])}else{()}).with_key("previous");
    let forward=control(c,"pager",tr(c,"next_overlap","Next overlap"),row([icon(next)]),center,
        if i+1<m.ordered.len{Fn("focus").curry(p.session,m.ordered[i+1])}else{()}).with_key("next");
    let index=c.format_number(i+1)+"/"+c.format_number(m.ordered.len);
    row([back,text(index).with_style(ellip().font_feature("tnum",1)),forward]).with_key("pager")
        .with_style(style().items_center().gap(s(0.5)).flex_shrink(false))
}
fn way(c,p,m,ticket,b,o){
    let a=m.people[o.agent];let selected=b.accepted==a.id;let other=b.accepted!=()&&!selected;
    let bits=[avatar(c,a),text(a.name).with_style(ellip().flex_grow().text_color(color("ink")).font_weight(500))];
    if selected{
        bits.push(paint(c,"kept",row([icon("check"),text(tr(c,"kept","Kept"))]),ellip().text_color(color("kept_ink"))
            .background(color("kept")).radius(theme_radius("sm")).padding_x(s(1)).gap(s(0.75)).items_center()));
    }
    let summaries=[];
    for summary in o.summaries{summaries.push(summary);}
    if summaries.len==0{summaries.push(info(c,b.start,b.end));}
    let lines=[];
    for summary in summaries{lines.push(paint(c,"summary",text(summary),ink().line_clamp(2).text_color(color("text"))));}
    let nodes=[row(bits).with_key("heading").with_style(style().min_width(px(0)).items_center().gap(s(1.5))),
        column(lines).with_key("summary").with_style(style().min_width(px(0)).gap(s(0.75)))];
    let action=Fn("propose").curry(ticket,#{id:b.id,file:b.file,witness:b.witness},a.id);
    let allowed=!p.pending&&(!ticket.controlled||get(p,"on_resolve",())!=());
    let style=ring().min_width(px(0)).min_height(u(17)).flex_grow().gap(s(1.25)).padding(s(2.5))
        .background(color(if selected{"kept"}else{"card"})).radius(theme_radius("sm")).hover(style().background(color(if selected{"kept"}else{"hover"})))
        .opacity(if other{0.6}else{1.0});
    let parts=if selected{["way","kept"]}else if other{["way","left"]}else{["way"]};
    let n=paint(c,parts,column(nodes).with_key(a.id).accessibility_role("radio").accessibility_label(a.name)
        .accessibility_checked(selected).tab_stop(allowed).disabled(!allowed),style).paint_transition(200,"ease_out");
    if allowed{press(n,action)}else{n}
}
fn decision(c,p,m,ticket,l){
    if m.selected==""{
        let label=if m.data.running>0{tr(c,"watching","Watching for overlaps")}
            else if m.data.branches>0{tr(c,"clean","No overlapping lines")}else{tr(c,"no_files","No changes yet")};
        return paint(c,"decision",row([text(label).with_style(ink())]).with_key("decision").accessibility_role("status"),
            style().min_height(u(19)).items_center().padding(s(3)).radius(theme_radius("lg")).background(color("card")));
    }
    let b=m.indexed[m.selected];
    let title=if b.accepted==(){tr(c,"pick","Pick a way")}else{tr(c,"settled","Settled")};
    let location=file_name(b.path)+" · "+info(c,b.start,b.end);
    let atoms=[paint(c,"decision_title",text(title).accessibility_role("heading").accessibility_level(3),ellip().font_weight(500).text_color(color("ink"))),
        paint(c,"where",text(location).accessibility_role("status").accessibility_label(location),ellip().flex_grow())];
    let paging=pager(c,p,m,b,l);if paging!=(){atoms.push(paging);}
    let header=paint(c,"decision_head",row(atoms).with_key("decision-head"),
        style().min_width(px(0)).items_center().gap(s(2)).flex_wrap());
    let alternatives=[];
    for agent in m.names{
        for choice in b.options{if choice.agent==agent.id{alternatives.push(way(c,p,m,ticket,b,choice));break;}}
    }
    let body=box(alternatives).with_key("choices:"+b.id).accessibility_role("radiogroup").accessibility_label(tr(c,"choose","Resolution choices"))
        .with_style(style().grid().grid_cols(if l.compact{1}else{min(3,alternatives.len)}).gap(s(2)).min_width(px(0)))
        .enter_motion(motion_transition("opacity",0.0,1.0,#{duration_ms:240,easing:ease(),intent:"decorative"}));
    paint(c,"decision",column([header,body]).with_key("decision"),style().min_width(px(0)).padding(s(3)).gap(s(2.5))
        .radius(theme_radius("lg")).background(color("card")))
}
fn render_radar(c,p){
    if p.session==""||utf8_byte_count(p.session)>96{throw "ConflictRadar requires a bounded session identity";}
    let state=local(c,p.session);
    let m=read(c,p,state);
    let ticket=#{session:p.session,revision:p.revision,controlled:"resolved" in p};
    let ref=element_ref("matrix");let size=c.element_size(ref);
    let available=if size==(){560.0}else{max(110.0,size.width)};
    let compact=available<480.0;let file=if m.names.len==0{available}else if compact{136.0}else{216.0};
    let cell=max(if compact{69.0}else{83.0},(available-file)/max(1,m.names.len).to_float());
    let layout=#{width:file+cell*m.names.len.to_float(),file:file,cell:cell,compact:compact,scope:ref.semantic_id};
    let nodes=[title_bar(c,p,m,ticket),matrix(c,p,m,layout,ref),legends(c),decision(c,p,m,ticket,layout)];
    paint(c,"root",column(nodes).with_key(p.key).accessibility_role("region")
        .accessibility_label(get(p,"title",tr(c,"title","Parallel work"))),style().width(relative(1.0)).min_width(px(0)).max_width(px(640))
        .padding(s(1)).gap(s(1)).radius(theme_radius("lg")).background(color("tray")))
}
