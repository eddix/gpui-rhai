// Simulated control plane; all files, arguments and permissions are local sample data.
import "patterns/approval_gates" as gates;
import "patterns/autonomy_dial" as dial;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/gates_demo","export":"GatesDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/approval_gates","patterns/autonomy_dial","components/button"],capabilities:#{}},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        mode:#{schema:#{type:"string",allowed:["gates","dial"]},required:true,sensitive:false},
    },state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},events:#{},slots:#{},parts:["root"]},
    render:Fn("demo_view")
});
fn GatesDemo(props){render_component("gallery/gates_demo",props)}
fn initial(serial){
    #{serial:serial,run:"release/"+serial.to_string(),policy:"release-policy/"+serial.to_string(),revision:0,level:"risky",
        pinned:[],reject:false,busy:(),notice:(),rows:[
            #{id:"summary",revision:0,kind:"tool",agent:"Review assistant",action:"Prepare a release summary",
                reason:"Use this audience and length for the local preview.",
                input:"{\n  \"audience\": \"team\",\n  \"length\": \"short\"\n}",status:"open",editable:true,busy:false,
                context:["Read the release outline","Compare the two draft summaries"],files:["release-outline.txt"],holds:["Review the resulting summary"]},
            #{id:"file/preview",revision:0,kind:"file",agent:"Review assistant",action:"Update the release draft",
                reason:"Show the proposed wording before it goes to the reviewer.",status:"open",editable:true,busy:false,
                diff:[#{name:"release-draft.txt",before:"Release notes\n\nDeployment starts Thursday.\n",after:"Release notes\n\nDeployment starts Friday.\n"}],
                files:["release-draft.txt"],context:["Validate the revised schedule"],holds:["Prepare the next review"]},
        ]}
}
fn model(ctx){let m=ctx.get_state("model");if m==(){initial(1)}else{m}}
fn save(ctx,m){ctx.set_state("model",m);}
fn stale_gate(m,p){p.run!=m.run}
fn propose_gate(ctx,p){
    let m=model(ctx);if stale_gate(m,p)||m.busy!=(){return;}
    let rows=m.rows;
    for i in 0..rows.len{
        let row=rows[i];
        if row.id!=p.gate||row.revision!=p.revision||row.status!="open"{continue;}
        if p.action=="approve_edited"&&row.kind=="file"||p.action=="request_changes"&&row.kind!="file"{return;}
        let ticket=p;ticket.reject=m.reject;
        row.busy=true;row.note=();rows[i]=row;m.rows=rows;m.busy=ticket;m.reject=false;save(ctx,m);return;
    }
}
fn complete_gate(ctx,p){
    let m=model(ctx);if stale_gate(m,p)||m.busy!=p{return;}
    let rows=m.rows;
    for i in 0..rows.len{
        let row=rows[i];
        if row.id!=p.gate||row.revision!=p.revision||row.status!="open"{continue;}
        row.busy=false;
        if p.reject{row.note="The demo Host declined this proposal.";}
        else{
            row.revision+=1;
            row.status=if p.action=="approve_edited"{"edited"}else if p.action=="request_changes"{"changes_requested"}
                else if p.action=="reject"{"rejected"}else{"approved"};
            row.at="just now";
            row.note=if p.action=="approve_edited"||p.action=="request_changes"{p.text}else{"The demo Host recorded the decision."};
            if p.action=="approve_edited"{row.input=p.text;}
        }
        rows[i]=row;
    }
    m.rows=rows;m.busy=();save(ctx,m);
}
fn propose_policy(ctx,p){
    let m=model(ctx);if p.policy!=m.policy||p.revision!=m.revision||m.busy!=(){return;}
    if p.kind=="pin"&&!(p.action in ["status","notes"]){return;}
    let ticket=p;ticket.reject=m.reject;m.reject=false;m.busy=ticket;m.notice=();save(ctx,m);
}
fn complete_policy(ctx,p){
    let m=model(ctx);if m.busy!=p||m.policy!=p.policy||m.revision!=p.revision{return;}
    m.busy=();
    if p.reject{m.notice="The demo Host declined this policy change.";save(ctx,m);return;}
    if p.kind=="level"{m.level=p.level;}
    else{
        let pins=[];for id in m.pinned{if id!=p.action{pins.push(id);}}
        if p.pinned{pins.push(p.action);}m.pinned=pins;
    }
    m.revision+=1;save(ctx,m);
}
fn reject(ctx,payload){let m=model(ctx);m.reject=true;m.notice="The next proposal will be declined.";save(ctx,m);}
fn reset(ctx,payload){let m=model(ctx);save(ctx,initial(m.serial+1));}
fn add(ctx,payload){
    let m=model(ctx);for row in m.rows{if row.id=="message"{return;}}
    m.rows.push(#{id:"message",revision:0,kind:"message",agent:"Notes helper",action:"Share the preview",reason:"Publish the already reviewed sample.",
        input:"The release review is ready for a second look.",status:"open",editable:true,busy:false,holds:["Share the local preview"]});
    save(ctx,m);
}
fn controls(key,label,fn_ptr){
    button::Button(#{key:key,text:label,size:"sm",variant:"secondary",on_click:fn_ptr})
}
fn demo_view(ctx,props){
    let m=model(ctx);let gate_mode=props.mode=="gates";
    if m.busy!=(){timeout("complete",520,false,if gate_mode{Fn("complete_gate")}else{Fn("complete_policy")},m.busy);}
    let panel=if gate_mode{
        gates::ApprovalGates(#{key:"gates",run:m.run,task:"Release review",live:true,gates:m.rows,on_decide:Fn("propose_gate")})
    }else{
        dial::AutonomyDial(#{key:"dial",policy:m.policy,revision:m.revision,level:m.level,pinned:m.pinned,busy:m.busy!=(),message:m.notice,
            actions:[#{id:"status",label:"Read status",detail:"Local preview",risk:"safe"},
                #{id:"notes",label:"Update notes",detail:"Shared draft",risk:"risky"},
                #{id:"archive",label:"Replace archive",detail:"Protected sample",risk:"destructive",required_confirmation:true}],
            on_change:Fn("propose_policy")})
    };
    let options=[controls("reject","Decline next proposal",Fn("reject")),controls("reset","Reset demo",Fn("reset"))];
    if gate_mode{options.push(controls("add","Add sample gate",Fn("add")));}
    column([panel,row(options).with_style(style().gap(px(6)).flex_wrap())]).with_key(props.key).audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).items_start().gap(px(if gate_mode{16}else{8}))))
}
