/* gpui-rhai
{"id":"patterns/collaboration_stream","export":"CollaborationStream","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/textarea","components/dialog"],"capabilities":{},"assets":[],"tokens":["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","typography.body","typography.caption","typography.code","typography.metadata","stream.tray","stream.card","stream.bubble","stream.hover","stream.ink","stream.text","stream.muted","stream.faint","stream.fill","stream.well","stream.you","stream.line","stream.accent","stream.solid","stream.inverse","stream.wait_fg","stream.wait_bg","stream.blue","stream.violet","stream.cyan","stream.orange","stream.green","stream.pink","stream.shine"],"environment":[]}
*/
// A chronological, virtualized agent conversation with Host-owned messages and draft.
// Example: CollaborationStream(#{key:"feed",session:"review",agents:[],messages:[]})
import "components/textarea" as editor;
import "components/dialog" as dialog;

define_component(#{
    metadata:#{id:"patterns/collaboration_stream","export":"CollaborationStream",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/textarea","components/dialog"],capabilities:#{},assets:[],
        tokens:["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","typography.body","typography.caption","typography.code","typography.metadata","stream.tray","stream.card","stream.bubble","stream.hover","stream.ink","stream.text","stream.muted","stream.faint","stream.fill","stream.well","stream.you","stream.line","stream.accent","stream.solid","stream.inverse","stream.wait_fg","stream.wait_bg","stream.blue","stream.violet","stream.cyan","stream.orange","stream.green","stream.pink","stream.shine"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Conversation identity; changing it resets local interaction state and scroll."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted revision carried by send proposals."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accessible conversation title."},
        task:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional task description in the participant bar's accessible description."},
        agents:#{schema:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique participant ID. The IDs you and all are reserved."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible participant name."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional model label."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional artwork replacing the pixel mark."}
        }}},required:true,sensitive:false,doc:"Participants in the conversation."},
        messages:#{schema:#{type:"array",max_items:384,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable message ID."},
            from:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Participant ID or you."},
            to:#{schema:#{type:"array",max_items:24,items:#{type:"string"}},required:true,sensitive:false,doc:"Recipient IDs; use the single ID all for everyone."},
            kind:#{schema:#{type:"string",allowed:["ask","reply","handoff","share","question","spawn"]},required:true,sensitive:false,doc:"Message type; spawn is a quiet system event."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Plain message text, never evaluated."},
            at:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Caller-formatted time label."},
            reply_to:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Stable original message ID for quote navigation."},
            typing:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Show writing activity rather than unfinished text."},
            attachments:#{schema:#{type:"array",max_items:8,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable attachment ID within its message."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Display label, never a path or URL to fetch."},
                kind:#{schema:#{type:"optional",value:#{type:"string",allowed:["file","folder","table","tool","link"]}},required:false,sensitive:false,doc:"Preview presentation, inferred from the label if absent."},
                preview:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional supplied snapshot; never loads a file or executes a tool."},
                meta:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional preview caption."}
            }}},"default":#{type:"array",value:[]},required:false,sensitive:true,doc:"Inline attachment snapshots."},
            facts:#{schema:#{type:"array",max_items:8,items:#{type:"object",allow_unknown:false,fields:#{
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Fact name."},
                value:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Fact value."}
            }}},"default":#{type:"array",value:[]},required:false,sensitive:true,doc:"Facts shown for the selected message."},
            items:#{schema:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Item name."},
                detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional item detail."}
            }}},"default":#{type:"array",value:[]},required:false,sensitive:true,doc:"Related artifacts and tools for the selected message."}
        }}},required:true,sensitive:true,doc:"Accepted chronological messages; up to 384 rows and 512 KiB of transcript contents."},
        live:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Enable native tail following until the user scrolls away."},
        max_height:#{schema:#{type:"number",min:72,max:1400},"default":#{type:"float",value:520.0},required:false,sensitive:false,doc:"Transcript cap or, with a composer, fixed height in logical pixels."},
        selected:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional controlled message selection; null clears selection."},
        focus_agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional controlled participant focus; null means everyone."},
        on_selection_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives an optional proposed message ID."},
        on_focus_agent_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives an optional proposed participant ID."},
        draft:#{schema:#{type:"string"},"default":#{type:"string",value:""},required:false,sensitive:true,doc:"Controlled draft, retained until the Host accepts or clears it."},
        placeholder:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional composer placeholder."},
        pending:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disables editing and duplicate sends while the Host decides."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives proposed draft text."},
        on_send:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Enables the composer and receives a send proposal."}
    },state:#{fields:#{
        ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        scroll:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        arrived:#{schema:#{type:"ui_value"},"default":#{type:"null"}}
    }},events:#{
        selection_change:#{payload:#{type:"optional",value:#{type:"string"}},doc:"Propose a message selection."},
        focus_agent_change:#{payload:#{type:"optional",value:#{type:"string"}},doc:"Propose a participant focus."},
        change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Proposed draft text."}
        }},doc:"Propose a draft edit."},
        send:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current conversation identity."},
            revision:#{schema:#{type:"integer"},required:true,sensitive:false,doc:"Accepted revision at submission."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Nonblank trimmed content."},
            to:#{schema:#{type:"array",items:#{type:"string"},max_items:24},required:true,sensitive:false,doc:"Deduplicated recipients or the single ID all."},
            reply_to:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Original message identity when answering."}
        }},doc:"Propose a send; the SDK does not send or append messages."}
    },slots:#{},parts:["root","top","tabs","tab","tab_selected","mark","live","panel","viewport","row","system","bubble","own","active","body","meta","sender",
        "time","quote","quote_own","quote_text","quote_name","badge","needs","reply","chip","details","fact","item","latest","composer","editor","tools","at","send","targets",
        "target_person","target_overflow","mentions","cursor",
        "mention","preview","preview_meta","preview_table","preview_cell","empty"],effects:["arrived","accepted"]},render:Fn("render_stream")
});

fn CollaborationStream(p){render_component("patterns/collaboration_stream",p)}
fn val(p,k,d){if p!=()&&k in p&&p[k]!=(){p[k]}else{d}}
fn co(k){let token="stream."+k;theme_color(token)}
fn u(n){theme_length("space.unit").times(n)}
fn s(n){theme_length("space.spacing_unit").times(n)}
fn tr(c,k,v){c.t_or("stream."+k,v)}
fn paint(c,n,part,st){n.with_style(c.component_style(part,st))}
fn ink(){style().min_width(px(0)).typography("caption").text_color(co("muted"))}
fn ellip(){ink().whitespace_nowrap().text_ellipsis()}
fn ring(){style().focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))}
fn easing(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn pulse(){motion_keyframes("opacity",[#{offset:0.0,value:1.0},#{offset:0.5,value:0.35,easing:"ease_in_out"},
    #{offset:1.0,value:1.0,easing:"ease_in_out"}],#{duration_ms:1400,iterations:"infinite",intent:"feedback"})}
fn writing_pulse(delay,bottom){
    let ease="cubic_bezier(0.42,0,0.58,1)";
    motion_keyframes("opacity",[#{offset:0.0,value:1.0},#{offset:0.5,value:bottom,easing:ease},
        #{offset:1.0,value:1.0,easing:ease}],#{duration_ms:1200,delay_ms:delay,iterations:"infinite",intent:"decorative"})
}
fn press(n,f){n.on_click(f).on_key_value("plain+enter",f,()).on_key_value("plain+space",f,())}
fn plain(c,part,label,contents,st,action){
    let b=paint(c,row(contents).accessibility_role("button").accessibility_label(label).tab_stop(action!=()).disabled(action==()),part,
        ring().min_width(px(0)).items_center().merge(st));
    if action==(){b}else{press(b,action)}
}
fn identity(i){i!=""&&utf8_byte_count(i)<=96}
fn palette(i){["blue","violet","cyan","orange","green","pink"][i%6]}
fn participant_pattern(id){
    let h=0;
    for ch in id{
        let code=to_int(ch);
        // The authored hash takes the first UTF-16 unit of each scalar.
        let unit=if code>65535{55296+(code-65536)/1024}else{code};
        h=(h*31+unit)&0xffffffff;
    }
    for attempt in 0..64{
        let bits=(h>>(attempt%26))&63;
        let mask=0;let dots=0;
        for r in 0..3{
            let a=(bits>>(r*2))&1;
            let b=(bits>>(r*2+1))&1;
            mask+=(5*a+2*b)<<(r*3);dots+=2*a+b;
        }
        if dots>=5{return mask;}
        // Retain the reference's double-precision recurrence before its
        // uint32 conversion, including rounding for large products.
        h=to_int(h.to_float()*1103515245.0+12345.0)&0xffffffff;
    }
    341
}
fn participant(c,index,id){
    if id in index{index[id]}else{#{id:id,name:tr(c,"everyone","Everyone"),color:"muted",mask:0}}
}
fn source(c,p){
    let agents=#{"you":#{id:"you",name:tr(c,"you","You"),color:"ink",mask:511}};
    let raw=p.agents;
    for i in 0..raw.len{
        let a=raw[i];
        if !identity(a.id)||a.id=="all"||a.id in agents||a.name==""||utf8_byte_count(a.name)>256{throw "Invalid stream participant";}
        a.color=palette(i);a.mask=participant_pattern(a.id);agents[a.id]=a;
    }
    let by=#{};let answered=#{};let ordered=[];let spoke=#{};let bytes=0;
    for item in p.messages{
        let m=item;
        m.typing=val(m,"typing",false);
        m.attachments=val(m,"attachments",[]);
        m.facts=val(m,"facts",[]);
        m.items=val(m,"items",[]);
        if !identity(m.id)||m.id in by||!(m.from in agents){throw "Invalid collaboration message identity or sender";}
        if m.to.len==0||("all" in m.to&&m.to.len!=1){throw "Invalid collaboration recipient list";}
        for id in m.to{if id!="all"&&!(id in agents){throw "Unknown collaboration recipient";}}
        if val(m,"reply_to","")==m.id{throw "A collaboration message cannot quote itself";}
        bytes+=utf8_byte_count(m.text)+utf8_byte_count(val(m,"at",""));
        let unique=#{};
        for a in m.attachments{
            if !identity(a.id)||a.id in unique{throw "Invalid attachment identity";}unique[a.id]=true;
            bytes+=utf8_byte_count(a.label)+utf8_byte_count(val(a,"meta",""))+utf8_byte_count(val(a,"preview",""));
        }
        for f in m.facts{bytes+=utf8_byte_count(f.label)+utf8_byte_count(f.value);}
        for x in m.items{bytes+=utf8_byte_count(x.label)+utf8_byte_count(val(x,"detail",""));}
        if bytes>524288{throw "Collaboration transcript exceeds 512 KiB";}
        if m.from=="you"&&val(m,"reply_to","")!=""{answered[m.reply_to]=true;}
        if m.kind!="spawn"&&!(m.from in spoke){ordered.push(m.from);spoke[m.from]=true;}
        by[m.id]=m;
    }
    #{agents:agents,by:by,answered:answered,ordered:ordered}
}
fn view(c,session){
    let v=c.get_state("ui");
    if v!=()&&v.session==session{v}else{#{session:session,picked:(),agent:(),preview:(),reply:(),mentions:false,dismissed:false,targets:[],flash:(),seq:0,cursor:(),show_cursor:false,submitted:()}}
}
fn collection(session){"transcript:"+session}
fn selection(ticket,next,c,e){
    let v=view(c,ticket.session);
    if !ticket.controlled{v.picked=next;}
    if next!=(){v.cursor=next;}v.show_cursor=false;
    if !ticket.agent_controlled{v.agent=();}
    c.set_state("ui",v);c.emit("selection_change",next);
    if ticket.agent!=(){c.emit("focus_agent_change",());}handled()
}
fn choose_agent(ticket,next,c,e){
    let v=view(c,ticket.session);
    if !ticket.controlled{v.picked=();}
    if !ticket.agent_controlled{v.agent=next;}
    c.set_state("ui",v);c.emit("focus_agent_change",next);
    if ticket.selected!=(){c.emit("selection_change",());}handled()
}
fn follow(session,c,e){c.set_state("scroll",#{session:session,unseen:e.unseen,following:e.following});}
fn latest(session,c,e){c.scroll_collection_to_end(collection(session));handled()}
fn reveal(session,id,c,e){
    let v=view(c,session);v.flash=id;v.seq+=1;c.set_state("ui",v);
    c.scroll_collection_to_key(collection(session),"m:"+id);handled()
}
fn move_cursor(session,ids,initial,operation,c,e){
    let v=view(c,session);
    let old=ids.index_of(v.cursor);let i=if old<0{initial}else{old};
    let n=switch operation{"home"=>0,"end"=>ids.len-1,"down"=>min(i+1,ids.len-1),_=>max(i-1,0)};
    v.cursor=ids[n];v.show_cursor=true;c.set_state("ui",v);
    c.focus("stream-messages");c.scroll_collection_to_key(collection(session),"m:"+v.cursor);handled()
}
fn select_cursor(ticket,ids,initial,c,e){
    let v=view(c,ticket.session);let current=if v.cursor in ids{v.cursor}else{ids[initial]};
    selection(ticket,if current==ticket.selected{()}else{current},c,e)
}
fn preview(session,mid,aid,c,e){
    let v=view(c,session);v.preview=#{mid:mid,aid:aid};c.set_state("ui",v);handled()
}
fn close_preview(session,c,e){let v=view(c,session);v.preview=();c.set_state("ui",v);handled()}
fn reply(session,id,c,e){
    let v=view(c,session);v.reply=id;v.submitted=();c.set_state("ui",v);
    if id!=(){c.focus("composer");}handled()
}
fn toggle_mentions(session,c,e){let v=view(c,session);v.mentions=!v.mentions;c.set_state("ui",v);handled()}
fn dismiss_mentions(session,c,e){let v=view(c,session);v.mentions=false;v.dismissed=true;c.set_state("ui",v);handled()}
fn set_recipient(session,id,c,e){
    let v=view(c,session);
    if id in v.targets{v.targets.remove(v.targets.index_of(id));}else{v.targets.push(id);}
    v.mentions=false;c.set_state("ui",v);c.focus("composer");handled()
}
fn store_arrivals(c,data){
    let old=c.get_state("arrived");let fresh=#{};
    if old!=()&&old.session==data.session{
        for id in keys(data.ids){if !(id in old.ids){fresh[id]=true;}}
    }
    data.fresh=fresh;c.set_state("arrived",data);
}
fn noop(c,data){}
fn reset_accepted(c,receipt){
    let v=view(c,receipt.session);
    if v.submitted==receipt.submitted{
        v.reply=();v.targets=[];v.mentions=false;v.dismissed=false;v.submitted=();c.set_state("ui",v);
    }
}
fn changed(session,c,value){
    let v=view(c,session);
    if v.dismissed||v.mentions{v.dismissed=false;v.mentions=false;c.set_state("ui",v);}
    c.emit("change",#{text:value});
}
fn send(ticket,text,c,e){submit(ticket,c,text);handled()}
fn submit(ticket,c,value){
    let clean=value;clean.trim();
    if clean!=""&&utf8_byte_count(clean)<=16000{
        let v=view(c,ticket.session);v.submitted=ticket.revision;c.set_state("ui",v);
        c.emit("send",#{session:ticket.session,revision:ticket.revision,text:clean,to:recipients(ticket,clean),reply_to:ticket.reply});
    }
}
fn writing_dots(c,mask){
    let animate=c.motion_preference()=="normal";
    let groups=[];
    for phase in 0..5{
        let dots=[];
        for y in 0..3{
            let x=phase-y;
            if x<0||x>2{continue;}
            let on=(mask&(1<<(3*y+x)))!=0;
            dots.push(box([]).with_key("dot-"+y.to_string()).with_style(style().absolute()
                .left(u(0.5+1.125*x)).top(u(0.5+1.125*y)).width(u(0.75)).height(u(0.75))
                .radius(u(0.1875)).background(current_color()).opacity(if animate||on{1.0}else{0.16})));
        }
        let group=box(dots).with_key("diagonal-"+phase.to_string()).with_style(style().absolute()
            .left(px(0)).top(px(0)).width(u(4)).height(u(4)));
        groups.push(if animate{group.motion(writing_pulse(120*phase,0.2))}else{group});
    }
    box(groups).with_style(style().relative().width(u(4)).height(u(4)).flex_shrink(false))
}
fn mark(c,a,busy){
    let icon=val(a,"icon",());
    let body=icon;
    if icon==()&&busy{body=writing_dots(c,a.mask);}
    else if icon==(){
        let dots="";
        for i in 0..9{
            if (a.mask&(1<<i))==0{continue;}
            let x=2.0+4.5*(i%3);let y=2.0+4.5*(i/3);
            dots+="<rect x='"+x.to_string()+"' y='"+y.to_string()+"' width='3' height='3' rx='0.75'/>";
        }
        body=svg("<svg viewBox='0 0 16 16' fill='currentColor'>"+dots+"</svg>")
            .accessibility_hidden(true).with_style(style().width(relative(1.0)).height(relative(1.0)));
    }
    let n=paint(c,row([body]).accessibility_hidden(true),"mark",style().width(u(4)).height(u(4)).items_center()
        .justify_center().text_color(co(a.color)).flex_shrink(false));
    if busy&&icon!=(){n.motion(writing_pulse(0,0.35))}else{n}
}
fn glyph(icon){
    let d=switch icon{
        "folder"=>"M2 5a1 1 0 0 1 1-1h4l1.5 2H13a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z",
        "table"=>"M2.5 3.5h11v9h-11zM2.5 7h11M6 7v5.5M10 7v5.5",
        "link"=>"M9.5 3.5h3v3m0-3L7 9m.5-4H4.25A1.25 1.25 0 0 0 3 6.25v5.5C3 12.44 3.56 13 4.25 13h5.5c.69 0 1.25-.56 1.25-1.25V8.5",
        "tool"=>"m4 5 3 3-3 3m5 0h3M2 3h12v10H2z",
        "send"=>"M8 12.5v-9m-4 4 4-4 4 4",
        "close"=>"m4.5 4.5 7 7m0-7-7 7",
        _=>"M4 2.5h5.5l3 3V13a.5.5 0 0 1-.5.5H4a.5.5 0 0 1-.5-.5V3a.5.5 0 0 1 .5-.5zm5 0V6h3.5M6 8.5h4M6 11h4"
    };
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.2' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
        .accessibility_hidden(true).with_style(style().width(u(3.5)).height(u(3.5)).flex_shrink(false))
}
fn kind(a){
    let name=a.label.to_lower();
    val(a,"kind",if name.ends_with("/"){"folder"}else if name.ends_with(".csv")||name.ends_with(".tsv"){"table"}else{"file"})
}
fn quotes(c,m){
    let q=m.quote;let people=m.people;
    if m.same&&q==(){return ();}
    let parts=if m.from=="you"{["quote","quote_own"]}else{["quote"]};
    let frame=style().min_width(px(0)).self_start().max_width(relative(1.0)).gap(s(0.25)).padding_x(s(2)).padding_y(s(1))
        .radius(theme_radius("sm")).background(co("well"));
    let name_style=style().min_width(px(0)).items_center().gap(s(1.25));
    if q==(){
        let line=[text(tr(c,"to","to")).with_style(ellip())];
        for i in 0..min(3,people.len){
            if people.len>3&&i==2{line.push(text("+"+(people.len-2).to_string()).with_style(ellip()));break;}
            let a=people[i];if a.id!="all"{line.push(mark(c,a,false));}line.push(text(a.name).with_style(ellip().text_color(co("text"))));
        }
        return paint(c,column([paint(c,row(line),"quote_name",name_style)]),parts,frame).with_key("recipient");
    }
    let label=c.t_or("stream.jump_quote","Show message from {name}",#{name:q.person.name});
    let names=paint(c,row([mark(c,q.person,false),text(q.person.name).with_style(ellip().font_weight(500).text_color(co("text")))]),"quote_name",name_style);
    press(paint(c,column([names,paint(c,text(q.text),"quote_text",ellip())]),parts,ring().merge(frame)
        .width(relative(1.0)).hover(style().background(co("fill")))).with_key("quote").accessibility_role("button").tab_stop(true)
        .accessibility_label(label),Fn("reveal").curry(m.ticket.session,q.id))
}
fn details(c,m){
    let rows=[];
    if m.facts.len>0{
        let facts=[];
        for f in m.facts{facts.push(paint(c,column([text(f.label).with_style(ellip()),text(f.value).with_style(ellip().text_color(co("ink")).font_weight(500))]),
            "fact",style().min_width(px(0)).gap(s(0.5))));}
        rows.push(row(facts).with_key("facts").with_style(style().min_width(px(0)).gap(s(3)).flex_wrap()));
    }
    if m.items.len>0{
        let items=[];
        for x in m.items{items.push(paint(c,row([glyph("file"),text(x.label).with_style(ellip().flex_grow().text_color(co("ink"))),
            text(val(x,"detail","")).with_style(ellip())]),"item",style().min_width(px(0)).items_center().gap(s(1.5))));}
        rows.push(column(items).with_key("items").with_style(style().gap(s(1))));
    }
    if rows.len==0{return ();}
    let label=c.t_or("stream.details","Details from {name}",#{name:m.author.name});
    paint(c,column(rows).with_key("details").accessibility_role("group").accessibility_label(label),"details",style().min_width(px(0)).gap(s(2))
        .padding(s(2.5)).radius(theme_radius("lg")).background(co("card")))
        .enter_motion(motion_transition("opacity",0.0,1.0,#{duration_ms:220,easing:easing(),intent:"decorative"}))
}
fn message_meta(c,m){
    let name=paint(c,text(m.author.name).with_key("sender"),"sender",ellip().font_weight(500).text_color(co("ink")));
    let bits=[name];
    if m.kind=="handoff"{bits.push(paint(c,text(tr(c,"handoff","Handoff")).accessibility_role("status"),"badge",ink().text_color(co("text"))
        .background(co("fill")).radius(theme_radius("sm")).padding_x(s(1.5)).self_center().flex_shrink(false)));}
    if m.needs{bits.push(paint(c,text(tr(c,"needs","Needs you")).accessibility_role("status"),"needs",ink().text_color(co("wait_fg")).radius(theme_radius("sm"))
        .background(co("wait_bg")).padding_x(s(1.5)).self_center().flex_shrink(false)));}
    if m.needs&&m.can_send{
        let t=c.t_or("stream.reply","Reply to {name}",#{name:m.author.name});
        bits.push(plain(c,"reply",t,[text(tr(c,"reply_caption","Reply"))],ink().text_color(co("text")).self_center()
            .flex_shrink(false).padding_x(s(1.5)).hover(style().background(co("fill")).text_color(co("ink"))),
            Fn("reply").curry(m.ticket.session,m.id)).with_key("reply").paint_transition(140,"ease_out"));
    }
    if !m.typing&&val(m,"at","")!=""{bits.push(paint(c,text(m.at).with_key("at"),"time",ellip().typography("metadata").text_color(co("faint"))
        .margin_start(auto()).padding_start(s(3)).font_feature("tnum",1).flex_shrink(false)));}
    paint(c,row(bits).with_key("meta"),"meta",style().min_width(px(0)).items_baseline().gap(s(1.5)).flex_wrap())
}
fn bubble(c,m,label,activate){
    let w=[];
    if !m.same{w.push(message_meta(c,m));}
    let quote=quotes(c,m);if quote!=(){w.push(quote);}
    let words=if m.typing{tr(c,"writing","Writing…")}else{m.text};
    let st=style().min_width(px(0)).typography("body").text_color(co(if m.selected{"ink"}else{"text"}));
    if !m.selected{st=st.line_clamp(3);}
    if m.typing{st=st.text_color(co("muted")).glyph_sweep(co("shine"),0.36,0);}
    let body=paint(c,text(if words==""{tr(c,"message","Message")}else{words}),"body",st).with_key("text");
    if m.typing{
        body=body.motion(motion_keyframes("glyph_sweep",[#{offset:0.0,value:0.0},#{offset:1.0,value:1.0,easing:"linear"}],
            #{duration_ms:1600,iterations:"infinite",intent:"decorative"}));
    }
    w.push(plain(c,"row",label,[body],style().self_start().max_width(relative(1.0)),activate)
        .with_key("message-control").accessibility_pressed(m.selected));
    if !m.typing&&m.attachments.len>0{
        let chips=[];
        for a in m.attachments{
            let t=c.t_or("stream.open_attachment","Open {name}",#{name:a.label});
            let icon=glyph(kind(a)).with_style(style().text_color(co("muted")));
            let caption=text(a.label).with_key("label").with_style(style().min_width(px(0)).whitespace_nowrap().text_ellipsis());
            chips.push(plain(c,"chip",t,[icon,caption],
                ink().text_color(co("text")).min_height(u(6)).max_width(relative(1.0)).gap(s(1.25))
                    .padding_start(s(1.75)).padding_end(s(2)).radius(theme_radius("sm")).background(co("card")),
                Fn("preview").curry(m.ticket.session,m.id,a.id)).with_key("file:"+a.id).paint_transition(140,"ease_out"));
        }
        w.push(row(chips).with_key("attachments").with_style(style().min_width(px(0)).padding_top(u(0.25)).gap(s(1.5)).flex_wrap()));
    }
    if m.selected{let info=details(c,m);if info!=(){w.push(info);}}
    let part=["bubble"];if m.from=="you"{part.push("own");}if m.selected{part.push("active");}
    let slab=paint(c,column(w).with_key("body"),part,style().min_width(px(0)).max_width(relative(1.0))
        .gap(s(1.5)).padding_x(s(3)).padding_y(s(2.5)).radius(theme_radius("lg"))
        .background(co(if m.from=="you"{"you"}else{"bubble"})).hover(style().background(co("hover")))).paint_transition(180,"ease_out");
    if m.flash!=(){slab=slab.motion(motion_transition("opacity",0.58,1.0,#{duration_ms:800,easing:easing(),intent:"feedback"})).motion_replay_key(m.flash);}
    slab
}
fn plain_person(a){
    #{id:a.id,name:a.name,color:a.color,mask:a.mask}
}
fn with_art(a,icons){
    if a.id in icons{a.icon=icons[a.id];}a
}
fn compose_row(icons,c,slot){
    let m=slot.item;
    if m.kind=="empty"{return paint(c,text(m.text).with_key(slot.key),"empty",ink().padding(s(4)));}
    m.author=with_art(m.author,icons);
    for i in 0..m.people.len{m.people[i]=with_art(m.people[i],icons);}
    if m.quote!=(){m.quote.person=with_art(m.quote.person,icons);}
    let activate=Fn("selection").curry(m.ticket,if m.selected{()}else{m.id});
    let label=m.author.name+": "+(if m.typing{tr(c,"writing","Writing…")}else{m.text});
    let nodes=();
    if m.kind=="spawn"{
        nodes=plain(c,"system",label,[text(m.author.name+" · "+m.text).with_style(ellip())],
            style().min_height(u(7.5)).padding_start(u(8)).width(relative(1.0)),activate).with_key("system");
    }else{
        let children=[];
        if m.from!="you"{children.push(column(if m.same{[]}else{[mark(c,m.author,m.typing).with_key("author-mark")]})
            .with_style(style().width(u(4)).padding_top(u(2.75)).flex_shrink(false)));}
        children.push(column([bubble(c,m,label,activate)]).with_style(style().min_width(px(0)).max_width(px(560)).flex_shrink(true)));
        nodes=row(children).with_style(style().min_width(px(0)).width(relative(1.0)).gap(s(2.5))
            .merge(if m.from=="you"{style().justify_end()}else{style()}));
    }
    let row_style=style().min_width(px(0)).padding_x(s(3)).padding_y(s(if m.same{0.5}else{1})).isolated_opacity(true)
        .opacity(if m.keep{1.0}else{0.32});
    let result=column([nodes]).with_key(slot.key).accessibility_role("group").with_style(row_style);
    if m.cursor{
        result=paint(c,result,"cursor",row_style.radius(theme_radius("sm"))
            .focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))).accessibility_current("true");
    }
    if m.fresh{
        result=result.motion(motion_transition("opacity",0.0,if m.keep{1.0}else{0.32},#{duration_ms:400,easing:easing(),intent:"decorative"}))
            .motion_replay_key("arrived:"+m.id);
    }
    result
}
fn focus_participant(ref,c,e){c.focus(ref);c.scroll_into_view(ref);handled()}
fn bar(c,p,d,ticket){
    let everyone=participant(c,d.agents,"all");
    let options=[everyone];for id in d.ordered{options.push(d.agents[id]);}
    let refs=[];for option in options{refs.push(element_ref("participant",option.id));}
    let tabs=[];let rtl=c.text_direction()=="rtl";
    for i in 0..options.len{
        let a=options[i];let current=if a.id=="all"{ticket.agent==()}else{ticket.agent==a.id};
        let parts=if current{["tab","tab_selected"]}else{["tab"]};
        let bits=[];if a.id!="all"{bits.push(mark(c,a,false));}
        bits.push(text(a.name).with_style(style().min_width(px(0)).whitespace_nowrap().text_ellipsis().max_width(px(160))));
        let st=ink().gap(s(1.75)).padding_x(s(2)).min_height(u(7)).radius(theme_radius("sm")).flex_shrink(false)
            .text_color(co(if current{"ink"}else{"muted"})).background(if current{co("card")}else{rgba(0x00000000)});
        if !current{st=st.hover(style().text_color(co("text")));}
        let n=plain(c,parts,a.name,bits,st,
            Fn("choose_agent").curry(ticket,if current||a.id=="all"{()}else{a.id}))
            .with_key(a.id).with_ref(refs[i]).accessibility_pressed(current)
            .accessibility_description(val(a,"model","")).paint_transition(160,"ease_out");
        for option in [[if rtl{"plain+right"}else{"plain+left"},max(0,i-1)],
            [if rtl{"plain+left"}else{"plain+right"},min(i+1,options.len-1)],["plain+home",0],["plain+end",options.len-1]]{
            n=n.on_key_value(option[0],Fn("focus_participant").curry(refs[option[1]].key),());
        }
        tabs.push(n);
    }
    let name=val(p,"title",tr(c,"title","Agent chat"));
    let strip=paint(c,row(tabs).with_key("agents").scrollbars("hidden","hidden").accessibility_role("toolbar")
        .accessibility_label(name).accessibility_description(val(p,"task","")),"tabs",style().min_width(px(0)).overflow_x_scroll()
        .scroll_edge_fade("x",32,32).padding(s(0.5)).gap(s(0.5)).flex_grow());
    let children=[strip];
    if p.live{
        let led=box([]).with_key("indicator").accessibility_hidden(true).with_style(style().width(u(1.5)).height(u(1.5))
            .radius(u(0.75)).background(co("accent"))).motion(pulse());
        let live=tr(c,"live","Live");
        children.push(paint(c,row([led,text(live)]).with_key("live").accessibility_role("status").accessibility_label(live),
            "live",ink().items_center().gap(s(1.5)).flex_shrink(false)));
    }
    paint(c,row(children),"top",style().min_width(px(0)).items_center().gap(s(3)).padding_bottom(s(1)))
}
fn project(c,p,model,ticket,v){
    let old=c.get_state("arrived");let track=old!=()&&old.session==p.session;
    let ids=#{};let data=[];let previous=();
    for item in p.messages{
        let m=model.by[item.id];
        ids[m.id]=true;
        let show= ticket.selected==m.id;
        let prev=previous;previous=m;
        let quoted=val(m,"reply_to","");
        let quote=if quoted in model.by&&(prev==()||prev.id!=quoted){model.by[quoted]}else{()};
        let x=m;x.key="m:"+m.id;x.ticket=ticket;x.selected=show;
        x.author=plain_person(model.agents[m.from]);x.people=[];
        for id in m.to{x.people.push(plain_person(participant(c,model.agents,id)));}
        x.quote=if quote==(){()}else{#{id:quote.id,text:if quote.typing{tr(c,"writing","Writing…")}else{quote.text},person:plain_person(model.agents[quote.from])}};
        x.same=prev!=()&&prev.kind!="spawn"&&prev.from==m.from&&prev.to==m.to&&!prev.typing&&m.kind!="question"&&m.kind!="handoff";
        x.needs=m.kind=="question"&&"you" in m.to&&!m.typing&&!(m.id in model.answered);
        x.can_send=val(p,"on_send",())!=();x.flash=if v.flash==m.id{p.session+":"+v.seq.to_string()}else{()};
        x.cursor=val(v,"show_cursor",false)&&m.id==v.cursor;
        x.keep=if ticket.selected!=(){show}else{ticket.agent==()||ticket.agent==m.from||ticket.agent in m.to||"all" in m.to};
        x.fresh=track&&(!(m.id in old.ids)||m.id in old.fresh);
        data.push(x);
    }
    effect("arrived",#{session:p.session,ids:ids},Fn("store_arrivals"),Fn("noop"));
    if data.len==0{data.push(#{key:"empty",kind:"empty",text:if p.live{tr(c,"waiting","Waiting for the first message…")}else{tr(c,"empty","No messages in this run")}});}
    data
}
fn transcript(c,p,d,ticket,v){
    let data=project(c,p,d,ticket,v);
    let label=tr(c,"messages","Collaboration messages");
    let cfg=#{key:collection(p.session),label:label,data:data,estimated_height:88,overdraw_pixels:180,content_padding_top:18,content_padding_bottom:20,
        alignment:if p.live{"bottom"}else{"top"},follow_tail:p.live,on_follow_change:Fn("follow").curry(p.session)};
    let cap=p.max_height;
    if val(p,"on_send",())!=(){cfg.height=cap;}else{cfg.max_height=px(cap);}
    let icons=#{};for a in p.agents{if val(a,"icon",())!=(){icons[a.id]=a.icon;}}
    let list=virtual_collection(cfg,Fn("compose_row").curry(icons)).accessibility_role("list").accessibility_label(label).tab_stop(true)
        .with_ref(element_ref("stream-messages"))
        .on_key_value("plain+escape",Fn("selection").curry(ticket,()),())
        .with_style(c.component_style("viewport",ring().min_width(px(0)).min_height(px(0)).edge_fade("y",24,24)));
    if p.messages.len>0{
        let ids=[];for m in p.messages{ids.push(m.id);}
        let initial=if p.live{ids.len-1}else{0};
        for key in ["home","end","up","down"]{list=list.on_key_value("plain+"+key,Fn("move_cursor").curry(p.session,ids,initial,key),());}
        list=list.on_key_value("plain+enter",Fn("select_cursor").curry(ticket,ids,initial),());
    }
    let children=[list];let status=c.get_state("scroll");
    if p.live&&status!=()&&status.session==p.session&&!status.following{
        let name=if status.unseen>0{c.t_or("stream.unseen","{count} new ↓",#{count:c.format_number(status.unseen)})}else{tr(c,"latest","Jump to latest")};
        let jump=plain(c,"latest",name,[text(name)],ink().text_color(co("ink")).padding_x(s(2.5)).min_height(u(6.5))
            .radius(u(3.25)).background(co("card")),Fn("latest").curry(p.session)).with_key("latest");
        children.push(row([jump]).with_key("footer").with_style(style().absolute().bottom(offset_px(7)).left(px(0)).right(px(0)).justify_center()));
    }
    paint(c,column(children).with_key("panel"),"panel",style().relative().min_width(px(0)).radius(theme_radius("lg")).background(co("card")))
}
fn named_recipients(agents,draft,explicit){
    let named=#{};let chunks=draft.to_lower().split("@");
    for i in 1..chunks.len{
        let prefix=chunks[i-1];
        if !(prefix==""&&i==1)&&!prefix.ends_with(" ")&&!prefix.ends_with("\n")&&!prefix.ends_with("\t"){continue;}
        let word=chunks[i];let match_id="";let longest=0;let unique=true;
        for a in agents{
            let candidate=a.name.to_lower();
            if !word.starts_with(candidate){continue;}
            let suffix=word.sub_string(candidate.len,1);
            if !(suffix in [""," ","\t","\n",",",".",":","!","?",";",")","，","。","！","？"]){continue;}
            let len=utf8_byte_count(candidate);
            if len>longest{longest=len;match_id=a.id;unique=true;}
            else if len==longest{unique=false;}
        }
        if unique&&match_id!=""{named[match_id]=true;}
    }
    let targets=[];for a in agents{if a.id in explicit||a.id in named{targets.push(a.id);}}
    targets
}
fn recipients(ticket,draft){
    let targets=named_recipients(ticket.agents,draft,ticket.explicit);
    if ticket.reply_agent!=()&&ticket.reply_agent!="you"&&!(ticket.reply_agent in targets){targets.insert(0,ticket.reply_agent);}
    if targets.len==0{targets.push("all");}targets
}
fn mention_query(draft){
    let bits=draft.split("@");let idx=bits.len-1;
    if idx<1{return ();}
    let word=bits[idx];let preceding=bits[idx-1];
    if word.contains(" ")||word.contains("\t")||word.contains("\n"){return ();}
    if !(idx==1&&preceding=="")&&!preceding.ends_with(" ")&&!preceding.ends_with("\t")&&!preceding.ends_with("\n"){return ();}
    #{prefix:draft.sub_string(0,draft.len-word.len-1),query:word.to_lower()}
}
fn complete(session,query,name,c,e){
    let v=view(c,session);v.mentions=false;v.dismissed=false;c.set_state("ui",v);
    c.emit("change",#{text:query.prefix+"@"+name+" "});c.focus("composer");handled()
}
fn complete_enter(session,name,c,value){
    let query=mention_query(value);
    if query!=()&&name.to_lower().contains(query.query){complete(session,query,name,c,());}
}
fn target_summary(c,d,targets){
    let names="";for who in targets{if names!=""{names+=", ";}names+=participant(c,d.agents,who).name;}
    let to=c.t_or("stream.targets","to {names}",#{names:names});
    let rail=ink().flex_grow().flex_basis(px(0)).items_center().gap_x(s(2)).overflow_hidden();
    let nowrap=style().whitespace_nowrap();
    let all=targets.len==1&&targets[0]=="all";
    let cells=[];
    if all{cells.push(text(to).with_style(nowrap));}
    else{
        cells.push(text(tr(c,"to","to")).with_style(nowrap));
        // Reserve the authored third slot for +N when there are more than
        // three recipients. Native fitting may omit additional whole cells.
        let display=if targets.len>3{2}else{targets.len};
        let fixed_hidden=targets.len-display;
        for i in 0..display{
            let person=participant(c,d.agents,targets[i]);
            let badge=mark(c,person,false).with_style(style().width(u(3.2)).height(u(3.2)));
            let label=text(person.name).with_style(nowrap);
            cells.push(paint(c,row([badge,label]).with_key("target:"+person.id),"target_person",
                style().items_center().gap(s(1.25)).text_color(co("text")).overflow_priority(display-i)));
        }
        for omitted in (if fixed_hidden>0{0}else{1})..(display+1){
            let number=fixed_hidden+omitted;
            cells.push(paint(c,text("+"+c.format_number(number)).with_key("summary:"+omitted.to_string())
                .overflow_summary_for(omitted),"target_overflow",nowrap.font_feature("tnum",1)));
        }
    }
    let group=row(cells).with_key("targets").accessibility_role("group").accessibility_label(to);
    paint(c,group,"targets",if all{rail}else{rail.overflow_row()})
}
fn composer(c,p,d,v){
    let enabled=val(p,"on_change",())!=()&&!p.pending;
    let cleared=v.submitted!=()&&p.revision>v.submitted&&p.draft=="";
    if cleared{effect("accepted",#{session:p.session,submitted:v.submitted},Fn("reset_accepted"),Fn("noop"));}
    let respond=if !cleared&&v.reply!=()&&v.reply in d.by{d.by[v.reply]}else{()};
    let draft=p.draft;let agents=[];for a in p.agents{agents.push(#{id:a.id,name:a.name});}
    let send_ticket=#{session:p.session,revision:p.revision,agents:agents,explicit:if cleared{[]}else{v.targets},
        reply:if respond==(){()}else{respond.id},reply_agent:if respond==(){()}else{respond.from}};
    let targets=recipients(send_ticket,draft);
    let content=[];
    if respond!=(){
        let a=d.agents[respond.from];let title=c.t_or("stream.answering","Replying to {name}",#{name:a.name});
        let cancel=plain(c,"at",tr(c,"cancel_reply","Cancel reply"),[glyph("close")],ink().justify_center().width(u(5)).height(u(5)),
            Fn("reply").curry(p.session,())).with_key("cancel");
        content.push(paint(c,column([row([mark(c,a,false),text(title).with_style(ellip().flex_grow()),cancel])
            .with_style(style().items_center().gap(s(1.5))),text(respond.text).with_style(ellip())]).with_key("reply-context"),
            "quote",style().min_width(px(0)).padding(s(2)).gap(s(0.5)).background(co("well")).radius(theme_radius("sm"))));
    }
    let query=if enabled&&!v.dismissed&&!v.mentions{mention_query(draft)}else{()};
    let hits=[];if enabled&&(v.mentions||query!=()){
        for a in p.agents{if query==()||a.name.to_lower().contains(query.query){hits.push(a);}}
    }
    let popup=();
    if hits.len>0{
        let choices=[];
        for i in 0..min(hits.len,if query==(){24}else{6}){
            let a=hits[i];
            let label=c.t_or("stream.name_agent","Address {name}",#{name:a.name});
            let choice=if query==(){Fn("set_recipient").curry(p.session,a.id)}else{Fn("complete").curry(p.session,query,a.name)};
            choices.push(plain(c,"mention",label,[mark(c,d.agents[a.id],false),text(a.name).with_style(ellip().flex_grow().text_color(co("ink"))),
                text(val(a,"model","")).with_style(ellip())],style().height(u(7.5)).gap(s(2)).padding_x(s(2))
                .radius(theme_radius("sm")).hover(style().background(co("fill"))),choice)
                .with_key(a.id).accessibility_pressed(!cleared&&a.id in v.targets));
        }
        popup=paint(c,column(choices).with_key("recipient-choices").accessibility_role("group").accessibility_label(tr(c,"agent_list","Agents")),
            "mentions",style().absolute().bottom(offset_relative(1.0)).left(px(8)).width(u(63)).max_width(relative(0.94))
                .max_height(u(46)).overflow_y_scroll().padding(s(1)).gap(s(0.5)).background(co("card")).radius(theme_radius("lg")));
    }
    let label=tr(c,"draft","Message the agents");let typed=draft;typed.trim();
    let allow=!p.pending&&typed!=""&&utf8_byte_count(typed)<=16000;
    let completing=query!=()&&hits.len>0;
    let on_submit=if completing{Fn("complete_enter").curry(p.session,hits[0].name)}else if allow{Fn("submit").curry(send_ticket)}else{()};
    // The composer owns the focus outline; the embedded editor owns the caret.
    let field=c.component_style("editor",style().background(rgba(0x00000000)).border(px(0)).shadows([])
        .focus(style().shadows([])).padding_x(s(1)).padding_y(px(0)));
    content.push(editor::Textarea(#{key:"draft:"+p.session,editor_ref:element_ref("composer"),label:label,value:draft,
        placeholder:val(p,"placeholder",tr(c,"placeholder","Message the agents – @ to name one")),min_rows:1,max_rows:1000,
        min_viewport_height:u(10),max_viewport_height:u(30),max_length:4000,
        disabled:p.pending,read_only:!enabled,part_styles:#{editor:field,
            placeholder:style().text_color(co("faint")),caret:style().background(co("accent"))},
        on_change:if enabled{Fn("changed").curry(p.session)}else{()},submit_key:if on_submit!=(){"enter"}else{()},on_submit:on_submit})
        .with_style(style().min_width(px(0))));
    let mention=plain(c,"at",tr(c,"mention","Name an agent"),[text("@")],ink().font_weight(500).justify_center().width(u(7))
        .height(u(7)).radius(u(3.5)).hover(style().background(co("fill"))),
        if enabled&&p.agents.len>0{Fn("toggle_mentions").curry(p.session)}else{()}).with_key("mention").accessibility_pressed(v.mentions);
    let button=plain(c,"send",tr(c,"send","Send message"),[glyph("send")],style().width(u(7)).height(u(7)).justify_center().radius(u(3.5))
        .background(co(if allow{"solid"}else{"fill"})).text_color(co(if allow{"inverse"}else{"faint"})),
        if allow{Fn("send").curry(send_ticket,draft)}else{()}).with_key("send").paint_transition(180,"ease_out");
    content.push(paint(c,row([mention,target_summary(c,d,targets),button]),"tools",style().min_width(px(0)).items_center().gap(s(2))));
    if popup!=(){content.push(popup);}
    let shell=paint(c,column(content).with_key("compose"),"composer",style().relative().min_width(px(0)).gap(s(2)).padding(s(2.5))
        .radius(theme_radius("lg")).background(co("card")).focus_within(style().focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))));
    if popup!=(){shell.on_key_value("plain+escape",Fn("dismiss_mentions").curry(p.session),())}else{shell}
}
fn preview_grid(c,rows){
    let cells=[];
    for y in 0..rows.len{
        let r=rows[y];
        for x in 0..r.len{
            let style=style().grid_cell(x+1,y+1).min_width(px(0)).typography("caption").font_feature("tnum",1)
                .font_weight(if y==0{600}else{400}).text_color(co(if y==0{"ink"}else{"text"}))
                .border_bottom(px(1)).border_color(co("line")).padding(s(1.5)).line_clamp(3);
            cells.push(paint(c,text(r[x]).selectable(true),"preview_cell",style));
        }
    }
    paint(c,box(cells).with_key("table").accessibility_role("group").accessibility_label(tr(c,"preview_table","Table preview")),
        "preview_table",style().grid().grid_cols(rows[0].len).min_width(px(0)))
}
fn attach_preview(c,p,d,v,ref){
    if v.preview==()||!(v.preview.mid in d.by){return ();}
    let m=d.by[v.preview.mid];let a=();
    for x in m.attachments{if x.id==v.preview.aid{a=x;break;}}
    if a==(){return ();}
    let g=kind(a);let agent=d.agents[m.from];let close=Fn("close_preview").curry(p.session);
    let caption=agent.name;let meta=val(a,"meta","");if meta!=""{caption+=" · "+meta;}
    let context=paint(c,row([mark(c,agent,false),text(caption).with_style(ellip())]),"preview_meta",style().min_width(px(0)).items_center().gap(s(1.25)));
    let content=val(a,"preview",tr(c,"no_preview","No preview supplied."));
    let body=[];
    if g=="folder"{
        for item in content.split("\n"){
            if item!=""{body.push(row([glyph(if item.ends_with("/"){"folder"}else{"file"}),text(item).with_style(ink().text_color(co("ink")))])
                .with_style(style().min_width(px(0)).items_center().gap(s(1.5))));}
            if body.len>=128{body.push(text(tr(c,"preview_more","More entries omitted")).with_style(ink()));break;}
        }
    }else if g=="table"{
        let cells=parse_delimited_preview(content,a.label.to_lower().ends_with(".tsv"));
        if cells==(){body.push(text(content).selectable(true).with_style(style().min_width(px(0)).typography("code").text_color(co("text"))));}
        else{body.push(preview_grid(c,cells));}
    }else{
        body.push(text(content).selectable(true).with_style(style().min_width(px(0)).text_color(co("text")).typography(if g=="table"||g=="tool"{"code"}else{"body"})));
    }
    let hide=plain(c,"at",tr(c,"close_preview","Close preview"),[glyph("close")],ink().justify_center().width(u(7)).height(u(7))
        .radius(theme_radius("sm")).hover(style().background(co("fill"))),close).with_key("close");
    let title=row([glyph(g),column([text(a.label).with_style(style().typography("body").font_weight(500).line_clamp(2)),context])
        .with_style(style().min_width(px(0)).flex_grow()),hide]).with_style(style().min_width(px(0)).items_center().gap(s(2)));
    let area=paint(c,column([title,column(body).with_style(style().min_width(px(0)).gap(s(1.5)))]),"preview",
        style().min_width(px(0)).gap(s(3)));
    dialog::Dialog(#{key:ref+":attachment",open:true,title:a.label,title_visible:false,initial_focus:"first",width:u(140),content:area,on_open_change:close})
}
fn render_stream(c,p){
    if !identity(p.session){throw "CollaborationStream requires a session identity";}
    let d=source(c,p);let v=view(c,p.session);
    let selected=if "selected" in p{p.selected}else{v.picked};
    let active=if "focus_agent" in p{p.focus_agent}else{v.agent};
    let ticket=#{session:p.session,controlled:"selected" in p,agent_controlled:"focus_agent" in p,
        selected:if selected!=()&&selected in d.by{selected}else{()},agent:if active!=()&&active in d.agents{active}else{()}};
    let ref=element_ref("root");
    let nodes=[bar(c,p,d,ticket),transcript(c,p,d,ticket,v)];
    if val(p,"on_send",())!=(){nodes.push(composer(c,p,d,v));}
    let modal=attach_preview(c,p,d,v,ref.semantic_id);if modal!=(){nodes.push(modal);}
    paint(c,column(nodes).with_key(p.key).with_ref(ref).accessibility_role("region").accessibility_label(val(p,"title",tr(c,"title","Agent chat"))),
        "root",style().width(relative(1.0)).min_width(px(0)).padding(s(1)).gap_y(s(1))
        .radius(theme_radius("lg")).background(co("tray")))
}
