import "patterns/tool_access_map" as access;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/tool_access_map_demo","export":"ToolAccessMapDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/tool_access_map","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn ToolAccessMapDemo(p){render_component("gallery/tool_access_map_demo",p)}
fn state(ctx){
    let s=ctx.get_state("sample");
    if s!=(){s}else{#{id:0,revision:0,agent:"planner",narrow:false,deny:false,notice:"",last:(),finished:false}}
}
fn run_id(s){"document-review:"+s.id.to_string()}
fn tools(){[
    #{id:"search",name:"Search library",description:"Find related pages in the example library.",icon:"⌕"},
    #{id:"read",name:"Read documents",description:"Open documents in the sample workspace.",icon:"R"},
    #{id:"outline",name:"Build outline",description:"Summarize the structure of a document.",icon:"O"},
    #{id:"styles",name:"Check styles",description:"Find inconsistent heading levels.",icon:"C"},
    #{id:"notes",name:"Send notes",description:"Queue notes for review.",icon:"N"},
    #{id:"publish",name:"Publish draft",description:"Request publication from the workspace.",icon:"P"}
]}
fn roster(s){
    let search=[#{id:"search-basics",summary:"Find setup notes",status:"done",input:#{query:"setup",limit:4},output:#{found:2,pages:["Setup","Overview"]},at_ms:4100,duration_ms:180},
        #{id:"search-empty",summary:"Find archived diagrams",status:"failed",input:#{query:"archived diagrams",limit:4},error:"No access to the archive in this example",at_ms:9200,duration_ms:410}];
    if s.last!=(){search.push(s.last);}
    let r=if s.finished{"done"}else{"running"};
    [
        #{id:"planner",name:"Planner",model:"Atlas 2.5",status:r,task:"Collect references for a clearer onboarding guide.",tools:[
            #{id:"search",calls:search},#{id:"read",calls:[#{id:"open-setup",summary:"Read setup guide",status:r,input:"guide/setup.md",
                output:if s.finished{"Read two sections"}else{()},at_ms:11100,duration_ms:if s.finished{520}else{()}}]},#{id:"notes"}
        ]},
        #{id:"reviewer",name:"Reviewer",model:"Nova",status:"waiting",task:"Check the heading levels and confirm editorial feedback.",tools:[
            #{id:"outline"},#{id:"styles",calls:[#{id:"check-setup",summary:"Inspect guide headings",status:"done",
                input:#{document:"guide/setup.md"},output:#{headings:7,issues:1},duration_ms:192}]}]},
        #{id:"editor",name:"Editor",model:"Sprout",status:"queued",task:"Fold accepted feedback into the next draft.",tools:[
            #{id:"outline"},#{id:"notes"},#{id:"publish"}]}
    ]
}
fn agent_chosen(ctx,r){
    let s=state(ctx);if r.session!=run_id(s)||r.revision!=s.revision{return;}
    s.agent=r.agent;s.notice="Showing "+r.agent;s.revision+=1;ctx.set_state("sample",s);
}
fn propose(ctx,r){
    let s=state(ctx);if r.session!=run_id(s)||r.revision!=s.revision||r.agent!=s.agent{return;}
    if s.deny{s.deny=false;s.notice="Replay refused by Host";}
    else{
        s.notice=if r.edited{"Edited replay request accepted"}else{"Replay request accepted"};
        if r.agent=="planner"&&r.tool=="search"{
            s.last=#{id:"replay:"+s.revision.to_string(),summary:"Replay receipt",status:"done",input:r.input,
                output:"The gallery records a receipt; it does not invoke a tool.",duration_ms:110,replayed:true};
        }
    }
    s.revision+=1;ctx.set_state("sample",s);
}
fn change(op,ctx,e){
    let s=state(ctx);switch op{
        "narrow"=>{s.narrow=!s.narrow;},
        "deny"=>{s.deny=true;s.notice="The next replay will be refused";},
        "finish"=>{s.finished=true;s.revision+=1;s.notice="Reader finished";},
        _=>{s.id+=1;s.agent="planner";s.revision=0;s.deny=false;s.last=();s.finished=false;s.notice="New run";},
    }
    ctx.set_state("sample",s);
}
fn b(op,label){button::Button(#{key:op,text:label,size:"xs",variant:"ghost",on_click:Fn("change").curry(op)})}
fn render_demo(ctx,p){
    let s=state(ctx);
    let opts=row([b("narrow",if s.narrow{"Wide map"}else{"Narrow map"}),b("deny","Refuse next replay"),b("finish","Finish reader"),b("reset","New run")])
        .with_style(style().min_width(px(0)).flex_wrap().gap(theme_length("space.spacing_unit")));
    let view=access::ToolAccessMap(#{key:"case",session:run_id(s),revision:s.revision,agent:s.agent,agents:roster(s),tools:tools(),
        on_agent_change:Fn("agent_chosen"),on_replay:Fn("propose")});
    let body=column([view]).with_key("sized").with_style(style().min_width(px(0)).width(if s.narrow{px(385)}else{relative(1.0)})
        .max_width(if s.narrow{relative(1.0)}else{px(850)}));
    let receipt=text(s.notice).accessibility_role("status").accessibility_label(s.notice)
        .with_style(style().typography("caption").text_color(theme_color("text_muted")));
    column([opts,body,receipt]).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2.5))))
}
