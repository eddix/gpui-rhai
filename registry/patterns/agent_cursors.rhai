/* gpui-rhai
{"id":"patterns/agent_cursors","export":"AgentCursors","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/input","components/spinner","patterns/agent_tree","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","agent_cursors.card","agent_cursors.ink","border","focus_ring","radius.md","radius.pill","radius.sm","space.spacing_unit","space.unit","success","surface","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.code","typography.label","typography.title","warning"],"environment":[]}
*/
// Present a collaboratively reviewed document with live grapheme-anchored cursors.
// Example: AgentCursors(#{key:"review",document:"draft",revision:0,agents:[],blocks:[]})
import "components/button" as button;
import "components/input" as input;
import "components/spinner" as spinner;
import "patterns/agent_tree" as ids;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_cursors","export":"AgentCursors",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/input","components/spinner","patterns/agent_tree","patterns/approval_card"],capabilities:#{},
        tokens:["accent","agent_cursors.card","agent_cursors.ink","border","focus_ring","radius.md","radius.pill","radius.sm","space.spacing_unit","space.unit","success","surface","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.code","typography.label","typography.title","warning"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable retained review surface."},
        document:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Host-owned document identity. Replacing it resets local inspection and drafts."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision of the accepted document and review data used for a proposal."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Workspace heading."},
        height:#{schema:#{type:"integer",min:120,max:1200},"default":#{type:"integer",value:384},required:false,sensitive:false,doc:"Document viewport height in logical px."},
        fade:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Fade the sheet paint toward its bottom. Review controls, scrolling, hit regions and semantics are not masked."},
        tabs:#{schema:#{type:"optional",value:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable tab identity."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Visible tab label."},
        }}}},required:false,sensitive:true,doc:"Controlled tabs; blocks and reviews belong to the active tab."},
        active_tab:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted active tab. Required when tabs are supplied."},
        following:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted followed agent; omit for an undimmed document."},
        agents:#{schema:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable reviewer identity."},
            name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Reviewer name, limited to 256 UTF-8 bytes."},
            avatar:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative reviewer mark. Defaults to an initial."},
            ink:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:16777215}},required:false,sensitive:false,doc:"Optional RGB24 cursor/markup ink. Otherwise use the theme's primary ink."},
            cost_micros:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Accumulated cost in millionths of a US dollar."},
            cursor:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
                target:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current block, table cell, or comment thread ID."},
                at:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:16384}},required:false,sensitive:false,doc:"Extended-grapheme caret offset; omit for a pointer to a block's box."},
                in_suggestion:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Interpret offsets within this edit's proposed insertion, not the accepted block."},
                select:#{schema:#{type:"optional",value:#{type:"integer",min:1,max:16384}},required:false,sensitive:false,doc:"Selection length in graphemes or, for a box-only image pointer, request an image frame."},
                point:#{schema:#{type:"optional",value:#{type:"string",allowed:["center","corner","add_row"]}},required:false,sensitive:false,doc:"Placement of a box-only pointer. Add-row targets require a table."},
                doing:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short activity hint alongside the name."},
                typing:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Show live writing activity and a solid caret."},
                pressed:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Highlight the box target under the simulated press; no input is dispatched."},
            }}},required:false,sensitive:true,doc:"Accepted presence coordinates; omitted means away."},
        }}},required:true,sensitive:true,doc:"Up to 12 presence records with optional cursors. Text indices are extended grapheme offsets."},
        blocks:#{schema:#{type:"array",max_items:48,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable region identity, distinct from other blocks, cells and threads."},
            kind:#{schema:#{type:"string",allowed:["title","heading","paragraph","item","code","image","table"]},required:true,sensitive:false,doc:"Document block layout."},
            text:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Accepted copy for textual blocks, or the caption when kind is image."},
            src:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"An app-declared, prepared image asset ID, not a URL."},
            media:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional prepared image or custom illustration. Takes precedence over src."},
            alt:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Accessible image description and missing-asset fallback label."},
            width:#{schema:#{type:"optional",value:#{type:"number",min:0.2,max:1.0}},required:false,sensitive:false,doc:"Image width as a share of the document content width."},
            ratio:#{schema:#{type:"optional",value:#{type:"number",min:0.5,max:4.0}},required:false,sensitive:false,doc:"Image width divided by height."},
            float:#{schema:#{type:"optional",value:#{type:"string",allowed:["start","end"]}},required:false,sensitive:false,doc:"Place the image beside the following text in a wide pane, in logical direction."},
            loading:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Render the motion-aware placeholder without changing the image box."},
            rows:#{schema:#{type:"optional",value:#{type:"array",max_items:16,items:#{type:"array",max_items:8,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable anchor identity for this cell."},
                text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Accepted, anchorable cell content."},
                tone:#{schema:#{type:"optional",value:#{type:"string",allowed:["muted","info","success","warning"]}},required:false,sensitive:false,doc:"Optional status pill appearance; text still carries the meaning."},
            }}}}},required:false,sensitive:true,doc:"Rectangular table rows starting with the header."},
            add_row:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Offer a Host-owned row addition; emits the table ID."},
        }}},required:true,sensitive:true,doc:"Ordered document blocks. Table rows contain equally many keyed cells; row zero is the header."},
        suggestions:#{schema:#{type:"optional",value:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable proposal ID."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Author's reviewer ID."},
            block:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted text block, image caption, or cell target."},
            at:#{schema:#{type:"integer",min:0,max:16384},required:true,sensitive:false,doc:"Start grapheme in the accepted text."},
            remove:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:16384}},required:false,sensitive:false,doc:"Number of accepted graphemes to remove."},
            insert:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Proposed replacement or insertion."},
            note:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Reason shown in the suggestion inspector."},
        }}}},required:false,sensitive:true,doc:"Proposed, nonoverlapping text edits keyed by text target and accepted grapheme offsets."},
        comments:#{schema:#{type:"optional",value:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable thread identity, distinct from document targets."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Author's reviewer ID."},
            block:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Document block or cell where the thread is pinned."},
            at:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:16384}},required:false,sensitive:false,doc:"Optional grapheme offset. Without it use a box-relative pin."},
            x:#{schema:#{type:"optional",value:#{type:"number",min:0.0,max:1.0}},required:false,sensitive:false,doc:"Logical-leaf box fraction from the physical left for a box-relative pin."},
            y:#{schema:#{type:"optional",value:#{type:"number",min:0.0,max:400.0}},required:false,sensitive:false,doc:"Offset in logical px below the top of the box."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Accepted comment text."},
            replies:#{schema:#{type:"optional",value:#{type:"array",max_items:12,items:#{type:"string"}}},required:false,sensitive:true,doc:"Host-accepted replies, in order."},
            open:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Keep this thread open when there is no other local inspector."},
        }}}},required:false,sensitive:true,doc:"Host-accepted document threads; pin to a grapheme or use a position relative to a block."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives versioned review requests. Omit for a read-only review."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disable proposals while the Host is settling a mutation."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host reconciliation feedback."},
    },slots:#{},state:#{fields:#{
        ui:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
        cursor_frame:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
    }},
    events:#{action:#{payload:#{type:"object",allow_unknown:false,fields:#{
        document:#{schema:#{type:"string"},required:true,sensitive:false},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
        tab:#{schema:#{type:"string"},required:true,sensitive:false},
        kind:#{schema:#{type:"string",allowed:["follow","tab","accept","reject","accept_all","resolve","reply","add_row"]},required:true,sensitive:false},
        target:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business identity. Empty for stop-following and accept-all."},
        text:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Reply draft for kind reply only."},
    }},doc:"Validate the document, revision and tab; do not treat this proposal as accepted state."}},
    parts:["root","header","title","summary","presence","presence_face","presence_more","presence_following","presence_menu","tabs","tab","tab_active",
        "sheet","stage","page","page_narrow","flow","doc_title","heading","paragraph","item","code","caption","image","image_loading","table","table_row","table_header","cell","tone","add_row",
        "flag","flag_name","flag_activity","pin","popup","popup_title","popup_text","replies","commands","notice"],effects:["follow","reply_ack","prune_review","remember_cursors"]},
    render:Fn("render_cursors")
});

fn AgentCursors(props){render_component("patterns/agent_cursors",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,v){common::value_or(m,k,v)}
fn u(n){theme_length("space.unit").times(n)}
fn gap_unit(n){theme_length("space.spacing_unit").times(n)}
fn key_ok(id){ids::valid(id)&&utf8_byte_count(id)<=90}
fn check_key(id){if !key_ok(id){throw "AgentCursors IDs must contain 1–90 UTF-8 bytes without control characters";}}
fn rp(prefix,id){
    // Lossless, stable ref keys; neither visible position nor a lossy hash is identity.
    let alphabet="ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let bytes=id.to_blob();let out=prefix+":";
    for i in range(0,bytes.len,3){
        let a=bytes[i];let b=if i+1<bytes.len{bytes[i+1]}else{0};let c=if i+2<bytes.len{bytes[i+2]}else{0};
        out+=alphabet[a>>2].to_string()+alphabet[((a&3)<<4)|(b>>4)].to_string();
        if i+1<bytes.len{out+=alphabet[((b&15)<<2)|(c>>6)].to_string();}
        if i+2<bytes.len{out+=alphabet[c&63].to_string();}
    }
    out
}
fn target(id,kind,words){
    check_key(id);
    if words!=()&&utf8_byte_count(words)>12288{throw "AgentCursors block text exceeds 12 KiB";}
    #{id:id,kind:kind,text:words,count:if words==(){0}else{grapheme_count(words)},
        text_ref:if words==(){""}else{rp("text",id)},box_ref:rp("box",id),edits:[],text_marks:[],box_marks:[]}
}
fn insert_target(targets,t){
    if t.id in targets||targets.len()>=96{throw "AgentCursors needs distinct target IDs and permits at most 96 document targets";}
    targets[t.id]=t;
    targets
}
fn offset_ok(t,at,len){
    t.text!=()&&at>=0&&len>=0&&at<=t.count&&len<=t.count-at
}
fn compare_edits(a,b){a.at-b.at}
fn add_mark(t,m,at_text){
    if at_text{let items=t.text_marks;items.push(m);t.text_marks=items;}
    else{let items=t.box_marks;items.push(m);t.box_marks=items;}
    t
}
fn build_model(props){
    check_key(props.document);
    let store=#{agents:#{},tabs:#{},targets:#{},edits:#{},comments:#{},cost:0,has_cost:false};
    let targets=#{};let total=0;
    for a in props.agents{
        check_key(a.id);
        if a.id in store.agents||a.name==""||utf8_byte_count(a.name)>256{throw "AgentCursors needs distinct agents with bounded names";}
        let cents=value(a,"cost_micros",0);store.cost+=cents;store.has_cost=store.has_cost||opt(a,"cost_micros")!=();
        store.agents[a.id]=a;
    }
    if opt(props,"following")!=()&&!(props.following in store.agents){throw "AgentCursors following must be a known agent";}
    for t in value(props,"tabs",[]){
        check_key(t.id);if t.id in store.tabs||t.label==""{throw "AgentCursors needs distinct named tabs";}
        store.tabs[t.id]=t;
    }
    if store.tabs.len()>0&&!(value(props,"active_tab","") in store.tabs){throw "AgentCursors active_tab must be one of tabs";}
    for b in props.blocks{
        let words=if b.kind=="table"{()}else if b.kind=="image"{opt(b,"text")}else{value(b,"text","")};
        targets=insert_target(targets,target(b.id,b.kind,words));
        if words!=(){total+=utf8_byte_count(words);}
        if b.kind=="table"{
            let cols=0;
            for line in value(b,"rows",[]){
                if cols==0{cols=line.len;}
                if line.len==0||line.len!=cols{throw "AgentCursors table rows must have equally many cells";}
                for cell in line{
                    targets=insert_target(targets,target(cell.id,"cell",cell.text));total+=utf8_byte_count(cell.text);
                }
            }
            // Table addition is addressed by the table's ID and a separate mark role.
        }
    }
    for c in value(props,"comments",[]){
        if !(c.agent in store.agents)||!(c.block in targets){throw "AgentCursors comments need a known author and document block";}
        let at=opt(c,"at");
        if at!=()&&!offset_ok(targets[c.block],at,0){throw "AgentCursors comment offset is outside its block";}
        targets=insert_target(targets,target(c.id,"thread",c.text));
        targets[c.block]=add_mark(targets[c.block],#{role:"comment",id:c.id,at:at},at!=());
        total+=utf8_byte_count(c.text);
        for reply in value(c,"replies",[]){total+=utf8_byte_count(reply);}
        store.comments[c.id]=c;
    }
    for s in value(props,"suggestions",[]){
        check_key(s.id);
        if s.id in store.edits||!(s.agent in store.agents)||!(s.block in targets){throw "AgentCursors suggestions need distinct IDs, a known author and a text target";}
        let amount=value(s,"remove",0);let addition=value(s,"insert","");
        if targets[s.block].kind=="thread"||!offset_ok(targets[s.block],s.at,amount)||(amount==0&&addition==""){throw "AgentCursors suggestion is empty or outside its document text";}
        total+=utf8_byte_count(addition)+utf8_byte_count(value(s,"note",""));
        let t=targets[s.block];let edits=t.edits;edits.push(s);t.edits=edits;targets[s.block]=t;
        store.edits[s.id]=s;
    }
    if total>131072{throw "AgentCursors review text exceeds 128 KiB";}
    for id in targets.keys(){
        let t=targets[id];
        if t.edits.len!=0{
            let edits=t.edits;edits.sort(Fn("compare_edits"));let last=-1;let end=0;
            for s in edits{
                if s.at<end||s.at==last{throw "AgentCursors suggestions on one block may not overlap or share a start";}
                last=s.at;end=s.at+value(s,"remove",0);
            }
            t.edits=edits;targets[id]=t;
        }
    }
    for a in props.agents{
        let c=opt(a,"cursor");
        if c==(){continue;}
        if !(c.target in targets){throw "AgentCursors presence must target a current block or thread";}
        let t=targets[c.target];let at=opt(c,"at");let span=value(c,"select",0);let inside=opt(c,"in_suggestion");
        if opt(c,"point")=="add_row"&&t.kind!="table"{throw "AgentCursors add-row pointers belong to a table";}
        if t.kind=="thread"&&at==(){at=0;}
        if inside!=(){
            if at==()||!(inside in store.edits)||store.edits[inside].block!=t.id{throw "AgentCursors insertion cursor needs an edit in the same block";}
            let n=grapheme_count(value(store.edits[inside],"insert",""));
            if at>n||span>n-at{throw "AgentCursors insertion cursor is outside the insertion";}
        }else if at!=()&&!offset_ok(t,at,span){throw "AgentCursors cursor offset is outside its block";}
        targets[t.id]=add_mark(t,#{role:"cursor",id:a.id,at:at,select:span,inside:inside},at!=());
    }
    store.targets=targets;
    store
}
fn declare_boxes(props,model){
    let refs=#{};
    for id in model.targets.keys(){
        let target=model.targets[id];
        if target.kind!="thread"{refs[target.box_ref]=element_ref(target.box_ref);}
    }
    for b in props.blocks{
        if b.kind=="table"&&value(b,"add_row",false){
            let id=rp("add",b.id);refs[id]=element_ref(id);
        }
    }
    refs
}

fn agent_ink(a){if opt(a,"ink")==(){theme_color("agent_cursors.ink")}else{rgb(a.ink)}}
fn logical_byte(t,at,ends_at_boundary){
    let n=utf8_byte_count(grapheme_slice(t.text,0,at));
    for e in t.edits{
        // At a pure insertion, keep a starting caret before the proposal. At a
        // replacement's logical end a new caret goes after the proposed text.
        let end=e.at+value(e,"remove",0);
        if at>end||(at==end&&end!=e.at&&!ends_at_boundary){n+=utf8_byte_count(value(e,"insert",""));}
    }
    n
}
fn project_text(t,agents){
    let parts=[];let full="";let cursor=0;let positions=#{};
    for e in t.edits{
        let plain=grapheme_slice(t.text,cursor,e.at);
        if plain!=""{parts.push(span(plain));full+=plain;}
        let start=utf8_byte_count(full);cursor=e.at+value(e,"remove",0);
        let gone=grapheme_slice(t.text,e.at,cursor);let incoming=value(e,"insert","");let ink=agent_ink(agents[e.agent]);
        if gone!=""{parts.push(span(gone).color(ink).background(alpha(ink,0.06)).strikethrough());full+=gone;}
        let addition=utf8_byte_count(full);
        if incoming!=""{parts.push(span(incoming).color(ink).background(alpha(ink,0.12)).underline());full+=incoming;}
        positions[e.id]=#{start:start,addition:addition,end:utf8_byte_count(full),text:incoming};
    }
    let tail=grapheme_slice(t.text,cursor,t.count);
    if tail!=""{parts.push(span(tail));full+=tail;}
    if full==""{full="\u200b";parts.push(span(full));}
    if utf8_byte_count(full)>16384{throw "AgentCursors shaped text and insertions exceed 16 KiB in one target";}
    #{parts:parts,full:full,positions:positions}
}
fn project_mark(t,p,m){
    if value(m,"inside",())!=(){
        let e=p.positions[m.inside];
        #{start:e.addition+utf8_byte_count(grapheme_slice(e.text,0,m.at)),
            end:e.addition+utf8_byte_count(grapheme_slice(e.text,0,m.at+m.select))}
    }else{
        #{start:logical_byte(t,m.at,false),end:logical_byte(t,m.at+value(m,"select",0),true)}
    }
}
fn text_record(ctx,props,model,t,part,base,marks){
    let p=project_text(t,model.agents);let requests=[];let tags=[];
    for m in t.text_marks{
        let b=project_mark(t,p,m);
        let start=grapheme_index_at_byte(p.full,b.start,false);let tail=grapheme_index_at_byte(p.full,b.end,true);
        let k=":"+tags.len.to_string();let caret="point"+k;let range_id="range"+k;
        requests.push(#{id:caret,start:start});
        if value(m,"select",0)>0&&tail>start{requests.push(#{id:range_id,start:start,end:tail});}
        tags.push(#{role:m.role,id:m.id,point:caret,range:range_id});
    }
    for e in t.edits{
        let pos=p.positions[e.id];
        let start=grapheme_index_at_byte(p.full,pos.start,false);
        let end=grapheme_index_at_byte(p.full,pos.end,true);
        let k="edit:"+tags.len.to_string();
        requests.push(#{id:k,start:start});
        let range_id="mark:"+tags.len.to_string();
        if end>start{requests.push(#{id:range_id,start:start,end:end});}
        tags.push(#{role:"suggestion",id:e.id,point:k,range:range_id});
    }
    if requests.len>32{throw "AgentCursors permits at most 32 geometry requests per target (a selection or edit uses two)";}
    let ref=element_ref(t.text_ref);
    let node=text(p.parts).with_key("text:"+t.id).with_ref(ref)
        .with_style(ctx.component_style(part,base.min_width(px(0)).width(relative(1.0))));
    if requests.len>0{
        let plan=text_anchor_plan(p.full,props.revision,requests);
        node=node.capture_text_anchors(plan);
        marks.push(#{snapshot:ctx.text_anchors(ref,plan),tags:tags});
    }
    #{node:node,marks:marks}
}
fn default_text_part(kind){
    switch kind {
        "title"=>"doc_title", "heading"=>"heading", "item"=>"item", "code"=>"code", _=>"paragraph",
    }
}
fn details(ctx,s,part,base){text(s).with_style(ctx.component_style(part,base))}
fn ink_label(ctx,s,part){details(ctx,s,part,style().typography("caption").text_color(theme_color("text_muted")))}
fn text_block(ctx,props,model,b,marks){
    let t=model.targets[b.id];let part=default_text_part(b.kind);
    let base=style().typography("body").text_color(theme_color("text_primary"));
    if b.kind=="title"{base=base.typography("title").font_weight(650);}
    if b.kind=="heading"{base=base.typography("label").font_weight(600);}
    if b.kind=="code"{base=base.typography("code").padding(u(2.0)).background(theme_color("surface_hover")).radius(theme_radius("md"));}
    let item=text_record(ctx,props,model,t,part,base,marks);
    if b.kind=="title"||b.kind=="heading"{item.node=item.node.accessibility_role("heading").accessibility_level(if b.kind=="title"{2}else{3});}
    let block=column([item.node]);
    if b.kind=="item"{
        block=row([text("•").accessibility_hidden(true).with_style(style().flex_shrink(false)),item.node])
            .with_style(style().gap(gap_unit(1.0)).min_width(px(0)).width(relative(1.0)));
    }
    #{node:block.with_key("block:"+b.id).with_ref(model.boxes[t.box_ref]),marks:item.marks}
}
fn label_for(ctx,k,fallback){ctx.t_or("agent_cursors."+k,fallback)}
fn token(props){#{document:props.document,revision:props.revision,tab:value(props,"active_tab","")}}
fn send(token,kind,target,ctx,e){
    ctx.emit("action",#{document:token.document,revision:token.revision,tab:token.tab,kind:kind,target:target});handled()
}
fn inactive(props){props.busy||opt(props,"on_action")==()}
fn command(key,label,variant,props,kind,target){
    button::Button(#{key:key,text:label,variant:variant,size:"xs",disabled:inactive(props),
        on_click:Fn("send").curry(token(props),kind,target)})
}
fn image_holders(props,id){
    let agents=[];
    for a in props.agents{
        let c=opt(a,"cursor");
        if c!=()&&c.target==id&&opt(c,"at")==()&&opt(c,"select")!=(){agents.push(a);}
    }
    agents
}
fn image_holds_corner(a){opt(a.cursor,"point")=="corner"&&opt(a.cursor,"select")!=()&&opt(a.cursor,"at")==()}
fn fill_region(){style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))}
fn image_frame(a,followed){
    let dim=followed!=""&&followed!=a.id;
    let hue=alpha(agent_ink(a),if dim{0.44}else{1.0});
    let marks=[box([]).with_key("outline").with_style(fill_region()
        .border(px(1.5)).border_color(hue))];
    for i in 0..4{
        let grip=style().absolute().width(px(6)).height(px(6)).background(hue);
        grip=if i%2==0{grip.left(offset_px(-3))}else{grip.right(offset_px(-3))};
        grip=if i<2{grip.top(offset_px(-3))}else{grip.bottom(offset_px(-3))};
        marks.push(box([]).with_key("grip:"+i.to_string()).with_style(grip));
    }
    stack(marks).with_key("frame:"+a.id).accessibility_hidden(true)
        .with_style(fill_region())
}
fn image_block(ctx,props,model,b,marks,floated){
    let t=model.targets[b.id];let loading=value(b,"loading",false);
    let alt=value(b,"alt",label_for(ctx,"illustration","Document illustration"));
    let source=opt(b,"src");
    let body=if loading{
        let gleam=box([]).with_style(style().absolute().top(px(0)).left(relative(0.0)).width(relative(0.56)).height(relative(1.0))
            .background(theme_color("surface_raised"))).accessibility_hidden(true)
            .motion(motion_transition("translate_x",-100.0,450.0,#{duration_ms:1500,iterations:"infinite",intent:"decorative"}))
            .motion(motion_transition("opacity",0.2,0.55,#{duration_ms:1500,iterations:"infinite",autoreverse:true,intent:"decorative"}));
        stack([gleam,spinner::Spinner(#{key:"loading:"+b.id,label:label_for(ctx,"loading","Loading illustration"),size:"xs"})
            .with_style(style().absolute().top(u(2.0)).left(u(2.0)))])
            .with_style(ctx.component_style("image_loading",style().width(relative(1.0)).height(relative(1.0)).clip().background(theme_color("surface_hover"))))
    }else if opt(b,"media")!=(){b.media.with_style(style().width(relative(1.0)).height(relative(1.0)))
    }else if source!=(){
        image(asset(source)).accessibility_label(alt).image_fallback_text(alt).with_style(style().width(relative(1.0)).height(relative(1.0)))
    }else{
        column([ink_label(ctx,alt,"caption")]).with_style(style().width(relative(1.0)).height(relative(1.0)).items_center().justify_center())
    };
    let parts=[stack([body]).with_key("media-surface").with_style(ctx.component_style("image",fill_region()
        .clip().radius(theme_radius("md")).background(theme_color("surface_hover"))))];
    let holders=image_holders(props,b.id);
    let followed=value(props,"following","");
    if holders.len>0{parts.push(image_frame(holders[0],followed));}
    for a in holders{
        if image_holds_corner(a){parts.push(held_cursor(ctx,a,followed));}
    }
    let media=stack(parts).with_key("figure:"+b.id).with_ref(model.boxes[t.box_ref])
        .accessibility_role("group").accessibility_label(alt)
        .with_style(style().relative().width(relative(1.0)).aspect_ratio(value(b,"ratio",16.0/9.0)));
    let children=[media];
    if t.text!=(){
        let caption=text_record(ctx,props,model,t,"caption",style().typography("caption").text_color(theme_color("text_muted")),marks);
        children.push(caption.node);marks=caption.marks;
    }
    let share=value(b,"width",if floated{0.40}else{1.0});
    let shape=column(children).with_key("image:"+b.id)
        .with_style(style().width(relative(share)).max_width(relative(1.0)).min_width(px(0)).flex_shrink(false).gap(gap_unit(0.75)))
        .layout_size_motion(share.to_string(),"width",620,cubic_bezier(0.22,1.0,0.36,1.0));
    if opt(b,"float")=="end"&&!floated{shape=shape.with_style(style().self_end());}
    #{node:shape,marks:marks}
}
fn table_block(ctx,props,model,b,marks){
    let t=model.targets[b.id];let rows=[];let source=value(b,"rows",[]);
    for i in 0..source.len{
        let cells=[];
        for cell in source[i]{
            let ct=model.targets[cell.id];let col=style().min_width(px(0)).flex_basis(px(0)).flex_grow(1).padding_x(u(1.5)).padding_y(u(1.0));
            let ink=style().typography(if i==0{"label"}else{"body"}).text_color(theme_color("text_primary"));
            let tone=opt(cell,"tone");
            if tone!=(){
                let color=switch tone{"info"=>theme_color("accent"),"success"=>theme_color("success"),
                    "warning"=>theme_color("warning"),_=>theme_color("text_muted")};
                ink=ctx.component_style("tone",ink.background(alpha(color,0.12)).text_color(color).padding_x(u(0.75)).radius(theme_radius("sm")));
            }
            let r=text_record(ctx,props,model,ct,"cell",ink,marks);marks=r.marks;
            cells.push(column([r.node]).with_key("cell:"+cell.id).with_ref(model.boxes[ct.box_ref])
                .accessibility_role(if i==0{"columnheader"}else{"gridcell"}).with_style(col));
        }
        let item=row(cells).with_key("row:"+source[i][0].id).accessibility_role("row").with_style(ctx.component_style(
            if i==0{"table_header"}else{"table_row"},style().min_width(px(0)).width(relative(1.0))
                .border_bottom(px(1)).border_color(theme_color("border")).background(if i==0{theme_color("surface_hover")}else{theme_color("surface_raised")})));
        if i>0{
            item=item.layout_motion(260,"ease_out")
                .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:240,intent:"decorative"}));
        }
        rows.push(item);
    }
    let table=column(rows).with_key("data").accessibility_role("table").with_style(style().width(relative(1.0)).min_width(px(0)));
    let children=[table];
    if value(b,"add_row",false){
        let add=command("add:"+b.id,label_for(ctx,"add_row","+ Add row"),"ghost",props,"add_row",b.id)
            .with_style(ctx.component_style("add_row",style().width(relative(1.0)).justify_start()));
        children.push(column([add]).with_key("add-zone:"+b.id).with_ref(model.boxes[rp("add",b.id)]));
    }
    #{node:column(children).with_key("table:"+b.id).with_ref(model.boxes[t.box_ref])
        .layout_size_motion(source.len.to_string(),"height",420,cubic_bezier(0.22,1.0,0.36,1.0))
        .with_style(ctx.component_style("table",style().min_width(px(0)).min_height(px(0)).flex_shrink(false).width(relative(1.0)).clip()
            .background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border")).radius(theme_radius("md")))),marks:marks}
}

fn render_blocks(ctx,props,model){
    let nodes=[];let marks=[];let i=0;
    while i<props.blocks.len{
        let b=props.blocks[i];
        let floats=b.kind=="image"&&opt(b,"float")!=()&&i+1<props.blocks.len&&props.blocks[i+1].kind=="paragraph";
        let result=if b.kind=="image"{image_block(ctx,props,model,b,marks,floats)}
            else if b.kind=="table"{table_block(ctx,props,model,b,marks)}
            else{text_block(ctx,props,model,b,marks)};
        marks=result.marks;
        if floats{
            let items=[result.node.with_style(style().margin_bottom(u(2.0)))];
            while i+1<props.blocks.len&&props.blocks[i+1].kind=="paragraph" {
                i+=1;
                let adjacent=text_block(ctx,props,model,props.blocks[i],marks);marks=adjacent.marks;
                items.push(adjacent.node.with_style(style().width(relative(1.0)).flex_shrink(false)));
            }
            // A heading, list, code block or another figure clears this
            // float. The native column uses the same children when stacked.
            nodes.push(column(items).with_key("flow:"+b.id).leading_float(b.float,432,126)
                .with_style(ctx.component_style("flow",style().width(relative(1.0)).min_width(px(0))
                    .flex_shrink(false).gap_x(u(6.0)).gap_y(u(3.5)))));
        }else{nodes.push(result.node);}
        i+=1;
    }
    #{nodes:nodes,marks:marks}
}

fn ui(ctx,token){
    let s=ctx.get_state("ui");
    if s!=()&&s.document==token.document&&s.tab==token.tab{s}
    else{#{document:token.document,tab:token.tab,inspect:(),drafts:#{},expand:false,pending:(),dismissed:()}}
}
fn more(token,ctx,e){
    let s=ui(ctx,token);s.expand=!s.expand;ctx.set_state("ui",s);handled()
}
fn inspect(token,desc,ctx,e){
    let s=ui(ctx,token);
    if s.inspect!=desc{s.inspect=desc;s.dismissed=();ctx.set_state("ui",s);}handled()
}
fn trigger_hover(ctx,e){
    if e.hovered{inspect(e.value.token,e.value.desc,ctx,());}event_response()
}
fn trigger_focus(ctx,e){
    let s=ui(ctx,e.value.token);
    if e.focused&&s.dismissed!=e.value.desc{inspect(e.value.token,e.value.desc,ctx,());}event_response()
}
fn close(token,ref,desc,ctx,e){
    let s=ui(ctx,token);
    s.inspect=();s.dismissed=desc;ctx.set_state("ui",s);
    if ref!=""{ctx.focus(ref);}
    handled()
}
fn edit_draft(token,target,ctx,draft){
    let s=ui(ctx,token);
    if utf8_byte_count(draft)<=2048{s.drafts[target]=draft;ctx.set_state("ui",s);}
}
fn send_reply(token,target,seen,ctx,e){
    let s=ui(ctx,token);let draft=value(s.drafts,target,"");
    let trimmed=draft;trimmed.trim();if trimmed==""{return handled();}
    s.pending=#{id:target,text:draft,seen:seen};ctx.set_state("ui",s);
    ctx.emit("action",#{document:token.document,revision:token.revision,tab:token.tab,kind:"reply",target:target,text:draft});handled()
}
fn reply_received(s,model){
    let pending=s.pending;
    if pending==()||!(pending.id in model.comments){return false;}
    let accepted=value(model.comments[pending.id],"replies",[]);
    for i in pending.seen..accepted.len{if accepted[i]==pending.text{return true;}}
    false
}
fn follow(ctx,deps){if deps.target!=""{ctx.scroll_into_view(deps.target);}}
fn no_effect(ctx,deps){ }
fn apply_reply_ack(ctx,deps){
    let s=ui(ctx,deps.token);
    if s.pending!=()&&s.pending==deps.pending{
        if value(s.drafts,s.pending.id,"")==s.pending.text{s.drafts.remove(s.pending.id);}
        s.pending=();ctx.set_state("ui",s);
    }
}
fn inspect_alive(inspect,comments,edits){
    inspect!=()&&(if inspect.kind=="comment"{inspect.id in comments}else{inspect.id in edits})
}
fn prune_review(ctx,deps){
    let s=ui(ctx,deps.token);let changed=false;
    for id in s.drafts.keys(){
        if !(id in deps.comments){s.drafts.remove(id);changed=true;}
    }
    if s.pending!=()&&!(s.pending.id in deps.comments){s.pending=();changed=true;}
    if s.inspect!=()&&!inspect_alive(s.inspect,deps.comments,deps.edits){s.inspect=();changed=true;}
    if s.dismissed!=()&&!inspect_alive(s.dismissed,deps.comments,deps.edits){s.dismissed=();changed=true;}
    if changed{ctx.set_state("ui",s);}
}
fn clean_words(words,limit){grapheme_slice(words,0,min(limit,grapheme_count(words)))}
fn cost_label(ctx,cost){
    if cost>0&&cost<10000{return "<$0.01";}
    let n=cost.to_float()/1000000.0;let out="$"+n.to_string();
    try{out="$"+ctx.format_number(n,#{min_fraction_digits:2,max_fraction_digits:2});}catch{}
    out
}
fn face(a){
    let icon=opt(a,"avatar");
    if icon!=(){return icon.accessibility_hidden(true).with_style(style().width(u(3.5)).height(u(3.5)));}
    text(grapheme_slice(a.name,0,1).to_upper()).accessibility_hidden(true)
        .with_style(style().typography("caption").font_weight(600).text_color(agent_ink(a)))
}
fn describe_agent(ctx,a){
    let c=opt(a,"cursor");let activity=if c==(){label_for(ctx,"away","Not in document")}else{value(c,"doing","")};
    if activity==""{a.name}else{a.name+" · "+activity}
}
fn presence_frame(){
    style().padding(px(0)).width(u(6.5)).height(u(6.5)).radius(theme_radius("pill")).justify_center().items_center()
        .background(theme_color("agent_cursors.card")).cursor_pointer()
        .focus_ring(px(2),offset_px(-1),theme_color("focus_ring"))
}
fn presence(ctx,props,model,s,narrow){
    let agents=props.agents;let followed=value(props,"following","");let out=[];let shown=[];
    let visible=if narrow{3}else{6};
    for i in 0..min(visible,agents.len){shown.push(agents[i].id);}
    if followed!=""&&!(followed in shown){shown[shown.len-1]=followed;}
    for id in shown{
        let a=model.agents[id];let cursor=opt(a,"cursor");let on=id==followed;
        let badge=face(a);
        if cursor!=()&&value(cursor,"typing",false){
            badge=row([badge,spinner::Spinner(#{key:"typing:"+id,label:label_for(ctx,"typing","Writing"),size:"xs"})
                .accessibility_hidden(true).with_style(style().absolute().left(px(0)).top(px(0))
                    .width(relative(1.0)).height(relative(1.0)).text_color(agent_ink(a)))])
                .with_style(style().relative().width(relative(1.0)).height(relative(1.0)).items_center().justify_center());
        }
        let paper=theme_color("agent_cursors.card");
        let base=presence_frame();
        let part=if on{["presence_face","presence_following"]}else{["presence_face"]};
        if on{base=base.background(alpha(agent_ink(a),0.16).over(paper));}
        // Following is independent of whether a participant is currently here.
        if cursor==(){base=base.isolated_opacity(true).opacity(0.4);}
        let face_button=row([badge]).with_key("follow:"+id).accessibility_role("button").tab_stop(!inactive(props)).disabled(inactive(props))
            .accessibility_label((if on{label_for(ctx,"unfollow","Stop following ")}else{label_for(ctx,"follow","Follow ")})+a.name)
            .accessibility_description(describe_agent(ctx,a)).accessibility_pressed(on).with_style(ctx.component_style(part,base));
        if !inactive(props){face_button=face_button.on_click(Fn("send").curry(token(props),"follow",if on{""}else{id}));}
        out.push(face_button);
    }
    if shown.len<agents.len{
        // The overflow badge shares the avatars' material and height.
        // Keep Button as the native interaction owner for disclosure.
        let root=ctx.component_style(["presence_face","presence_more"], presence_frame().border(px(0))
            .width(auto()).min_width(u(6)).padding_x(gap_unit(1.25)).flex_shrink(false)
            .typography("caption").font_weight(600).letter_spacing_em(0)
            .font_feature("tnum",1).text_color(theme_color("text_muted")));
        out.push(button::Button(#{key:"more",text:if s.expand{"−"}else{"+"+ctx.format_number(agents.len-shown.len)},
                variant:"ghost",size:"xs",hoverable:false,part_styles:#{root:root},
                on_click:Fn("more").curry(token(props))})
            .with_key("more").accessibility_expanded(s.expand).accessibility_label(label_for(ctx,"more","More agents")));
    }
    let controls=row(out).with_key("faces").accessibility_role("group").accessibility_label(label_for(ctx,"presence","Agents in document"))
        .with_style(ctx.component_style("presence",style().flex_wrap().items_center().gap(gap_unit(0.75))));
    let overflow=[];
    if s.expand{
        for a in agents{
            if !(a.id in shown){
                overflow.push(command("follow:"+a.id,a.name,"ghost",props,"follow",a.id)
                    .accessibility_label(label_for(ctx,"follow","Follow ")+a.name).accessibility_description(describe_agent(ctx,a)));
            }
        }
    }
    #{row:controls,menu:if overflow.len==0{()}else{row(overflow).with_key("presence-menu").with_style(ctx.component_style(
        "presence_menu",style().min_width(px(0)).gap(gap_unit(1.0)).flex_wrap().padding_x(u(2.0))))}}
}
fn top_bar(ctx,props,model,s,narrow){
    let group=presence(ctx,props,model,s,narrow);
    let title=value(props,"title",label_for(ctx,"title","Collaborative document"));
    let head=[details(ctx,title,"title",style().typography("label").font_weight(600).text_color(theme_color("text_primary")))
        .with_key("title").accessibility_role("heading").accessibility_level(2),group.row];
    let ns=model.edits.len();
    let summary=if ns==0{label_for(ctx,"no_suggestions","No suggestions")}else{ns.to_string()+" "+
        (if ns==1{label_for(ctx,"suggestion","suggestion")}else{label_for(ctx,"suggestions","suggestions")})};
    if model.has_cost&&!narrow{summary+=" · "+cost_label(ctx,model.cost);}
    head.push(ink_label(ctx,summary,"summary").with_key("summary").accessibility_role("status"));
    if ns>1{head.push(command("accept-all",label_for(ctx,"accept_all","Accept all"),"secondary",props,"accept_all",""));}
    let bar=row(head).with_key("header").with_style(ctx.component_style("header",style().width(relative(1.0)).min_width(px(0))
        .flex_wrap().items_center().justify_between().padding_x(u(3.0)).padding_y(u(2.0)).gap(gap_unit(1.0))
        .border_bottom(px(1)).border_color(theme_color("border"))));
    // Disclosing the overflow never reparents the focused trigger.
    let children=[bar];if group.menu!=(){children.push(group.menu);}
    column(children).with_key("header-stack")
}
fn navigate_tabs(ctx,props){
    let tabs=value(props,"tabs",[]);let nodes=[];
    let active=value(props,"active_tab","");
    for i in 0..tabs.len{
        let t=tabs[i];let chosen=t.id==active;
        let next=tabs[(i+1)%tabs.len].id;let prev=tabs[(i+tabs.len-1)%tabs.len].id;let rtl=ctx.text_direction()=="rtl";
        let node=row([text(t.label)]).with_key("tab:"+t.id).accessibility_role("tab").accessibility_label(t.label)
            .accessibility_selected(chosen).disabled(inactive(props)).tab_stop(!inactive(props))
            .with_ref(element_ref(rp("tab",t.id)))
            .with_style(ctx.component_style(if chosen{["tab","tab_active"]}else{["tab"]},style().min_height(u(7.5))
                .items_center().typography("label").padding_x(u(3.0)).radius(theme_radius("sm"))
                .border(px(1)).border_color(rgba(0x00000000)).focus(style().border_color(theme_color("focus_ring")))
                .background(if chosen{theme_color("surface_raised")}else{theme_color("surface_hover")})))
            .paint_transition(160,cubic_bezier(0.25,0.1,0.25,1.0));
        if !inactive(props){
            node=node.on_click(Fn("send").curry(token(props),"tab",t.id))
                .on_key_value("left",Fn("tab_arrow"),#{token:token(props),target:if rtl{next}else{prev}})
                .on_key_value("right",Fn("tab_arrow"),#{token:token(props),target:if rtl{prev}else{next}});
        }
        nodes.push(node);
    }
    row(nodes).with_key("tabs").accessibility_role("tablist").accessibility_label(label_for(ctx,"tabs","Document views"))
        .with_style(ctx.component_style("tabs",style().flex_wrap().min_width(px(0)).gap(gap_unit(1.0))))
}
fn tab_arrow(ctx,e){send(e.token,"tab",e.target,ctx,())}

fn overlap(a,b){
    let x=max(a.x,b.x);let y=max(a.y,b.y);
    let end_x=min(a.x+a.width,b.x+b.width);let end_y=min(a.y+a.height,b.y+b.height);
    if end_x<=x||end_y<=y{()}else{#{x:x,y:y,width:end_x-x,height:end_y-y}}
}
fn pos(x,y,stage){#{x:x-stage.x,y:y-stage.y}}
fn quad_fill(q,stage){
    let a=pos(q[0].x,q[0].y,stage);let b=pos(q[1].x,q[1].y,stage);
    let c=pos(q[2].x,q[2].y,stage);let d=pos(q[3].x,q[3].y,stage);
    [path_move(a.x,a.y),path_line(b.x,b.y),path_line(c.x,c.y),path_line(d.x,d.y),path_close()]
}
fn point_visible(mark){
    mark!=()&&"caret" in mark&&mark.caret!=()&&mark.rects.len>0&&mark.rects[0].clipped!=()
}
fn pin_ref(desc){rp(if desc.kind=="suggestion"{"edit"}else{"pin"},desc.id)}
fn pin(ctx,props,desc,s,point,stage){
    let is_edit=desc.kind=="suggestion";
    let label=if is_edit{label_for(ctx,"inspect_suggestion","Inspect suggestion")}else{label_for(ctx,"open_comment","Open comment")};
    let mark=if is_edit{"✦"}else{"◇"};
    let event=#{token:token(props),desc:desc};
    let ref=element_ref(pin_ref(desc));
    let size=ctx.element_size(ref);
    let width=if size==(){24.0}else{size.width};
    let height=if size==(){21.0}else{size.height};
    let x=max(2.0,min(stage.width-width-2.0,point.x-width*0.4));
    let y=max(2.0,min(stage.height-height-2.0,point.y-height*0.86));
    row([text(mark).accessibility_hidden(true)]).with_key(pin_ref(desc)).accessibility_role("button").tab_stop(true)
        .on_click(Fn("inspect").curry(event.token,desc))
        .with_ref(ref).accessibility_label(label+" · "+desc.id).accessibility_expanded(s.inspect==desc)
        .on_hover_value(Fn("trigger_hover"),event).on_focus_within_value(Fn("trigger_focus"),event)
        .with_style(ctx.component_style("pin",style().absolute().left(px(x)).top(px(y)).width(u(6.0)).min_height(u(5.25))
            .items_center().justify_center().typography("caption").background(theme_color("surface_raised"))
            .border(px(1)).border_color(theme_color("border")).focus(style().border_color(theme_color("focus_ring")))
            .padding(px(0)).radius(theme_radius("sm"))))
}
fn cursor_flag(ctx,agent,followed,left,top,max_width){
    let c=agent.cursor;let dim=followed!=""&&followed!=agent.id;
    let first=[details(ctx,clean_words(agent.name,22),"flag_name",style().typography("caption").font_weight(600)
        .whitespace_nowrap()).with_key("name")];
    if opt(agent,"cost_micros")!=(){first.push(ink_label(ctx,cost_label(ctx,agent.cost_micros),"flag_activity")
        .with_key("cost").with_style(style().whitespace_nowrap()));}
    let lines=[row(first).with_style(style().items_center().gap(u(1.0)))];
    if !dim&&value(c,"doing","")!=""{lines.push(ink_label(ctx,clean_words(c.doing,32),"flag_activity").with_key("activity")
        .with_style(style().whitespace_nowrap().clip()));}
    column(lines).with_key("flag:"+agent.id).accessibility_role("status").accessibility_label(describe_agent(ctx,agent))
        .with_style(ctx.component_style("flag",style().absolute().left(offset_px(left)).top(offset_px(top))
            .max_width(px(max_width)).max_height(u(14.0)).padding_x(u(1.5)).padding_y(u(0.7)).clip()
            .radius(theme_radius("sm")).text_color(theme_color("text_primary"))
            .background(theme_color("surface_raised")).border(px(1)).border_color(agent_ink(agent))
            .isolated_opacity(dim).opacity(if dim{0.44}else{1.0})))
}
fn cursor_tip(a,followed,x,y){
    let dim=followed!=""&&followed!=a.id;
    let shape=[path_move(1.2,1.2),path_cubic(5.4,1.7,2.0,0.4,3.9,1.1),path_line(17.2,6.2),
        path_cubic(19.0,8.8,18.8,6.8,19.8,7.9),path_cubic(15.4,10.8,18.4,10.0,16.8,10.2),
        path_line(11.1,12.2),path_cubic(9.9,13.4,10.5,12.4,10.1,12.8),path_line(8.6,18.4),
        path_cubic(6.7,21.8,8.1,20.3,7.4,21.8),path_cubic(4.8,19.0,5.9,21.8,5.2,20.2),
        path_line(1.1,6.0),path_cubic(1.2,1.2,0.4,3.9,0.3,2.1),path_close()];
    let ink=alpha(agent_ink(a),if dim{0.36}else{1.0});
    let line=alpha(theme_color("surface_raised"),if dim{0.36}else{1.0});
    let pressed=value(a.cursor,"pressed",false);let size=if pressed{0.82}else{1.0};
    canvas(canvas_scene([canvas_stroke_path("edge",shape,2.1,line),canvas_fill_path("fill",shape,ink)]).view_box(0,0,20,24))
        .with_key("tip").accessibility_hidden(true)
        .with_style(style().absolute().left(offset_px(x)).top(offset_px(y)).width(u(4.75*size)).aspect_ratio(20.0/24.0))
        .layout_size_motion(pressed.to_string(),"width",140,"ease_out");
}
fn held_cursor(ctx,a,followed){
    // The point and frame use the real image layout in the same native paint
    // pass. They do not depend on a sampled script-side rectangle mid-resize.
    let w=170.0;let h=76.0;
    stack([cursor_tip(a,followed,w-1.2,h-1.2),cursor_flag(ctx,a,followed,w-144.0,h-44.0,142.0)])
        .with_key("held:"+a.id).with_style(style().absolute().right(px(0)).bottom(px(0)).width(px(w)).height(px(h)))
}
fn caret_node(a,followed,at,p,q){
    let typing=value(a.cursor,"typing",false);let dim=followed!=""&&followed!=a.id;
    let x=min(p.x,q.x)-2.0;let y=min(p.y,q.y)-2.0;
    let segment=[path_move(p.x-x,p.y-y),path_line(q.x-x,q.y-y)];
    let mark=canvas(canvas_scene([canvas_stroke_path("caret",segment,2,alpha(agent_ink(a),if dim{0.22}else{1.0}))]))
        .with_key("caret").accessibility_hidden(true).with_style(style().absolute()
            .left(offset_px(x-at.x)).top(offset_px(y-at.y)).width(px(abs(q.x-p.x)+4.0)).height(px(abs(q.y-p.y)+4.0)));
    if !typing{
        mark=mark.motion(motion_keyframes("opacity",[#{offset:0.0,value:1.0},#{offset:0.55,value:1.0},
            #{offset:0.551,value:0.0},#{offset:0.999,value:0.0},#{offset:1.0,value:1.0}],
            #{duration_ms:1100,iterations:"infinite",intent:"decorative"}));
    }
    mark
}
fn cursor_sprite(ctx,a,followed,at,stage,caret,in_thread,fade){
    let parts=[];let dy=0.0;let dx=0.0;
    if caret!=(){
        parts.push(caret_node(a,followed,at,caret.p,caret.q));
        dx=caret.q.x-at.x;dy=caret.q.y-at.y;
    }
    if !in_thread{
        parts.push(cursor_tip(a,followed,dx-1.2,dy-(if caret==(){1.2}else{6.0})));
        let left=max(2.0,min(stage.width-152.0,at.x+dx+15.0));
        let top=max(2.0,min(stage.height-46.0,at.y+dy+14.0));
        parts.push(cursor_flag(ctx,a,followed,left-at.x,top-at.y,min(146.0,stage.width-4.0)));
    }
    let typing=value(a.cursor,"typing",false);
    let w=min(180.0,stage.width);let h=min(stage.height,max(86.0,abs(dy)+72.0));
    if fade<1.0{
        parts=[stack(parts).with_key("cursor-ink")
            .with_style(style().width(relative(1.0)).height(relative(1.0)).isolated_opacity(true).opacity(fade))];
    }else{
        // Keep the retained owner the same on either side of the fade line.
        parts=[stack(parts).with_key("cursor-ink").with_style(style().width(relative(1.0)).height(relative(1.0)))];
    }
    stack(parts).with_key("cursor:"+a.id).with_style(style().absolute()
        .left(offset_px(at.x)).top(offset_px(at.y)).width(px(w)).height(px(h)))
        .layout_motion(if typing{110}else{560},if typing{"linear"}else{cubic_bezier(0.22,1.0,0.36,1.0)})
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:260,intent:"decorative"}))
}
fn caret_sprite(mark,stage,in_thread){
    let r=mark.rects[0].quad;let at=pos(mark.caret.x,mark.caret.y,stage);
    let caret=if r==(){()}else{#{p:pos(r[0].x,r[0].y,stage),q:pos(r[3].x,r[3].y,stage)}};
    #{at:at,caret:caret,in_note:in_thread}
}
fn presentation(){
    #{ink:[],sprites:#{},waiting:#{},anchors:[]}
}
fn present_text(model,records,stage,followed,out,in_thread){
    for rec in records{
        let pub=rec.snapshot;
        if pub==(){
            for tag in rec.tags{if tag.role=="cursor"{out.waiting[tag.id]=in_thread;}}
            continue;
        }
        for tag in rec.tags{
            let item=opt(pub,tag.point);
            if tag.role=="cursor"&&tag.range in pub{
                let n=0;let color=agent_ink(model.agents[tag.id]);
                for r in pub[tag.range].rects{
                    if r.clipped!=()&&r.quad!=(){
                        out.ink.push(canvas_fill_path("selection:"+tag.id+":"+n.to_string(),quad_fill(r.quad,stage),
                            alpha(color,if followed!=""&&followed!=tag.id{0.10}else{0.19})));n+=1;
                    }
                }
            }
            if !point_visible(item){continue;}
            if tag.role=="cursor"{out.sprites[tag.id]=caret_sprite(item,stage,in_thread);}
            else{
                let selected=if tag.role=="suggestion"&&tag.range in pub{pub[tag.range].rects}else{[]};
                let at=pos(item.caret.x,item.caret.y,stage);
                if selected.len>0&&selected[0].clipped!=(){
                    let bounds=selected[0].clipped;at=pos(bounds.x+bounds.width,bounds.y,stage);
                }
                out.anchors.push(#{kind:tag.role,id:tag.id,at:at,regions:selected});
            }
        }
    }
    out
}
fn box_cursor(a,kind,full,visible,stage,out){
    let c=a.cursor;let hue=agent_ink(a);
    let corner=pos(full.x+full.width,full.y+full.height,stage);
    let clipped=pos(visible.x,visible.y,stage);
    let at=if value(c,"point","")=="corner"{corner}else{pos(visible.x+visible.width/2.0,visible.y+visible.height/2.0,stage)};
    if value(c,"pressed",false){
        out.ink.push(canvas_rect("press:"+a.id,clipped.x,clipped.y,visible.width,visible.height,alpha(hue,0.12)));
    }
    let p=at;
    if !(kind=="image"&&image_holds_corner(a))&&p.x>=0.0&&p.y>=0.0&&p.x<=stage.width&&p.y<=stage.height{
        out.sprites[a.id]=#{at:p,caret:(),in_note:false};
    }
    out
}
fn bounds_for(ctx,ref,stage){
    if ref==(){return ();}
    let geom=ctx.element_bounds(ref);
    if geom==(){return ();}
    let visible=overlap(geom.visual,stage);
    if visible!=()&&geom.clip!=(){visible=overlap(visible,geom.clip);}
    if visible==(){()}else{#{full:geom.visual,visible:visible}}
}
fn present_boxes(ctx,model,stage,out){
    for key in model.targets.keys(){
        let t=model.targets[key];
        if t.box_marks.len==0{continue;}
        for m in t.box_marks{
            if m.role=="comment"{
                let bounds=bounds_for(ctx,opt(model.boxes,t.box_ref),stage);
                if bounds==(){continue;}
                let c=model.comments[m.id];
                let at=pos(bounds.full.x+bounds.full.width*value(c,"x",0.9),bounds.full.y+value(c,"y",0.0),stage);
                if at.x>=0.0&&at.y>=0.0&&at.x<=stage.width&&at.y<=stage.height{
                    out.anchors.push(#{kind:"comment",id:m.id,at:at,regions:[]});
                }
            }else if m.role=="cursor"{
                let a=model.agents[m.id];
                if t.kind=="image"&&image_holds_corner(a)&&!value(a.cursor,"pressed",false){continue;}
                let ref=if opt(a.cursor,"point")=="add_row"{rp("add",t.id)}else{t.box_ref};
                let b=bounds_for(ctx,opt(model.boxes,ref),stage);
                if b!=(){out=box_cursor(a,t.kind,b.full,b.visible,stage,out);}
            }
        }
    }
    out
}
fn hit_regions(props,anchor,stage){
    let desc=#{kind:anchor.kind,id:anchor.id};let nodes=[];let i=0;
    let event=#{token:token(props),desc:desc};
    for r in anchor.regions{
        if r.clipped==(){continue;}
        let b=r.clipped;let p=pos(b.x,b.y,stage);
        nodes.push(box([]).with_key("hit:"+pin_ref(desc)+":"+i.to_string()).accessibility_hidden(true)
            .on_hover_value(Fn("trigger_hover"),event).on_click(Fn("inspect").curry(event.token,desc))
            .with_style(style().absolute().left(px(p.x)).top(px(p.y)).width(px(b.width)).height(px(b.height))));
        i+=1;
    }
    nodes
}
fn popup(ctx,props,model,s,focus,stage){
    let is_edit=focus.kind=="suggestion";let desc=#{kind:focus.kind,id:focus.id};
    let data=if is_edit{model.edits[focus.id]}else{model.comments[focus.id]};
    let author=model.agents[data.agent];let popup_label=if is_edit{label_for(ctx,"suggested_edit","Suggested edit")}else{label_for(ctx,"thread","Comment thread")};
    let heading=[details(ctx,author.name,"popup_title",style().typography("label").font_weight(600))
        .with_key("author").accessibility_role("heading"),ink_label(ctx,popup_label,"summary").with_key("kind")];
    let children=[row(heading).with_style(style().gap(u(1.25)).items_center().flex_wrap()).with_key("title")];
    let actions=[];let marks=[];
    if is_edit{
        let note=opt(data,"note");
        if note!=(){children.push(ink_label(ctx,note,"popup_text").with_key("note"));}
        actions.push(command("reject:"+focus.id,label_for(ctx,"reject","Reject suggestion"),"ghost",props,"reject",focus.id));
        actions.push(command("accept:"+focus.id,label_for(ctx,"accept","Accept suggestion"),"primary",props,"accept",focus.id));
    }else{
        let info=text_record(ctx,props,model,model.targets[focus.id],"popup_text",style().typography("body"),[]);
        children.push(info.node);marks=info.marks;
        let replies=value(data,"replies",[]);let texts=[];
        for i in 0..replies.len{texts.push(ink_label(ctx,replies[i],"popup_text").with_key("reply:"+i.to_string()));}
        if texts.len>0{children.push(column(texts).with_key("replies").with_style(ctx.component_style("replies",style().gap(u(1.0))
            .padding_start(u(1.25)).border_start(px(1)).border_color(theme_color("border")))));}
        if opt(props,"on_action")!=(){
            children.push(input::Input(#{key:"reply:"+focus.id,label:label_for(ctx,"reply","Reply to comment"),
                placeholder:label_for(ctx,"reply_hint","Write a reply…"),value:value(s.drafts,focus.id,""),disabled:props.busy,
                on_change:Fn("edit_draft").curry(token(props),focus.id),on_submit:Fn("send_reply").curry(token(props),focus.id,replies.len)}));
        }
        actions.push(command("resolve:"+focus.id,label_for(ctx,"resolve","Resolve comment"),"secondary",props,"resolve",focus.id));
    }
    let closing=Fn("close").curry(token(props),pin_ref(desc),desc);
    actions.push(button::Button(#{key:"dismiss",text:label_for(ctx,"close","Close"),variant:"ghost",size:"xs",on_click:closing}));
    // Reserve the command row when the anchor is near the bottom and the
    // body must scroll. Otherwise a visually clipped button inherits a
    // rectangle which cannot actually receive a pointer click.
    let body=column(children).with_key("popup-body").with_style(style().min_height(px(0)).min_width(px(0))
        .flex_shrink(true).gap(u(1.25)).overflow_y_scroll());
    let footer=row(actions).with_key("commands").with_style(ctx.component_style("commands",style().justify_end()
        .flex_wrap().flex_shrink(false).gap(u(1.0))));
    let width=max(0.0,min(286.0,stage.width-12.0));
    let left=max(6.0,min(stage.width-width-6.0,focus.at.x-20.0));
    let top=max(6.0,min(stage.height-(if is_edit{124.0}else{186.0}),focus.at.y+10.0));
    let node=column([body,footer]).with_key("popup:"+pin_ref(desc)).accessibility_role("group").accessibility_label(popup_label)
        .on_key_value("escape",closing,())
        .with_style(ctx.component_style("popup",style().absolute().left(px(left)).top(px(top)).width(px(width))
            .min_width(px(0)).max_height(px(max(0.0,stage.height-top-6.0))).clip().padding(u(2.0))
            .gap(u(1.25)).radius(theme_radius("md")).border(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface_raised"))))
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:160,intent:"feedback"}));
    #{node:node,marks:marks}
}
fn paint_ink(key,commands){
    canvas(canvas_scene(commands)).with_key(key).accessibility_hidden(true)
        .with_style(style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)))
}
fn remember_cursors(ctx,frame){
    if ctx.get_state("cursor_frame")!=frame{ctx.set_state("cursor_frame",frame);}
}
fn fade_start(){0.50}
fn fade_end(){0.97}
fn pointer_fade(props,stage,sheet,at){
    if !props.fade||sheet.height<=0.0{return 1.0;}
    let fraction=(stage.y-sheet.y+at.y)/sheet.height;
    max(0.0,min(1.0,(fade_end()-fraction)/(fade_end()-fade_start())))
}
fn paint_cursors(ctx,props,model,out,g,sheet_bounds,followed){
    let previous=ctx.get_state("cursor_frame");
    let compatible=previous!=()&&previous.document==props.document&&previous.tab==value(props,"active_tab","")
        &&previous.width==g.width&&previous.height==g.height;
    let active=out.sprites;
    // A changed plan first mounts, then its actual text layout publishes.
    // Keep the outgoing position during that gap only. A clipped target,
    // missing presence, or a different document/tab never borrows its sprite.
    if compatible{
        for id in out.waiting.keys(){
            if !(id in active)&&id in previous.sprites&&previous.sprites[id].in_note==out.waiting[id]{
                active[id]=previous.sprites[id];
            }
        }
    }
    let frame=#{document:props.document,tab:value(props,"active_tab",""),width:g.width,height:g.height,sprites:active};
    effect("remember_cursors",frame,Fn("remember_cursors"),Fn("no_effect"));
    let sheet=[];let notes=[];
    for id in active.keys(){
        if !(id in model.agents)||opt(model.agents[id],"cursor")==(){continue;}
        let spot=active[id];
        let fade=if spot.in_note{1.0}else{pointer_fade(props,g,sheet_bounds,spot.at)};
        let sprite=cursor_sprite(ctx,model.agents[id],followed,spot.at,g,spot.caret,spot.in_note,fade);
        if spot.in_note{notes.push(sprite);}else{sheet.push(sprite);}
    }
    #{sheet:sheet,notes:notes}
}
fn layers(ctx,props,model,s,recs,stage,sheet_bounds){
    if stage==()||sheet_bounds==()||stage.visual.width<=12.0||stage.visual.height<=12.0{return #{ink:[],controls:[]};}
    let g=stage.visual;
    let followed=value(props,"following","");
    let out=present_text(model,recs,g,followed,presentation(),false);
    out=present_boxes(ctx,model,g,out);
    let pins=[];let pop=();
    let inspect=if inspect_alive(s.inspect,model.comments,model.edits){s.inspect}else{()};
    for c in value(props,"comments",[]){
        let desc=#{kind:"comment",id:c.id};
        if inspect==()&&value(c,"open",false)&&s.dismissed!=desc{inspect=desc;}
    }
    for a in out.anchors{
        let desc=#{kind:a.kind,id:a.id};
        if a.kind=="suggestion"{pins+=hit_regions(props,a,g);}
        pins.push(pin(ctx,props,desc,s,a.at,g));
        if inspect==desc{pop=popup(ctx,props,model,s,a,g);}
    }
    let thread_ink=[];
    if pop!=(){
        let cursors=present_text(model,pop.marks,g,followed,presentation(),true);
        for id in cursors.sprites.keys(){out.sprites[id]=cursors.sprites[id];}
        for id in cursors.waiting.keys(){out.waiting[id]=true;}
        thread_ink=cursors.ink;
    }
    let sprites=paint_cursors(ctx,props,model,out,g,sheet_bounds.visual,followed);
    let nodes=[];let ink=[];
    if out.ink.len>0{ink.push(paint_ink("document-ink",out.ink));}
    nodes+=sprites.sheet;
    nodes+=pins;
    if pop!=(){nodes.push(pop.node);}
    if thread_ink.len>0{nodes.push(paint_ink("thread-ink",thread_ink));}
    nodes+=sprites.notes;
    #{ink:ink,controls:nodes}
}
fn render_cursors(ctx,props){
    let model=build_model(props);model.boxes=declare_boxes(props,model);let k=token(props);let s=ui(ctx,k);
    let root=element_ref(":cursors");let stage_ref=element_ref(":stage");let sheet_ref=element_ref(":sheet");
    let narrow=ctx.element_width_class(root,[520])!=1;
    let body=render_blocks(ctx,props,model);
    let nodes=body.nodes;
    if nodes.len==0{nodes.push(ink_label(ctx,label_for(ctx,"empty","No document blocks"),"notice").with_key("empty"));}
    let target="";
    if opt(props,"following")!=(){
        let c=opt(model.agents[props.following],"cursor");
        if c!=(){
            let id=if c.target in model.comments{model.comments[c.target].block}else{c.target};
            target=if value(c,"point","")=="add_row"&&rp("add",id) in model.boxes{rp("add",id)}else{model.targets[id].box_ref};
        }
    }
    effect("follow",#{target:target,document:k.document,tab:k.tab,agent:value(props,"following","")},Fn("follow"),Fn("no_effect"));
    if reply_received(s,model){effect("reply_ack",#{token:k,pending:s.pending},Fn("apply_reply_ack"),Fn("no_effect"));}
    effect("prune_review",#{token:k,comments:model.comments.keys(),edits:model.edits.keys()},Fn("prune_review"),Fn("no_effect"));
    let page=column(nodes).with_key("document").with_ref(element_ref(":scroll")).accessibility_role("document")
        .with_style(ctx.component_style(if narrow{["page","page_narrow"]}else{["page"]},style().width(relative(1.0)).height(relative(1.0)).min_width(px(0))
            .padding(u(4.0)).gap(u(2.0)).overflow_y_scroll().background(theme_color("surface_raised"))));
    let viewport=ctx.element_bounds(stage_ref);let shell=ctx.element_bounds(sheet_ref);
    let ink=layers(ctx,props,model,s,body.marks,viewport,shell);
    let stage=stack([page]+ink.ink).with_key("stage").with_ref(stage_ref)
        .with_style(ctx.component_style("stage",style().relative().width(relative(1.0)).min_width(px(0))
            .height(px(props.height)).clip().radius(theme_radius("md"))));
    let result=[];
    if model.tabs.len()>0{result.push(navigate_tabs(ctx,props));}
    let paper_style=ctx.component_style("sheet",style().width(relative(1.0)).min_width(px(0)));
    paper_style=if props.fade{paper_style.bottom_fade(fade_start(),fade_end())}else{paper_style.without_bottom_fade()};
    let surface=[column([top_bar(ctx,props,model,s,narrow),stage]).with_key("sheet").with_ref(sheet_ref).with_style(paper_style)];
    if viewport!=()&&shell!=(){
        // Put editable popovers and hit regions above the masked paper.
        let v=viewport.layout;let p=shell.layout;
        surface.push(stack(ink.controls).with_key("review-overlay")
            .with_style(style().absolute().left(offset_px(v.x-p.x)).top(offset_px(v.y-p.y))
                .width(px(v.width)).height(px(v.height)).clip().radius(theme_radius("md"))));
    }
    result.push(stack(surface).with_key("review-surface").with_style(style().width(relative(1.0)).min_width(px(0))));
    if opt(props,"notice")!=(){result.push(ink_label(ctx,props.notice,"notice").with_key("notice").accessibility_role("status"));}
    column(result).with_key(props.key).with_ref(root).accessibility_role("group").accessibility_label(label_for(ctx,"title","Collaborative document"))
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).gap(u(1.5)).padding(u(1.5))
            .border(px(1)).border_color(theme_color("border")).radius(theme_radius("md")).background(theme_color("surface"))))
}
