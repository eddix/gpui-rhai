/* gpui-rhai
{"id":"patterns/tool_access_map","export":"ToolAccessMap","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/textarea","components/code_viewer","components/spinner","patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","typography.body","typography.caption","typography.code","tool_access.tray","tool_access.card","tool_access.well","tool_access.fill","tool_access.ink","tool_access.text","tool_access.muted","tool_access.rail","tool_access.used","tool_access.glint","tool_access.live","tool_access.live_fill","tool_access.success","tool_access.success_fill","tool_access.warning","tool_access.warning_fill","tool_access.danger","tool_access.danger_fill","tool_access.solid","tool_access.on_solid","typography.initials","typography.metadata"],"environment":[]}
*/
// Inspect a Host's authorized tool snapshot and recorded calls; never invokes a tool.
// Example: ToolAccessMap(#{key:"map",session:"build",agents:[#{id:"review",name:"Reviewer",status:"done",tools:[#{id:"diff",name:"Diff"}]}]})
import "components/textarea" as edit;
import "components/code_viewer" as code;
import "components/spinner" as spin;
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/tool_access_map","export":"ToolAccessMap",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/textarea","components/code_viewer","components/spinner","patterns/agent_tree"],capabilities:#{},assets:[],tokens:["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","typography.body","typography.caption","typography.code","tool_access.tray","tool_access.card","tool_access.well","tool_access.fill","tool_access.ink","tool_access.text","tool_access.muted","tool_access.rail","tool_access.used","tool_access.glint","tool_access.live","tool_access.live_fill","tool_access.success","tool_access.success_fill","tool_access.warning","tool_access.warning_fill","tool_access.danger","tool_access.danger_fill","tool_access.solid","tool_access.on_solid","typography.initials","typography.metadata"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable visual instance."},
        session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Host identity of the current run."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted snapshot revision used to scope editable input and replay."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Defaults to Tool access."},
        agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled visible agent. An absent ID falls back to the first agent."},
        default_agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial selection in local mode."},
        agents:#{schema:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable agent ID used to scope tool selection and replay proposals."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Agent name shown in the picker and access graph."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional model caption shown with the agent."},
            task:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Private description of the work assigned to this agent."},
            status:#{schema:#{type:"string",allowed:["queued","running","waiting","done","failed"]},required:true,sensitive:false,doc:"Host-reported agent status; controls presentation, not its authorization."},
            tools:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable tool identifier; matches the same tool in the optional catalog."},
                name:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Agent-specific display name, falling back to the catalog entry."},
                icon:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Short decorative text mark for the tool, not a file path or SVG."},
                description:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Agent-specific explanation of the tool, falling back to the catalog caption."},
                calls:#{schema:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
                    id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Call identity, unique within this agent and tool's call history."},
                    summary:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short private caption for the call list, separate from input or output."},
                    status:#{schema:#{type:"string",allowed:["running","done","failed"]},required:true,sensitive:false,doc:"Accepted state of this invocation, used for pending, output or failure presentation."},
                    input:#{schema:#{type:"ui_value"},required:true,sensitive:true,doc:"Supplied argument snapshot; shown as text or structured data and never executed by the component."},
                    output:#{schema:#{type:"optional",value:#{type:"ui_value"}},required:false,sensitive:true,doc:"Optional read-only result snapshot displayed in the call inspector."},
                    error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host error text for the failed result; not an SDK exception."},
                    at_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Run-relative start time in milliseconds, rendered as elapsed minutes and seconds."},
                    duration_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Elapsed call duration in milliseconds, supplied by the Host."},
                    replayed:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Marks an accepted call as a replay; setting it does not replay the tool."}
                }}},required:false,sensitive:true,doc:"Up to 24 calls, oldest first; IDs unique per tool."}
            }}},required:true,sensitive:true,doc:"Tools this agent can access; unlisted tools are drawn as blocked."}
        }}},required:true,sensitive:true,doc:"An ordered, bounded run snapshot."},
        tools:#{schema:#{type:"optional",value:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique tool ID shared with the corresponding per-agent entries."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Default user-facing tool name, including on a blocked tile."},
            description:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Default description unless the agent supplies one."},
            icon:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Default decorative text mark for a catalog tool."}
        }}}},required:false,sensitive:false,doc:"Optional complete catalog; otherwise ordered by first appearance."},
        replay_pending:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disable replay controls while the Host decides a proposal."},
        on_agent_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"The requested agent and current run revision."},
        on_replay:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"An explicit replay request, not tool execution."}
    },state:#{fields:#{
        ui:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:false},
        draft:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}
    }},events:#{
        agent_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            agent:#{schema:#{type:"string"},required:true,sensitive:false}
        }},doc:"Selection intent for the active run."},
        replay:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            agent:#{schema:#{type:"string"},required:true,sensitive:false},tool:#{schema:#{type:"string"},required:true,sensitive:false},
            call_id:#{schema:#{type:"string"},required:true,sensitive:false},format:#{schema:#{type:"string",allowed:["text","json"]},required:true,sensitive:false},
            input:#{schema:#{type:"string"},required:true,sensitive:true},edited:#{schema:#{type:"bool"},required:true,sensitive:false}
        }},doc:"Human-triggered request with the selected call ID and validated input. Authorization and execution remain with the Host."}
    },slots:#{},parts:["root","head","mark","title","stats","badge","agent_status","live","agents","tab","tab_active","tab_avatar","avatar","body","stage","wire","agent","person","model","task","layout","tool","blocked","blocked_content","tool_name","tool_info","tool_count","tile","panel","panel_head","panel_desc","back","calls","call","call_active","detail","section","code","editor","actions","button","primary","empty","invalid"],effects:["focus","selection"]},
    render:Fn("render_map")
});
fn ToolAccessMap(p){render_component("patterns/tool_access_map",p)}
fn get(m,k,d){if m!=()&&k in m&&m[k]!=(){m[k]}else{d}}
fn u(n){theme_length("space.unit").times(n)}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn tx(ctx,key,v){ctx.t_or("tool_access."+key,v)}
fn ink(part){let k="tool_access."+part;theme_color(k)}
fn ease(){cubic_bezier(0.32,0.72,0.24,1.0)}
fn fade(){motion_transition("opacity",0.0,1.0,#{duration_ms:220,easing:ease(),intent:"decorative"})}
fn hint(){style().typography("caption").text_color(ink("muted")).min_width(px(0))}
fn line(){hint().whitespace_nowrap().text_ellipsis()}
fn visible(ctx,n,k,s){n.with_style(ctx.component_style(k,s))}
fn label(ctx,k,v,s){visible(ctx,text(v),k,s)}
fn by_id(list,id){for i in 0..list.len{if list[i].id==id{return i;}}-1}
fn has_ids(list){let seen=#{};for v in list{if !ids::valid(v.id)||v.id in seen{return false;}seen[v.id]=true;}true}
fn catalog(p){
    if !ids::valid(p.session)||!has_ids(p.agents){throw "ToolAccessMap requires unique stable session and agent IDs";}
    let ordered=get(p,"tools",[]);let explicit=get(p,"tools",())!=();
    if !has_ids(ordered){throw "Tool catalog IDs must be unique";}
    let known=#{};for t in ordered{known[t.id]=true;}
    for a in p.agents{
        if a.name==""||!has_ids(a.tools){throw "Each agent needs a name and unique accessible tool IDs";}
        for t in a.tools{
            if !has_ids(get(t,"calls",[])){throw "Call IDs must be unique within a tool";}
            if !(t.id in known){
                if explicit{throw "Accessible tool is absent from the supplied tool catalog";}
                if ordered.len>=16{throw "Tool catalog exceeds 16 entries";}
                ordered.push(#{id:t.id,name:get(t,"name",t.id),description:get(t,"description",""),icon:get(t,"icon","")});
                known[t.id]=true;
            }
        }
    }
    for t in ordered{if t.name==""{throw "Each tool needs a label";}}
    ordered
}
fn state(ctx,session){
    let s=ctx.get_state("ui");
    if s!=()&&s.session==session{s}else{#{session:session,local:"",owner:"",tool:"",call_id:"",focus:""}}
}
fn change_agent(t,id,to,ctx,e){
    if !t.controlled&&id!=t.agent{
        let s=state(ctx,t.session);
        if s.local!=id{s.local=id;s.owner="";s.tool="";s.call_id="";ctx.set_state("draft",());}
        ctx.set_state("ui",s);
    }
    if to!=""{ctx.focus(to);ctx.scroll_into_view(to);}
    if id!=t.agent&&t.notify{ctx.emit("agent_change",#{session:t.session,revision:t.revision,agent:id});}
    handled()
}
fn decision(ctx,p,s,all){
    let a=by_id(p.agents,get(p,"agent",if s.local==""{get(p,"default_agent","")}else{s.local}));
    if a<0&&p.agents.len>0{a=0;}
    let tools=[];let reach=0;let calls=0;let run=0;
    let chosen=if a<0{()}else{p.agents[a]};
    if chosen!=(){
        for def in all{
            let index=by_id(chosen.tools,def.id);
            let t=if index<0{()}else{chosen.tools[index]};
            let entries=get(t,"calls",[]);
            let status=if index<0{"blocked"}else if entries.len==0{"idle"}else if entries.some(|c| c.status=="running"){"running"}
                else if entries[entries.len-1].status=="failed"{"failed"}else{"used"};
            if index>=0{reach+=1;} if status=="running"{run+=1;}calls+=entries.len;
            tools.push(#{id:def.id,name:get(t,"name",def.name),description:get(t,"description",get(def,"description","")),icon:get(t,"icon",get(def,"icon","")),state:status,calls:entries});
        }
    }
    let open=if chosen!=()&&s.owner==chosen.id{by_id(tools,s.tool)}else{-1};
    if open>=0&&tools[open].state=="blocked"{open=-1;}
    #{chosen:chosen,tools:tools,reach:reach,calls:calls,run:run,open:open,index:a}
}
fn open(t,id,ctx,e){let s=state(ctx,t.session);s.owner=t.agent;s.tool=id;s.call_id="";s.focus="back";ctx.set_state("draft",());ctx.set_state("ui",s);handled()}
fn back(t,id,ctx,e){let s=state(ctx,t.session);s.tool="";s.call_id="";s.focus="tool:"+id;ctx.set_state("draft",());ctx.set_state("ui",s);handled()}
fn settle_focus(ctx,info){
    let s=state(ctx,info.session);
    if s.focus==info.key{s.focus="";ctx.set_state("ui",s);ctx.focus(info.key);}
}
fn prune_selection(ctx,info){
    let s=state(ctx,info.session);let dirty=false;
    if s.tool!=""&&info.open==""{s.owner="";s.tool="";s.call_id="";s.focus=info.fallback;dirty=true;}
    else if s.call_id!=""&&!info.has_call{s.call_id="";dirty=true;}
    if dirty{ctx.set_state("draft",());ctx.set_state("ui",s);}
}
fn pick_call(session,id,refkey,ctx,e){
    let s=state(ctx,session);s.call_id=id;
    ctx.set_state("draft",());ctx.set_state("ui",s);
    if refkey!=""{ctx.focus(refkey);ctx.scroll_into_view(refkey);}
    handled()
}
fn same_ticket(a,b){
    a!=()&&b!=()&&a.session==b.session&&a.revision==b.revision&&a.agent==b.agent&&a.tool==b.tool&&a.call_id==b.call_id
}
fn set_draft(ticket,value,ctx,e){ctx.set_state("draft",#{ticket:ticket,input:value});handled()}
fn change_input(ticket,ctx,value){ctx.set_state("draft",#{ticket:ticket,input:value});}
fn remove_draft(session,focus,ctx,e){
    ctx.set_state("draft",());let s=state(ctx,session);s.focus=focus;ctx.set_state("ui",s);handled()
}
fn noop(ctx,data){}
fn propose(t,kind,original,ctx){
    let d=ctx.get_state("draft");let edited=d!=()&&same_ticket(d.ticket,t);
    let value=if edited{d.input}else{original};
    if !validate_replay_input(value,kind){return;}
    let payload=#{session:t.session,revision:t.revision,agent:t.agent,tool:t.tool,call_id:t.call_id,format:kind,input:value,edited:edited};
    ctx.emit("replay",payload);
    ctx.set_state("draft",());
}
fn submit(t,kind,ctx,value){
    ctx.set_state("draft",#{ticket:t,input:value});propose(t,kind,value,ctx);
}
fn replay(t,kind,original,ctx,e){propose(t,kind,original,ctx);handled()}
fn delta(ctx,n){
    if n==(){return "";}if n<1000{return ctx.format_number(n)+" ms";}
    ctx.format_number(n.to_float()/1000.0,#{max_fraction_digits:1})+" s"
}
fn time(n){
    if n==(){return "";}let s=n/1000;let tail=(s%60).to_string();
    (s/60).to_string()+":"+if tail.len==1{"0"+tail}else{tail}
}
fn glyph(ctx,w){
    let d=switch w{
        "map"=>"M3 3v4m0 2v4m10-10v4m0 2v4M3 5h2l2 3h2l2-3h2M3 11h2l2-3h2l2 3h2",
        "back"=>"M12 8H4m4-4L4 8l4 4","forward"=>"M4 8h8M8 4l4 4-4 4","replay"=>"M3 7a5 5 0 1 1 1.8 5.1 M3 3v4h4",
        "done"=>"M3 8l3 3 7-7", "failed"=>"M8 2a6 6 0 1 1 0 12 6 6 0 0 1 0-12ZM8 5v3.2m0 2.2h.01",
        _=>""
    };
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
        .accessibility_hidden(true).with_style(style().width(u(3.5)).height(u(3.5)).flex_shrink(false))
}
fn initials(name,max_letters){
    let out="";let n=0;for w in name.split(" "){if w!=""&&n<max_letters{out+=grapheme_slice(w,0,1).to_string().to_upper();n+=1;}}out
}
fn tag(ctx,title,tone,bits,part){
    bits.push(text(title));
    visible(ctx,row(bits).with_key(part).accessibility_role("status").accessibility_hidden(true),part,
        hint().items_center().gap(sp(1)).padding_x(sp(1.5)).flex_shrink(false)
        .min_height(u(4.5)).font_weight(600).radius(theme_radius("lg")).background(ink(if tone=="neutral"{"fill"}else{tone+"_fill"})).text_color(ink(if tone=="neutral"{"text"}else{tone})))
}
fn spinner(key){spin::Spinner(#{key:key,label:"Running",size:"xs"}).accessibility_hidden(true)}
fn agent_status(ctx,s){
    let name=switch s{"running"=>tx(ctx,"running","Running"),"waiting"=>tx(ctx,"waiting","Needs you"),"done"=>tx(ctx,"done","Done"),
        "failed"=>tx(ctx,"failed","Failed"),_=>tx(ctx,"queued","Queued")};
    let mood=switch s{"running"=>"live","waiting"=>"warning","done"=>"success","failed"=>"danger",_=>"neutral"};
    tag(ctx,name,mood,if s=="running"{[spinner("agent-spinner")]}else{[]},"agent_status")
}
fn activate(n,cb){n.on_click(cb).on_key_value("plain+enter",cb,()).on_key_value("plain+space",cb,())}
fn focus_style(){style().focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))}
fn header(ctx,p,m){
    let total=ctx.t_or("tool_access.total","{allowed}/{tool_count} tools · {call_count} calls",
        #{allowed:ctx.format_number(m.data.reach),tool_count:ctx.format_number(m.data.tools.len),call_count:ctx.format_number(m.data.calls)});
    let summary=if m.narrow{ctx.t_or("tool_access.compact_total","{n} calls",#{n:ctx.format_number(m.data.calls)})}else{total};
    let title=get(p,"title",tx(ctx,"title","Tool access"));let name=label(ctx,"title",title,line().typography("body").font_weight(600).text_color(theme_color("text_primary")))
        .with_key("title").accessibility_role("heading").accessibility_level(2);
    let bits=[visible(ctx,glyph(ctx,"map"),"mark",style().width(u(4)).height(u(4)).text_color(ink("live")).flex_shrink(false)),name];
    bits.push(box([]).with_style(style().flex_grow()));
    if m.data.run>0{bits.push(visible(ctx,row([tag(ctx,ctx.format_number(m.data.run)+(if m.narrow{""}else{" "+tx(ctx,"running_lower","running")}),"live",[spinner("summary-spinner")],"badge")]).with_key("running")
        .accessibility_role("status").accessibility_label(ctx.format_number(m.data.run)+" "+tx(ctx,"running_lower","running")),"live",style().flex_shrink(false)).enter_motion(fade()));}
    bits.push(label(ctx,"stats",summary,line().font_feature("tnum",1)).accessibility_role("status").accessibility_label(total));
    visible(ctx,row(bits).with_key("head"),"head",style().min_width(px(0)).items_center().gap(sp(2)).padding(sp(2.5)))
}
fn tab_ref(id){"agent:"+id}
fn tab_key(ctx,m,id,focus){Fn("change_agent").curry(m.ticket,id,if focus{tab_ref(id)}else{""})}
fn switcher(ctx,p,m){
    let entries=[];let qty=p.agents.len;let rtl=ctx.text_direction()=="rtl";
    let selectable=!m.ticket.controlled||m.ticket.notify;
    for i in 0..qty{
        let a=p.agents[i];let key=tab_ref(a.id);let on=i==m.data.index;let ref=element_ref(key);
        let face=visible(ctx,row([text(initials(a.name,2))]).with_key("monogram").accessibility_hidden(true),"tab_avatar",
            hint().typography("initials").text_color(ink("text")).width(u(5)).height(u(5)).flex_shrink(false)
            .items_center().justify_center().background(ink("card")).radius(theme_radius("lg")));
        let parts=if on{["tab","tab_active"]}else{["tab"]};let next=(i+1)%qty;let prev=(i+qty-1)%qty;
        let name=text(a.name).with_key("name").with_style(style().min_width(px(0)).max_width(u(40)).whitespace_nowrap().text_ellipsis());
        let nav=row([face,name]).with_key(key).with_ref(ref).accessibility_role("tab").accessibility_label(a.name)
            .accessibility_selected(on).disabled(!selectable).tab_stop(on&&selectable)
            .with_style(ctx.component_style(parts,focus_style().items_center().gap(sp(1.5)).max_width(u(54)).flex_shrink(false).min_height(u(7.5)).padding_start(sp(1)).padding_end(sp(3))
                .radius(theme_radius("lg")).typography("caption").font_weight(500).background(ink(if on{"card"}else{"fill"}))
                .text_color(ink(if on{"ink"}else{"text"})).hover(style().text_color(ink("ink"))).cursor_pointer()))
            .paint_transition(150,"ease_out");
        if selectable{
            nav=activate(nav,tab_key(ctx,m,a.id,false))
                .on_key_value("plain+left",tab_key(ctx,m,p.agents[if rtl{next}else{prev}].id,true),())
                .on_key_value("plain+right",tab_key(ctx,m,p.agents[if rtl{prev}else{next}].id,true),())
                .on_key_value("plain+home",tab_key(ctx,m,p.agents[0].id,true),()).on_key_value("plain+end",tab_key(ctx,m,p.agents[qty-1].id,true),());
        }
        entries.push(nav);
    }
    visible(ctx,row(entries).with_key("agents").accessibility_role("tablist").accessibility_label(tx(ctx,"agents","Agents")).scrollbars("hidden","hidden"),
        "agents",style().overflow_x_scroll().min_width(px(0)).gap(sp(2)).padding_x(sp(1)).padding_y(sp(1)))
}
fn card(ctx,m){
    let a=m.data.chosen;let status=agent_status(ctx,a.status);
    let face=visible(ctx,row([text(initials(a.name,2))]).with_key("avatar").accessibility_hidden(true),"avatar",hint().typography("initials").text_color(ink("text")).width(u(5.5)).height(u(5.5))
        .items_center().justify_center().flex_shrink(false).background(ink("card")).radius(theme_radius("sm")));
    let who=[label(ctx,"person",a.name,line().text_color(theme_color("text_primary")).font_weight(500)).with_key("person")];
    if get(a,"model","")!=""{who.push(label(ctx,"model",a.model,line().typography("metadata")).with_key("model"));}
    let head=row([face,column(who).with_style(style().flex_grow().min_width(px(0))),status]).with_style(style().items_center().gap(sp(1.5)).min_width(px(0)));
    let parts=[head];
    if get(a,"task","")!=""{parts.push(label(ctx,"task",a.task,hint().text_color(ink("text")).line_clamp(2)).with_key("task"));}
    let details=column(parts).with_key("person:"+a.id).with_style(style().min_width(px(0)).gap(sp(1.5))).enter_motion(fade());
    visible(ctx,column([details]).with_key("agent-card").with_ref(m.agent_ref).accessibility_role("group").accessibility_label(a.name),
        "agent",style().width(if m.narrow{relative(1.0)}else{u(50)}).max_width(relative(1.0)).gap(sp(2)).flex_shrink(false)
            .min_width(px(0)).padding(sp(2.5)).background(ink("card")).radius(theme_radius("lg")))
}
fn tool_info(ctx,t){
    switch t.state{
        "blocked"=>tx(ctx,"no_access","No access"),"idle"=>tx(ctx,"unused","Not used yet"),"running"=>tx(ctx,"running","Running"),"failed"=>tx(ctx,"failed","Failed"),
        _=>tx(ctx,"last","Last call")+" "+get_duration(ctx,t.calls[t.calls.len-1])
    }
}
fn get_duration(ctx,c){let n=get(c,"duration_ms",());if n==(){tx(ctx,"done","Done")}else{delta(ctx,n)}}
fn tile(ctx,t){
    let words=get(t,"icon","");let c=if words!=""{words}else{initials(t.name,1)};
    let style=hint().text_color(ink("text")).font_weight(600).width(u(4)).height(u(4)).items_center()
        .justify_center().flex_shrink(false).background(ink("card")).radius(theme_radius("sm"));
    if t.state=="blocked"{style=style.foreground_grayscale(1);}
    visible(ctx,row([text(c)]).with_key("tile").accessibility_hidden(true),"tile",style)
}
fn tool(ctx,m,t){
    let off=t.state=="blocked";let info=tool_info(ctx,t);let key="tool:"+t.id;
    let top=[tile(ctx,t),label(ctx,"tool_name",t.name,line().text_color(ink("ink")).font_weight(500).flex_grow()).with_key("name")];
    if t.calls.len>0{top.push(label(ctx,"tool_count",ctx.format_number(t.calls.len)+"×",line().font_feature("tnum",1).flex_shrink(false)).with_key("count"));}
    let detail=if t.state=="running"||t.state=="failed"{tag(ctx,info,if t.state=="running"{"live"}else{"danger"},[],"badge")}
        else{label(ctx,"tool_info",info,line().font_feature("tnum",1)).with_key("info")};
    let first=style().items_center().gap(sp(1.5)).min_width(px(0));
    let second=style().min_width(px(0)).padding_start(sp(5.5));
    if off{first=ctx.component_style("blocked_content",first.isolated_opacity(true).opacity(0.45));
        second=ctx.component_style("blocked_content",second.isolated_opacity(true).opacity(0.45));}
    let n=column([row(top).with_style(first),row([detail]).with_style(second)])
        .with_key(key).with_ref(m.tool_refs[t.id]).accessibility_role("button").accessibility_label(t.name).accessibility_description(info)
        .disabled(off).tab_stop(!off);
    if !off{n=activate(n,Fn("open").curry(m.ticket,t.id));}
    let base=focus_style().min_width(px(0)).min_height(u(11.5)).width(relative(1.0)).gap(sp(0.5)).justify_center()
        .padding_x(sp(2)).padding_y(sp(0.75)).radius(theme_radius("sm")).background(ink(if off{"well"}else{"card"}));
    if !off{base=base.cursor_pointer().hover(style().background(ink("fill")));}
    let n=visible(ctx,n,if off{["tool","blocked"]}else{["tool"]},base).paint_transition(250,"ease_out");
    if !off{n=n.shared_layout(m.scope,key).layout_motion(340,ease());}
    n
}
fn bounds(ctx,k){let b=ctx.element_bounds(k);if b==(){()}else{b.layout}}
fn elbow(x,y,tx,ty){
    let bx=(x+tx)*0.5;let sx=if tx<x{-1.0}else{1.0};let sy=if ty<y{-1.0}else{1.0};
    let radius=min(8.0,min(abs(y-ty)*0.5,abs(x-bx)));
    if radius<0.5{return [path_move(x,y),path_line(tx,ty)];}
    [path_move(x,y),path_line(bx-sx*radius,y),path_quadratic(bx,y+sy*radius,bx,y),path_line(bx,ty-sy*radius),
        path_quadratic(bx+sx*radius,ty,bx,ty),path_line(tx,ty)]
}
fn wires(ctx,m){
    let stage=bounds(ctx,m.stage);let agent=bounds(ctx,m.agent_ref);
    if stage==()||agent==()||stage.width<1.0||stage.height<1.0{return [];}
    let dashed=[];let paths=[];let lights=[];let ends=#{};
    for t in m.data.tools{
        let b=bounds(ctx,m.tool_refs[t.id]);if b==(){continue;}
        let left=b.x<agent.x;
        if left&&b.x+b.width>agent.x||!left&&b.x<agent.x+agent.width{continue;}
        let ax=if left{agent.x}else{agent.x+agent.width};
        let sx=ax-stage.x;let sy=agent.y-stage.y+agent.height*0.5;
        let ex=(if left{b.x+b.width}else{b.x})-stage.x;let ey=b.y-stage.y+b.height*0.5;
        let segs=elbow(sx,sy,ex,ey);
        let ghost=t.state=="blocked";let path="wire:"+t.id;let side=if left{"left"}else{"right"};
        let line=canvas_stroke_path(path,if ghost{fallback_dash_route(segs,0.012,0.024)}else{segs},1.5,
            ink(if ghost{"rail"}else{"used"})).stroke_cap("round");
        if ghost{dashed.push(line);}else{paths.push(line);}
        if !(side in ends)||!ghost{ends[side]=#{x:sx,y:sy,on:!ghost};}
        if t.state=="running"&&ctx.motion_preference()=="normal"{
            let transparent=canvas_stroke_path("pulse",segs,1.0,rgba(0x00000000));
            let scene=canvas_scene([transparent]).view_box(0,0,stage.width,stage.height).path_pulse("pulse",2.5,ink("glint"),false,false);
            lights.push(canvas(scene).with_key("pulse:"+m.ticket.agent+":"+t.id).accessibility_hidden(true).with_style(wire_layer(ctx))
                .motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:1300,iterations:"infinite",intent:"decorative"})));
        }
    }
    for side in ends.keys(){let q=ends[side];dashed.push(canvas_circle("joint:"+side,q.x,q.y,3.75,ink(if q.on{"used"}else{"rail"})));
        dashed.push(canvas_circle("joint-fill:"+side,q.x,q.y,2.25,ink("card")));}
    if dashed.len==0{return [];}
    let output=[canvas(canvas_scene(dashed).view_box(0,0,stage.width,stage.height)).with_key("wires").accessibility_hidden(true).with_style(wire_layer(ctx))];
    if paths.len>0{
        output.push(canvas(canvas_scene(paths).view_box(0,0,stage.width,stage.height)).with_key("granted-wires:"+m.ticket.agent)
            .accessibility_hidden(true).with_style(wire_layer(ctx)).motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:460,easing:ease(),intent:"decorative"})));
    }
    output+lights
}
fn wire_layer(ctx){ctx.component_style("wire",style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)))}
fn board(ctx,m){
    let children=[];
    if m.data.chosen==(){children.push(empty(ctx,tx(ctx,"no_agents","No agents yet")));}
    else{
        let agents=card(ctx,m);let tools=m.data.tools;
        if m.narrow||tools.len==0{
            let items=[];for t in tools{items.push(tool(ctx,m,t));}
            let content=if items.len==0{empty(ctx,tx(ctx,"no_tools","No tools"))}
                else{column(items).with_key("grid").with_style(style().auto_grid_columns(u(29.5),u(32.5),u(70)).gap(sp(2)).min_width(px(0)))};
            children.push(column([agents,content]).with_key("layout").with_style(style().gap(sp(3)).min_width(px(0))));
        }else{
            let left=[];let right=[];let cut=(tools.len+1)/2;
            for i in 0..tools.len{if i<cut{left.push(tool(ctx,m,tools[i]));}else{right.push(tool(ctx,m,tools[i]));}}
            let side=style().width(u(32.5)).gap(sp(3)).flex_shrink(false).min_width(px(0));
            children+=wires(ctx,m);
            children.push(row([column(left).with_key("start").with_style(side),agents,column(right).with_key("end").with_style(side)])
                .with_key("layout").with_style(ctx.component_style("layout",style().min_width(px(0)).min_height(u(37)).items_center().justify_between().padding_y(sp(3.5)))));
        }
    }
    visible(ctx,column(children).with_key("map-stage").with_ref(m.stage).accessibility_role("tabpanel").accessibility_label(tx(ctx,"board","Tool access map")),
        "stage",style().relative().min_width(px(0)))
}
fn empty(ctx,v){label(ctx,"empty",v,hint().text_center().padding(sp(6)))}
fn call_ref(id){"call:"+id}
fn navigation(ctx,m,t,shown){
    let list=[];let entries=t.calls;
    for i in 0..entries.len{
        let at=entries.len-1-i;let c=entries[at];let key=call_ref(c.id);let active=shown==at;
        let title=get(c,"summary",c.id);let mark=if c.status=="running"{spinner("call-spinner")}else{glyph(ctx,c.status)};
        let parts=[mark,label(ctx,"tool_name",title,line().typography("code").text_color(theme_color("text_primary")).flex_grow())];
        if get(c,"replayed",false){parts.push(tag(ctx,tx(ctx,"replayed","Replay"),"neutral",[],"badge"));}
        parts.push(label(ctx,"stats",if c.status=="running"{"…"}else{get_duration(ctx,c)},line().font_feature("tnum",1).flex_shrink(false)));
        let base=focus_style().min_width(px(0)).min_height(u(7.5)).items_center().padding_x(sp(1.5)).gap(sp(1.5))
            .radius(theme_radius("sm")).background(if active{ink("fill")}else{rgba(0x00000000)}).hover(style().background(ink("well"))).cursor_pointer();
        let chosen=activate(row(parts).with_key(key).with_ref(element_ref(key)).accessibility_role("option").accessibility_label(title).accessibility_selected(active)
            .accessibility_description(c.status).tab_stop(active).with_style(ctx.component_style(if active{["call","call_active"]}else{["call"]},base)),Fn("pick_call").curry(m.ticket.session,c.id,""));
        if at>0{let target=entries[at-1].id;chosen=chosen.on_key_value("plain+down",Fn("pick_call").curry(m.ticket.session,target,call_ref(target)),());}
        if at+1<entries.len{let target=entries[at+1].id;chosen=chosen.on_key_value("plain+up",Fn("pick_call").curry(m.ticket.session,target,call_ref(target)),());}
        chosen=chosen.on_key_value("plain+home",Fn("pick_call").curry(m.ticket.session,entries[entries.len-1].id,call_ref(entries[entries.len-1].id)),())
            .on_key_value("plain+end",Fn("pick_call").curry(m.ticket.session,entries[0].id,call_ref(entries[0].id)),());
        list.push(chosen);
    }
    visible(ctx,column(list).with_key("calls").accessibility_role("listbox").accessibility_label(tx(ctx,"calls","Calls")).scrollbars("hidden","auto"),
        "calls",style().min_width(px(0)).width(if m.narrow{relative(1.0)}else{u(56)}).max_height(u(if m.narrow{38}else{102})).overflow_y_scroll().gap_y(sp(0.5)).flex_shrink(false))
}
fn control(ctx,key,name,bits,cb,enabled,primary){
    let base=focus_style().min_height(u(7.5)).padding_x(sp(3)).gap(sp(1.5)).items_center().justify_center().flex_shrink(false)
        .typography("caption").font_weight(600).radius(theme_radius("lg")).background(ink(if primary{"solid"}else{"card"}))
        .text_color(if primary{ink("on_solid")}else{theme_color("text_primary")}).cursor_pointer()
        .hover(if primary{style().opacity(0.86)}else{style().background(ink("well"))}).disabled(style().opacity(0.4));
    let n=row(bits).with_key(key).accessibility_role("button").accessibility_label(name).disabled(!enabled).tab_stop(enabled)
        .with_style(ctx.component_style(if primary{["button","primary"]}else{["button"]},base));
    if enabled{activate(n,cb)}else{n}
}
fn section(ctx,key,name,meta,content){
    let title=row([label(ctx,"section",name.to_upper(),line().font_weight(500).flex_grow()),label(ctx,"stats",meta,line().font_feature("tnum",1))])
        .with_style(style().gap(sp(2)).min_width(px(0)).items_center());
    column([title,content]).with_key(key).with_style(style().gap_y(sp(0.5)).min_width(px(0)))
}
fn doc_view(ctx,key,name,v,is_error){
    let doc=diagnostic_document(v);let src=if doc.omitted{tx(ctx,"too_long","Content is too large to display")}else{doc.text};
    let fg=if is_error{ink("danger")}else{theme_color("text_primary")};
    let surface=code::CodeViewer(#{key:key,label:name,source:src,language:"plain",show_gutter:false,show_line_numbers:false,
        height:"content",max_height:140,wrap:"viewport",scrollbar:"auto",
        part_styles:#{text:style().text_color(fg)}}).with_style(style().border(px(0)).background(rgba(0x00000000)).shadows([]));
    visible(ctx,column([surface]).with_key(key+"-shell"),"code",style().min_width(px(0)).min_height(u(8.5)).background(ink("well"))
        .padding(sp(2)).radius(theme_radius("sm")).overflow_hidden().text_color(fg))
}
fn inspector(ctx,p,m,t,c){
    let tkt=#{session:m.ticket.session,revision:m.ticket.revision,agent:m.ticket.agent,tool:t.id,call_id:c.id};
    let old=ctx.get_state("draft");let editing=old!=()&&same_ticket(tkt,old.ticket);
    let replayable=get(p,"on_replay",())!=()&&c.status!="running";let allow=replayable&&!p.replay_pending;
    let doc=diagnostic_document(c.input);let kind=if type_of(c.input)=="string"{"text"}else{"json"};
    let original=if kind=="text"{c.input}else{doc.text};let valid=!doc.omitted&&validate_replay_input(if editing{old.input}else{original},kind);
    let input_view=if editing&&replayable{
        let flags=edit::Textarea(#{key:"input-editor",label:tx(ctx,"replay_input","Input to replay with"),value:old.input,autofocus:true,min_rows:4,max_rows:8,
            max_length:8192,error:!valid,on_change:Fn("change_input").curry(tkt),submit_key:"mod_enter",
            on_submit:if allow{Fn("submit").curry(tkt,kind)}else{()},
            part_styles:#{root:ctx.component_style("editor",style().min_width(px(0)))}})
            .on_key_value("plain+escape",Fn("remove_draft").curry(p.session,"edit-input"),());
        flags
    }else{doc_view(ctx,"input","Input",c.input,false)};
    let failed=c.status=="failed";let name=if failed{tx(ctx,"error","Error")}else{tx(ctx,"output","Output")};
    let value=if c.status=="running"{tx(ctx,"waiting_result","Waiting for the result")}
        else if failed{get(c,"error",get(c,"output",tx(ctx,"failed","Failed")))}else{get(c,"output",tx(ctx,"no_output","No output"))};
    let lines=[section(ctx,"input-section",tx(ctx,"input","Input"),time(get(c,"at_ms",())),input_view),
        section(ctx,"output-section",name,delta(ctx,get(c,"duration_ms",())),doc_view(ctx,"output",name,value,failed))];
    if get(p,"on_replay",())!=(){
        let buttons=[];
        if editing&&!valid{buttons.push(label(ctx,"invalid",tx(ctx,"invalid","Invalid or oversized input"),hint().text_color(ink("danger")).flex_grow()));}
        if editing{
            let title=tx(ctx,"cancel","Cancel");buttons.push(control(ctx,"cancel",title,[text(title)],Fn("remove_draft").curry(p.session,"edit-input"),true,false));
        }else{
            let title=tx(ctx,"edit","Edit input");buttons.push(control(ctx,"edit",title,[text(title)],Fn("set_draft").curry(tkt,if doc.omitted{""}else{original}),allow&&valid,false)
                .with_ref(element_ref("edit-input")));
        }
        let title=if editing{tx(ctx,"run","Run")}else{tx(ctx,"replay","Replay")};
        buttons.push(control(ctx,"replay",title,[glyph(ctx,"replay"),text(title)],Fn("replay").curry(tkt,kind,if valid{original}else{""}),
            allow&&valid,true));
        lines.push(visible(ctx,row(buttons).with_key("actions"),"actions",style().justify_end().items_center().gap(sp(2)).flex_wrap().min_width(px(0))));
    }
    visible(ctx,column(lines).with_key("detail:"+c.id),"detail",style().min_width(px(0)).flex_grow().gap(sp(2.5)))
}
fn call_panel(ctx,p,m,t,shown){
    let return_to=Fn("back").curry(m.ticket,t.id);let direction=if ctx.text_direction()=="rtl"{"forward"}else{"back"};
    let title=tx(ctx,"back","Back to the map");
    let back_button=visible(ctx,activate(row([glyph(ctx,direction)]).with_key("back").with_ref(element_ref("back")).accessibility_role("button")
        .accessibility_label(title).tab_stop(true),return_to),"back",focus_style().items_center().justify_center().width(u(6)).height(u(6)).flex_shrink(false)
        .radius(theme_radius("sm")).text_color(ink("muted")).hover(style().background(ink("fill"))).cursor_pointer());
    let count=ctx.t_or("tool_access.call_count","{n} calls",#{n:ctx.format_number(t.calls.len)});
    let bits=[back_button,tile(ctx,t),label(ctx,"tool_name",t.name,line().font_weight(600).text_color(theme_color("text_primary"))),
        label(ctx,"panel_desc",t.description,line().flex_grow()),label(ctx,"stats",count,line().flex_shrink(false))];
    let head=visible(ctx,row(bits).with_key("header"),"panel_head",style().gap(sp(1.5)).items_center().min_width(px(0)).padding_bottom(sp(2)));
    let content=if shown<0{empty(ctx,tx(ctx,"no_calls","No calls yet"))}else{
        let list=navigation(ctx,m,t,shown);let detail=inspector(ctx,p,m,t,t.calls[shown]);
        (if m.narrow{column([list,detail])}else{row([list,detail])}).with_style(style().min_width(px(0)).items_start().gap(sp(3)))
    };
    let panel=column([head,content]).with_key("panel").accessibility_role("region")
        .accessibility_label(ctx.t_or("tool_access.panel","Calls to {tool}",#{tool:t.name}))
        .on_key_value("plain+escape",return_to,());
    visible(ctx,panel,"panel",style().min_width(px(0)).min_height(u(if m.narrow{68}else{80})).padding(sp(3)).background(ink("card"))
        .radius(theme_radius("lg"))).shared_layout(m.scope,"tool:"+t.id).layout_motion(340,ease()).enter_motion(fade())
}
fn render_map(ctx,p){
    let s=state(ctx,p.session);let data=decision(ctx,p,s,catalog(p));
    let shell=element_ref("shell");let slot=element_ref("content-width");let stage=if data.open<0{element_ref("map-stage")}else{()};
    let layout=ctx.element_width_class(slot,[540]);let narrow=layout==0;
    let active=if data.chosen==(){""}else{data.chosen.id};
    let ticket=#{session:p.session,revision:p.revision,agent:active,controlled:get(p,"agent",())!=(),notify:get(p,"on_agent_change",())!=()};
    let agent_ref=if data.open<0&&active!=""{element_ref("agent-card")}else{()};
    let tool_refs=#{};if data.open<0{for t in data.tools{tool_refs[t.id]=element_ref("tool:"+t.id);}}
    let m=#{data:data,ticket:ticket,narrow:narrow,stage:stage,scope:shell.semantic_id,agent_ref:agent_ref,tool_refs:tool_refs};
    let body=[];if p.agents.len>1{body.push(switcher(ctx,p,m));}
    let opened="";let has_call=false;let fallback="";
    if data.open<0{
        body.push(content_slot(board(ctx,m),slot));
        if data.chosen!=()&&p.agents.len>1{fallback=tab_ref(active);}
        else{for tool in data.tools{if fallback==""&&tool.state!="blocked"{fallback="tool:"+tool.id;}}}
    }else{
        let t=data.tools[data.open];let at=by_id(t.calls,s.call_id);opened=t.id;has_call=at>=0;
        if at<0{at=t.calls.len-1;}
        body.push(content_slot(call_panel(ctx,p,m,t,at),slot));
    }
    effect("selection",#{session:p.session,open:opened,has_call:has_call,fallback:fallback},Fn("prune_selection"),Fn("noop"));
    if s.focus!=""{effect("focus",#{session:p.session,key:s.focus},Fn("settle_focus"),Fn("noop"));}
    let surface=visible(ctx,column(body).with_key("body"),"body",style().gap(sp(2)).min_width(px(0)).padding(sp(1.5)).radius(theme_radius("lg")).background(ink("card")));
    visible(ctx,column([header(ctx,p,m),surface]).with_key(p.key).with_ref(shell),"root",style().min_width(px(0)).width(relative(1.0))
        .padding(sp(1)).radius(theme_radius("lg")).background(ink("tray"))).layout_size_motion(if data.open<0{"map"}else{"calls"},"height",340,ease())
}
fn content_slot(node,ref){column([node]).with_key("content").with_ref(ref).with_style(style().width(relative(1.0)).min_width(px(0)))}
