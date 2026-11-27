// A deterministic Host simulation for the RunControls Gallery workflow.
import "patterns/run_controls" as run_controls;
import "components/button" as button;

fn RunDemo(props){render_component("gallery/run_demo",props)}

define_component(#{
    metadata:#{id:"gallery/run_demo","export":"RunDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/run_controls","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{
        model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
    }},events:#{},slots:#{},parts:["root"]},
    render:Fn("run_demo_view")
});
fn run_demo_model(ctx){
    let current=ctx.get_state("model");
    if current!=(){return current;}
    #{run:"",serial:0,lease:0,status:"idle",tick:0,prompt:"",answer:"",draft:"Investigate the stalled rollout.",
        queue:[],steers:[],queue_serial:0,step_serial:0,mode:"act",permission:"ask",reject:false,note:""}
}
fn run_demo_save(ctx,model){ctx.set_state("model",model);}
fn run_demo_begin(ctx,model,prompt){
    model.serial+=1;model.lease+=1;model.run="run/"+model.serial.to_string();
    model.status="running";model.tick=0;model.prompt=prompt;model.answer="";model.steers=[];model.draft="";
    model.note="";run_demo_save(ctx,model);
}
fn run_demo_draft(ctx,value){
    let model=run_demo_model(ctx);if value.run!=model.run{return;}
    model.draft=value.text;run_demo_save(ctx,model);
}
fn run_demo_submit(kind,ctx,value){
    let model=run_demo_model(ctx);
    if value.run!=model.run{return;}
    if model.reject{
        model.reject=false;model.note="Request declined. Draft preserved.";run_demo_save(ctx,model);return;
    }
    if kind=="send"&&model.status!="running"{run_demo_begin(ctx,model,value.text);return;}
    if model.status!="running"{return;}
    if kind=="queue"&&model.queue.len<12{
        model.queue_serial+=1;model.queue.push(#{id:"next/"+model.queue_serial.to_string(),text:value.text});model.draft="";
    }else if kind=="steer"&&model.steers.len<16{
        model.step_serial+=1;model.steers.push(#{id:"note/"+model.step_serial.to_string(),at:model.answer.len,text:value.text});
        model.draft="";
    }
    run_demo_save(ctx,model);
}
fn run_demo_unqueue(ctx,value){
    let model=run_demo_model(ctx);if value.run!=model.run{return;}
    let queue=[];for entry in model.queue{if entry.id!=value.id{queue.push(entry);}}
    model.queue=queue;run_demo_save(ctx,model);
}
fn run_demo_action(kind,ctx,run){
    let model=run_demo_model(ctx);if run!=model.run{return;}
    if kind=="stop"&&model.status=="running"{model.status="stopped";model.lease+=1;}
    if kind=="resume"&&(model.status=="stopped"||model.status=="error"){model.status="running";model.lease+=1;}
    if kind=="retry"&&model.status!="running"{run_demo_begin(ctx,model,model.prompt);return;}
    if kind=="dismiss"&&model.status!="running"{
        model.status="idle";model.run="";model.lease+=1;model.prompt="";model.answer="";model.tick=0;model.steers=[];
    }
    run_demo_save(ctx,model);
}
fn run_demo_mode(ctx,value){
    let model=run_demo_model(ctx);model.mode=value;run_demo_save(ctx,model);
}
fn run_demo_permission(ctx,value){
    let model=run_demo_model(ctx);if model.mode!="act"{return;}
    if value=="full"{model.note="The demo Host declined full access.";}
    else{model.permission=value;model.note="Application policy updated.";}
    run_demo_save(ctx,model);
}
fn run_demo_reject(ctx,payload){
    let model=run_demo_model(ctx);model.reject=true;model.note="The Host will decline the next input.";run_demo_save(ctx,model);
}
fn run_demo_fail(ctx,payload){
    let model=run_demo_model(ctx);if model.status!="running"{return;}
    model.status="error";model.lease+=1;run_demo_save(ctx,model);
}
fn run_demo_next(ctx,payload){
    let model=run_demo_model(ctx);if model.status=="running"||model.queue.len==0{return;}
    let item=model.queue.remove(0);let draft=model.draft;
    // Starting an accepted queued prompt must not clear a separate new draft.
    model.serial+=1;model.lease+=1;model.run="run/"+model.serial.to_string();
    model.status="running";model.tick=0;model.prompt=item.text;model.answer="";model.steers=[];model.draft=draft;
    model.note="";run_demo_save(ctx,model);
}
fn run_demo_tick(ctx,event){
    let model=run_demo_model(ctx);if model.status!="running"||model.lease!=event.lease||model.tick!=event.tick{return;}
    let chunks=["The last rollout was interrupted after the first health check. ",
        "The active configuration is still intact, so there is no reason to replay the entire deployment. ",
        "Compare the changed hosts against the expected revision before taking another step. ",
        "Check one host at a time and keep the last good configuration available. ",
        "Then rerun the probe and record what changed. "];
    model.answer+=chunks[model.tick%chunks.len];
    model.tick+=1;if model.tick>=16{model.status="done";}
    run_demo_save(ctx,model);
}
fn run_demo_view(ctx,props){
    let model=run_demo_model(ctx);let busy=model.status=="running";
    if busy{timeout("stream",315,false,Fn("run_demo_tick"),#{lease:model.lease,tick:model.tick});}
    let card=run_controls::RunControls(#{key:"work",run_id:model.run,agent_id:"ops/assistant",agent_name:"Rollout assistant",
        status:model.status,draft:model.draft,prompt:model.prompt,answer:model.answer,steers:model.steers,queue:model.queue,
        mode:model.mode,permission:model.permission,context:0.28+model.tick*0.035,elapsed_ms:model.tick*315,
        on_change:Fn("run_demo_draft"),on_send:Fn("run_demo_submit").curry("send"),
        on_steer:Fn("run_demo_submit").curry("steer"),on_queue:Fn("run_demo_submit").curry("queue"),
        on_unqueue:Fn("run_demo_unqueue"),on_stop:Fn("run_demo_action").curry("stop"),
        on_resume:Fn("run_demo_action").curry("resume"),on_retry:Fn("run_demo_action").curry("retry"),
        on_dismiss:Fn("run_demo_action").curry("dismiss"),on_mode_change:Fn("run_demo_mode"),
        on_permission_change:Fn("run_demo_permission")});
    let tools=[
        button::Button(#{key:"reject",text:"Decline next input",size:"sm",variant:"secondary",on_click:Fn("run_demo_reject")}),
        button::Button(#{key:"fail",text:"Simulate disconnect",size:"sm",variant:"secondary",disabled:!busy,on_click:Fn("run_demo_fail")}),
        button::Button(#{key:"next",text:"Start queued run",size:"sm",variant:"secondary",disabled:busy||model.queue.len==0,on_click:Fn("run_demo_next")}),
    ];
    column([card,row(tools).with_style(style().gap(px(6)).flex_wrap()),
        text(model.note).test_id("run-host-note").with_style(style().typography("caption").text_color(theme_color("text_muted"))),
        text(model.status+" · "+model.tick.to_string()).test_id("run-host-progress").with_style(style().typography("caption"))
    ]).with_key(props.key).with_style(ctx.component_style("root",style().gap(px(8)).min_width(px(0)).items_start()))
}
