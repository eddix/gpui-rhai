// Host-owned document and review proposals for the Gallery.
import "patterns/agent_cursors" as cursors;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_cursors_demo","export":"AgentCursorsDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_cursors","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        slots:#{},events:#{},parts:["root"]},
    render:Fn("demo")
});
fn AgentCursorsDemo(props){render_component("gallery/agent_cursors_demo",props)}
fn initial(serial){
    #{document:"release/review:"+serial.to_string(),serial:serial,revision:0,tab:"draft",following:(),phase:0,loading:false,nudge:false,width:0.52,fade:true,
        status:(),pending:(),refuse:false,removed:#{},accepted:#{},replies:[],resolved:false,rows:0}
}
fn model(ctx){let s=ctx.get_state("sample");if s==(){initial(1)}else{s}}
fn save(ctx,s,note){s.status=note;ctx.set_state("sample",s);}
fn reset(ctx,e){ctx.set_state("sample",initial(model(ctx).serial+1));}
fn move(ctx,e){let s=model(ctx);s.phase=(s.phase+1)%3;save(ctx,s,"Reviewer positions changed.");}
fn move_caret(ctx,e){let s=model(ctx);s.nudge=!s.nudge;save(ctx,s,"Typing cursor moved.");}
fn resize_figure(ctx,e){let s=model(ctx);s.width=if s.width>0.4{0.30}else{0.65};save(ctx,s,"Illustration width changed.");}
fn toggle_fade(ctx,e){let s=model(ctx);s.fade=!s.fade;ctx.set_state("sample",s);}
fn show_loading(ctx,e){let s=model(ctx);s.loading=!s.loading;ctx.set_state("sample",s);}
fn refuse(ctx,e){let s=model(ctx);s.refuse=true;save(ctx,s,"The next review request will be refused.");}
fn words(s){
    let headline="The release opens to the early access group on Thursday. The preview includes a new review workflow with examples in 中文, العربية and 👩‍👧. Please confirm the date before we send the final notice.";
    if "cadence" in s.accepted{
        headline="The release opens to the early access group on Friday. The preview includes a new review workflow with examples in 中文, العربية and 👩‍👧. Please confirm the date before we send the final notice.";
    }
    headline
}
fn demo_art(){
    svg("<svg viewBox='0 0 300 160'><defs><linearGradient id='bg' x2='1' y2='1'><stop stop-color='#dee8fa'/><stop offset='.95' stop-color='#b9c9ee'/></linearGradient></defs><rect width='300' height='160' fill='url(#bg)'/><path d='M0 130 55 69 109 110 170 48 210 73 260 23 300 35V160H0Z' fill='#f9fbff'/><path d='M0 141 55 80 109 120 170 59 210 84 260 34 300 46' fill='none' stroke='#576bb0' stroke-width='2.5'/><circle cx='170' cy='59' r='5' fill='#576bb0'/></svg>")
        .accessibility_hidden(true)
}
fn rollout_prefix(){ "Keep an eye on the reports from the first group. One " }
fn rollout_detail(s){
    rollout_prefix()+(if "rollout_note" in s.accepted{"editor"}else{"reviewer"})+
        " checks the wording, another watches the graphs, and the on-call team stays ready to pause. Continue recording the useful details together as each new region joins. That record should remain readable beside the image, and continue across the whole measure once the text passes its bottom edge."
}
fn blocks(s){
    if s.tab=="checks"{
        let rows=[
            [#{id:"h-check",text:"Review check"},#{id:"h-owner",text:"Owner"},#{id:"h-state",text:"Status"}],
            [#{id:"scope",text:"Release scope"},#{id:"scope-owner",text:"Mira"},#{id:"scope-state",text:"Ready",tone:"success"}],
            [#{id:"timing",text:"Announcement timing"},#{id:"timing-owner",text:"Sora"},#{id:"timing-state",text:"Review",tone:"warning"}],
        ];
        for i in 0..s.rows{
            let n=i.to_string();
            rows.push([#{id:"check-"+n,text:"Follow-up "+n},#{id:"owner-"+n,text:"Team"},#{id:"state-"+n,text:"Planned",tone:"info"}]);
        }
        return [#{id:"checks-title",kind:"title",text:"Release checklist"},#{id:"checklist",kind:"table",rows:rows,add_row:s.rows<6}];
    }
    [#{id:"headline",kind:"title",text:"Launch brief"},
     #{id:"launch/预览",kind:"paragraph",text:words(s)},
     #{id:"criteria",kind:"heading",text:"Before announcement"},
     #{id:"translations",kind:"item",text:"Confirm that translated copy and the public support link are ready."},
     #{id:"graph",kind:"image",alt:"Preview of the rollout timeline",text:"Figure 1 · Staged release",width:s.width,ratio:1.875,
         loading:s.loading,float:"start",media:demo_art()},
     #{id:"rollout",kind:"paragraph",text:"Start with a small audience. Watch the first rehearsal before widening the release."},
     #{id:"rollout-detail",kind:"paragraph",text:rollout_detail(s)},
     #{id:"config",kind:"code",text:if "config_edit" in s.accepted{"channel = \"preview\"\nretry = 3"}else{"channel = \"preview\"\nretry = 2"}},
     #{id:"later",kind:"heading",text:"Final checkpoints"},
     #{id:"signoff",kind:"paragraph",text:"The team will confirm the final build and record the decision in the checklist."}]
}
fn suggestions(s){
    let edits=[];
    if s.tab=="draft"&&!("cadence" in s.removed){
        edits.push(#{id:"cadence",agent:"mira",block:"launch/预览",at:47,remove:8,insert:"Friday",
            note:"The final rehearsal ends on Thursday, so announce after it."});
    }
    if s.tab=="draft"&&!("config_edit" in s.removed){
        edits.push(#{id:"config_edit",agent:"sora",block:"config",at:28,remove:1,insert:"3",note:"Keep one extra retry during staged release."});
    }
    if s.tab=="draft"&&!("rollout_note" in s.removed){
        edits.push(#{id:"rollout_note",agent:"imani",block:"rollout-detail",at:grapheme_count(rollout_prefix()),
            remove:8,insert:"editor",note:"This role owns the wording rather than the entire review."});
    }
    edits
}
fn threads(s){
    if s.tab!="draft"||s.resolved{return [];}
    [#{id:"notice/thread",agent:"jules",block:"launch/预览",at:85,text:"Can we mention the review shortcuts in the notice?",
        replies:s.replies,open:s.phase==2}]
}
fn people(s){
    let mira=if s.tab=="checks"{#{target:"timing",at:4,select:7,doing:"Checking the date"}}
        else if s.phase==1{#{target:"signoff",at:4,select:7,doing:"Checking the sign-off"}}
        else if s.nudge{#{target:"launch/预览",at:7,typing:true,doing:"Proposing a date"}}
        else if "cadence" in s.removed{#{target:"launch/预览",at:47,doing:"Reading announcement"}}
        else{#{target:"launch/预览",at:3,in_suggestion:"cadence",typing:true,doing:"Proposing a date"}};
    let sora=if s.tab=="checks"{#{target:"checklist",point:if s.rows<6{"add_row"}else{"center"},pressed:true,doing:"Adding a check"}}
        else{#{target:"graph",select:1,point:"corner",doing:"Reviewing the timeline"}};
    let jules=if s.tab=="checks"{#{target:"scope-state",at:2,doing:"Confirming scope"}}
        else if s.phase==2&&!s.resolved{#{target:"notice/thread",at:10,typing:true,doing:"Writing a comment"}}
        else{#{target:"launch/预览",at:82,select:16,doing:"Reading examples"}};
    let imani=if s.tab=="draft"{#{target:"rollout-detail",at:grapheme_count(rollout_prefix()),
        select:32,doing:"Checking the rollout notes"}}else{()};
    [#{id:"mira",name:"Mira",cost_micros:42000,cursor:mira},
     #{id:"sora",name:"Sora",cost_micros:16300,cursor:sora},
     #{id:"jules",name:"Jules",cost_micros:1200,cursor:jules},
     #{id:"imani",name:"Imani",cursor:imani},#{id:"tess",name:"Tess"},#{id:"kim",name:"Kim"},#{id:"noor",name:"Noor"}]
}
fn begin(ctx,p){
    let s=model(ctx);
    if s.pending!=()||p.document!=s.document||p.revision!=s.revision||p.tab!=s.tab{return;}
    s.pending=#{proposal:p,refuse:s.refuse};s.refuse=false;
    save(ctx,s,"Waiting for the demo Host…");
}
fn accept_one(s,id,accept){
    s.removed[id]=true;if accept{s.accepted[id]=true;}s
}
fn settle(ctx,request){
    let s=model(ctx);
    if s.pending!=request{return;}
    s.pending=();let p=request.proposal;
    if p.document!=s.document||p.revision!=s.revision||p.tab!=s.tab{save(ctx,s,"Stale review request.");return;}
    if request.refuse{save(ctx,s,"The Host refused the review request.");return;}
    switch p.kind{
        "tab"=>{if p.target in ["draft","checks"]{s.tab=p.target;}},
        "follow"=>{s.following=if p.target==""{()}else{p.target};},
        "accept"=>{for edit in suggestions(s){if edit.id==p.target{s=accept_one(s,edit.id,true);}}},
        "reject"=>{for edit in suggestions(s){if edit.id==p.target{s=accept_one(s,edit.id,false);}}},
        "accept_all"=>{for edit in suggestions(s){s=accept_one(s,edit.id,true);}},
        "resolve"=>{if p.target=="notice/thread"{s.resolved=true;}},
        "reply"=>{if p.target=="notice/thread"&&s.replies.len<12{s.replies.push(p.text);}},
        "add_row"=>{if p.target=="checklist"&&s.rows<6{s.rows+=1;}},
    }
    s.revision+=1;save(ctx,s,"Accepted document revision "+s.revision.to_string());
}
fn control(key,label,callback){
    button::Button(#{key:key,text:label,size:"sm",variant:"ghost",on_click:callback})
}
fn demo(ctx,props){
    let s=model(ctx);
    if s.pending!=(){timeout("host-review",280,false,Fn("settle"),s.pending);}
    column([cursors::AgentCursors(#{key:"review",document:s.document,revision:s.revision,title:"Content review",height:386,fade:s.fade,
            active_tab:s.tab,tabs:[#{id:"draft",label:"Draft"},#{id:"checks",label:"Checklist"}],following:s.following,agents:people(s),
            blocks:blocks(s),suggestions:suggestions(s),comments:threads(s),busy:s.pending!=(),notice:s.status,on_action:Fn("begin")}),
        row([control("move-people","Move reviewers",Fn("move")),control("move-caret","Move typing cursor",Fn("move_caret")),
            control("resize-figure","Resize illustration",Fn("resize_figure")),control("loading-picture",if s.loading{"Show illustration"}else{"Load illustration"},Fn("show_loading")),
            control("toggle-fade",if s.fade{"Disable page fade"}else{"Enable page fade"},Fn("toggle_fade")),
            control("refuse","Refuse next review request",Fn("refuse")),control("reset","Reset review",Fn("reset"))]).with_key("controls")
            .with_style(style().min_width(px(0)).flex_wrap().gap(theme_spacing("xs")))]).with_key(props.key)
        .with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_spacing("md"))))
}
