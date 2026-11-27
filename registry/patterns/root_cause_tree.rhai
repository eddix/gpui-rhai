/* gpui-rhai
{"id":"patterns/root_cause_tree","export":"RootCauseTree","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/code_viewer","components/spinner"],"capabilities":{},"assets":[],"tokens":["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","typography.body","typography.caption","typography.code","root_cause.tray","root_cause.card","root_cause.meta","root_cause.muted","root_cause.text","root_cause.ink","root_cause.shine","root_cause.faint","root_cause.line","root_cause.fill","root_cause.well","root_cause.rail","root_cause.path","root_cause.brand","root_cause.brand_bg","root_cause.danger","root_cause.danger_bg","root_cause.solid","root_cause.on_solid"],"environment":[]}
*/
// Shows a Host-confirmed fault tree, evidence and scoped action proposals.
// Example: RootCauseTree(#{key:"review",session:"review",causes:[#{id:"failure",title:"Review could not finish"}]})
import "components/spinner" as spin;
import "components/code_viewer" as code;

define_component(#{
    metadata:#{id:"patterns/root_cause_tree","export":"RootCauseTree",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/code_viewer","components/spinner"],capabilities:#{},assets:[],tokens:["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","typography.body","typography.caption","typography.code","root_cause.tray","root_cause.card","root_cause.meta","root_cause.muted","root_cause.text","root_cause.ink","root_cause.shine","root_cause.faint","root_cause.line","root_cause.fill","root_cause.well","root_cause.rail","root_cause.path","root_cause.brand","root_cause.brand_bg","root_cause.danger","root_cause.danger_bg","root_cause.solid","root_cause.on_solid"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of the investigation; a new session resets local selection."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted investigation revision included in each action proposal."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional heading for the whole investigation."},
        causes:#{schema:#{type:"array",min_items:1,max_items:96,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable cause ID, unique in this investigation."},
            parent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Parent ID. Exactly one cause without a parent represents the failure."},
            title:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Readable name of the failure or proposed cause."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short supporting caption displayed on the hypothesis card."},
            gate:#{schema:#{type:"string",allowed:["and","or"]},"default":#{type:"string",value:"or"},required:false,sensitive:false,doc:"How this cause combines its immediate children's proofs."},
            status:#{schema:#{type:"string",allowed:["open","checking","ruled-out","confirmed"]},"default":#{type:"string",value:"open"},required:false,sensitive:false,
                doc:"Host's accepted verdict for a hypothesis, not an auto-generated conclusion."},
            verdict:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host's explanation for the accepted result, shown in details."},
            files:#{schema:#{type:"array",max_items:24,items:#{type:"string"}},"default":#{type:"array",value:[]},required:false,sensitive:true,doc:"Related path labels for inspection; the component does not open them."},
            evidence:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
                kind:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Heading for this evidence, for example Build log."},
                source:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional origin caption; never fetched by the widget."},
                content:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Host-supplied read-only evidence text displayed in the inspector."},
                language:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Syntax language for the evidence viewer; defaults to plain text."}
            }}},required:false,sensitive:true,doc:"Optional snapshot supporting this accepted cause, separate from its status."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional visual mark for this cause."},
            at_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Elapsed time of the accepted verdict."}
        }}},required:true,sensitive:true,doc:"Flat ordered tree; sibling order follows input, and parents can occur anywhere."},
        selected_id:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional controlled selection. An explicit null restores default selection."},
        pending:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Do not emit another action while the Host decides the previous proposal."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Fix/retry request for a proven, confirmed leaf. The component never executes it."},
        on_selection_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the requested cause ID or null for automatic selection."}
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer"},required:true,sensitive:false},
            id:#{schema:#{type:"string"},required:true,sensitive:false},
            kind:#{schema:#{type:"string",allowed:["fix","retry"]},required:true,sensitive:false}
        }},doc:"Explicit requested Host action. Raw evidence and file paths never leave the widget in this event."},
        selection_change:#{payload:#{type:"optional",value:#{type:"string"}},doc:"Selected ID, or null to restore the automatic selection."}
    },slots:#{},parts:["root","head","mark","title","summary","live","diagram","wires","card","card_selected","card_excluded","inner","inner_selected","inner_excluded",
       "tile","name","file_row","file","file_name","more","meta","status","badge","gate","guide","panel","panel_head","panel_title","panel_body",
       "verdict","panel_status","source","evidence_label","evidence","actions","button","primary","pending"],effects:["clean_selection"]},
    render:Fn("view_causes")
});
fn RootCauseTree(props){render_component("patterns/root_cause_tree",props)}
fn get(m,k,d){if m!=()&&k in m&&m[k]!=(){m[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn ink(p){let k="root_cause."+p;theme_color(k)}
fn tx(ctx,p,v){ctx.t_or("root_cause."+p,v)}
fn ease(){cubic_bezier(0.32,0.72,0.24,1.0)}
fn apply(ctx,n,k,s){n.with_style(ctx.component_style(k,s))}
fn label(ctx,k,v,s){apply(ctx,text(v),k,s)}
fn line(){style().min_width(px(0)).typography("caption").whitespace_nowrap().text_ellipsis().text_color(ink("muted"))}
fn small(){style().min_width(px(0)).typography("caption").text_color(ink("meta"))}
fn clickable(n,f){n.on_click(f).on_key_value("plain+enter",f,()).on_key_value("plain+space",f,())}
fn focus(){style().focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))}
fn state(ctx,session){
    let v=ctx.get_state("ui");if v!=()&&v.session==session{v}else{#{session:session,picked:"",stop:""}}
}
fn picked(t,id,ctx,e){
    let s=state(ctx,t.session);
    if !t.controlled{s.picked=id;ctx.set_state("ui",s);}
    ctx.emit("selection_change",if id==""{()}else{id});handled()
}
fn focused(t,id,ctx,inside){
    if inside{
        let s=state(ctx,t.session);if s.stop!=id{s.stop=id;ctx.set_state("ui",s);}
    }
}
fn focus_cause(key,ctx,e){ctx.focus(key);ctx.scroll_into_view(key);handled()}
fn propose(t,id,kind,ctx,e){ctx.emit("action",#{session:t.session,revision:t.revision,id:id,kind:kind});handled()}
fn clear(ctx,d){
    let a=state(ctx,d.session);
    if d.lost_pick!=""&&a.picked==d.lost_pick{a.picked="";}
    if d.lost_focus!=""&&a.stop==d.lost_focus{a.stop="";}
    ctx.set_state("ui",a);
}
fn noop(ctx,d){}
fn glyph(k){
    if k=="tree"{
        return svg("<svg viewBox='0 0 16 16' fill='none'><path d='M8 5.2v2.3M4 10.5V8.6h8v1.9' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round' opacity='.45'/><rect x='5.4' y='1.6' width='5.2' height='3.6' rx='1.2' fill='currentColor'/><circle cx='4' cy='12.4' r='1.9' fill='currentColor'/><circle cx='12' cy='12.4' r='1.9' fill='currentColor'/></svg>")
            .accessibility_hidden(true).with_style(style().width(u(4)).height(u(4)).flex_shrink(false));
    }
    let d=switch k{
       "file"=>"M2.5 5.5h4l1.1 1.3h6v5h-11z",
       "failed"=>"M8 2 14 13H2z M8 6v3m0 2h.01",
       "ruled-out"=>"m5 5 6 6m0-6-6 6",
       "confirmed"=>"m3.5 8 3 3 6-6",
       _=>"M8 5v6M5 8h6"};
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.35' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
       .accessibility_hidden(true).with_style(style().width(u(3.5)).height(u(3.5)).flex_shrink(false))
}
fn badge(ctx,title,tone){
    apply(ctx,row([text(title)]),"badge",line().min_height(u(4)).items_center().font_weight(500).flex_shrink(false)
        .radius(theme_radius("sm")).padding_x(sp(1.25))
        .background(ink(tone+"_bg")).text_color(ink(tone)))
}
fn header(ctx,p,d,narrow){
    let title=get(p,"title",tx(ctx,"title","Root cause"));
    let mark=apply(ctx,glyph("tree"),"mark",style().text_color(ink("muted")).flex_shrink(false));
    let heading=label(ctx,"title",title,style().min_width(px(0)).typography("body").font_weight(500).flex_grow()
        .whitespace_nowrap().text_ellipsis()).accessibility_role("heading").accessibility_level(2);
    let status=if d.solved{tx(ctx,"found","Cause found")}else if d.checking!=(){tx(ctx,"investigating","Investigating")}else{tx(ctx,"open","Unresolved")};
    let live=if d.checking!=()&&!d.solved{
        apply(ctx,row([spin::Spinner(#{key:"spin",size:"xs",label:status}).accessibility_hidden(true),text(status)])
            .with_key("investigating").accessibility_role("status").accessibility_label(status),"live",
            line().min_height(u(4.5)).items_center().gap(sp(1)).padding_x(sp(1.25)).radius(theme_radius("sm"))
            .background(ink("brand_bg")).text_color(ink("brand")).flex_shrink(false))
    }else{
        label(ctx,"status",status,line()).with_key("summary-state").accessibility_role("status").accessibility_label(status)
            .with_style(style().absolute().width(px(1)).height(px(1)).overflow_hidden().opacity(0.0))
    };
    let count=d.rows.len-1;
    let summary=if count==0{tx(ctx,"no_hypotheses","No hypotheses yet")}else if narrow{ctx.t_or("root_cause.short","{ruled}/{n} ruled out",#{n:count,ruled:d.ruled})}
        else{ctx.t_or("root_cause.count","{n} hypotheses · {ruled} ruled out",#{n:count,ruled:d.ruled})};
    apply(ctx,row([mark,heading,live,label(ctx,"summary",summary,line()).with_key("summary")]).with_key("head"),"head",
        style().min_width(px(0)).items_center().gap(sp(2)).padding_bottom(sp(3.5)))
}
fn tile(ctx,c){
    if get(c,"icon",())==(){return ();}
    apply(ctx,row([c.icon]).accessibility_hidden(true),"tile",style().width(u(4.5)).height(u(4.5)).flex_shrink(false)
        .items_center().justify_center().radius(theme_radius("sm")).background(ink("card")))
}
fn status_text(ctx,c,r,is_failure){
    if is_failure{return tx(ctx,"failed","Failed");}
    if r.actionable{return tx(ctx,"cause","Root cause");}
    if get(c,"status","open")=="confirmed"&&r.sub_count>0&&!r.proved{
        return tx(ctx,"awaiting","Awaiting proof");
    }
    switch get(c,"status","open"){
        "checking"=>tx(ctx,"checking","Checking"),
        "ruled-out"=>tx(ctx,"ruled","Ruled out"),
        "confirmed"=>tx(ctx,"confirmed","Confirmed"),
        _=>tx(ctx,"not_checked","Not checked")
    }
}
fn time(at){let s=(at+500)/1000;let secs=(s%60).to_string();(s/60).to_string()+":"+if secs.len==1{"0"+secs}else{secs}}
fn basename(path){
    let parts=path.split("/");
    let name=path;for p in parts{if p!=""{name=p;}}name
}
fn file(ctx,path,full){
    let label_text=if full{path}else{basename(path)};
    apply(ctx,row([glyph("file"),label(ctx,"file_name",label_text,line().text_color(ink("text")))]).accessibility_label(path),"file",
        line().items_center().gap(sp(1)).padding_x(sp(1.25)).min_height(u(4.5))
        .background(ink("fill")).radius(theme_radius("sm")))
}
fn files(ctx,c,full){
    let out=[];
    let paths=get(c,"files",[]);
    if paths.len>0{
        if full{for p in paths{out.push(file(ctx,p,true));}}
        else{
            out.push(file(ctx,paths[0],false));
            if paths.len>1{out.push(label(ctx,"more","+"+ctx.format_number(paths.len-1),line().flex_shrink(false)));}
        }
    }
    let n=row(out).with_key("files").with_style(style().min_width(px(0)).flex_wrap().items_center().gap(sp(1)));
    if full{n.accessibility_role("group").accessibility_label(tx(ctx,"files","Files"))}
    else{apply(ctx,n.accessibility_hidden(true),"file_row",style().min_height(u(4.5)))}
}
fn gate(ctx,r){
    let n=if r.all{"AND"}else{"OR"};
    apply(ctx,row([text(n)]).with_key("pill").accessibility_hidden(true),"gate",line().font_weight(600).items_center().justify_center()
        .padding_x(sp(1.25)).min_height(u(4)).radius(theme_radius("sm")).background(ink("card"))
        .text_color(if r.path{ink("brand")}else{ink("muted")}).flex_shrink(false))
}
fn body_card(ctx,p,m,idx){
    let c=p.causes[idx];let r=m.data.rows[idx];let root=idx==m.data.root;
    let status=get(c,"status","open");let selected=m.shown==idx;let excluded=status=="ruled-out"&&!root;
    let words=[];let tile=tile(ctx,c);if tile!=(){words.push(tile);}
    let face=line().text_color(if excluded{ink("muted")}else if root{theme_color("text_primary")}else{ink("ink")})
        .font_weight(500).flex_grow();
    let checking=status=="checking"&&!root;
    if checking{face=face.glyph_ink_sweep(ink("shine"),0.25,1.5,0);}
    let title=label(ctx,"name",c.title,face).with_key("name");
    if checking{
        title=title.motion(motion_transition("glyph_sweep",0.0,1.0,#{duration_ms:900,easing:"linear",iterations:"infinite",intent:"decorative"}));
    }
    words.push(title);if m.narrow&&r.sub_count>0{words.push(gate(ctx,r));}
    let bits=[row(words).with_style(style().min_width(px(0)).items_center().gap(sp(1.75)))];
    if !m.narrow||get(c,"files",[]).len>0{bits.push(files(ctx,c,false));}
    let front=apply(ctx,column(bits).with_key("interior"),if excluded{["inner","inner_excluded"]}else if selected{["inner","inner_selected"]}else{["inner"]},
        style().min_width(px(0)).flex_grow().gap(sp(1.5)).padding_x(sp(2)).padding_y(sp(1.5))
            .radius(theme_radius("sm")).background(if excluded{rgba(0x00000000)}else{ink("card")}));
    let msg=status_text(ctx,c,r,root);let kind=if root{"failed"}else{status};
    let symptom=if kind=="checking"{spin::Spinner(#{key:"checking",label:msg,size:"xs"}).accessibility_hidden(true)}
        else{row([glyph(kind)]).with_style(style().flex_shrink(false).text_color(if root||r.actionable{ink("danger")}else{ink("muted")}))};
    let meta=[symptom];
    if root||r.actionable{meta.push(badge(ctx,msg,"danger"));}
    else{
        let when=get(c,"at_ms",());
        meta.push(label(ctx,"status",msg+if when!=()&&status in ["confirmed","ruled-out"]{" · "+time(when)}else{""},
            line().font_feature("tnum",1).text_color(if status=="open"{ink("faint")}else{ink("muted")})).with_key("status"));
    }
    if root{
        meta.push(label(ctx,"status",get(c,"detail",""),line().flex_grow()).with_key("detail"));
        if m.data.solved{meta.push(badge(ctx,tx(ctx,"found","Cause found"),"brand"));}
    }
    let details=apply(ctx,row(meta).with_key("meta"),"meta",style().min_width(px(0)).min_height(u(5.5)).items_center().gap(sp(1)).padding_x(sp(1.25)));
    let ctl=column([front,details]).with_key("cause:"+c.id).with_ref(m.refs[idx])
        .accessibility_role("treeitem").accessibility_label(c.title).accessibility_level(r.depth+1).accessibility_selected(selected)
        .accessibility_position_in_set(r.position).accessibility_size_of_set(r.count)
        .accessibility_description(msg+if r.sub_count>0{if r.all{" · All branches required"}else{" · Any branch sufficient"}}else{""})
        .tab_stop(m.stop==c.id).on_focus_within_change(Fn("focused").curry(m.ticket,c.id));
    let click=Fn("picked").curry(m.ticket,if c.id==m.picked{""}else{c.id});
    let n=clickable(ctl,click);if m.picked!=""{n=n.on_key_value("plain+escape",Fn("picked").curry(m.ticket,""),());}
    n=bind_nav(n,m,idx,r);
    let parts=["card"];if selected{parts.push("card_selected");}if excluded{parts.push("card_excluded");}
    let s=focus().min_width(px(0)).padding(sp(0.75)).radius(theme_radius("lg")).background(ink("tray"))
        .cursor_pointer().isolated_opacity(true).opacity(if m.picked!=""&&!selected{0.5}else{1.0})
        .focus(style().opacity(1.0)).hover(style().opacity(1.0));
    if m.narrow{s=s.min_height(u(13)).margin_start(sp(4.5).times(min(8,r.depth)));}
    else{s=s.width(relative(1.0)).height(px(m.data.card_height*m.sy));}
    let n=apply(ctx,n,parts,s).paint_transition(210,"ease_out");
    let body=[];
    if m.narrow&&r.depth>0{
        let guide=style().absolute().top(px(0)).inset_start(sp(4.5*min(8,r.depth)-2.25)).height(relative(0.5)).width(sp(2))
            .border_start(px(1.5)).border_bottom(px(1.5)).border_color(ink(if r.path{"path"}else{"rail"}));
        guide=if m.rtl{guide.radius_bottom_right(u(1.25))}else{guide.radius_bottom_left(u(1.25))};
        body.push(apply(ctx,box([]).with_key("sub-guide").accessibility_hidden(true),"guide",guide));
    }
    body.push(n);
    let box_style=if m.narrow{style().relative().min_width(px(0))}
       else{style().absolute().left(px(center(m,r)-r.w/2.0)).top(px(r.y*m.sy)).width(px(r.w)).height(px(m.data.card_height*m.sy))};
    // Only the geometry changes on reflow; the focusable cause keeps its tree path.
    box(body).with_key("branch:"+c.id).with_style(box_style)
}
fn bind_nav(n,m,idx,r){
    let list=m.data.order;let position=list.index_of(idx);
    let previous=if position>0{list[position-1]}else{()};let next=if position+1<list.len{list[position+1]}else{()};
    let targets=[["plain+down",if m.narrow{next}else{r.child}],["plain+up",if m.narrow{previous}else{r.parent}],
        [if m.rtl{"plain+left"}else{"plain+right"},if m.narrow{r.child}else{r.after}],
        [if m.rtl{"plain+right"}else{"plain+left"},if m.narrow{r.parent}else{r.before}],
        ["plain+home",list[0]],["plain+end",list[list.len-1]]];
    for d in targets{if d[1]!=(){n=n.on_key_value(d[0],Fn("focus_cause").curry(m.refs[d[1]].key),());}}
    n
}
fn center(m,r){if m.rtl{m.width-r.cx}else{r.cx}}
fn edge(m,from,to){
    let x=center(m,from);let y=from.y*m.sy;let tx=center(m,to);let ty=(to.y+m.data.card_height)*m.sy;
    let joint=ty+m.data.level*0.62*m.sy;let sign=if tx<x{-1.0}else{1.0};
    let rad=min(8.0*m.sy,min(abs(tx-x)*0.5,min(y-joint,joint-ty)));
    if rad<=0.5{return [path_move(x,y),path_line(tx,ty)];}
    [path_move(x,y),path_line(x,joint+rad),path_quadratic(x+sign*rad,joint,x,joint),
        path_line(tx-sign*rad,joint),path_quadratic(tx,joint-rad,tx,joint),path_line(tx,ty)]
}
fn wires_style(ctx){ctx.component_style("wires",style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)))}
fn diagram(ctx,p,m,ref){
    let bits=[];let d=m.data;
    // Measure the native board width and themed vertical unit independently of the
    // current graph. Card, label and Canvas coordinates use one committed metric.
    bits.push(box([]).with_key("diagram-metrics").with_ref(ref).accessibility_hidden(true)
        .with_style(style().absolute().top(px(0)).left(px(0)).width(relative(1.0)).height(u(d.card_height/4.0)).invisible()));
    if !m.narrow{
        let normal=[];let proof=[[],[],[],[],[]];let gates=[];
        for k in d.order{
            let r=d.rows[k];let c=p.causes[k];
            if r.parent!=(){
                let base=edge(m,r,d.rows[r.parent]);
                let dashed=get(c,"status","open")=="ruled-out";
                // These are frame-local commands. Navigation and the proof
                // animation's replay key retain the actual business IDs.
                let drawing_key="branch:"+k.to_string();
                normal.push(canvas_stroke_path(drawing_key,if dashed{fallback_dash_route(base,0.014,0.024)}else{base},
                    1.5,ink("rail")).stroke_cap("round"));
                if r.path{proof[min(4,d.depth-r.depth)].push(canvas_stroke_path(drawing_key,base,1.7,ink("path")).stroke_cap("round"));}
            }
            if r.sub_count>0{
                let x=center(m,r);let y=(r.y+d.card_height+d.level*0.31)*m.sy;
                let position=style().absolute().left(px(x-r.w/2.0)).width(px(r.w))
                    .top(px(y-8.0*m.sy)).height(px(16.0*m.sy)).justify_center().items_center();
                gates.push(row([gate(ctx,r)]).with_key("gate:"+c.id).accessibility_hidden(true).with_style(position));
            }
        }
        if normal.len>0{
            bits.push(canvas(canvas_scene(normal).view_box(0,0,m.width,d.height*m.sy)).with_key("quiet-wires").accessibility_hidden(true).with_style(wires_style(ctx)));
        }
        for i in 0..proof.len{
            if proof[i].len>0{
                bits.push(canvas(canvas_scene(proof[i]).view_box(0,0,m.width,d.height*m.sy)).with_key("proof:"+i.to_string())
                    .accessibility_hidden(true).with_style(wires_style(ctx))
                    .motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:520,delay_ms:i*380,easing:ease(),intent:"decorative"}))
                    .motion_replay_key(m.proof_stamp));
            }
        }
        bits+=gates;
    }
    for i in d.order{bits.push(body_card(ctx,p,m,i));}
    let layout=style().relative().min_width(px(0)).width(relative(1.0)).gap(sp(1.5));
    let plan=if m.narrow{layout}else{layout.height(px(d.height*m.sy))};
    apply(ctx,column(bits).with_key("diagram").accessibility_role("tree")
        .accessibility_label(tx(ctx,"why","Failure hypotheses")),"diagram",plan)
}
fn evidence(ctx,c){
    let d=get(c,"evidence",());
    if d==(){return ();}
    let labels=[label(ctx,"evidence_label",d.kind.to_upper(),line().font_weight(500).flex_shrink(false)).with_key("kind")];
    let source=get(d,"source","");
    if source!=""{labels.push(label(ctx,"source",source,line().text_color(ink("faint"))).with_key("source"));}
    let caption=row(labels).with_key("caption").with_style(style().min_width(px(0)).items_start().gap(sp(1.5)));
    let doc=code::CodeViewer(#{key:"code:"+c.id,label:d.kind,source:d.content,language:get(d,"language","plain"),show_gutter:false,
        show_line_numbers:false,wrap:"viewport",height:"content",max_height:112,scrollbar:"auto",
        part_styles:#{root:style().background(rgba(0x00000000)).shadows([]),
            text:style().typography("code").font_size(u(2.875)).line_height(u(4.25)).text_color(ink("ink"))}});
    column([caption,box([doc]).with_key("scroll").with_style(ctx.component_style("evidence",style().min_width(px(0))
        .padding_x(sp(2.5)).padding_y(sp(2)).radius(theme_radius("sm")).background(ink("well"))))]).with_key("evidence-record")
        .with_style(style().min_width(px(0)).gap(sp(1)))
}
fn button(ctx,p,m,c,kind,primary){
    let name=if primary{tx(ctx,"fix","Fix it")}else{tx(ctx,"retry","Retry from here")};
    let button=apply(ctx,row([text(name)]).with_key(kind).accessibility_role("button").accessibility_label(name)
        .disabled(p.pending).tab_stop(!p.pending),if primary{["button","primary"]}else{["button"]},
        focus().typography("caption").min_height(u(6)).items_center().padding_x(sp(2)).radius(theme_radius("sm")).cursor_pointer()
        .background(ink(if primary{"solid"}else{"card"})).text_color(if primary{ink("on_solid")}else{theme_color("text_primary")})
        .hover(style().opacity(0.86)).disabled(style().opacity(0.5)));
    if p.pending{button}else{clickable(button,Fn("propose").curry(m.ticket,c.id,kind))}
}
fn inspector(ctx,p,m){
    let r=m.data.rows[m.shown];let c=p.causes[m.shown];let root=m.shown==m.data.root;
    let msg=status_text(ctx,c,r,root);let when=get(c,"at_ms",());
    let title=label(ctx,"panel_title",c.title,style().min_width(px(0)).typography("body").font_weight(500).flex_grow())
        .accessibility_role("heading").accessibility_level(3).accessibility_label(c.title+" evidence");
    let status=get(c,"status","open");
    let custom=tile(ctx,c);
    let mark=if custom!=(){custom}else{glyph(if root{"failed"}else{status})
        .with_style(style().text_color(if root||status=="confirmed"{ink("danger")}else{ink("muted")}))};
    let heading_bits=[mark.with_style(style().margin_top(sp(0.25)).flex_shrink(false)),title];
    if !root{heading_bits.push(label(ctx,"panel_status",msg+if when!=()&&status in ["confirmed","ruled-out"]{" · "+time(when)}else{""},
        line().flex_shrink(false).font_feature("tnum",1).padding_top(sp(0.5))).with_key("panel-status"));}
    let heading=apply(ctx,row(heading_bits)
        .with_key("heading"),"panel_head",style().min_width(px(0)).items_start().gap(sp(1.5)).padding_x(sp(2)).padding_y(sp(1)));
    let fallback=if root&&m.data.rows.len==1{tx(ctx,"no_hypotheses","No hypotheses yet")}
        else if root&&!m.data.solved{tx(ctx,"unresolved","No complete explanation is confirmed.")}
        else if get(c,"status","open")=="checking"{tx(ctx,"checking_detail","Checking this now…")}
        else{msg};
    let verdict=get(c,"verdict",get(c,"detail",()));
    let bits=[label(ctx,"verdict",if verdict==(){fallback}else{verdict},
        small().text_color(if verdict==(){ink("muted")}else{ink("text")})).with_key("verdict")];
    if get(c,"files",[]).len>0{bits.push(files(ctx,c,true));}
    let doc=evidence(ctx,c);if doc!=(){bits.push(doc);}
    let base=apply(ctx,column(bits).with_key("body"),"panel_body",style().min_width(px(0)).gap(sp(2))
        .padding(sp(2.5)).background(ink("card")).radius(theme_radius("lg")));
    let elements=[heading,base];
    if r.actionable&&get(p,"on_action",())!=(){
        let actions=[button(ctx,p,m,c,"retry",false),button(ctx,p,m,c,"fix",true)];
        let row=if p.pending{
            let txt=tx(ctx,"pending","Waiting for confirmation");
            actions.insert(0,label(ctx,"pending",txt,line().flex_grow()).accessibility_role("status").accessibility_label(txt));row(actions)
        }else{row(actions)};
        elements.push(apply(ctx,row.with_key("actions"),"actions",style().min_width(px(0)).items_center().justify_end().flex_wrap().gap(sp(1.5))));
    }
    let surface=column(elements).with_key("selected:"+c.id).with_style(style().min_width(px(0)).gap(sp(1.25)))
        .enter_motion(motion_transition("opacity",0.0,1.0,#{duration_ms:220,easing:ease(),intent:"decorative"}));
    apply(ctx,column([surface]).with_key("inspector").accessibility_role("region").accessibility_label(tx(ctx,"evidence","Evidence"))
        .layout_size_motion(c.id,"height",280,ease()),"panel",
        style().min_width(px(0)).padding(sp(1)).radius(theme_radius("lg")).background(ink("tray")))
}
fn view_causes(ctx,p){
    if utf8_byte_count(p.session)==0||utf8_byte_count(p.session)>96{throw "RootCauseTree requires a stable session ID";}
    let ref=element_ref("diagram-metrics");let size=ctx.element_size(ref);let width=if size==(){720.0}else{min(20000.0,max(1.0,size.width))};
    let d=root_cause_projection(p.causes,width);
    let sy=if size==()||size.height<=0.0{1.0}else{size.height/d.card_height};
    let s=state(ctx,p.session);let controlled="selected_id" in p;
    let asked=if controlled{get(p,"selected_id","")}else{s.picked};
    let indices=#{};
    for i in 0..p.causes.len{indices[p.causes[i].id]=i;}
    let pick=if asked in indices{asked}else{""};
    let shown=if pick!=""{indices[pick]}else if d.checking!=(){d.checking}else if d.cause!=(){d.cause}else{d.root};
    let stop=if s.stop in indices{s.stop}else{p.causes[d.root].id};
    let lost_pick=if s.picked!=""&&!(s.picked in indices){s.picked}else{""};
    let lost_focus=if s.stop!=""&&!(s.stop in indices){s.stop}else{""};
    if lost_pick!=""||lost_focus!=""{effect("clean_selection",#{session:p.session,lost_pick:lost_pick,lost_focus:lost_focus},Fn("clear"),Fn("noop"));}
    let stamp=p.session;for k in d.order{if d.rows[k].path{stamp+=utf8_byte_count(p.causes[k].id).to_string()+":"+p.causes[k].id;}}
    let refs=[];for cause in p.causes{refs.push(element_ref("cause",cause.id));}
    let m=#{data:d,width:width,rtl:ctx.text_direction()=="rtl",shown:shown,picked:pick,stop:stop,
        narrow:d.narrow,sy:sy,refs:refs,proof_stamp:stamp,ticket:#{session:p.session,revision:p.revision,controlled:controlled}};
    apply(ctx,column([header(ctx,p,d,m.narrow),diagram(ctx,p,m,ref),inspector(ctx,p,m)]).with_key(p.key),
        "root",style().width(relative(1.0)).min_width(px(0)).gap(sp(3)))
}
