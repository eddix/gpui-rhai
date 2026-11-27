/* gpui-rhai
{"id":"patterns/judge_bracket","export":"JudgeBracket","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/spinner"],"capabilities":{},"assets":[],"tokens":["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","typography.body","typography.caption","bracket.tray","bracket.card","bracket.ink","bracket.muted","bracket.faint","bracket.line","bracket.dash","bracket.hint","bracket.rail","bracket.won","bracket.lost","bracket.path","bracket.blue","bracket.tag","bracket.well","bracket.solid","bracket.on_solid","bracket.shine"],"environment":[]}
*/
// An ordered elimination bracket, with comparison and a Host-owned champion choice.
// Example: JudgeBracket(#{key:"judge",session:"drafts",contenders:[#{id:"a",name:"A"},#{id:"b",name:"B"}]})
import "components/spinner" as spin;

define_component(#{
    metadata:#{id:"patterns/judge_bracket","export":"JudgeBracket",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/spinner"],capabilities:#{},assets:[],tokens:["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","typography.body","typography.caption","bracket.tray","bracket.card","bracket.ink","bracket.muted","bracket.faint","bracket.line","bracket.dash","bracket.hint","bracket.rail","bracket.won","bracket.lost","bracket.path","bracket.blue","bracket.tag","bracket.well","bracket.solid","bracket.on_solid","bracket.shine"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of the evaluation. A new session resets local match selection."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted Host revision attached to winner proposals."},
        contenders:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable contender ID."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Display name of the attempt."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative entrant icon."},
            meta:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Supporting label shown with the output."},
            output:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Read-only draft text for the comparison; never executes."}
        }}},required:true,sensitive:true,doc:"Attempts in seed order. A trailing vacant seed is a true bye; an unresolved match is not."},
        capacity:#{schema:#{type:"integer",min:0,max:16},"default":#{type:"integer",value:0},required:false,sensitive:false,
            doc:"0 picks the smallest adequate bracket. Explicit capacities 2, 4, 8, and 16 preserve the layout as the field changes."},
        verdicts:#{schema:#{type:"map",values:#{type:"object",allow_unknown:false,fields:#{
            a:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Expected entrant ID in the first slot."},
            b:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Expected entrant ID in the second slot."},
            winner:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Winning entrant ID. Must be a or b."},
            reason:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Explanation shown only for a verdict still bound to this pairing."},
            scores:#{schema:#{type:"array",max_items:2,items:#{type:"number",min:-1000,max:1000}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Optional displayed scores in entrant order, not used to determine the winner."}
        }}},"default":#{type:"map",value:#{}},required:false,sensitive:true,
            doc:"Accepted judge decisions indexed by zero-based round:match. Both ordered entrant IDs must still match for a verdict to count."},
        judging:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"ID of the ready match the Host is evaluating."},
        judge:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Display name of the judge."},
        task:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short evaluation prompt shown when room is available."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Heading of this bracket."},
        selected_match:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional controlled match selection; explicit null uses the automatic selection."},
        pending:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Host is deciding a Use winner proposal."},
        on_pick:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Request use of the validated champion. Does not execute the chosen output."},
        on_selection_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a requested match ID or null for automatic selection."}
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        pick:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer"},required:true,sensitive:false},
            id:#{schema:#{type:"string"},required:true,sensitive:false},
            match_id:#{schema:#{type:"string"},required:true,sensitive:false}
        }},doc:"Explicit proposal for the current champion; never contains their output or judge's reason."},
        selection_change:#{payload:#{type:"optional",value:#{type:"string"}},doc:"A selected match, or null for automatic selection."}
    },slots:#{},parts:["root","head","mark","title","task","summary","live","bracket","round","round_active","wires","match","match_active","slot",
        "slot_empty","slot_lost","slot_won","slot_active","slot_champion","tile","name","score","waiting","panel","panel_head","panel_title","panel_status",
        "answers","answer","answer_won","answer_lost","answer_name","answer_meta","output","badge","reason","notice","actions","button"],
        effects:["prune"]},render:Fn("render_bracket")
});
fn JudgeBracket(p){render_component("patterns/judge_bracket",p)}
fn get(m,k,d){if m!=()&&k in m&&m[k]!=(){m[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn ink(name){let k="bracket."+name;theme_color(k)}
fn tx(ctx,k,v){ctx.t_or("judge_bracket."+k,v)}
fn ease(){cubic_bezier(0.32,0.72,0.24,1.0)}
fn paint(ctx,n,k,s){n.with_style(ctx.component_style(k,s))}
fn label(ctx,k,t,s){paint(ctx,text(t),k,s)}
fn line(){style().min_width(px(0)).typography("caption").text_color(ink("muted")).whitespace_nowrap().text_ellipsis()}
fn focus(){style().focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))}
fn click_key(n,f){n.on_click(f).on_key_value("plain+enter",f,()).on_key_value("plain+space",f,())}
fn match_ref(key){"match:"+key}
fn state(ctx,session){
    let s=ctx.get_state("ui");
    if s!=()&&s.session==session{s}else{#{session:session,picked:""}}
}
fn select_match(t,id,ctx,e){
    let s=state(ctx,t.session);
    if !t.controlled{s.picked=id;ctx.set_state("ui",s);}
    ctx.emit("selection_change",if id==""{()}else{id});handled()
}
fn focus_match(id,ctx,e){ctx.focus(match_ref(id));ctx.scroll_into_view(match_ref(id));handled()}
fn pick(t,id,m,ctx,e){ctx.emit("pick",#{session:t.session,revision:t.revision,id:id,match_id:m});handled()}
fn prune(ctx,x){
    let s=state(ctx,x.session);
    if s.picked==x.id{s.picked="";ctx.set_state("ui",s);}
}
fn noop(ctx,x){}
fn glyph(kind){
    let d=switch kind{
        "mark"=>"M2 3.5h3v9H2M5 8h4m1-2h4v4h-4z",
        "cup"=>"M5 3h6v4a3 3 0 0 1-6 0zm0 1H2v2a3 3 0 0 0 3 3m6-5h3v2a3 3 0 0 1-3 3m-3 1v2m-3 1h6",
        _=>"m3 8 3 3 7-7"
    };
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.35' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
        .with_style(style().width(u(3.5)).height(u(3.5)).flex_shrink(false)).accessibility_hidden(true)
}
fn round_name(ctx,round,size){
    if round==size-1{return tx(ctx,"final","Final");}
    if round==size-2{return tx(ctx,"semi","Semifinal");}
    if round==size-3{return tx(ctx,"quarter","Quarterfinal");}
    ctx.t_or("judge_bracket.round","Round {index}",#{index:round+1})
}
fn title_match(ctx,z,d){
    let name=round_name(ctx,z.round,d.size);
    if d.rounds[z.round].len>1{name+" "+ctx.format_number(z.index+1)}else{name}
}
fn score(ctx,v){ctx.format_number(v,#{min_fraction_digits:1,max_fraction_digits:1})}
fn contender(p,i){if i==(){()}else{p.contenders[i]}}
fn accepted(p,z){if z.state=="decided"&&z.key in p.verdicts{p.verdicts[z.key]}else{()}}
fn score_at(v,i){let xs=get(v,"scores",[]);if i<xs.len{xs[i]}else{()}}
fn placeholder(ctx,slot){
    if slot=="vacant"{tx(ctx,"no_opponent","No opponent")}else{tx(ctx,"to_be_decided","To be decided")}
}
fn match_status(ctx,p,z,m){
    if z.key==m.live{return ctx.t_or("judge_bracket.comparing","{judge} is comparing…",#{judge:get(p,"judge",tx(ctx,"the_judge","The judge"))});}
    switch z.state{
        "decided"=>ctx.t_or("judge_bracket.judged","Judged by {judge}",#{judge:get(p,"judge",tx(ctx,"the_judge","The judge"))}),
        "bye"=>tx(ctx,"advanced","Advanced without an opponent"),
        "waiting"=>tx(ctx,"waiting","Waiting for an earlier match"),
        "empty"=>tx(ctx,"empty","No attempts yet"),
        _=>tx(ctx,"unjudged","Not judged yet")
    }
}
fn header(ctx,p,m){
    let d=m.data;
    let caption=get(p,"title",tx(ctx,"title","Judge"));
    let title=label(ctx,"title",caption,style().min_width(px(0)).typography("body").font_weight(500).whitespace_nowrap().text_ellipsis())
        .accessibility_role("heading").accessibility_level(2);
    let bits=[paint(ctx,row([glyph("mark")]),"mark",style().text_color(ink("muted")).flex_shrink(false)),title];
    if !m.narrow&&get(p,"task","")!=""{bits.push(label(ctx,"task",p.task,line().typography("body").flex_grow()));}
    else{bits.push(box([]).with_style(style().flex_grow()));}
    if m.live!=""{
        let live=tx(ctx,"judging","Judging");
        bits.push(paint(ctx,row([spin::Spinner(#{key:"spinner",label:live,size:"xs"}).accessibility_hidden(true),text(live)])
            .with_key("live").accessibility_role("status").accessibility_label(live),"live",
            line().min_height(u(4.5)).items_center().gap(sp(1)).padding_x(sp(1.25)).radius(theme_radius("sm"))
                .background(ink("tag")).text_color(ink("blue")).flex_shrink(false)));
    }
    let name=if d.champion!=(){ctx.t_or("judge_bracket.wins","{winner} wins",#{winner:p.contenders[d.champion].name})}
        else{(if d.current==(){""}else{round_name(ctx,d.current,d.size)+" · "})+d.played.to_string()+"/"+d.total.to_string()};
    bits.push(label(ctx,"summary",name,line()).with_key("summary").accessibility_role("status").accessibility_label(name));
    paint(ctx,row(bits).with_key("header"),"head",style().min_width(px(0)).items_center().gap(sp(2)).padding_bottom(sp(3)))
}
fn badge(ctx,name){
    paint(ctx,row([text(name)]),"badge",line().min_height(u(4)).items_center().padding_x(sp(1.25)).radius(theme_radius("sm"))
        .background(ink("tag")).text_color(ink("blue")).flex_shrink(false))
}
fn tile(ctx,c){
    if get(c,"icon",())==(){return ();}
    paint(ctx,row([c.icon]).accessibility_hidden(true),"tile",style().width(u(4.5)).height(u(4.5)).items_center()
        .justify_center().flex_shrink(false).radius(theme_radius("sm")).background(ink("card")))
}
fn competitor_slot(ctx,p,m,z,i){
    let ci=if i==0{z.a}else{z.b};
    let st=if i==0{z.slot_a}else{z.slot_b};
    let c=contender(p,ci);let won=ci!=()&&z.winner==ci;
    let champion=won&&z.round==m.data.size-1&&m.data.champion==ci;
    let lost=ci!=()&&z.winner!=()&&!won;
    let selected=m.shown==z.key;
    let pair=[];
    if c!=(){
        let inner=[];let icon=tile(ctx,c);if icon!=(){inner.push(icon);}
        let font=line().text_color(if lost{ink("muted")}else{ink("ink")}).font_weight(500).flex_grow();
        let live=z.key==m.live;
        if live{font=font.glyph_ink_sweep(ink("shine"),0.25,1.5,0);}
        let name=label(ctx,"name",c.name,font).with_key("name");
        if live{
            name=name.motion(motion_keyframes("glyph_sweep",[#{offset:0.0,value:0.0},#{offset:1.0,value:1.0,easing:"linear"}],
                #{duration_ms:900,iterations:"infinite",intent:"decorative"}));
        }
        inner.push(name);
        if champion{inner.push(row([glyph("cup")]).with_style(style().text_color(ink("blue")).flex_shrink(false)));}
        let n=score_at(accepted(p,z),i);
        if n!=()&&!m.narrow{inner.push(label(ctx,"score",score(ctx,n),line().font_feature("tnum",1).flex_shrink(false)));}
        else if won&&!champion&&!m.narrow{inner.push(row([glyph("check")]).with_style(style().text_color(ink("blue"))));}
        let wrapped=row(inner).with_key("entry:"+c.id).with_style(style().min_width(px(0)).width(relative(1.0)).items_center().gap(sp(1.5)));
        if z.round>0{wrapped=wrapped.enter_motion(motion_transition("opacity",0.0,1.0,#{duration_ms:360,easing:ease(),intent:"decorative"}));}
        pair.push(wrapped);
    }else{
        let title=if st=="waiting"{tx(ctx,"to_be_decided","To be decided")}
            else if z.state=="bye"{tx(ctx,"no_opponent","No opponent")}else{""};
        pair.push(label(ctx,"waiting",title,line().text_color(ink("faint"))));
    }
    let parts=["slot"];if c==(){parts.push("slot_empty");}
    else if lost{parts.push("slot_lost");}
    else if won&&!champion{parts.push("slot_won");}
    if selected&&c!=()&&!lost{parts.push("slot_active");}
    if champion{parts.push("slot_champion");}
    let surface=style().min_width(px(0)).height(if m.narrow{u(9)}else{px(m.data.slot_height)}).items_center()
        .padding_x(sp(2.5)).radius(theme_radius("lg")).background(ink(if c==(){"hint"}else{"card"}));
    let surface=if c==(){surface.border(px(1)).border_color(ink("dash")).border_dashed()}else{surface};
    paint(ctx,row(pair).with_key(if i==0{"upper"}else{"lower"}),parts,
        if m.narrow{surface.flex_basis(px(0)).flex_grow()}else{surface.width(relative(1.0))})
        .paint_transition(280,"ease_out")
}
fn at(m,direction,key){
    let at=m.keys.index_of(key);if at<0{return "";}
    let pos=at+direction;
    if pos>=0&&pos<m.keys.len{m.keys[pos]}else{""}
}
fn nav(n,m,z){
    let d=m.data;let r=z.round;let local=d.rounds[r];let i=z.index;
    let previous=if m.narrow{at(m,-1,z.key)}else if i>0&&local[i-1].state!="empty"{local[i-1].key}else{""};
    let next=if m.narrow{at(m,1,z.key)}else if i+1<local.len&&local[i+1].state!="empty"{local[i+1].key}else{""};
    let child=if r+1<d.size{d.rounds[r+1][i/2].key}else{""};
    let parent=if r>0&&d.rounds[r-1][i*2].state!="empty"{d.rounds[r-1][i*2].key}else{""};
    let opts=[["plain+up",previous],["plain+down",next],["plain+home",m.keys[0]],["plain+end",m.keys[m.keys.len-1]]];
    if !m.narrow{opts.push([if m.rtl{"plain+left"}else{"plain+right"},child]);opts.push([if m.rtl{"plain+right"}else{"plain+left"},parent]);}
    for o in opts{if o[1]!=""{n=n.on_key_value(o[0],Fn("focus_match").curry(o[1]),());}}
    n
}
fn box_x(m,r){let x=r.to_float()*(m.col+28.0);if m.rtl{m.width-m.col-x}else{x}}
fn match_box(ctx,p,m,z){
    let atoms=[competitor_slot(ctx,p,m,z,0),competitor_slot(ctx,p,m,z,1)];
    let visible_title=title_match(ctx,z,m.data);
    let a=contender(p,z.a);let b=contender(p,z.b);
    let description=get(a,"name",placeholder(ctx,z.slot_a))+" · "+get(b,"name",placeholder(ctx,z.slot_b))+" · "+match_status(ctx,p,z,m)
        +(if z.winner==(){""}else{" · "+p.contenders[z.winner].name+" "+tx(ctx,"won","won")});
    let can=z.state!="empty";let shown=m.shown==z.key;
    let body=(if m.narrow{row(atoms)}else{column(atoms)}).with_key("contest").with_ref(element_ref(match_ref(z.key)))
        .accessibility_role("button").accessibility_label(visible_title).accessibility_description(description)
        .accessibility_pressed(shown).tab_stop(can).disabled(!can);
    if can{
        body=click_key(body,Fn("select_match").curry(m.ticket,if m.picked==z.key{""}else{z.key}));
        body=nav(body,m,z);
        if m.picked!=""{body=body.on_key_value("plain+escape",Fn("select_match").curry(m.ticket,""),());}
    }
    let parts=if shown{["match","match_active"]}else{["match"]};
    let style=focus().min_width(px(0)).width(relative(1.0)).gap(if m.narrow{sp(1)}else{px(max(0.0,z.yb-z.ya-m.data.slot_height))})
        .opacity(if m.picked!=""&&!shown{0.58}else{1.0}).focus(style().opacity(1.0)).hover(style().opacity(1.0));
    body=paint(ctx,body,parts,if can{style.cursor_pointer()}else{style}).paint_transition(160,"ease_out");
    let shape=if m.narrow{style().min_width(px(0)).relative()}
        else{style().absolute().left(px(box_x(m,z.round))).top(px(24.0+z.ya-m.data.slot_height/2.0)).width(px(m.col))};
    box([body]).with_key("match-box:"+z.key).with_style(shape)
}
fn rail(m,z,side){
    let x=box_x(m,z.round)+(if m.rtl{0.0}else{m.col});
    let end=box_x(m,z.round+1)+(if m.rtl{m.col}else{0.0});
    let y=24.0+if side==0{z.ya}else{z.yb};
    let aim=24.0+(z.ya+z.yb)*0.5;let joint=(x+end)*0.5;
    let direction=if aim<y{-1.0}else{1.0};let along=if m.rtl{-1.0}else{1.0};
    let arc=min(6.0,min(abs(aim-y)*0.5,abs(end-x)*0.25));
    [path_move(x,y),path_line(joint-along*arc,y),path_quadratic(joint,y+direction*arc,joint,y),
        path_line(joint,aim-direction*arc),path_quadratic(joint+along*arc,aim,joint,aim),path_line(end,aim)]
}
fn wire_style(ctx){ctx.component_style("wires",style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)))}
fn wires(ctx,p,m){
    let edges=[];let highlight=[[],[],[],[]];
    for round in m.data.rounds{
        for z in round{
            if z.round>=m.data.size-1{continue;}
            for side in 0..2{
                let id=if side==0{z.a}else{z.b};
                let segments=rail(m,z,side);let good=id!=()&&z.winner==id;
                let tone=if good{"won"}else if z.winner!=(){"lost"}else{"rail"};
                let key=z.key+":"+side.to_string();
                let length=28.0+abs(z.yb-z.ya)*0.5;
                let path=if tone=="lost"{fallback_dash_route(segments,2.0/length,4.0/length)}else{segments};
                edges.push(canvas_stroke_path(key,path,1.5,ink(tone)).stroke_cap("round"));
                if z.champion_path&&good{
                    highlight[z.round].push(canvas_stroke_path(key,segments,1.75,ink("path")).stroke_cap("round"));
                }
            }
        }
    }
    if edges.len==0{return [];}
    let bits=[canvas(canvas_scene(edges).view_box(0,0,m.width,m.data.height+24.0)).with_key("rails").accessibility_hidden(true).with_style(wire_style(ctx))];
    for i in 0..highlight.len{
        if highlight[i].len>0{
            bits.push(canvas(canvas_scene(highlight[i]).view_box(0,0,m.width,m.data.height+24.0))
                .with_key("champion-path:"+i.to_string()).with_style(wire_style(ctx)).accessibility_hidden(true)
                .motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:440,delay_ms:i*310,easing:ease(),intent:"decorative"}))
                .motion_replay_key(p.session+":"+p.contenders[m.data.champion].id));
        }
    }
    bits
}
fn board(ctx,p,m,ref){
    let parts=if m.narrow{[]}else{wires(ctx,p,m)};
    for i in 0..m.data.size{
        let name=round_name(ctx,i,m.data.size);
        let base=line().font_weight(500).text_color(ink(if m.data.current==i{"ink"}else{"faint"}));
        let base=if m.narrow{base.padding_top(sp(if i==0{1}else{3}))}
            else{base.absolute().left(px(box_x(m,i))).top(px(0)).width(px(m.col))};
        parts.push(label(ctx,if m.data.current==i{["round","round_active"]}else{["round"]},name.to_upper(),base).with_key("round:"+i.to_string()));
        for z in m.data.rounds[i]{parts.push(match_box(ctx,p,m,z));}
    }
    let s=style().relative().min_width(px(0)).width(relative(1.0)).gap(sp(1.5));
    paint(ctx,column(parts).with_key("bracket").with_ref(ref).accessibility_role("group").accessibility_label(tx(ctx,"schedule","Match bracket")),
        "bracket",if m.narrow{s}else{s.height(px(m.data.height+24.0))})
}
fn answer(ctx,p,m,z,i){
    let index=if i==0{z.a}else{z.b};let slot=if i==0{z.slot_a}else{z.slot_b};
    let c=contender(p,index);
    let title=get(c,"name",placeholder(ctx,slot));
    let win=index!=()&&z.winner==index;let lost=index!=()&&z.winner!=()&&!win;
    let heading=[];
    let icon=tile(ctx,c);if icon!=(){heading.push(icon);}
    heading.push(label(ctx,"answer_name",title,line().font_weight(500).text_color(ink("ink")).flex_grow()));
    if win{heading.push(badge(ctx,tx(ctx,"winner","Winner")));}
    let n=score_at(accepted(p,z),i);
    if n!=(){heading.push(label(ctx,"score",score(ctx,n),line().font_feature("tnum",1).flex_shrink(false)));}
    let bits=[row(heading).with_style(style().min_width(px(0)).items_center().gap(sp(1.5)))];
    if get(c,"meta","")!=""{bits.push(label(ctx,"answer_meta",c.meta,line().text_color(ink("faint")).font_feature("tnum",1)));}
    let content=get(c,"output","");
    let fallback=if index!=(){tx(ctx,"no_output","No output given.")}
        else if slot=="waiting"{tx(ctx,"waiting_answer","Waits for the earlier match.")}
        else{tx(ctx,"empty_answer","No entrant in this slot.")};
    let output=column([text(if content==""{fallback}else{content}).with_style(style().min_width(px(0)).typography("caption")
        .text_color(ink(if index==(){"muted"}else{"ink"})))])
        .with_key("output").accessibility_role("document").accessibility_label(if content==""{title}else{ctx.t_or("judge_bracket.answer","Answer of {name}",#{name:title})})
        .tab_stop(content!="").scrollbars("hidden","auto");
    bits.push(paint(ctx,output,"output",focus().min_width(px(0)).max_height(u(30)).overflow_y_scroll().padding(sp(2))
        .radius(theme_radius("sm")).background(ink("well"))));
    let parts=["answer"];if win{parts.push("answer_won");}if lost{parts.push("answer_lost");}
    paint(ctx,column(bits).with_key("answer:"+i.to_string()),parts,style().min_width(px(0)).gap(sp(1.5)).padding(sp(2.5))
        .radius(theme_radius("lg")).background(ink("card")))
}
fn detail(ctx,p,m,z){
    let name=title_match(ctx,z,m.data);
    let head=paint(ctx,row([label(ctx,"panel_title",name,line().font_weight(500).text_color(ink("ink")).flex_grow())
        .accessibility_role("heading").accessibility_level(3),
        label(ctx,"panel_status",match_status(ctx,p,z,m),line())]).with_key("panel-head"),"panel_head",
        style().min_width(px(0)).gap(sp(2)).items_center().padding_x(sp(2)).padding_y(sp(1)));
    let answers=paint(ctx,box([answer(ctx,p,m,z,0),answer(ctx,p,m,z,1)]).with_key("answers"),"answers",
        style().grid().grid_cols(if m.width<=450.0{1}else{2}).min_width(px(0)).gap(sp(1.5)));
    let bits=[head,answers];
    let decision=accepted(p,z);let reason=get(decision,"reason","");
    if reason!=""{bits.push(label(ctx,"reason",reason,style().typography("caption").min_width(px(0)).text_color(ink("muted"))
        .padding_x(sp(2))).with_key("reason"));}
    if z.stale{bits.push(label(ctx,"notice",tx(ctx,"stale","An earlier verdict did not match this pairing."),line())
        .with_key("stale").accessibility_role("status").accessibility_label(tx(ctx,"stale","An earlier verdict did not match this pairing.")));}
    if m.data.champion!=()&&z.round==m.data.size-1&&get(p,"on_pick",())!=(){
        let c=p.contenders[m.data.champion];let name=ctx.t_or("judge_bracket.use","Use {name}",#{name:c.name});
        let n=paint(ctx,row([text(name)]).with_key("use").accessibility_role("button").accessibility_label(name).disabled(p.pending).tab_stop(!p.pending),
           "button",focus().min_height(u(6.5)).padding_x(sp(2.5)).items_center().radius(theme_radius("sm")).cursor_pointer().typography("caption")
            .background(ink("solid")).text_color(ink("on_solid")).hover(style().opacity(0.86)).disabled(style().opacity(0.57)));
        let controls=[];
        if p.pending{controls.push(label(ctx,"notice",tx(ctx,"pending","Waiting for confirmation"),line()).accessibility_role("status"));}
        controls.push(if p.pending{n}else{click_key(n,Fn("pick").curry(m.ticket,c.id,z.key))});
        bits.push(paint(ctx,row(controls).with_key("actions"),"actions",style().min_width(px(0)).flex_wrap().gap(sp(2)).justify_end().items_center()));
    }
    let panel=column(bits).with_key("comparison:"+z.key).with_style(style().min_width(px(0)).gap(sp(1.5)))
        .enter_motion(motion_transition("opacity",0.0,1.0,#{duration_ms:220,easing:ease(),intent:"decorative"}));
    paint(ctx,column([panel]).with_key("match-details").accessibility_role("region").accessibility_label(tx(ctx,"match","Match details"))
        .layout_size_motion(z.key,"height",280,ease()),"panel",style().min_width(px(0)).padding(sp(1)).radius(theme_radius("lg")).background(ink("tray")))
}
fn render_bracket(ctx,p){
    if utf8_byte_count(p.session)==0||utf8_byte_count(p.session)>96{throw "JudgeBracket requires a stable session";}
    let d=judge_bracket_project(p.contenders,p.verdicts,p.capacity);
    let ref=element_ref("bracket");let sz=ctx.element_size(ref);
    let w=if sz==(){720.0}else{max(1.0,sz.width)};
    let col=(w-28.0*(d.size-1).to_float())/d.size.to_float();
    let by=#{};let keys=[];let last="";
    for round in d.rounds{
        for z in round{
            by[z.key]=z;if z.state!="empty"{keys.push(z.key);}
            if z.state=="decided"{last=z.key;}
        }
    }
    let ui=state(ctx,p.session);let controlled="selected_match" in p;
    let selected=if controlled{get(p,"selected_match","")}else{ui.picked};
    let picked=if selected in keys{selected}else{""};
    if ui.picked!=""&&!(ui.picked in keys){effect("prune",#{session:p.session,id:ui.picked},Fn("prune"),Fn("noop"));}
    let wanted=get(p,"judging","");let live=if wanted in by&&by[wanted].state=="open"{wanted}else{""};
    let shown=if picked!=""{picked}else if live!=""{live}else if d.champion!=(){(d.size-1).to_string()+":0"}
        else if last!=""{last}else if keys.len>0{keys[0]}else{"0:0"};
    let m=#{data:d,keys:keys,picked:picked,shown:shown,live:live,col:col,width:w,narrow:col<176.0,rtl:ctx.text_direction()=="rtl",
        ticket:#{session:p.session,revision:p.revision,controlled:controlled}};
    paint(ctx,column([header(ctx,p,m),board(ctx,p,m,ref),detail(ctx,p,m,by[shown])]).with_key(p.key),"root",
        style().width(relative(1.0)).min_width(px(0)).gap(sp(3)))
}
