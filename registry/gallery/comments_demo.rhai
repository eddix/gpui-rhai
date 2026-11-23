// In-memory review scenarios. No message leaves the Gallery; edits change only the demo.
import "patterns/comment_thread" as conversation;
import "patterns/comments_panel" as panel;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/comments_demo","export":"CommentsDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/comment_thread","patterns/comments_panel","components/button"],capabilities:#{}},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        mode:#{schema:#{type:"string",allowed:["thread","panel"]},required:true,sensitive:false},
    },state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{},slots:#{},parts:["root"]},
    render:Fn("demo_view")
});
fn CommentsDemo(props){render_component("gallery/comments_demo",props)}
fn initial(mode){
    let items=if mode=="thread"{
        [#{id:"review/7",author:"Morgan Lee",time:"5 min ago",text:"Could we call out why the rollout was paused?"},
         #{id:"assistant",author:"Review assistant",time:"2 min ago",text:"I checked the deployment notes and found the relevant entry.",
           assistant:true,status:"Reviewed two notes",details:["The health check failed before release.","The previous configuration is still active."]}]
    }else{
        [#{id:"status",author:"Morgan Lee",time:"2 min ago",timestamp:4,text:"The preview is ready for a second look.",resolved:false,revision:0},
         #{id:"review/7",author:"Riley Kim",time:"8 min ago",timestamp:3,text:"Could we show the last known good deploy?",resolved:false,revision:0},
         #{id:"edit/7",author:"Tess Rivera",time:"13 min ago",timestamp:2,text:"/Edit Add a rollout note below the status.",edit_state:"idle",resolved:false,revision:0},
         #{id:"unapproved",author:"Sam Lee",time:"21 min ago",timestamp:1,text:"/edit Update the notification copy.",resolved:false,revision:0}]
    };
    #{items:items,draft:"",mode:mode,open:true,next:4,pending:(),answer:(),task:(),reject:false,feedback:"",posted:()}
}
fn model(ctx,mode){let m=ctx.get_state("model");if m==()||m.mode!=mode{initial(mode)}else{m}}
fn changed(mode,ctx,text){
    let m=model(ctx,mode);if m.pending==()&&m.answer==(){m.draft=text;ctx.set_state("model",m);}
}
fn send(mode,ctx,text){
    let m=model(ctx,mode);
    if !m.open||m.pending!=()||m.answer!=(){return;}
    m.next+=1;m.pending=#{id:m.next,text:text};m.feedback="";ctx.set_state("model",m);
}
fn reply_ready(mode,ctx,id){
    let m=model(ctx,mode);if m.pending==()||m.pending.id!=id{return;}
    if m.reject{m.reject=false;m.feedback="The demo Host declined the comment. Draft retained.";}
    else if mode=="thread"{
        m.items.push(#{id:"message/"+id.to_string(),author:"Jules",time:"just now",text:m.pending.text});
        m.answer=id;m.draft="";
    }else{
        m.posted="comment/"+id.to_string();
        m.items.push(#{id:m.posted,author:"Jules",time:"just now",timestamp:id,text:m.pending.text,resolved:false,revision:0});
        m.draft="";m.feedback="Comment added.";
    }
    m.pending=();ctx.set_state("model",m);
}
fn assistant_ready(ctx,id){
    let m=model(ctx,"thread");if m.answer!=id{return;}
    m.items.push(#{id:"answer/"+id.to_string(),author:"Review assistant",time:"just now",assistant:true,
        text:"I added a note to this demo conversation. No files were changed.",status:"Reply ready",
        details:["This sample was produced by the Gallery Host."]});
    m.answer=();ctx.set_state("model",m);
}
fn resolve(ctx,value){
    let m=model(ctx,"panel");let list=m.items;
    for i in 0..list.len{
        let item=list[i];
        if item.id==value.id&&item.revision==value.revision&&conversation::opt(item,"edit_state")!="running"{
            item.resolved=value.resolved;item.revision+=1;list[i]=item;m.items=list;ctx.set_state("model",m);return;
        }
    }
}
fn edit(ctx,value){
    let m=model(ctx,"panel");let list=m.items;
    for i in 0..list.len{
        let item=list[i];
        if item.id!=value.id||item.revision!=value.revision||item.resolved||item.id!="edit/7"{continue;}
        if value.action=="undo"&&item.edit_state=="applied"{
            item.edit_state="idle";item.edit_message="The demo change was undone.";
        }else if value.action=="run"&&(item.edit_state=="idle"||item.edit_state=="failed"){
            item.edit_state="running";item.edit_message="";m.task=#{id:item.id,revision:item.revision+1};
        }else{return;}
        item.revision+=1;list[i]=item;m.items=list;ctx.set_state("model",m);return;
    }
}
fn edit_ready(ctx,task){
    let m=model(ctx,"panel");if m.task!=task{return;}
    let list=m.items;
    for i in 0..list.len{
        let item=list[i];
        if item.id==task.id&&item.revision==task.revision&&item.edit_state=="running"{
            item.revision+=1;item.edit_state="applied";list[i]=item;
        }
    }
    m.task=();m.items=list;ctx.set_state("model",m);
}
fn toggle(mode,state,ctx,event){
    let m=model(ctx,mode);m.open=state;ctx.set_state("model",m);
}
fn decline(mode,ctx,event){
    let m=model(ctx,mode);m.reject=true;m.feedback="The next comment will be declined.";ctx.set_state("model",m);
}
fn demo_view(ctx,props){
    let mode=props.mode;let m=model(ctx,mode);
    if m.pending!=(){timeout("send-accept",310,false,Fn("reply_ready").curry(mode),m.pending.id);}
    if m.answer!=(){timeout("assistant-reply",780,false,Fn("assistant_ready"),m.answer);}
    if m.task!=(){timeout("edit-ready",480,false,Fn("edit_ready"),m.task);}
    let surface=if !m.open{
        button::Button(#{key:"reopen",text:"Show comments",variant:"secondary",on_click:Fn("toggle").curry(mode,true)})
    }else if mode=="thread"{
        conversation::CommentThread(#{key:"thread",title:"Review comment",messages:m.items,draft:m.draft,assistant_name:"Review assistant",
            sending:m.pending!=()||m.answer!=(),thinking:m.answer!=(),message:m.feedback,
            on_change:Fn("changed").curry(mode),on_send:Fn("send").curry(mode),on_close:Fn("toggle").curry(mode,false)})
    }else{
        panel::CommentsPanel(#{key:"panel",title:"Release comments",comments:m.items,draft:m.draft,current_author:"Jules",
            sending:m.pending!=(),message:m.feedback,posted_id:m.posted,on_change:Fn("changed").curry(mode),on_send:Fn("send").curry(mode),
            on_resolve:Fn("resolve"),on_edit:Fn("edit"),on_close:Fn("toggle").curry(mode,false)})
    };
    column([surface,button::Button(#{key:"decline",text:"Decline next comment",size:"sm",variant:"secondary",
        on_click:Fn("decline").curry(mode)})]).with_key(props.key)
        .with_style(ctx.component_style("root",style().min_width(px(0)).items_start().gap(px(8))))
}
