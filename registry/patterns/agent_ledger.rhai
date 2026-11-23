/* gpui-rhai
{"id":"patterns/agent_ledger","export":"AgentLedger","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/avatar","patterns/agent_tree","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],"environment":[]}
*/
// Describe the changes an agent made and preview atomic undo or redo batches.
// Example: AgentLedger(#{key:"ledger",run:"snapshot/1",revision:0,entries:[]})
import "components/avatar" as avatar;
import "patterns/agent_tree" as ids;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_ledger","export":"AgentLedger",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/avatar","patterns/agent_tree","patterns/approval_card"],capabilities:#{},
        tokens:["accent","border","focus_ring","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable retained surface key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of this ledger series; replace it to start a new ledger."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision of the Host-accepted entries and undo batches."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Heading, defaulting to Ledger."},
        agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Agent who made the changes."},
        now_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:253402300799999}},required:false,sensitive:false,
            doc:"Accepted epoch millisecond clock. Omit for a view-owned fifteen-second relative time refresh."},
        time_zone:#{schema:#{type:"string"},"default":#{type:"string",value:"UTC"},required:false,sensitive:false,doc:"Zone for absolute date labels."},
        entries:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable change ID."},
            action:#{schema:#{type:"string"},required:true,sensitive:true,doc:"What was changed."},
            target:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Object of the change."},
            system:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Application or system where it happened."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative system mark; defaults to initials."},
            at_ms:#{schema:#{type:"integer",min:0,max:253402300799999},required:true,sensitive:false,doc:"Epoch milliseconds of the original change."},
            change:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
                before:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Previous value."},
                after:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Accepted new value."},
            }}},required:false,sensitive:true,doc:"Before/after pair; takes precedence over detail."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Single secondary line when change is omitted."},
            reversible:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"False for an irreversible change."},
            depends_on:#{schema:#{type:"optional",value:#{type:"array",max_items:8,items:#{type:"string"}}},required:false,sensitive:false,
                doc:"Existing change IDs that this one builds on; the dependency graph must be acyclic."},
        }}},required:true,sensitive:true,doc:"An accepted ledger of up to 64 changes. The display sorts newest first."},
        undone_batches:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            root:#{schema:#{type:"string"},required:true,sensitive:false,doc:"The original Undo command's change ID."},
            changes:#{schema:#{type:"array",max_items:64,items:#{type:"string"}},required:true,sensitive:false,
                doc:"The reversible changes actually undone together, including the root. No ID may be in two batches."},
        }}}},required:false,sensitive:false,doc:"Host-accepted undo batches. Redo restores exactly one such batch."},
        point_at:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional Undo preview target for a walkthrough. Local action hover and focus take precedence."},
        show_actions:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Always show commands instead of waiting for hover or focus."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Block more requests while the Host settles a transaction; retain command focus."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives one versioned Undo or Redo request. Without it the ledger is read-only."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host progress, acceptance or refusal announced as status."},
    },state:#{fields:#{
        interaction:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        initial:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        clock:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Ledger series owning the request."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision the command used."},
            kind:#{schema:#{type:"string",allowed:["undo","redo"]},required:true,sensitive:false,doc:"Proposed operation."},
            root:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Undo root, or original root of the batch being redone."},
            anchor:#{schema:#{type:"string"},required:true,sensitive:false,doc:"The change whose command was activated."},
            changes:#{schema:#{type:"array",max_items:64,items:#{type:"string"}},required:true,sensitive:false,
                doc:"Proposed reversible IDs, prerequisites before dependents. For Undo, execute inversions in reverse dependency order if required by the application."},
            locked:#{schema:#{type:"array",max_items:64,items:#{type:"string"}},required:true,sensitive:false,
                doc:"Active irreversible descendants left unchanged by Undo. Empty for Redo."},
        }},doc:"The Host validates the revision and executes or refuses the proposed batch atomically."},
    },slots:#{},parts:["root","bar","mark","title","agent","summary","total","undone_total","panel","empty","row","preview","also","kept",
        "divider","icon","icon_undone","main","headline","action","target","quiet","change","before","before_line","arrow","after","detail","strike",
        "end","end_narrow","time","undone_time","command","command_narrow","command_hot","command_busy","command_text","flag","flag_also","flag_kept","locked","notice"],
        effects:["remember_initial","prune_pointer"]},
    render:Fn("render_ledger")
});

fn AgentLedger(props){render_component("patterns/agent_ledger",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,v){common::value_or(m,k,v)}
fn lg(n){theme_length("space.unit").times(n)}
fn parents(e){value(e,"depends_on",[])}
fn reversible(e){value(e,"reversible",true)}
fn newer(a,b){if a.entry.at_ms==b.entry.at_ms{a.ordinal-b.ordinal}else if a.entry.at_ms>b.entry.at_ms{-1}else{1}}
fn has(mask,bit){(mask & bit)!=0}
fn graph(props){
    if !ids::valid(props.run){throw "AgentLedger requires a stable run ID";}
    let entries=#{};let children=#{};let previous=#{};let counts=#{};let timeline=[];let bytes=0;
    for i in 0..props.entries.len{
        let e=props.entries[i];
        if !ids::valid(e.id)||e.id in entries{throw "AgentLedger requires distinct valid change IDs";}
        if e.action==""||e.system==""{throw "Ledger changes need an action and system";}
        for field in ["action","target","system","detail"]{bytes+=utf8_byte_count(value(e,field,""));}
        if opt(e,"change")!=(){bytes+=utf8_byte_count(e.change.before)+utf8_byte_count(e.change.after);}
        entries[e.id]=e;children[e.id]=[];counts[e.id]=0;previous[e.id]=[];timeline.push(#{entry:e,ordinal:i});
    }
    if bytes>196608{throw "AgentLedger text exceeds 192 KiB";}
    for e in props.entries{
        let seen=#{};
        for id in parents(e){
            if !(id in entries)||id==e.id||id in seen{throw "Ledger dependencies must be distinct existing changes";}
            seen[id]=true;
            let down=children[id];down.push(e.id);children[id]=down;
            let up=previous[e.id];up.push(id);previous[e.id]=up;
            counts[e.id]+=1;
        }
    }
    let order=[];
    for e in props.entries{if counts[e.id]==0{order.push(e.id);}}
    let i=0;
    while i<order.len{
        for next in children[order[i]]{counts[next]-=1;if counts[next]==0{order.push(next);}}
        i+=1;
    }
    if order.len!=props.entries.len{throw "Ledger dependencies contain a cycle";}
    // The schema caps this graph at 64 vertices. One exact bit per vertex
    // keeps dependency queries cheap even for dense ledgers or a hover preview.
    let bits=#{};let upwards=#{};let downwards=#{};
    for j in 0..order.len{
        let id=order[j];let bit=1<<j;
        bits[id]=bit;upwards[id]=bit;downwards[id]=bit;
    }
    for j in 0..order.len{
        let above=order[j];let below=order[order.len-j-1];
        for head in previous[above]{upwards[above]=upwards[above] | upwards[head];}
        for tail in children[below]{downwards[below]=downwards[below] | downwards[tail];}
    }
    timeline.sort(Fn("newer"));
    #{entries:entries,timeline:timeline,order:order,bits:bits,up:upwards,down:downwards}
}
fn batch_state(props,g){
    let owner=#{};let batches=#{};let masks=#{};let required=#{};let delay=#{};let blocked=#{};let undone=0;
    for b in value(props,"undone_batches",[]){
        if !(b.root in g.entries)||!(b.root in b.changes)||b.root in batches{throw "A ledger batch must have its own existing root";}
        let related=g.down[b.root];let members=0;let prerequisites=0;
        for id in b.changes{
            if !(id in g.bits)||!has(related,g.bits[id])||id in owner||!reversible(g.entries[id]){
                throw "Undo batches must be disjoint reversible changes reachable from their root";
            }
            owner[id]=b.root;members=members | g.bits[id];prerequisites=prerequisites | g.up[id];
        }
        let order=[];for id in g.order{if has(members,g.bits[id]){delay[id]=order.len;order.push(id);}}
        batches[b.root]=order;masks[b.root]=members;required[b.root]=prerequisites;undone=undone | members;
    }
    for root in batches.keys(){
        if (required[root] & undone)!=masks[root]{blocked[root]=true;}
    }
    #{owner:owner,batches:batches,delay:delay,blocked:blocked}
}
fn undo_scope(id,g,state){
    let reachable=g.down[id];let changes=[];let locked=[];
    for next in g.order{
        if has(reachable,g.bits[next])&&!(next in state.owner){
            if reversible(g.entries[next]){changes.push(next);}else{locked.push(next);}
        }
    }
    #{changes:changes,locked:locked}
}
fn local(ctx,run){
    let u=ctx.get_state("interaction");
    if u!=()&&u.run==run{return u;}
    #{run:run,hover:"",hit:"",focus:""}
}
fn changed(run,field,id,ctx,inside){
    let u=local(ctx,run);let was=u[field];
    let next=if inside{id}else if was==id{""}else{was};
    if next!=was{u[field]=next;ctx.set_state("interaction",u);}
}
fn prune(ctx,p){
    let u=local(ctx,p.run);let update=false;
    for field in ["hover","hit","focus"]{if u[field]!=""&&!(u[field] in p.ids){u[field]="";update=true;}}
    if update{ctx.set_state("interaction",u);}
}
fn idle(ctx,p){}
fn init(ctx,p){
    let s=ctx.get_state("initial");
    if s==()||s.run!=p.run{ctx.set_state("initial",p);}
}
fn project_keys(g){let keys=#{};for id in g.order{keys[id]=true;}keys}
fn dispatch_ledger(payload,ctx,e){ctx.emit("action",payload);handled()}
fn proposal(props,g,state,e){
    if opt(props,"on_action")==(){return ();}
    if e.id in state.owner{
        let root=state.owner[e.id];
        if root in state.blocked{return ();}
        return #{run:props.run,revision:props.revision,kind:"redo",root:root,anchor:e.id,changes:state.batches[root],locked:[]};
    }
    if !reversible(e){return ();}
    let scope=undo_scope(e.id,g,state);
    #{run:props.run,revision:props.revision,kind:"undo",root:e.id,anchor:e.id,changes:scope.changes,locked:scope.locked}
}
fn tick(ctx,run){ctx.set_state("clock",#{run:run,now:ctx.now()});}
fn clock(ctx,props){
    if opt(props,"now_ms")!=(){return props.now_ms;}
    if props.entries.len==0{return 0;}
    interval("ledger-clock",15000,false,Fn("tick"),props.run);
    let saved=ctx.get_state("clock");
    if saved!=()&&saved.run==props.run{saved.now}else{ctx.now()}
}
fn when(ctx,at,now,zone){
    let d=now-at;
    if d<=-60000||d>=2592000000{
        let date=time_parts(at,zone).date;
        try{return ctx.format_date(date,"medium");}catch{return date;}
    }
    let s=max(0,d)/1000;
    if s<10{return ctx.t_or("agent_ledger.now","Just now");}
    let unit=if s<60{[1,"s"]}else if s<3600{[60,"m"]}else if s<86400{[3600,"h"]}else{[86400,"d"]};
    let n=s/unit[0];let age="";
    try{age=ctx.format_duration(n*unit[0]*1000);}catch{age=ctx.format_number(n)+unit[1];}
    ctx.t_or("agent_ledger.ago","{age} ago",#{age:age})
}
fn title_for(e){
    let target=value(e,"target","");if target==""{e.action}else{e.action+" "+target}
}
fn line(ctx,words,parts,base){
    text(words).with_style(ctx.component_style(parts,base.min_width(px(0)).whitespace_nowrap().text_ellipsis()))
}
fn caption(ctx,words,parts){line(ctx,words,parts,style().typography("caption").text_color(theme_color("text_muted")))}
fn glyph(kind){
    let p=switch kind{
        "ledger"=>"M4 2h8q2 0 2 2v8q0 2-2 2H4q-2 0-2-2V4q0-2 2-2zM5 5.4h6M5 8h6M5 10.6h3.6",
        "undo"=>"M7 4 3 8l4 4M3 8h11a5 5 0 1 1 0 10h-5",
        "redo"=>"m15 4 4 4-4 4M19 8H8a5 5 0 1 0 0 10h5",
        _=>"M6 10h10a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-6a2 2 0 0 1 2-2zM7 10V7a4 4 0 0 1 8 0v3",
    };
    let box=if kind=="ledger"{"0 0 16 16"}else{"0 0 22 22"};
    svg(`<svg viewBox='${box}' fill='none' stroke='currentColor' stroke-width='1.45' stroke-linecap='round' stroke-linejoin='round'><path d='${p}'/></svg>`)
        .accessibility_hidden(true).with_style(style().width(lg(3.5)).height(lg(3.5)).flex_shrink(false))
}
fn header(ctx,props,cfg,state,title){
    let primary=[glyph("ledger").with_key("icon").with_style(ctx.component_style("mark",style().text_color(theme_color("accent")))),
        line(ctx,title,"title",style().typography("body").font_weight(600).text_color(theme_color("text_primary")))
            .with_key("title").accessibility_role("heading").accessibility_level(2)];
    if opt(props,"agent")!=(){primary.push(caption(ctx,props.agent,"agent").with_key("owner"));}
    let all=ctx.t_or("agent_ledger.total","{n} changes",#{n:ctx.format_number(props.entries.len)});
    let count=[caption(ctx,all,"total").with_key("all")];
    let summary_label=all;let undone=state.owner.len();
    if undone>0{
        let pending=ctx.t_or("agent_ledger.undone_total","{n} undone",#{n:ctx.format_number(undone)});
        summary_label+=" · "+pending;
        count.push(caption(ctx,pending,"undone_total")
            .with_key("undone").motion(motion_transition("opacity",0.0,1.0,#{duration_ms:240,intent:"decorative"})));
    }
    // These are peer metrics with their own spacing, independent of the title
    // and avatar rhythm in the other group.
    let summary=row(count).with_key("summary").accessibility_role("status").accessibility_label(summary_label)
        .audit_allow(["spacing-not-nested"])
        .with_style(ctx.component_style("summary",style().items_center().gap(lg(3)).min_width(px(0)).flex_shrink(false).flex_nowrap()));
    let identity=row(primary).with_key("identity").audit_allow(["mixed-type-in-row"])
        .with_style(style().gap(theme_length("space.spacing_unit").times(2)).items_center().min_width(px(0)).flex_grow());
    let group=if cfg.narrow{column([identity,summary])}else{row([identity,summary])};
    group.with_key("bar").with_style(ctx.component_style("bar",style().min_width(px(0)).gap(lg(2)).padding_x(lg(3)).padding_y(lg(2))))
}
fn mark_for(ctx,e,undone,delay){
    let icon=value(e,"icon",text(avatar::first_grapheme(e.system).to_upper()).with_key("initial"));
    row([icon]).with_key("icon").accessibility_hidden(true)
        .with_style(ctx.component_style(if undone{["icon","icon_undone"]}else{["icon"]},style().width(lg(8)).height(lg(8))
            .items_center().justify_center().flex_shrink(false).background(theme_color("surface_raised")).radius(lg(2)).clip()))
        .paint_transition(300,cubic_bezier(0.25,0.1,0.25,1.0),#{group_effects:true,delay_ms:delay})
}
fn stripe(ctx,key,part,body,undone,delay){
    let end=if undone{1.0}else{0.0};
    let stroke=row([]).with_key("line").accessibility_hidden(true)
        .with_style(ctx.component_style(part,style().absolute().left(px(0)).top(relative(0.50)).height(px(1))
            .width(relative(end)).background(theme_color("text_muted"))));
    if part=="strike"{
        stroke=stroke.motion(motion_transition("width_fraction",1.0-end,end,
            #{duration_ms:380,delay_ms:if undone{delay}else{0},easing:cubic_bezier(0.22,1.0,0.36,1.0),intent:"decorative"}))
            .motion_on_change(if undone{"undone"}else{"active"});
    }
    stack([body,stroke]).with_key(key).with_style(style().relative().min_width(px(0)).max_width(relative(1.0)))
}
fn description(ctx,e,undone,delay){
    let word="";let details=[];
    let ch=opt(e,"change");
    if ch!=()&&(ch.before!=""||ch.after!=""){
        word=ctx.t_or("agent_ledger.diff","{before} to {after}",#{before:ch.before,after:ch.after});
        details=[stripe(ctx,"before-box","before_line",caption(ctx,ch.before,"before").with_key("before"),true,0),
            caption(ctx,if ctx.text_direction()=="rtl"{"←"}else{"→"},"arrow").with_key("arrow").accessibility_hidden(true),
            caption(ctx,ch.after,if undone{["after","quiet"]}else{["after"]}).with_key("after")];
    }else{
        word=value(e,"detail","");
        details=[caption(ctx,word,"detail").with_key("detail")];
    }
    let what=[line(ctx,e.action,if undone{["action","quiet"]}else{["action"]},
        style().typography("body").font_weight(600).text_color(theme_color("text_primary"))).with_key("verb")];
    let target=value(e,"target","");
    if target!=""{what.push(line(ctx,target,if undone{["target","quiet"]}else{["target"]},style().typography("body")
        .text_color(theme_color("text_muted"))).with_key("target"));}
    let title=row(what).with_key("words").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("headline",
        style().min_width(px(0)).items_center().gap(lg(1.5))));
    let main=column([stripe(ctx,"header-line","strike",title,undone,delay),
        row(details).with_key("change").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("change",
            style().min_width(px(0)).items_center().gap(lg(1.5))))]).with_key("main")
        .with_style(ctx.component_style("main",style().min_width(px(0)).flex_grow().items_start()));
    #{body:main,spoken:word}
}
fn flag(ctx,kind){
    let kept=kind=="kept";
    let label=if kept{ctx.t_or("agent_ledger.kept","Can't undo")}else{ctx.t_or("agent_ledger.also","Also undone")};
    let parts=if kept{["flag","flag_kept"]}else{["flag","flag_also"]};
    let words=[text(label).with_key("label").with_style(style().min_width(px(0)).whitespace_nowrap().text_ellipsis())];
    if kept{words.insert(0,glyph("lock").with_key("lock"));}
    row(words).with_key("flag").accessibility_hidden(true)
        .with_style(ctx.component_style(parts,style().min_width(px(0)).items_center().gap(lg(1)).typography("caption")))
}
fn end_slot(ctx,props,cfg,state,e,req,kind){
    let undone=e.id in state.owner;let who=title_for(e);let token=req!=();
    let visible=token&&(props.show_actions||cfg.ui.hover==e.id||cfg.ui.focus==e.id||cfg.point==e.id||cfg.ui.hit==e.id);
    let time=if undone{ctx.t_or("agent_ledger.undone","Undone")}else{when(ctx,e.at_ms,cfg.now,props.time_zone)};
    let desc=if cfg.narrow||undone{time}else{time+" · "+e.system};
    let text_part=if undone{["time","undone_time"]}else{["time"]};
    let children=[];
    if kind!=""&&!visible{
        children.push(row([flag(ctx,kind)]).with_key("flag-line").with_style(style().absolute()
            .left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)).items_center().justify_end()));
    }
    else if !visible{
        if undone&&state.owner[e.id] in state.blocked{
            children.push(caption(ctx,ctx.t_or("agent_ledger.prereq","Restore prerequisites first"),["time","locked"]).with_key("time"));
        }else{children.push(caption(ctx,desc,text_part).with_key("time").accessibility_hidden(true).with_style(style().text_end()));}
        children=[row(children).with_key("time-line").with_style(style().absolute().left(px(0)).top(px(0))
            .width(relative(1.0)).height(relative(1.0)).items_center().justify_end())];
    }
    if token{
        let redo=req.kind=="redo";let label=if redo{ctx.t_or("agent_ledger.redo","Redo")}else{ctx.t_or("agent_ledger.undo","Undo")};
        let extra=if req.changes.len<=1{""}else{ctx.t_or("agent_ledger.batch_size"," · {n} changes",#{n:ctx.format_number(req.changes.len)})};
        let items=[glyph(req.kind).with_key("arrow")];
        if !cfg.narrow{items.push(caption(ctx,label,"command_text").with_key("label"));}
        let parts=["command"];
        if cfg.narrow{parts.push("command_narrow");}
        if cfg.point==e.id{parts.push("command_hot");}
        if props.busy{parts.push("command_busy");}
        let fn_press=Fn("dispatch_ledger").curry(req);
        let button=row(items).with_key("command").accessibility_role("button").accessibility_label(label+": "+who)
            .accessibility_description(extra).tab_stop(true).disabled(props.busy)
            .on_click(fn_press).on_hover_change(Fn("changed").curry(props.run,"hit",e.id))
            .on_focus_within_change(Fn("changed").curry(props.run,"focus",e.id))
            .audit_allow(["mixed-type-in-row"])
            .with_style(ctx.component_style(parts,style().absolute().inset_end(px(0)).top(px(0)).height(relative(1.0))
                .min_width(lg(7)).justify_center().items_center().gap(lg(1.25)).padding_x(lg(1.75)).radius(lg(1.5))
                .background(theme_color("surface_raised")).focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))))
            .with_style(style().opacity(if visible{1.0}else{0.0}));
        children.push(button);
    }
    if cfg.narrow{
        // Measure both localised flags in a single invisible grid cell. They
        // share the real flag recipe, but don't add a hit or semantic owner.
        // Leaving the end slot wide enough in every state avoids hover reflow.
        let cell=style().grid_cell(1,1);
        let probe=box([flag(ctx,"also").with_key("width-also").with_style(cell),
            flag(ctx,"kept").with_key("width-kept").with_style(cell)])
            .with_key("flag-width").accessibility_hidden(true)
            .with_style(style().grid().grid_cols_max_content(1).height(relative(1.0)).invisible());
        children.insert(0,probe);
    }
    stack(children).with_key("end").with_style(ctx.component_style(if cfg.narrow{["end","end_narrow"]}else{["end"]},
        style().relative().min_width(lg(if cfg.narrow{19}else{30})).height(lg(7)).flex_shrink(false).items_center().justify_end()))
}
fn record(ctx,props,cfg,g,state,result,index,previous_mark){
    let e=result.entry;let req=proposal(props,g,state,e);let undone=e.id in state.owner;
    let kind=value(cfg.marks,e.id,"");let highlighted=e.id==cfg.point||kind!="";
    let delay=if undone{min(1200,state.delay[e.id]*110)}else{0};
    let main=description(ctx,e,undone,delay);
    let summary=e.system+" · "+when(ctx,e.at_ms,cfg.now,props.time_zone);
    if main.spoken!=""{summary+=" · "+main.spoken;}
    if undone{summary+=" · "+ctx.t_or("agent_ledger.undone","Undone");}
    if undone&&state.owner[e.id] in state.blocked{summary+=" · "+ctx.t_or("agent_ledger.prereq","Restore prerequisites first");}
    if !reversible(e){summary+=" · "+ctx.t_or("agent_ledger.irreversible","Irreversible");}
    if kind=="also"{summary+=" · "+ctx.t_or("agent_ledger.preview_also","Will also be undone");}
    if kind=="kept"{summary+=" · "+ctx.t_or("agent_ledger.preview_kept","Irreversible; will stay applied");}
    let parts=["row"];
    if e.id==cfg.point{parts.push("preview");}else if kind!=""{parts.push(kind);}
    let chrome=[row([mark_for(ctx,e,undone,delay),main.body,end_slot(ctx,props,cfg,state,e,req,kind)]).with_key("columns")
        .audit_allow(["mixed-type-in-row","text-edge-misaligned"])
        .with_style(style().min_width(px(0)).width(relative(1.0)).items_center().gap(lg(if cfg.narrow{2.5}else{3})))];
    if index>0&&!highlighted&&!previous_mark{
        chrome.push(row([]).with_key("rule").accessibility_hidden(true)
            .with_style(ctx.component_style("divider",style().absolute().top(px(0)).inset_start(lg(13))
                .inset_end(lg(3)).height(px(1)).background(theme_color("border")))));
    }
    let body=stack(chrome).with_key("body").with_style(ctx.component_style(parts,style().relative().min_width(px(0))
        .min_height(lg(15.5)).padding_x(lg(2.5)).padding_y(lg(2)).flex_shrink(false).items_center().radius(lg(2))
        .hover(style().background(theme_color("surface_hover")))))
        .on_hover_change(Fn("changed").curry(props.run,"hover",e.id)).paint_transition(200,"ease_out");
    if index==0{body=body.with_ref(element_ref("row-measure"));}
    let fresh=!(e.id in cfg.initial);
    if fresh&&cfg.animate{
        body=body.motion(motion_transition("opacity",0.0,1.0,#{duration_ms:420,delay_ms:100,intent:"decorative"}))
            .motion(motion_transition("translate_y",-4.0,0.0,#{duration_ms:420,delay_ms:100,intent:"decorative"}));
    }
    let root=column([body]).with_key("entry:"+cfg.run_key+e.id).accessibility_role("listitem")
        .accessibility_label(title_for(e)).accessibility_description(summary)
        .with_style(style().min_width(px(0)).width(relative(1.0)).flex_shrink(false));
    if fresh&&cfg.animate&&cfg.measure!=()&&cfg.measure.height>0.0{
        root=root.motion(motion_transition("clip_height",0.0,cfg.measure.height,
            #{duration_ms:520,easing:cubic_bezier(0.22,1.0,0.36,1.0),intent:"decorative"}));
    }
    root
}
fn render_ledger(ctx,props){
    let g=graph(props);let state=batch_state(props,g);
    let u=local(ctx,props.run);let title=value(props,"title",ctx.t_or("agent_ledger.title","Ledger"));
    let keys=project_keys(g);let baseline=ctx.get_state("initial");
    let initial=if baseline!=()&&baseline.run==props.run{baseline.ids}else{keys};
    effect("remember_initial",#{run:props.run,ids:keys},Fn("init"),Fn("idle"));
    effect("prune_pointer",#{run:props.run,ids:keys},Fn("prune"),Fn("idle"));
    let point=if u.hit!=""{u.hit}else if u.focus!=""{u.focus}else{value(props,"point_at","")};
    if !(point in g.entries)||point in state.owner||!reversible(g.entries[point])||props.busy{point="";}
    let marks=#{};
    if point!=""{
        let preview=undo_scope(point,g,state);
        for id in preview.changes{if id!=point{marks[id]="also";}}
        for id in preview.locked{marks[id]="kept";}
    }
    let shell=element_ref("ledger");let narrow=ctx.element_width_class(shell,[480])!=1;
    let animate=ctx.motion_preference()=="normal";
    let measure=if animate{ctx.element_size("row-measure")}else{()};
    let cfg=#{run_key:utf8_byte_count(props.run).to_string()+":"+props.run+":",point:point,marks:marks,ui:u,initial:initial,
        narrow:narrow,now:clock(ctx,props),animate:animate,measure:measure};
    let rows=[];let previous_mark=false;
    for i in 0..g.timeline.len{
        let r=g.timeline[i];rows.push(record(ctx,props,cfg,g,state,r,i,previous_mark));
        previous_mark=r.entry.id==point||r.entry.id in marks;
    }
    if rows.len==0{rows.push(caption(ctx,ctx.t_or("agent_ledger.empty","No changes yet"),"empty").with_key("empty"));}
    let body=[header(ctx,props,cfg,state,title),column(rows).with_key("entries").accessibility_role("list").accessibility_label(title)
        .with_style(ctx.component_style("panel",style().min_width(px(0)).padding(lg(1.5)).background(theme_color("surface_raised")).radius(lg(3)).clip()))];
    if opt(props,"notice")!=(){body.push(caption(ctx,props.notice,"notice").with_key("notice").accessibility_role("status"));}
    column(body).with_key(props.key).with_ref(shell).accessibility_role("group").accessibility_label(title)
        .audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).gap(lg(1))))
}
