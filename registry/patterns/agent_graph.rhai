/* gpui-rhai
{"id":"patterns/agent_graph","export":"AgentGraph","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","agent_tree.link","border","focus_ring","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text_muted","text_primary","text.warning","typography.body","typography.caption"],"environment":[]}
*/
// Requests and results travel across the accepted run's communication graph.
// Example: AgentGraph(#{key:"graph",run:"preview",revision:0,now_ms:0,nodes:[]})
import "components/button" as button;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_graph","export":"AgentGraph",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","patterns/approval_card"],capabilities:#{},
        tokens:["accent","agent_tree.link","border","focus_ring","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text_muted","text_primary",
            "text.warning","typography.body","typography.caption"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable presentation key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run identity; change this when the call ID namespace is replaced."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision for selection intents."},
        now_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Host clock for link heat and the calls-per-minute count."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Graph heading."},
        height:#{schema:#{type:"optional",value:#{type:"integer",min:190,max:1600}},required:false,sensitive:false,doc:"Preferred logical canvas height. May grow to fit dense graphs without overlapping targets."},
        heat_window_ms:#{schema:#{type:"integer",min:1000,max:3600000},"default":#{type:"integer",value:45000},
            required:false,sensitive:false,doc:"Age at which requests no longer contribute to link heat."},
        currency:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Currency label for costs; defaults to USD."},
        nodes:#{schema:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable node identity."},
            name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Full name, also used by assistive technology."},
            short:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional visible short label for compact layouts."},
            kind:#{schema:#{type:"string",allowed:["agent","tool","user"]},required:true,sensitive:false,doc:"Agents are on the inner ring; tools and users are on the outer ring."},
            lead:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Optional single lead agent; defaults to the first agent."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative mark; defaults to initials."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Model or short description shown in the inspector."},
            doing:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Current activity summary."},
            status:#{schema:#{type:"optional",value:#{type:"string",allowed:["idle","working","waiting","error","done"]}},
                required:false,sensitive:false,doc:"Accepted status; defaults to idle."},
            cost_micros:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Own cost in millionths of the currency unit."},
        }}},required:true,sensitive:true,doc:"Up to 24 distinct nodes. Reordering nodes can change their orbit positions, not their callback identity."},
        edges:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            from:#{schema:#{type:"string"},required:true,sensitive:false,doc:"One endpoint."},
            to:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Other endpoint."},
        }}}},required:false,sensitive:false,doc:"Standing connections. Opposite declarations represent one connection."},
        calls:#{schema:#{type:"optional",value:#{type:"array",max_items:96,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of this call in the run; do not reuse."},
            from:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Caller."},
            to:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Receiver."},
            label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short context for the call."},
            at_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"When the call was made, in the Host clock domain."},
            elapsed_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Duration of the terminal reply."},
            status:#{schema:#{type:"optional",value:#{type:"string",allowed:["pending","ok","error"]}},
                required:false,sensitive:false,doc:"Pending goes outbound; accepted terminal transitions return along the same edge."},
        }}}},required:false,sensitive:true,doc:"Recent calls in chronological order, newest last. Calls can add temporary links (64 distinct total)."},
        selected:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted inspected node in controlled mode."},
        on_select:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Enable Host-owned selection and receive versioned proposals. Omit for local inspection."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional Host feedback for a proposed selection."},
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        select:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run that owns the nodes."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Snapshot observed by the user."},
            node:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Requested node ID; empty string requests dismissal."},
        }},doc:"In controlled mode the inspector changes only when the Host updates selected."},
    },slots:#{},parts:["root","bar","title","stats","count","total","panel","stage","orbit","wires","pulse","node","agent","tool","user","lead",
        "node_selected","node_neighbor","node_quiet","mark","node_label","state","inspector","inspector_head","inspector_title","detail","status",
        "status_working","status_waiting","status_error","status_done","calls","call","call_peer","call_label","call_end","inspect_cost","close","legend",
        "legend_label","empty","notice"],effects:["observe_calls"]},
    render:Fn("render_AgentGraph")
});

fn AgentGraph(props){render_component("patterns/agent_graph",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,d){common::value_or(m,k,d)}
fn valid(id){
    if id==""||utf8_byte_count(id)>96{return false;}
    for c in id.chars(){if c<' '||(c>=''&&c<=''){return false;}}
    true
}
fn pair(a,b){
    let first=if a<b{a}else{b};let second=if a<b{b}else{a};
    utf8_byte_count(first).to_string()+":"+first+second
}
fn phase(n){value(n,"status","idle")}
fn call_phase(c){value(c,"status","pending")}
fn link(a,b,rows){
    if a==b||!(a in rows)||!(b in rows){throw "Graph link endpoints must be distinct existing nodes";}
    #{a:if a<b{a}else{b},b:if a<b{b}else{a},heat:0,pending:false,error:false,standing:false}
}
fn project(props){
    if !valid(props.run){throw "Graph needs a stable run ID";}
    let data=#{rows:#{},indexes:#{},agents:[],outer:[],lead:"",pairs:#{},order:[],calls:[],phases:[],money:0,minute:0};
    for i in 0..props.nodes.len{
        let n=props.nodes[i];if !valid(n.id)||n.id in data.rows{throw "Graph node IDs must be distinct and bounded";}
        data.rows[n.id]=n;data.indexes[n.id]=i;data.money+=value(n,"cost_micros",0);
        if value(n,"lead",false){
            if n.kind!="agent"||data.lead!=""{throw "Graph accepts only one lead agent";}
            data.lead=n.id;
        }
        if n.kind=="agent"{data.agents.push(n.id);}else{data.outer.push(n.id);}
    }
    if data.lead==""&&data.agents.len>0{data.lead=data.agents[0];}
    for edge in value(props,"edges",[]){
        let key=pair(edge.from,edge.to);
        if !(key in data.pairs){data.pairs[key]=link(edge.from,edge.to,data.rows);data.order.push(key);}
        data.pairs[key].standing=true;
    }
    let seen=#{};let previous=-1;
    for c in value(props,"calls",[]){
        if !valid(c.id)||c.id in seen||c.at_ms<previous||c.at_ms>props.now_ms{throw "Graph calls need unique IDs and chronological Host timestamps";}
        seen[c.id]=true;previous=c.at_ms;
        let key=pair(c.from,c.to);
        if !(key in data.pairs){data.pairs[key]=link(c.from,c.to,data.rows);data.order.push(key);}
        let elapsed=props.now_ms-c.at_ms;let status=call_phase(c);
        if elapsed<=60000{data.minute+=1;}
        if elapsed<=props.heat_window_ms{
            data.pairs[key].heat+=1;
            data.pairs[key].pending=data.pairs[key].pending||status=="pending";
            data.pairs[key].error=status=="error";
        }
        data.calls.push(c);
        data.phases.push(#{id:c.id,from:c.from,to:c.to,status:status,at:c.at_ms});
    }
    if data.order.len>64{throw "Graph accepts at most 64 distinct connections";}
    data
}
fn local(ctx,run){
    let s=ctx.get_state("ui");if s!=()&&s.run==run{return s;}
    #{run:run,choice:"",cursor:"",seen:#{},known:[],watermark:-1,started:false,next:0,slots:[(),(),(),(),(),()]}
}
fn observe(ctx,deps){
    let s=local(ctx,deps.run);let before=s.watermark;let changed=!s.started;
    for c in deps.calls{
        let old=opt(s.seen,c.id);
        if old==c{continue;}
        changed=true;
        let reply=old!=()&&old.status=="pending"&&c.status!="pending"&&old.from==c.from&&old.to==c.to&&old.at==c.at;
        if s.started&&((old==()&&c.at>=before)||reply){
            s.next+=1;s.slots[(s.next-1)%6]=#{serial:s.next,id:c.id,from:c.from,to:c.to,status:c.status};
        }
        if old==(){s.known.push(c.id);}
        s.seen[c.id]=c;s.watermark=max(s.watermark,c.at);
        if s.known.len>192{let retired=s.known.shift();s.seen.remove(retired);}
    }
    s.started=true;
    if changed{ctx.set_state("ui",s);}
}
fn stop(ctx,deps){}
fn ref_key(data,id){"anchor-"+data.indexes[id].to_string()}
fn attend(run,id,ref_name,ctx,e){
    let s=local(ctx,run);s.cursor=id;ctx.set_state("ui",s);ctx.focus(ref_name);handled()
}
fn focused(run,id,ctx,inside){
    if inside{let s=local(ctx,run);if s.cursor!=id{s.cursor=id;ctx.set_state("ui",s);}}
}
fn select_graph_agent(token,id,ref_name,ctx,e){
    let s=local(ctx,token.run);
    if !token.controlled{s.choice=id;}
    if id!=""{s.cursor=id;}
    ctx.set_state("ui",s);if ref_name!=""{ctx.focus(ref_name);}
    ctx.emit("select",#{run:token.run,revision:token.revision,node:id});handled()
}
fn node_status(ctx,s){
    switch s{
        "working"=>ctx.t_or("agent_queue.running","Running"),
        "waiting"=>ctx.t_or("agent_trail.waiting","Needs you"),
        "error"=>ctx.t_or("agent_queue.failed","Failed"),
        "done"=>ctx.t_or("agent_queue.done","Done"),
        _=>ctx.t_or("agent_graph.idle","Idle"),
    }
}
fn call_status(ctx,c){
    let status=call_phase(c);let ms=opt(c,"elapsed_ms");
    if status=="pending"{return ctx.t_or("agent_graph.sent","Pending");}
    if status=="error"{return ctx.t_or("agent_graph.error","Error");}
    if ms==(){return ctx.t_or("agent_graph.returned","Returned");}
    if ms<1000{return ctx.t_or("agent_graph.milliseconds","{n} ms",#{n:ctx.format_number(ms)});}
    ctx.t_or("agent_lanes.seconds","{n} s",#{n:ctx.format_number(ms.to_float()/1000.0,#{max_fraction_digits:1})})
}
fn spend(ctx,n,currency){
    let f=if n>0&&n<10000{0.01}else{n.to_float()/1000000.0};
    (if n>0&&n<10000{"<"}else{""})+ctx.format_number(f,#{min_fraction_digits:2,max_fraction_digits:2})+" "+currency
}
fn note(ctx,v,part){common::heading_note(ctx,v,part)}
fn dot_ink(s){theme_color(if s=="error"{"text.danger"}else if s=="waiting"{"text.warning"}else if s=="done"{"text.success"}else{"accent"})}
fn point(x,y){#{x:x,y:y}}
fn grip(n,lead,compact){
    if n.id==lead{return if compact{48.0}else{58.0};}
    if n.kind=="agent"{return if compact{44.0}else{48.0};}
    44.0
}
fn point_at(w,h,t,inner){
    let angle=(t-0.25)*6.283185307179586;
    let rx=max(8.0,w*0.5-52.0);let ry=max(8.0,h*0.5-54.0);
    point(w*0.5+angle.cos()*rx*(if inner{0.52}else{1.0}),h*0.5+angle.sin()*ry*(if inner{0.58}else{1.0}))
}
fn free(p,occupied,dx,dy){
    for q in occupied{if (p.x-q.x).abs()<dx&&(p.y-q.y).abs()<dy{return false;}}
    true
}
fn nearby(data,inner,id,index){
    // Only standing connections can order the outer orbit. A call or
    // reply changes the links, not the native node locations.
    let x=0.0;let y=0.0;
    for key in data.order{
        let edge=data.pairs[key];if !edge.standing{continue;}
        let other=if edge.a==id{edge.b}else if edge.b==id{edge.a}else{""};
        if other in inner{let t=inner[other]*6.283185307179586;x+=t.cos();y+=t.sin();}
    }
    if x.abs()+y.abs()<0.0001{return if data.rows[id].kind=="user"{0.0}else{(index.to_float()+0.5)/max(1,data.outer.len).to_float()};}
    let turn=y.atan(x)/6.283185307179586;
    if turn<0.0{turn+1.0}else{turn}
}
fn orbits(data,w,h){
    let spots=#{};let occupied=[];let angles=#{};let inner=[];
    if data.lead!=""{let center=point(w*0.5,h*0.5);spots[data.lead]=center;occupied.push(center);}
    for id in data.agents{if id!=data.lead{inner.push(id);}}
    for i in 0..inner.len{
        let id=inner[i];let at=(i.to_float()+0.5)/inner.len.to_float();let p=point_at(w,h,at,true);
        if !free(p,occupied,81.0,66.0){return ();}
        angles[id]=at;spots[id]=p;occupied.push(p);
    }
    let preferred=[];let px=0.0;let py=0.0;let n=data.outer.len;
    for i in 0..n{
        let id=data.outer[i];let pref=nearby(data,angles,id,i);
        let next=0;while next<preferred.len&&preferred[next].turn<=pref{next+=1;}
        preferred.insert(next,#{id:id,turn:pref});
    }
    for i in 0..n{
        let t=(preferred[i].turn-i.to_float()/n.to_float())*6.283185307179586;
        px+=t.cos();py+=t.sin();
    }
    let offset=if n==0||px.abs()+py.abs()<0.0001{0.0}else{py.atan(px)/6.283185307179586};
    for i in 0..n{
        let id=preferred[i].id;let base=offset+i.to_float()/n.to_float();let fit=();
        for j in [0,1,-1,2,-2,3,-3,4,-4,5,-5]{
            let p=point_at(w,h,base+j.to_float()*0.027,false);
            if free(p,occupied,82.0,68.0){fit=p;break;}
        }
        if fit==(){return ();}
        spots[id]=fit;occupied.push(fit);
    }
    spots
}
fn layout(props,data,w){
    let compact=w<540.0;let height=value(props,"height",if compact{442}else{390}).to_float();
    let h=max(height,188.0);let spots=if w<176.0{()}else{orbits(data,w,h)};let orbital=spots!=();
    if !orbital{
        h=max(h,520.0+max(0,props.nodes.len-8).to_float()*30.0);
        if w>=176.0{spots=orbits(data,w,h);orbital=spots!=();}
    }
    if !orbital{
        // Oversubscribed rings keep every node reachable in a regular grid.
        // This uses the same positioned controls and the same link router.
        spots=#{};let columns=max(1,min(6,(w/96.0).floor().to_int()));
        let rows=(props.nodes.len+columns-1)/columns;
        h=max(height,28.0+84.0*rows.to_float());
        for i in 0..props.nodes.len{
            let id=props.nodes[i].id;
            spots[id]=point(w*(0.5+(i%columns).to_float())/columns.to_float(),41.0+84.0*(i/columns).to_float());
        }
    }
    #{sp:spots,w:w,h:h,small:compact,orbital:orbital}
}
fn span(a,b){((a.x-b.x)*(a.x-b.x)+(a.y-b.y)*(a.y-b.y)).sqrt()}
fn toward(a,b,d){
    let l=span(a,b);if l<0.001{return a;}
    point(a.x+(b.x-a.x)*d/l,a.y+(b.y-a.y)*d/l)
}
fn points_for(a,b,ra,rb,turn){
    let mid=if turn==0{point(b.x,a.y)}else{point(a.x,b.y)};
    if span(a,mid)<=ra+4.0||span(mid,b)<=rb+4.0{return [];}
    [toward(a,mid,ra),mid,toward(b,mid,rb)]
}
fn orth_penalty(points,p,r){
    let score=0;
    for i in 1..points.len{
        let a=points[i-1];let b=points[i];
        if p.x>=min(a.x,b.x)-r&&p.x<=max(a.x,b.x)+r&&p.y>=min(a.y,b.y)-r&&p.y<=max(a.y,b.y)+r{score+=1;}
    }
    score
}
fn route(data,plan,edge){
    let a=plan.sp[edge.a];let b=plan.sp[edge.b];let ra=grip(data.rows[edge.a],data.lead,plan.small)*0.5;
    let rb=grip(data.rows[edge.b],data.lead,plan.small)*0.5;
    let one=points_for(a,b,ra,rb,0);let two=points_for(a,b,ra,rb,1);let o=if one.len==0{1000}else{0};let t=if two.len==0{1000}else{0};
    for id in plan.sp.keys(){
        if id==edge.a||id==edge.b{continue;}
        let p=plan.sp[id];let hit=grip(data.rows[id],data.lead,plan.small)*0.5+3.0;
        if o<1000{o+=orth_penalty(one,p,hit);}
        if t<1000{t+=orth_penalty(two,p,hit);}
    }
    // Keep a near-vertical leg on the vertical port when both bends are
    // clear, rather than stacking it on the adjacent horizontal channel.
    let horizontal=(a.x-b.x).abs()>(a.y-b.y).abs()*1.3;
    let chosen=if o<t||(o==t&&horizontal){one}else{two};
    if chosen.len==0{return [path_move(toward(a,b,ra).x,toward(a,b,ra).y),path_line(toward(b,a,rb).x,toward(b,a,rb).y)];}
    let first=chosen[0];let mid=chosen[1];let last=chosen[2];
    let radius=min(16.0,min(span(first,mid)*0.45,span(mid,last)*0.45));
    let dip=toward(mid,first,radius);let out=toward(mid,last,radius);
    [path_move(first.x,first.y),path_line(dip.x,dip.y),path_quadratic(out.x,out.y,mid.x,mid.y),path_line(last.x,last.y)]
}
fn full(ctx,part){
    ctx.component_style(part,style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)))
}
fn guide(ctx,plan){
    if !plan.orbital{return ();}
    let h=plan.h;let w=plan.w;let marks=[];
    for inner in [false,true]{
        let points=[];let key=if inner{"inner"}else{"outer"};
        for i in 0..49{
            let pt=point_at(w,h,i.to_float()/48.0,inner);
            points.push(if i==0{path_move(pt.x,pt.y)}else{path_line(pt.x,pt.y)});
        }
        marks.push(canvas_stroke_path(key,points,0.7,alpha(theme_color("border"),0.64)));
    }
    canvas(canvas_scene(marks).view_box(0,0,w,h)).with_key("orbits").accessibility_hidden(true).with_style(full(ctx,"orbit"))
}
fn rails(ctx,plan,data,paths,selected){
    let commands=[];
    for i in 0..data.order.len{
        let key=data.order[i];let e=data.pairs[key];
        let active=selected==""||selected in [e.a,e.b];let weight=1.15+min(8,e.heat).to_float()*0.25;
        let color=if e.error{"text.danger"}else if e.pending&&(phase(data.rows[e.b])=="waiting"||phase(data.rows[e.a])=="waiting"){"text.warning"}else{"agent_tree.link"};
        let alpha=if !active{0.16}else if e.heat==0{0.48}else{0.88};
        commands.push(canvas_stroke_path("edge-"+i.to_string(),paths[key],weight,alpha(theme_color(color),alpha)).stroke_cap("round"));
    }
    canvas(canvas_scene(commands).view_box(0,0,plan.w,plan.h)).with_key("wires").accessibility_hidden(true).with_style(full(ctx,"wires"))
}
fn pulses(ctx,cfg,plan,paths,slots){
    let output=[];
    for i in 0..slots.len{
        let p=slots[i];let scene=canvas_scene([]);let key=cfg.token.run+":"+i.to_string()+":base";
        if p!=(){
            key=cfg.token.run+":"+p.serial.to_string()+":call";
            let edge=pair(p.from,p.to);let response=p.status!="pending";
            if edge in paths{
                let reverse=(p.from>p.to)!=response;
                let paint=theme_color(if p.status=="error"{"text.danger"}else{"agent_tree.link"});
                scene=canvas_scene([canvas_stroke_path("route",paths[edge],1.0,rgba(0x00000000))])
                    .path_pulse("route",3.9,paint,response,reverse);
            }
        }
        output.push(canvas(scene.view_box(0,0,plan.w,plan.h)).with_key("pulse-"+i.to_string())
            .accessibility_hidden(true).with_style(full(ctx,"pulse"))
            .motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:720,easing:"linear",intent:"decorative"}))
            .motion(motion_keyframes("opacity",[#{offset:0.0,value:0.0},#{offset:0.08,value:1.0},#{offset:0.80,value:1.0},#{offset:1.0,value:0.0}],
                #{duration_ms:720,intent:"decorative"}))
            .motion_on_change(key));
    }
    output
}
fn neighbor(id,plan,dx,dy){
    let p=plan.sp[id];let best="";let score=10000000.0;
    for other in plan.sp.keys(){
        let q=plan.sp[other];let main=(q.x-p.x)*dx+(q.y-p.y)*dy;if main<=1.0{continue;}
        let cross=((q.y-p.y)*dx+(q.x-p.x)*dy).abs();
        let s=main+cross*2.25;
        if s<score{score=s;best=other;}
    }
    best
}
fn names(n){
    let c="";for word in n.name.split(" "){if word!=""{c+=word.sub_string(0,1).to_upper();}if c.len>=2{break;}}
    if c==""{"•"}else{c}
}
fn marker(ctx,n){
    let icon=opt(n,"icon");if icon!=(){icon}else{text(names(n)).with_style(ctx.component_style("mark",style().typography("caption").font_weight(600)))}
}
fn at_node(ctx,cfg,plan,data,n,reference,related){
    let p=plan.sp[n.id];let size=grip(n,data.lead,plan.small);let current=cfg.selected==n.id;let state=phase(n);
    let selected=if current{""}else{n.id};let ref_name=ref_key(data,n.id);
    let color=theme_color(if current{"accent"}else{"border"});
    let parts=["node",n.kind];if n.id==data.lead{parts.push("lead");}
    if current{parts.push("node_selected");}
    else if cfg.selected!=""{parts.push(if related{"node_neighbor"}else{"node_quiet"});}
    let children=[marker(ctx,n).with_key("mark")];
    if state!="idle"{
        children.push(box([]).with_key("state").accessibility_hidden(true).with_style(ctx.component_style("state",style().absolute()
            .left(px(size-11.0)).top(px(0)).width(px(10)).height(px(10)).radius(px(6))
            .border(px(1)).border_color(theme_color("surface_raised")).background(dot_ink(state)))));
    }
    let desc=n.kind+" · "+node_status(ctx,state);let detail=opt(n,"detail");
    if detail!=(){desc+=" · "+detail;}
    let tip=row(children).with_key("target:"+n.id).with_ref(reference).accessibility_role("button")
        .accessibility_label(n.name).accessibility_description(desc).accessibility_pressed(current).tab_stop(cfg.focus==n.id)
        .with_style(ctx.component_style(parts,style().items_center().justify_center().radius(theme_length("space.unit").times(3.25)).relative()
            .background(theme_color("surface_raised")).border(px(1)).border_color(color)
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .with_style(style().width(px(size)).height(px(size)).flex_shrink(false))
        .on_click(Fn("select_graph_agent").curry(cfg.token,selected,ref_name))
        .on_focus_within_change(Fn("focused").curry(cfg.token.run,n.id))
        .on_key_value("plain+enter",Fn("select_graph_agent").curry(cfg.token,selected,ref_name),());
    for key in [["plain+left",-1.0,0.0],["plain+right",1.0,0.0],["plain+up",0.0,-1.0],["plain+down",0.0,1.0]]{
        let other=neighbor(n.id,plan,key[1],key[2]);
        if other!=""{tip=tip.on_key_value(key[0],Fn("attend").curry(cfg.token.run,other,ref_key(data,other)),());}
    }
    if data.lead!=""{tip=tip.on_key_value("plain+home",Fn("attend").curry(cfg.token.run,data.lead,ref_key(data,data.lead)),());}
    let width=min(plan.w,if plan.small{78.0}else{100.0});
    let label=text(if plan.small{value(n,"short",n.name)}else{n.name}).with_key("label").accessibility_hidden(true)
        .with_style(ctx.component_style("node_label",style().min_width(px(0)).width(px(width)).typography("caption")
            .text_center().text_ellipsis().whitespace_nowrap().text_color(theme_color("text_primary"))));
    column([tip,label]).with_key("slot:"+n.id).with_style(style().absolute().left(px(max(0.0,min(plan.w-width,p.x-width*0.5))))
        .top(px(p.y-size*0.5)).width(px(width)).items_center().gap(theme_length("space.spacing_unit").times(1.25)))
}
fn inspector(ctx,cfg,plan,data,n){
    let place=plan.sp[n.id];let wide=min(310.0,plan.w-12.0);
    if wide<1.0{return ();}
    let inset=min(246.0,plan.h-12.0);
    let top=if place.y<plan.h*0.5{min(plan.h-inset-5.0,place.y+32.0)}else{max(5.0,place.y-inset-30.0)};
    let ref_name=ref_key(data,n.id);let close=ctx.t_or("agent_graph.close","Close inspector");
    let s=phase(n);let status=text(node_status(ctx,s)).with_key("status")
        .with_style(ctx.component_style(if s=="idle"{["status"]}else{["status","status_"+s]},style().typography("caption")));
    let header=[text(n.name).with_key("name").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("inspector_title",style().min_width(px(0)).flex_grow().typography("body").font_weight(600))),status,
        button::Button(#{key:"close-inspector",text:"×",size:"xs",variant:"ghost",on_click:Fn("select_graph_agent").curry(cfg.token,"",ref_name)})
            .with_key("close").accessibility_label(close).with_style(ctx.component_style("close",style().flex_shrink(false)))];
    let rows=[row(header).with_key("head").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("inspector_head",
        style().min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(1.25)).flex_wrap()))];
    for field in ["detail","doing"]{let v=opt(n,field);if v!=(){rows.push(note(ctx,v,"detail").with_key(field));}}
    let recents=[];
    for s in 0..data.calls.len{
        let c=data.calls[data.calls.len-s-1];if n.id!=c.from&&n.id!=c.to{continue;}
        let outgoing=n.id==c.from;let peer=if outgoing{data.rows[c.to]}else{data.rows[c.from]};
        let info=(if outgoing{"→ "}else{"← "})+peer.name;
        let items=[text(info).with_key("peer").with_style(ctx.component_style("call_peer",style().min_width(px(0)).flex_grow().typography("caption"))),
            note(ctx,call_status(ctx,c),"call_end").with_key("result")];
        let comment=opt(c,"label");if comment!=(){items.insert(1,note(ctx,comment,"call_label").with_key("comment"));}
        recents.push(row(items).with_key("call:"+c.id).accessibility_role("listitem")
            .accessibility_label(info+" · "+value(c,"label","")+" · "+call_status(ctx,c)).with_style(ctx.component_style("call",
                style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap())));
        if recents.len==4{break;}
    }
    if recents.len>0{rows.push(column(recents).with_key("calls").accessibility_role("list")
        .accessibility_label(ctx.t_or("agent_graph.calls","Recent calls")).with_style(ctx.component_style("calls",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(0.75)))));}
    rows.push(note(ctx,spend(ctx,value(n,"cost_micros",0),cfg.currency),"inspect_cost").with_key("cost"));
    column(rows).with_key("inspector").accessibility_role("region")
        .accessibility_label(ctx.t_or("agent_graph.inspector","Connections for {name}",#{name:n.name}))
        .with_style(ctx.component_style("inspector",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2.5)).gap(theme_length("space.spacing_unit").times(1.5)).radius(theme_length("space.unit").times(2.5))
            .background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border"))))
        .with_style(style().absolute().left(px(max(6.0,min(plan.w-wide-6.0,place.x-wide*0.5)))).top(px(top))
            .width(px(wide)).max_height(px(inset)).overflow_y_scroll()).scrollbars("hidden","auto")
}
fn legend(ctx){
    let bits=[];
    let labels=[["→",ctx.t_or("agent_graph.requests","Request")],["○",ctx.t_or("agent_graph.results","Reply")],
        ["─",ctx.t_or("agent_graph.connection","Connection")],["•",ctx.t_or("agent_trail.waiting","Needs you")]];
    for i in 0..labels.len{
        bits.push(row([text(labels[i][0]).accessibility_hidden(true).with_style(style().text_color(dot_ink(if i==3{"waiting"}else{"working"}))),
            note(ctx,labels[i][1],"legend_label")]).with_key("legend-"+i.to_string()).with_style(style().items_center().gap(theme_length("space.spacing_unit").times(0.75))));
    }
    row(bits).with_key("legend").with_style(ctx.component_style("legend",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2.5)).items_center().justify_center().flex_wrap()))
}
fn render_AgentGraph(ctx,props){
    let data=project(props);let ui=local(ctx,props.run);let stage_ref=element_ref("stage");
    let title=value(props,"title",ctx.t_or("agent_graph.title","Agent graph"));
    let controlled=opt(props,"on_select")!=();let selection=if controlled{value(props,"selected","")}else{ui.choice};
    let selected=if selection in data.rows{selection}else{""};
    let focus=if ui.cursor in data.rows{ui.cursor}else if data.lead!=""{data.lead}else if data.outer.len>0{data.outer[0]}else{""};
    let cfg=#{token:#{run:props.run,revision:props.revision,controlled:controlled},selected:selected,focus:focus,currency:value(props,"currency","USD")};
    let totals=[note(ctx,ctx.t_or("agent_graph.nodes","Nodes: {n}",#{n:ctx.format_number(props.nodes.len)}),"count").with_key("count"),
        note(ctx,ctx.t_or("agent_graph.rate","Calls/min: {n}",#{n:ctx.format_number(data.minute)}),"count").with_key("rate"),
        note(ctx,spend(ctx,data.money,cfg.currency),"total").with_key("total").accessibility_role("status")
            .accessibility_label(ctx.t_or("agent_tree.total","Run total: {amount}",#{amount:spend(ctx,data.money,cfg.currency)}))];
    let header=row([text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("title",style().min_width(px(0)).typography("body").font_weight(600))),
        row(totals).with_key("stats").with_style(ctx.component_style("stats",style().min_width(px(0)).flex_grow().gap(theme_length("space.spacing_unit").times(2.25)).justify_end().flex_wrap()))])
        .with_key("bar").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("bar",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap()));
    let inside=[];
    if props.nodes.len==0{inside.push(note(ctx,ctx.t_or("agent_graph.empty","No nodes yet"),"empty").with_key("empty"));}
    else{
        let m=ctx.element_size(stage_ref);let w=if m==(){480.0}else{max(1.0,m.width)};
        let plan=layout(props,data,w);
        let paths=#{};let related=#{};
        for key in data.order{
            let e=data.pairs[key];paths[key]=route(data,plan,e);
            if e.a==selected{related[e.b]=true;}else if e.b==selected{related[e.a]=true;}
        }
        let scene=[];let rings=guide(ctx,plan);if rings!=(){scene.push(rings);}
        scene.push(rails(ctx,plan,data,paths,selected));
        for motion in pulses(ctx,cfg,plan,paths,ui.slots){scene.push(motion);}
        for n in props.nodes{scene.push(at_node(ctx,cfg,plan,data,n,element_ref(ref_key(data,n.id)),n.id in related));}
        if selected!=""{let detail=inspector(ctx,cfg,plan,data,data.rows[selected]);if detail!=(){scene.push(detail);}}
        let layer=stack(scene).with_key("stage").with_ref(stage_ref).accessibility_role("group")
            .accessibility_label(ctx.t_or("agent_graph.diagram","Communication graph"))
            .with_style(ctx.component_style("stage",style().relative().min_width(px(0))))
            .with_style(style().width(relative(1.0)).height(px(plan.h)).clip());
        if selected!=""{layer=layer.on_bubble("key:plain+escape",Fn("select_graph_agent").curry(cfg.token,"",ref_key(data,selected)));}
        inside.push(layer);inside.push(legend(ctx));
    }
    let notice=opt(props,"notice");if notice!=(){inside.push(note(ctx,notice,"notice").with_key("notice").accessibility_role("status"));}
    effect("observe_calls",#{run:props.run,calls:data.phases},Fn("observe"),Fn("stop"));
    column([header,column(inside).with_key("panel").with_style(ctx.component_style("panel",
        style().min_width(px(0)).background(theme_color("surface_raised")).radius(theme_length("space.unit").times(3.25))))])
        .with_key(props.key).accessibility_role("group").accessibility_label(title).audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).padding(theme_length("space.spacing_unit").times(1)).gap(theme_length("space.spacing_unit").times(0.75))
            .background(theme_color("surface_hover")).radius(theme_length("space.unit").times(4.25))))
}
