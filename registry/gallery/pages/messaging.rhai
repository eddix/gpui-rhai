// A real controlled task: send a draft, stream native documents, pause following and stop.
import "gallery/kit" as kit;
import "patterns/composer" as composer;
import "components/ai_answer" as answer;
import "patterns/model_picker" as model_picker;
import "components/thinking_state" as thinking_state;
import "gallery/run_demo" as run_demo;
import "components/button" as button;
import "components/badge" as badge;
import "layouts/region" as region;

fn pages() {
    [#{ id: "scene.messages", title: "Assistant conversation", group: "Scenes" },
     #{ id: "run_controls", title: "RunControls", group: "Agents" }]
}

define_component(#{
    metadata: #{ id: "gallery/assistant", "export": "Assistant", version: "0.2.2",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["patterns/composer", "patterns/model_picker", "components/ai_answer", "components/thinking_state", "components/button", "components/badge", "layouts/region"],
        capabilities: #{} },
    schema: #{
        props: #{ key: #{schema:#{type:"string"},required:true,sensitive:false} },
        state: #{ fields: #{
            draft: #{schema:#{type:"string"},"default":#{type:"string",value:"Explain the failed rollout."}},
            busy: #{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            round: #{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
            progress: #{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
            messages: #{schema:#{type:"array",items:#{type:"ui_value"}},"default":#{type:"array",value:[]}},
            link: #{schema:#{type:"string"},"default":#{type:"string",value:""}},
            following: #{schema:#{type:"bool"},"default":#{type:"bool",value:true}},
            unseen: #{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
            model: #{schema:#{type:"ui_value"},"default":#{type:"map",value:#{
                model:#{type:"string",value:"auto"},thinking:#{type:"bool",value:false},
                search:#{type:"bool",value:false},effort:#{type:"string",value:"medium"}
            }}},
        } },
        events: #{}, slots: #{}, parts: ["root"],
    },
    render: Fn("assistant_render")
});

fn assistant_draft(ctx, value) { ctx.set_state("draft", value); }
fn has_message(value) {
    // Rhai trim mutates the string and returns unit, not the trimmed string.
    let trimmed = value;
    trimmed.trim();
    trimmed != ""
}
fn assistant_send(ctx, value) {
    if ctx.get_state("busy") || !has_message(value) { return; }
    let round = ctx.get_state("round") + 1;
    let messages = ctx.get_state("messages");
    // Keep the demonstration bounded; dropping old pairs exercises keyed anchoring.
    while messages.len >= 40 { messages.remove(0); messages.remove(0); }
    messages.push(#{key:`user-${round}`,kind:"user",text:value,id:round,progress:0});
    messages.push(#{key:`reply-${round}`,kind:"reply",text:value,id:round,progress:0,phase:"reading"});
    ctx.set_state("messages", messages);
    ctx.set_state("round", round);
    ctx.set_state("progress", 0);
    ctx.set_state("draft", "");
    ctx.set_state("busy", true);
    ctx.scroll_collection_to_end("conversation");
}
fn assistant_stop(ctx, payload) {
    if !ctx.get_state("busy"){return;}
    let messages=ctx.get_state("messages");
    messages[messages.len-1].phase="stopped";
    ctx.set_state("messages",messages);
    ctx.set_state("busy", false);
    ctx.cancel_timeout("reply.tick");
}
fn assistant_tick(ctx, payload) {
    if !ctx.get_state("busy") || payload.round != ctx.get_state("round") { return; }
    let progress = ctx.get_state("progress") + 1;
    let messages = ctx.get_state("messages");
    messages[messages.len - 1].progress = progress;
    messages[messages.len - 1].phase = if progress>=24{"done"}else{"writing"};
    ctx.set_state("messages", messages);
    ctx.set_state("progress", progress);
    if progress >= 24 { ctx.set_state("busy", false); }
}
fn assistant_link(ctx, payload) { ctx.set_state("link", payload.href); }
fn assistant_source(ctx, payload) { ctx.set_state("link", payload.url); }
fn assistant_work_source(ctx, payload) { ctx.set_state("link", payload.href); }
fn assistant_model(ctx,value){ctx.set_state("model",value);}
fn assistant_retry(prompt,ctx,payload){assistant_send(ctx,prompt);}
fn assistant_follow_up(ctx,prompt){ctx.set_state("draft",prompt);}
fn assistant_follow(ctx, payload) {
    ctx.set_state("following", payload.following);
    ctx.set_state("unseen", payload.unseen);
}
fn assistant_latest(ctx, payload) { ctx.scroll_collection_to_end("conversation"); }
fn assistant_history(ctx, payload) { ctx.scroll_collection_to_start("conversation"); }
fn assistant_page(distance, ctx, payload) { ctx.scroll_collection_by("conversation", distance); }

fn assistant_message(total, ctx, payload) {
    let message = payload.item;
    let content = if message.kind == "user" {
        text(message.text).selectable(true).with_style(style().typography("body"))
    } else {
        let steps=[#{id:"inspect",label:"Inspect the health check",state:if message.progress>0{"done"}else{"running"}}];
        if message.progress>0{
            steps.push(#{id:"sources",kind:"search",label:"Read rollout evidence",
                state:if message.progress>=14{"done"}else{"running"},sources:[
                    #{id:"runbook",label:"Rollout investigation",href:"https://example.com/runbook"},
                ]});
        }
        if message.progress>=14{steps.push(#{id:"write",label:"Write the reply",state:if message.phase=="done"{"done"}else{"running"}});}
        let work=thinking_state::ThinkingState(#{key:"public-work",status:if message.phase=="reading"||message.phase=="writing"{"working"}else{message.phase},
            label:"Work in progress",default_open:false,elapsed_ms:message.progress*70,steps:steps,on_open_source:Fn("assistant_work_source")});
        let reply=answer::AIAnswer(#{key:`answer-${message.id}`,label:`Reply ${message.id}`,
            source:if message.progress>0{gallery_reply_document(message.id,message.progress)}else{()},
            status:message.phase,
            sources:if message.progress>0{[#{id:"runbook",site:"Runbook",title:"Rollout investigation",
                url:"https://example.com/runbook",quote:"Inspect the failed health check and retry only the affected host."}]}else{[]},
            on_link:Fn("assistant_link"),on_open_source:Fn("assistant_source"),
            on_stop:Fn("assistant_stop"),on_regenerate:Fn("assistant_retry").curry(message.text),
            follow_ups:["Which host failed?","Show the retry instructions"],on_follow_up:Fn("assistant_follow_up")});
        column([work,reply]).with_style(style().min_width(px(0)).gap(theme_spacing("sm")))
    };
    column([
        text(if message.kind == "user" {"YOU"} else {"ASSISTANT"})
            .with_style(style().typography("label").text_color(theme_color("text_muted"))),
        content,
    ]).with_key(payload.key).with_style(style().gap(theme_spacing("sm"))
        .padding(theme_spacing("md")).min_width(px(0)))
        .accessibility_role("listitem").accessibility_label(if message.kind == "user" {"Your prompt"} else {`Reply ${message.id}`})
        .accessibility_position_in_set(payload.index + 1).accessibility_size_of_set(total)
}

fn assistant_render(ctx, props) {
    let busy = ctx.get_state("busy");
    if busy {
        timeout("reply.tick", 70, false, Fn("assistant_tick"),
            #{round:ctx.get_state("round"),progress:ctx.get_state("progress")});
    }
    let list = virtual_collection(#{key:"conversation",label:"Conversation",
        data:ctx.get_state("messages"),estimated_height:120,fill_height:true,
        overdraw_pixels:240,alignment:"bottom",follow_tail:true,
        on_follow_change:Fn("assistant_follow")},Fn("assistant_message").curry(ctx.get_state("messages").len))
        .accessibility_role("list").accessibility_label("Conversation history").tab_stop(true)
        .with_style(style().flex_grow().flex_basis(px(0)).min_height(px(0))
            .border(px(1)).border_color(theme_color("border"))
            .focus(style().border_color(theme_color("focus_ring"))))
        .on_key_value("home",Fn("assistant_history"),()).on_key_value("end",Fn("assistant_latest"),())
        .on_key_value("pageup",Fn("assistant_page").curry(-240),())
        .on_key_value("pagedown",Fn("assistant_page").curry(240),());
    let tools = [];
    if !ctx.get_state("following") {
        tools.push(button::Button(#{key:"latest",text:`Jump to latest (${ctx.get_state("unseen")})`,
            on_click:Fn("assistant_latest")}));
    }
    let editor = composer::Composer(#{key:"prompt",label:"Message",value:ctx.get_state("draft"),
        placeholder:"Enter sends; Shift+Enter adds a line",busy:busy,
        can_submit:has_message(ctx.get_state("draft")),max_length:4000,
        leading:model_picker::ModelPicker(#{key:"response-model",placement:"top",value:ctx.get_state("model"),
            models:[#{id:"auto",name:"Auto"},#{id:"quick",name:"Quick",description:"Fast triage",speed:5,quality:3,cost:1},
                #{id:"deep",name:"Review",description:"Detailed diagnosis",speed:2,quality:5,cost:3}],
            on_change:Fn("assistant_model")}),
        on_change:Fn("assistant_draft"),on_submit:Fn("assistant_send"),on_stop:Fn("assistant_stop")})
        .with_style(style().width(relative(1.0)));
    let link = ctx.get_state("link");
    if link != "" { tools.push(kit::note(`Link requested (not opened): ${link}`)); }
    region::Region(#{label:"Assistant conversation",title:kit::heading("Assistant conversation",
        "Send, stop, keep drafting, scroll back and follow again. Replies stay in Rust."),
        actions:[badge::Badge(#{text:if busy {"Streaming"} else {"Ready"}}),
            badge::Badge(#{text:`Reply ${ctx.get_state("round")} · ${ctx.get_state("progress")}/24 chunks`})
                .test_id("reply-progress").accessibility_role("status")],
        // The composer docks under the history in the body: Region's footer is a status bar
        // with a rule above it, which a framed field does not need.
        body:column([list, editor])
            .with_style(style().height(relative(1.0)).min_height(px(0)).gap(theme_length("space.group"))),
        toolbar:if tools.len == 0 {()}else{row(tools).with_style(style().gap(theme_length("space.related")).items_center().flex_wrap())}})
        .with_style(ctx.component_style("root",style().height(relative(1.0)).min_height(px(0))))
}

fn view(ctx,h,v,id) {
    if id == "scene.messages" { render_component("gallery/assistant",#{key:"assistant"}) }
    else if id == "run_controls" {
        kit::page(h,v,"run_controls","RunControls","Send, steer, queue or recover a Host-owned run.",[
            kit::sec("Run workspace",run_demo::RunDemo(#{key:"demo-run"}))
        ])
    }
    else { () }
}
