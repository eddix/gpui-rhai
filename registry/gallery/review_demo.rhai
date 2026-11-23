// Simulated review transactions. No real system permission or request is executed.
import "patterns/approval_card" as approval;
import "patterns/permission_request" as permission;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/review_demo","export":"ReviewDemo","version":"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/approval_card","patterns/permission_request","components/button"],capabilities:#{}},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        mode:#{schema:#{type:"string",allowed:["approval","permission"]},required:true,sensitive:false},
    },state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{},slots:#{},parts:["root"]},
    render:Fn("demo_view")
});
fn ReviewDemo(props){render_component("gallery/review_demo",props)}
fn initial(mode,serial,destructive){
    #{mode:mode,serial:serial,destructive:destructive,request:mode+"/"+serial.to_string(),revision:0,
        remaining:42000,pointer:false,focus:false,reject:false,pending:(),status:"pending",message:(),grant:(),topic:"Release summary"}
}
fn model(ctx,mode){
    let m=ctx.get_state("model");if m==()||m.mode!=mode{return initial(mode,1,false);}m
}
fn save(ctx,m){ctx.set_state("model",m);}
fn current(m,p){m.request==p.request&&m.revision==p.revision}
fn token(m){#{request:m.request,revision:m.revision}}
fn next(mode,destructive,ctx,event){
    let m=model(ctx,mode);save(ctx,initial(mode,m.serial+1,destructive));
}
fn reject(mode,ctx,event){
    let m=model(ctx,mode);m.reject=true;m.message="The next proposal will be declined by the demo Host.";save(ctx,m);
}
fn tick(ctx,p){
    let m=model(ctx,"approval");
    if !current(m,p)||m.pending!=()||m.status!="pending"||m.pointer||m.focus||m.remaining!=p.milliseconds{return;}
    m.remaining=max(0,m.remaining-1000);
    if m.remaining==0{m.status="expired";m.revision+=1;m.message="No release note was recorded.";}
    save(ctx,m);
}
fn activity(ctx,p){
    let m=model(ctx,"approval");if !current(m,p)||m.status!="pending"{return;}
    if p.channel=="pointer"{m.pointer=p.active;}else{m.focus=p.active;}
    save(ctx,m);
}
fn approval_decision(ctx,p){
    let m=model(ctx,"approval");if !current(m,p)||m.status!="pending"||m.pending!=(){return;}
    if p.action=="edit"{
        m.revision+=1;m.topic="Revised release summary";m.message="Preview updated; review it again.";
    }else if p.action=="deny"{
        m.revision+=1;m.status="denied";m.message="No release note was recorded.";
    }else{
        if m.remaining==0{return;}
        m.pending=#{request:p.request,revision:p.revision,always:p.always,reject:m.reject};
        m.reject=false;m.message=();
    }
    save(ctx,m);
}
fn approval_ready(ctx,p){
    let m=model(ctx,"approval");if !current(m,p)||m.pending!=p{return;}
    m.pending=();
    if p.reject{m.message="The demo Host did not accept the proposal.";save(ctx,m);return;}
    m.revision+=1;m.status="approved";
    m.message=if p.always{"Recorded with a durable preference for the demo."}else{"Recorded for this demo request only."};
    save(ctx,m);
}
fn permission_decision(ctx,p){
    let m=model(ctx,"permission");if !current(m,p)||m.pending!=(){return;}
    if p.action=="revoke"&&m.status=="allowed"&&m.grant!=()&&p.grant==m.grant.id{
        m.status="revoked";m.revision+=1;m.grant=();m.message="The demo grant was revoked.";
    }else if p.action=="renew"&&(m.status=="denied"||m.status=="expired"||m.status=="revoked"){
        m=initial("permission",m.serial+1,m.destructive);
    }else if m.status=="pending"&&p.action=="deny"{
        m.status="denied";m.revision+=1;m.message="No demo capability was granted.";
    }else if m.status=="pending"&&p.action=="allow"&&p.scope!=""&&!(m.destructive&&p.scope=="always"){
        m.pending=#{request:p.request,revision:p.revision,scope:p.scope,duration:p.duration_minutes,reject:m.reject};
        m.reject=false;m.message=();
    }
    save(ctx,m);
}
fn permission_ready(ctx,p){
    let m=model(ctx,"permission");if !current(m,p)||m.pending!=p{return;}
    m.pending=();
    if p.reject{m.message="The demo Host did not accept the proposal.";save(ctx,m);return;}
    let timed=p.scope=="timed";let total=p.duration*60000;
    m.revision+=1;m.status="allowed";m.grant=#{id:"grant/"+m.serial.to_string(),scope:p.scope,
        remaining_ms:if timed{total}else{()},total_ms:if timed{total}else{()}};
    save(ctx,m);
}
fn expire_grant(ctx,event){
    let m=model(ctx,"permission");
    if m.status!="allowed"||m.grant.scope!="timed"{return;}
    m.status="expired";m.revision+=1;m.grant=();m.message="The demo time window ended.";save(ctx,m);
}
fn control(key,label,action){
    button::Button(#{key:key,text:label,size:"sm",variant:"secondary",on_click:action})
}
fn demo_view(ctx,props){
    let m=model(ctx,props.mode);let tools=[];
    let panel=if props.mode=="approval"{
        if m.pending!=(){timeout("receipt",600,false,Fn("approval_ready"),m.pending);}
        if m.status=="pending"&&!m.pointer&&!m.focus&&m.pending==(){
            let payload=token(m);payload.milliseconds=m.remaining;timeout("countdown",1000,false,Fn("tick"),payload);
        }
        tools.push(control("new","New review request",Fn("next").curry("approval",false)));
        approval::ApprovalCard(#{key:"review",request:m.request,revision:m.revision,status:m.status,
            label:"Step 4 · release review",title:"Record a release note",description:"Review the proposed summary before it is accepted.",
            details:[#{label:"Workspace",value:"Operations"},#{label:"Subject",value:m.topic},#{label:"Visibility",value:"Team"}],
            risk:"medium",remaining_ms:m.remaining,busy:m.pending!=(),message:m.message,allow_always:true,editable:true,
            on_decision:Fn("approval_decision"),on_activity:Fn("activity")})
    }else{
        if m.pending!=(){timeout("receipt",520,false,Fn("permission_ready"),m.pending);}
        tools.push(control("new",if m.destructive{"New ordinary request"}else{"Use destructive request"},Fn("next").curry("permission",!m.destructive)));
        if m.status=="allowed"&&m.grant.scope=="timed"{tools.push(control("expire","End demo time window",Fn("expire_grant")));}
        permission::PermissionRequest(#{key:"access",request:m.request,revision:m.revision,agent:"Release assistant",
            action:if m.destructive{"replace the archived sample"}else{"review draft notes"},target:"Operations workspace",
            risk:if m.destructive{"destructive"}else{"risky"},reason:"Compare the current draft with the public release outline.",
            preview:[#{id:"brief",label:"Release brief",info:"Read only"},#{id:"notes",label:"Draft notes",info:"Read only"},
                #{id:"outline",label:"Public outline",info:"Read only"},#{id:"summary",label:"Comparison summary",info:"Local"}],
            preview_limit:2,scopes:["once","timed","always"],duration_minutes:60,status:m.status,busy:m.pending!=(),grant:m.grant,
            message:m.message,can_revoke:true,can_renew:true,on_request:Fn("permission_decision")})
    };
    tools.push(control("reject","Decline next proposal",Fn("reject").curry(props.mode)));
    column([panel,row(tools).with_style(style().gap(px(6)).flex_wrap())]).with_key(props.key)
        .audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).items_start().gap(px(8))))
}
