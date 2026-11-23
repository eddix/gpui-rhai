/* gpui-rhai
{"id":"patterns/agent_tree","export":"AgentTree","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/popover","components/spinner","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","agent_tree.link","agent_tree.link_muted","border","focus_ring","space.spacing_unit","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","space.unit"],"environment":[]}
*/
// A dispatch hierarchy with separate visual, focus and accepted run state.
// Example: AgentTree(#{key:"team",run:"preview",revision:0,now_ms:0,agents:[]})
import "components/button" as button;
import "components/popover" as popover;
import "components/spinner" as spinner;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_tree","export":"AgentTree",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/popover","components/spinner","patterns/approval_card"],capabilities:#{},
        tokens:["accent","agent_tree.link","agent_tree.link_muted","border","focus_ring","space.spacing_unit","surface_hover","surface_raised",
            "text.danger","text_muted","text_primary","typography.body","typography.caption","space.unit"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable component key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of the accepted run."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted business revision independent of clock ticks."},
        now_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"One shared clock for all active agents."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Heading for this dispatch tree."},
        root:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Agent at the root. If omitted, infer the unique non-supervisor root."},
        supervisor:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional peer watching the root, with no parent; not included in the root's subtree cost."},
        agents:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable business identity."},
            parent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"ID of the dispatcher; input order need not be topological."},
            name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Display name."},
            task:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Assigned task."},
            state:#{schema:#{type:"string",allowed:["queued","running","waiting","done","failed","stopped"]},required:true,sensitive:false,doc:"Accepted execution status."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Model caption."},
            avatar:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative avatar; defaults to initials."},
            tools:#{schema:#{type:"optional",value:#{type:"array",max_items:6,items:#{type:"object",allow_unknown:false,fields:#{
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Tool name."},
                icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative tool icon."},
            }}}},required:false,sensitive:false,doc:"Compact tool marks with named accessibility descriptions."},
            cost_micros:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Agent's own cost in millionths of the currency unit."},
            started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Clock reading when work started."},
            ended_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Fixed clock reading when work ended."},
            ask:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"What is waiting for a decision."},
            error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Failure evidence."},
            extra:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional details inside the card."},
        }}},required:true,sensitive:true,doc:"Every agent in the run, including folded nodes. Siblings respect input order."},
        top:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional run summary above the cards."},
        currency:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Currency label for costs; defaults to USD."},
        selected:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Host-selected agent. Its descendants are distinguished from unrelated branches."},
        collapsed:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"string"}}},required:false,sensitive:false,doc:"Controlled collapsed branch IDs. Omit to let the component own disclosure."},
        default_collapsed:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"string"}}},required:false,sensitive:false,doc:"Initial collapsed branch IDs when disclosure is local."},
        on_collapse:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the proposed collapsed snapshot."},
        on_select:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Requests selecting an agent, for example to show its log."},
        on_resolve:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives approval and failure-resolution proposals."},
        busy_agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Agent with an unresolved Host response; disables its resolution buttons."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Feedback about the latest proposal, also visible in an open review panel."},
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        collapse:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run identity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Observed revision."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Branch toggled."},
            collapsed:#{schema:#{type:"array",max_items:64,items:#{type:"string"}},required:true,sensitive:false,doc:"Proposed collapsed branches in tree order."},
        }},doc:"Apply this list to accept a controlled fold."},
        select:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run identity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Observed revision."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Requested agent."},
        }},doc:"Apply selected to update the highlighted branch."},
        resolve:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run identity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Observed revision."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Resolved agent."},
            action:#{schema:#{type:"string",allowed:["approve","reject","retry","dismiss"]},required:true,sensitive:false,doc:"Requested action; validate against the accepted agent state."},
        }},doc:"Does not change, retry or dismiss work until the Host accepts."},
    },slots:#{top:#{required:false,multiple:false,doc:"Optional summary above the dispatch tree."}},
        parts:["root","head","title","stats","count","running_count","total","summary_button","summary_waiting","summary_failed","stage","grid","cell","card","selected","related","outside","name","avatar",
            "task","model","tools","tool","status","status_running","status_waiting","status_failed","status_done","status_queued","status_stopped",
            "cost","time","fold","rail","supervisor","review","review_name","review_detail","review_actions",
            "review_viewport","empty","notice"],effects:["repair_focus"]},render:Fn("render_AgentTree")
});

fn AgentTree(props){render_component("patterns/agent_tree",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,d){common::value_or(m,k,d)}
fn valid(id){
    if id==""||utf8_byte_count(id)>96{return false;}
    for c in id.chars(){if c<' '||(c>=''&&c<=''){return false;}}
    true
}
// Shared roster normalization. AgentTree narrows this forest to one
// dispatcher and an optional observer; AgentFleet uses all roots.
fn roster(agents){
    let data=#{rows:#{},kids:#{},order:[],roots:[],family:#{},depth:#{},cost:#{},waiting:[],failed:[],running:0,total:0};
    for i in 0..agents.len{
        let a=agents[i];
        if !valid(a.id)||a.id in data.rows{throw "Agent IDs must be unique and bounded";}
        let start=opt(a,"started_ms");let end=opt(a,"ended_ms");
        if end!=()&&(start==()||end<start){throw "Agent end time must follow its start";}
        data.rows[a.id]=#{a:a,i:i,parent:value(a,"parent","")};data.kids[a.id]=[];
        let price=value(a,"cost_micros",0);data.cost[a.id]=price;data.total+=price;
        if a.state=="running"{data.running+=1;}
        if a.state=="waiting"{data.waiting.push(a.id);}
        if a.state=="failed"{data.failed.push(a.id);}
    }
    for a in agents{
        let parent=data.rows[a.id].parent;
        if parent==""{data.roots.push(a.id);}
        else{
            if !(parent in data.rows)||parent==a.id{throw "Every agent parent must identify another agent";}
            data.kids[parent].push(a.id);
        }
    }
    for root in data.roots{
        data.depth[root]=0;data.family[root]=root;let stack=[root];
        while stack.len>0{
            let id=stack.pop();data.order.push(id);
            let kids=data.kids[id];let n=kids.len;
            for rev in 0..n{
                let kid=kids[n-rev-1];stack.push(kid);
                data.depth[kid]=data.depth[id]+1;data.family[kid]=root;
            }
        }
    }
    if data.order.len!=agents.len{throw "Agent relationships must all reach the dispatch root";}
    for i in 0..data.order.len{
        let id=data.order[data.order.len-i-1];let parent=data.rows[id].parent;
        if parent!=""{data.cost[parent]+=data.cost[id];}
    }
    data
}
fn project(props){
    if !valid(props.run){throw "AgentTree run needs a stable ID";}
    let data=roster(props.agents);let roots=data.roots;
    data.root="";data.peer=value(props,"supervisor","");
    let declared=opt(props,"root");
    if props.agents.len==0{
        if declared!=()||data.peer!=""{throw "An empty tree cannot name a root or supervisor";}
        return data;
    }
    if data.peer!=""&&(!(data.peer in roots)||data.kids[data.peer].len>0){
        throw "The supervisor must be a separate, childless root";
    }
    let tree_roots=[];for id in roots{if id!=data.peer{tree_roots.push(id);}}
    if tree_roots.len!=1||(declared!=()&&declared!=tree_roots[0]){throw "AgentTree requires exactly one dispatch root";}
    data.root=tree_roots[0];
    if data.peer!=""{
        let order=[];for id in data.order{if id!=data.peer{order.push(id);}}
        data.order=order;
    }
    data
}
fn state(ctx,run){
    let s=ctx.get_state("ui");
    if s!=()&&s.run==run{return s;}
    #{run:run,cursor:"",within:false,collapsed:(),review:""}
}
fn ref_key(data,id){"card-"+data.rows[id].i.to_string()}
fn folding(props,ui,data){
    let controlled=opt(props,"collapsed");let input=if controlled!=(){controlled}else if ui.collapsed!=(){ui.collapsed}else{value(props,"default_collapsed",[])};
    let result=#{};for id in input{if id in data.rows&&data.kids[id].len>0{result[id]=true;}}
    result
}
fn plan(data,folded){
    let visible=[];let spans=#{};let starts=#{};let list=#{};let high=0;
    for id in data.order{
        let parent=data.rows[id].parent;
        if parent!=""&&(!(parent in list)||parent in folded){continue;}
        list[id]=true;visible.push(id);high=max(high,data.depth[id]);
    }
    for i in 0..visible.len{
        let id=visible[visible.len-i-1];
        let k=0;for child in data.kids[id]{if child in spans{k+=spans[child];}}
        spans[id]=max(1,k);
    }
    if data.root!=""{starts[data.root]=1;}
    for id in visible{
        let col=starts[id];for child in data.kids[id]{if child in list{starts[child]=col;col+=spans[child];}}
    }
    let leaf=if data.root==""{0}else{spans[data.root]};
    if data.peer!=""{
        visible.insert(1,data.peer);list[data.peer]=true;starts[data.peer]=leaf+1;spans[data.peer]=1;
    }
    #{visible:visible,list:list,spans:spans,start:starts,depth:high,tracks:leaf+(if data.peer!=""{1}else{0})}
}
fn nearest(id,data,view){
    while id!=""&&id in data.rows{if id in view.list{return id;}id=data.rows[id].parent;}
    data.root
}
fn selected_branch(id,data){
    let result=#{};
    for row in data.order{
        if row==id||data.rows[row].parent in result{result[row]=true;}
    }
    if id!=""&&id==data.peer{result[id]=true;}
    result
}
fn attend(run,id,target,ctx,e){
    let ui=state(ctx,run);ui.cursor=id;ctx.set_state("ui",ui);ctx.focus(target);ctx.scroll_into_view(target);handled()
}
fn focus_card(run,id,ctx,within){
    if within{let ui=state(ctx,run);ui.cursor=id;ctx.set_state("ui",ui);}
}
fn focus_tree(run,ctx,within){let ui=state(ctx,run);ui.within=within;ctx.set_state("ui",ui);}
fn select_tree_node(token,id,target,ctx,e){
    let ui=state(ctx,token.run);ui.cursor=id;ctx.set_state("ui",ui);ctx.focus(target);
    ctx.emit("select",#{run:token.run,revision:token.revision,agent:id});handled()
}
fn repair(ctx,deps){
    if deps==(){return;}
    let ui=state(ctx,deps.run);
    if ui.cursor!=deps.previous{return;}
    ui.cursor=deps.next;ctx.set_state("ui",ui);
    ctx.focus(deps.target);ctx.scroll_into_view(deps.target);
}
fn stopped(ctx,deps){}
fn inspect(run,phase,ctx,open){
    let ui=state(ctx,run);
    if open{ui.review=phase;}
    else if ui.review==phase{ui.review="";}
    ctx.set_state("ui",ui);
}
fn resolve(token,id,action,anchor,ctx,e){
    ctx.emit("resolve",#{run:token.run,revision:token.revision,agent:id,action:action});
    // The decision can remove the focused action on the next Host receipt.
    if anchor!=""{ctx.focus(anchor);}
    handled()
}
fn collapse(token,id,proposed,local,ctx,e){
    if local{let ui=state(ctx,token.run);ui.collapsed=proposed;ctx.set_state("ui",ui);}
    ctx.emit("collapse",#{run:token.run,revision:token.revision,agent:id,collapsed:proposed});handled()
}
fn opt_notice(props){opt(props,"notice")}
fn status(ctx,phase){
    if phase=="waiting"{return ctx.t_or("agent_trail.waiting","Needs you");}
    if phase=="queued"{return ctx.t_or("agent_queue.queued","Queued");}
    if phase=="running"{return ctx.t_or("agent_queue.running","Running");}
    if phase=="failed"{return ctx.t_or("agent_queue.failed","Failed");}
    if phase=="stopped"{return ctx.t_or("agent_checkpoints.stopped","Stopped");}
    ctx.t_or("agent_queue.done","Done")
}
fn duration(ctx,a,now){
    let start=opt(a,"started_ms");let end=opt(a,"ended_ms");
    if start==()||(end==()&&!(a.state in ["running","waiting"])){return "";}
    let sec=max(0,value(a,"ended_ms",now)-start)/1000;
    if sec<60{return ctx.t_or("agent_lanes.seconds","{n} s",#{n:ctx.format_number(sec)});}
    ctx.t_or("agent_inbox.minutes_seconds","{m} min {s} s",#{m:ctx.format_number(sec/60),s:ctx.format_number(sec%60)})
}
fn amount(ctx,n,currency){
    if n<0{return "-"+amount(ctx,-n,currency);}
    let label=if n>0&&n<10000{"<"+ctx.format_number(0.01,#{min_fraction_digits:2,max_fraction_digits:2})}
        else{ctx.format_number(n.to_float()/1000000.0,#{min_fraction_digits:2,max_fraction_digits:2})};
    label+" "+currency
}
fn note(ctx,v,part){common::heading_note(ctx,v,part)}
fn badge(ctx,a){
    let phase=a.state;let ink=theme_color(if phase=="failed"{"text.danger"}else if phase=="running"{"accent"}else{"text_primary"});
    let words=status(ctx,phase);let bits=[];
    if phase=="running"{bits.push(spinner::Spinner(#{key:"run:"+a.id,label:words,size:"xs"}).accessibility_hidden(true));}
    bits.push(text(words).with_key("status-text"));
    row(bits).with_key("status").with_style(ctx.component_style(["status","status_"+phase],
        style().min_width(px(0)).gap(theme_length("space.spacing_unit")).items_center().typography("caption").text_color(ink)))
}
fn avatar(ctx,a){
    let m=opt(a,"avatar");
    let child=if m!=(){m}else{text(if a.name==""{"?"}else{a.name.sub_string(0,1).to_upper()})};
    row([child]).with_key("avatar").accessibility_hidden(true).with_style(ctx.component_style("avatar",
        style().width(theme_length("space.unit").times(7)).height(theme_length("space.unit").times(7)).flex_shrink(false).items_center().justify_center()
        .radius(theme_length("space.unit").times(4.5)).background(theme_color("surface_hover")).typography("caption").font_weight(600)))
}
fn resolution_actions(ctx,cfg,a,phase){
    let actions=if phase=="waiting"{[["reject",ctx.t_or("gates.reject","Reject")],["approve",ctx.t_or("review.approve","Approve")]]}
        else{[["dismiss",ctx.t_or("agent_tree.dismiss","Dismiss")],["retry",ctx.t_or("agent_plan.retry","Retry")]]};
    let buttons=[];
    for i in 0..actions.len{
        let action=actions[i];
        buttons.push(button::Button(#{key:"resolve-"+action[0]+":"+a.id,text:action[1],size:"xs",variant:if i==1{"primary"}else{"secondary"},
            disabled:cfg.busy==a.id,on_click:Fn("resolve").curry(cfg.token,a.id,action[0],value(cfg,"return_focus",""))}).with_key(action[0])
            .accessibility_label(action[1]+" "+a.name));
    }
    row(buttons).with_key("actions").with_style(ctx.component_style("review_actions",
        style().gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap().justify_end().min_width(px(0))))
}
fn review_panel(ctx,cfg,data,ids,phase,notice){
    let items=[];
    for id in ids{
        let a=data.rows[id].a;let desc=if phase=="waiting"{opt(a,"ask")}else{opt(a,"error")};
        let bits=[text(a.name).with_key("name").accessibility_role("heading").accessibility_level(3)
            .with_style(ctx.component_style("review_name",style().min_width(px(0)).typography("body").font_weight(600))),
            note(ctx,a.task,"review_detail").with_key("task")];
        if desc!=(){bits.push(note(ctx,desc,"review_detail").with_key("evidence"));}
        if cfg.resolves{bits.push(resolution_actions(ctx,cfg,a,phase));}
        items.push(column(bits).with_key("case:"+id).with_style(ctx.component_style("review",
            style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(1.75)).gap(theme_length("space.spacing_unit")).radius(px(5)).background(theme_color("surface_hover")))));
    }
    let body=column(items).with_key("cases").scrollbars("hidden","auto").with_style(ctx.component_style("review_viewport",
        style().min_width(px(0)).max_height(px(280)).gap(theme_length("space.spacing_unit").times(1.5)).overflow_y_scroll()));
    let output=[body];
    if notice!=(){output.push(note(ctx,notice,"notice").with_key("feedback").accessibility_role("status"));}
    column(output).with_key("panel").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)))
}
fn summary_button(ctx,props,cfg,data,ui,phase,ids){
    let label=if phase=="waiting"{ctx.t_or("agent_tree.needs","Needs you: {n}",#{n:ctx.format_number(ids.len)})}
        else{ctx.t_or("agent_tree.failures","Failed: {n}",#{n:ctx.format_number(ids.len)})};
    let panel=if phase=="waiting"{ctx.t_or("agent_tree.queue","Agents that need you")}else{ctx.t_or("agent_tree.failed","Agents that failed")};
    let open=ui.review==phase;
    let trigger=button::Button(#{key:"summary-"+phase,text:label,size:"xs",variant:"secondary"}).with_key("trigger")
        .with_style(ctx.component_style(["summary_button","summary_"+phase],style())).accessibility_expanded(open);
    popover::Popover(#{key:phase,label:panel,trigger:trigger,open:open,placement:"bottom",align:"end",
        initial_focus:if cfg.resolves{"first"}else{"panel"},
        content:if open{review_panel(ctx,cfg,data,ids,phase,opt_notice(props))}else{column([])},
        on_open_change:Fn("inspect").curry(cfg.token.run,phase)}).with_key(phase)
}
fn header(ctx,props,cfg,data,ui,title){
    let groups=[note(ctx,ctx.t_or("agent_tree.agents","Agents: {n}",#{n:ctx.format_number(props.agents.len)}),"count").with_key("count")];
    if data.running>0{groups.push(note(ctx,ctx.t_or("agent_tree.running","Running: {n}",#{n:ctx.format_number(data.running)}),"running_count").with_key("running"));}
    if data.waiting.len>0{groups.push(summary_button(ctx,props,cfg,data,ui,"waiting",data.waiting));}
    if data.failed.len>0{groups.push(summary_button(ctx,props,cfg,data,ui,"failed",data.failed));}
    groups.push(note(ctx,amount(ctx,data.total,cfg.currency),"total").with_key("total").accessibility_role("status")
        .accessibility_label(ctx.t_or("agent_tree.total","Run total: {amount}",#{amount:amount(ctx,data.total,cfg.currency)})));
    row([text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
            .with_style(ctx.component_style("title",style().min_width(px(0)).typography("body").font_weight(600))),
        row(groups).with_key("stats").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("stats",
            style().min_width(px(0)).flex_grow().items_center().justify_end().gap(theme_length("space.spacing_unit").times(1.75)).flex_wrap()))])
        .with_key("head").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("head",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).items_center().flex_wrap()))
}
fn fold_action(cfg,data,id){
    let ids=[];for key in data.order{if (key in cfg.folded)!=(key==id){ids.push(key);}}
    Fn("collapse").curry(cfg.token,id,ids,cfg.local)
}
fn move_key(n,combo,cfg,data,id){
    if id==""{return n;}
    n.on_key_value(combo,Fn("attend").curry(cfg.token.run,id,ref_key(data,id)),())
}
fn card(ctx,cfg,data,view,id,index,reference){
    let a=data.rows[id].a;let children=data.kids[id];let parent=data.rows[id].parent;
    let selected=cfg.selected==id;let related=id in cfg.branch;let has=children.len>0;let closed=id in cfg.folded;
    let detail=status(ctx,a.state);let model=opt(a,"model");if model!=(){detail+=" · "+model;}
    let own=amount(ctx,value(a,"cost_micros",0),cfg.currency);let full=amount(ctx,data.cost[id],cfg.currency);
    let stamp=duration(ctx,a,cfg.now);if stamp!=""{detail+=" · "+stamp;}
    let price=if has{ctx.t_or("agent_tree.subtree","Branch: {total}",#{total:full})}else{full};
    detail+=" · "+price;
    if parent!=""{detail+=" · "+ctx.t_or("agent_tree.parent","Dispatched by {name}",#{name:data.rows[parent].a.name});}
    if id==data.peer{detail+=" · "+ctx.t_or("agent_tree.supervisor","Supervisor");}
    let title=[avatar(ctx,a),text(a.name).with_key("name").with_style(ctx.component_style("name",style().min_width(px(0)).flex_grow().typography("body").font_weight(600)))];
    if has{
        let verb=if closed{ctx.t_or("agent_tree.expand","Expand {name}",#{name:a.name})}else{ctx.t_or("agent_tree.collapse","Collapse {name}",#{name:a.name})};
        title.push(button::Button(#{key:"fold:"+id,text:if closed{"⌄"}else{"⌃"},size:"xs",variant:"ghost",
            disabled:!cfg.local&&!cfg.collapse,on_click:fold_action(cfg,data,id)})
            .with_key("fold").tab_stop(false).accessibility_label(verb).accessibility_expanded(!closed).with_style(ctx.component_style("fold",style().flex_shrink(false))));
    }
    let tools=[];let supplied=value(a,"tools",[]);
    for i in 0..supplied.len{
        let t=supplied[i];let icon=opt(t,"icon");detail+=" · "+t.label;
        tools.push(row([if icon!=(){icon}else{text(if t.label==""{"?"}else{t.label.sub_string(0,1).to_upper()})}])
            .with_key("tool-"+i.to_string()).accessibility_hidden(true).with_style(ctx.component_style("tool",
                style().width(theme_length("space.unit").times(4.25)).height(theme_length("space.unit").times(4.25)).items_center().justify_center().flex_shrink(false).typography("caption")
                .radius(px(4)).background(theme_color("surface_hover")))));
    }
    let contents=[row(title).with_key("headline").audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).items_center()),
        note(ctx,a.task,"task").with_key("task")];
    let small=[badge(ctx,a)];if model!=(){small.push(note(ctx,model,"model").with_key("model"));}
    if tools.len>0{small.push(row(tools).with_key("tools").with_style(ctx.component_style("tools",style().gap(theme_length("space.spacing_unit").times(0.5)).flex_wrap())));}
    contents.push(row(small).with_key("metadata").audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).flex_wrap().items_center()));
    let extra=opt(a,"extra");if extra!=(){contents.push(column([extra]).with_key("extra").with_style(style().min_width(px(0))));}
    let foot=[note(ctx,price,"cost").with_key("cost")];
    if stamp!=""{foot.push(note(ctx,stamp,"time").with_key("time"));}
    if has&&data.cost[id]!=value(a,"cost_micros",0){detail+=" · "+ctx.t_or("agent_tree.own","Own cost: {amount}",#{amount:own});}
    contents.push(row(foot).with_key("footer").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).justify_between().flex_wrap()));
    let parts=["card"];
    if id==data.peer{parts.push("supervisor");}
    if selected{parts.push("selected");}
    else if cfg.selected!=""{parts.push(if related{"related"}else{"outside"});}
    let key=ref_key(data,id);
    let n=column(contents).with_key("card:"+id).with_ref(reference).accessibility_role("treeitem").accessibility_label(a.name)
        .accessibility_description(a.task+" · "+detail).accessibility_level(data.depth[id]+1).accessibility_selected(selected)
        .tab_stop(cfg.focus==id).audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style(parts,style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2)).gap(theme_length("space.spacing_unit").times(1.25)).radius(theme_length("space.unit").times(2.25)).border(px(1))
            .border_color(theme_color(if selected{"accent"}else{"border"})).background(theme_color("surface_raised"))
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .on_focus_within_change(Fn("focus_card").curry(cfg.token.run,id))
        .on_click(Fn("select_tree_node").curry(cfg.token,id,key))
        .on_key_value("plain+enter",Fn("select_tree_node").curry(cfg.token,id,key),())
        .enter_motion(motion_transition("opacity",0.6,1.0,#{duration_ms:ctx.motion_duration("fast"),intent:"decorative"}));
    if has{n=n.accessibility_expanded(!closed);}
    let list=view.visible;
    n=move_key(n,"plain+up",cfg,data,list[max(0,index-1)]);
    n=move_key(n,"plain+down",cfg,data,list[min(list.len-1,index+1)]);
    n=move_key(n,"plain+home",cfg,data,list[0]);
    n=move_key(n,"plain+end",cfg,data,list[list.len-1]);
    let into=if cfg.rtl{"plain+left"}else{"plain+right"};let away=if cfg.rtl{"plain+right"}else{"plain+left"};
    let can=cfg.local||cfg.collapse;
    if has&&closed&&can{n=n.on_key_value(into,fold_action(cfg,data,id),());}
    else if has&&!closed{n=move_key(n,into,cfg,data,children[0]);}
    if has&&!closed&&can{n=n.on_key_value(away,fold_action(cfg,data,id),());}
    else if parent!=""{n=move_key(n,away,cfg,data,parent);}
    let grid=if cfg.wide{
        style().grid_cell(view.start[id],data.depth[id]+1).col_span(view.spans[id]).min_width(px(0)).justify_center()
    }else{
        let depth=if id==data.peer{0}else{data.depth[id]};
        style().grid_cell(1,index+1).min_width(px(0))
            .padding_start(theme_length("space.unit").times((10.0+depth.to_float()*cfg.indent)/4.0))
    };
    let width=if cfg.wide{style().width(relative(1.0)).max_width(px(242))}else{style().width(relative(1.0))};
    row([n.with_style(width)]).with_key("cell:"+id).with_style(ctx.component_style("cell",grid))
}
fn coords(ctx,references,id,frame){
    let ref_bounds=ctx.element_bounds(references[id]);if ref_bounds==(){return ();}
    let p=ref_bounds.visual;
    #{left:p.x-frame.x,right:p.x+p.width-frame.x,top:p.y-frame.y,bottom:p.y+p.height-frame.y,
        x:p.x+p.width*0.5-frame.x,y:p.y+p.height*0.5-frame.y}
}
fn dash(key,x,y,z,w){
    let commands=[];
    let span=((z-x)*(z-x)+(w-y)*(w-y)).sqrt();if span<1.0{return commands;}
    let count=min(50,max(1,(span/6.0).ceiling().to_int()));
    for i in 0..count{
        let a=i.to_float()/count.to_float();let b=(i.to_float()+0.55)/count.to_float();
        commands.push(canvas_line(key+i.to_string(),x+(z-x)*a,y+(w-y)*a,x+(z-x)*b,y+(w-y)*b,1.0,theme_color("text_muted")));
    }
    commands
}
fn wires(ctx,cfg,data,view,stage,refs){
    let region=ctx.element_bounds(stage);if region==(){return ();}
    let b=region.visual;if b.width<1.0||b.height<1.0{return ();}
    let points=#{};for id in view.visible{points[id]=coords(ctx,refs,id,b);}
    let commands=[];let joints=#{};
    for id in view.visible{
        let parent=data.rows[id].parent;
        if parent==""||!(parent in points)||points[id]==()||points[parent]==(){continue;}
        let a=points[parent];let z=points[id];let key="edge-"+data.rows[id].i.to_string();
        let highlighted=cfg.selected==""||(id in cfg.branch&&parent in cfg.branch);
        let ink=theme_color(if highlighted{"agent_tree.link"}else{"agent_tree.link_muted"});
        if cfg.wide&&z.top>a.bottom{
            let mid=(a.bottom+z.top)*0.5;
            commands.push(canvas_stroke_path(key,[path_move(a.x,a.bottom),path_cubic(z.x,z.top,a.x,mid,z.x,mid)],1.75,ink).stroke_cap("round"));
            if !(parent in joints)||highlighted{joints[parent]=#{x:a.x,y:a.bottom,ink:ink};}
        }else if !cfg.wide&&z.y>a.y{
            let x=if cfg.rtl{a.right}else{a.left};let tx=if cfg.rtl{z.right}else{z.left};
            let gutter=x+(if cfg.rtl{6.0}else{-6.0});
            commands.push(canvas_stroke_path(key,[path_move(x,a.y),path_line(gutter,a.y),path_line(gutter,z.y),path_line(tx,z.y)],1.75,ink)
                .stroke_cap("round").stroke_join("round"));
            if !(parent in joints)||highlighted{joints[parent]=#{x:gutter,y:a.y,ink:ink};}
        }
    }
    if data.peer!=""&&points[data.root]!=()&&points[data.peer]!=(){
        let a=points[data.root];let z=points[data.peer];
        let dashes=if cfg.wide{
            dash("watch-",if cfg.rtl{a.left}else{a.right},a.y,if cfg.rtl{z.right}else{z.left},z.y)
        }else if z.top>a.bottom{
            dash("watch-",a.x,a.bottom,z.x,z.top)
        }else{[]};
        for mark in dashes{commands.push(mark);}
    }
    for parent in joints.keys(){
        let m=joints[parent];let key=data.rows[parent].i.to_string();
        commands.push(canvas_circle("joint-"+key,m.x,m.y,3.3,m.ink));
        commands.push(canvas_circle("inset-"+key,m.x,m.y,1.6,theme_color("surface_raised")));
    }
    if commands.len==0{return ();}
    canvas(canvas_scene(commands).view_box(0,0,b.width,b.height)).with_key("agent-links").accessibility_hidden(true)
        .with_style(ctx.component_style("rail",style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0))))
}
fn render_AgentTree(ctx,props){
    let data=project(props);let ui=state(ctx,props.run);let folded=folding(props,ui,data);let view=plan(data,folded);
    let stage=element_ref("stage");let shell=element_ref("shell");
    let limit=190*view.tracks+14*max(0,view.tracks-1)+20;
    // The cards may reflow and change the canvas size. Only the allocated
    // shell width can choose the layout; it does not depend on card geometry.
    let wide=view.tracks>0&&view.tracks<=6&&ctx.element_width_class(shell,[limit])==1;
    let sel=opt(props,"selected");let selected=if sel!=()&&sel in data.rows{sel}else{""};
    let focus=nearest(ui.cursor,data,view);
    let cfg=#{token:#{run:props.run,revision:props.revision},wide:wide,
        rtl:ctx.text_direction()=="rtl",indent:min(26.0,90.0/max(1,view.depth).to_float()),
        currency:value(props,"currency","USD"),now:props.now_ms,focus:focus,selected:selected,branch:selected_branch(selected,data),
        folded:folded,local:opt(props,"collapsed")==(),collapse:opt(props,"on_collapse")!=(),resolves:opt(props,"on_resolve")!=(),busy:opt(props,"busy_agent")};
    let title=value(props,"title",ctx.t_or("agent_tree.title","Agent tree"));
    let output=[header(ctx,props,cfg,data,ui,title)];
    let top=opt(props,"top");if top!=(){output.push(column([top]).with_key("top").with_style(style().min_width(px(0))));}
    if view.visible.len==0{output.push(note(ctx,ctx.t_or("agent_tree.empty","No agents yet"),"empty").with_key("empty"));}
    else{
        let refs=#{};let cards=[];
        for id in view.visible{refs[id]=element_ref(ref_key(data,id));}
        for i in 0..view.visible.len{let id=view.visible[i];cards.push(card(ctx,cfg,data,view,id,i,refs[id]));}
        let grid=box(cards).with_key("cards").accessibility_role("tree").accessibility_label(title)
            .on_focus_within_change(Fn("focus_tree").curry(props.run))
            .with_style(ctx.component_style("grid",style().grid().grid_cols(if cfg.wide{view.tracks}else{1})
                .min_width(px(0)).gap_y(px(if cfg.wide{31}else{12})).gap_x(px(14))));
        let canvas=wires(ctx,cfg,data,view,stage,refs);
        let scene=if canvas==(){[grid]}else{[canvas,grid]};
        output.push(stack(scene).with_key("stage").with_ref(stage).accessibility_role("group")
            .accessibility_label(ctx.t_or("agent_tree.dispatch","Dispatch hierarchy"))
            .with_style(ctx.component_style("stage",style().min_width(px(0)).relative())));
    }
    let feedback=opt_notice(props);
    if feedback!=(){output.push(note(ctx,feedback,"notice").with_key("feedback").accessibility_role("status"));}
    let recovered=if ui.within&&ui.cursor!=""&&focus!=""&&focus!=ui.cursor{
        #{run:props.run,previous:ui.cursor,next:focus,target:ref_key(data,focus)}
    }else{()};
    effect("repair_focus",recovered,Fn("repair"),Fn("stopped"));
    column(output).with_key(props.key).with_ref(shell).accessibility_role("group").accessibility_label(title).audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).padding(theme_length("space.spacing_unit")).gap(theme_length("space.spacing_unit").times(2.25))
            .width(relative(1.0)).background(theme_color("surface_hover")).radius(theme_length("space.unit").times(3.75))))
}
