// A real controlled task: send a draft, stream native documents, pause following and stop.
import "gallery/kit" as kit;
import "patterns/composer" as composer;
import "components/markdown_viewer" as markdown;
import "components/button" as button;
import "components/badge" as badge;
import "layouts/region" as region;

fn pages() {
    [#{ id: "scene.messages", title: "Assistant conversation", group: "Scenes" }]
}

define_component(#{
    metadata: #{ id: "gallery/assistant", "export": "Assistant", version: "0.2.2",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["patterns/composer", "components/markdown_viewer", "components/button", "components/badge", "layouts/region"],
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
    messages.push(#{key:`reply-${round}`,kind:"reply",text:"",id:round,progress:0});
    ctx.set_state("messages", messages);
    ctx.set_state("round", round);
    ctx.set_state("progress", 0);
    ctx.set_state("draft", "");
    ctx.set_state("busy", true);
    ctx.scroll_collection_to_end("conversation");
}
fn assistant_stop(ctx, payload) {
    ctx.set_state("busy", false);
    ctx.cancel_timeout("reply.tick");
}
fn assistant_tick(ctx, payload) {
    if !ctx.get_state("busy") || payload.round != ctx.get_state("round") { return; }
    let progress = ctx.get_state("progress") + 1;
    let messages = ctx.get_state("messages");
    messages[messages.len - 1].progress = progress;
    ctx.set_state("messages", messages);
    ctx.set_state("progress", progress);
    if progress >= 24 { ctx.set_state("busy", false); }
}
fn assistant_link(ctx, payload) { ctx.set_state("link", payload.href); }
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
        markdown::MarkdownViewer(#{key:`markdown-${message.id}`,label:`Reply ${message.id}`,
            source:gallery_reply_document(message.id, message.progress),height:"content",
            on_link:Fn("assistant_link")})
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
        .with_style(style().border(px(1)).border_color(theme_color("border"))
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
        on_change:Fn("assistant_draft"),on_submit:Fn("assistant_send"),on_stop:Fn("assistant_stop")})
        .with_style(style().width(relative(1.0)));
    let link = ctx.get_state("link");
    if link != "" { tools.push(kit::note(`Link requested (not opened): ${link}`)); }
    region::Region(#{label:"Assistant conversation",title:kit::heading("Assistant conversation",
        "Send, stop, keep drafting, scroll back and follow again. Replies stay in Rust."),
        actions:[badge::Badge(#{text:if busy {"Streaming"} else {"Ready"}}),
            badge::Badge(#{text:`Reply ${ctx.get_state("round")} · ${ctx.get_state("progress")}/24 chunks`})
                .test_id("reply-progress").accessibility_role("status")],body:list,
        toolbar:if tools.len == 0 {()}else{row(tools).with_style(style().gap(theme_length("space.related")).items_center().flex_wrap())},
        footer:[editor]})
        .with_style(ctx.component_style("root",style().height(relative(1.0)).min_height(px(0))))
}

fn view(ctx,h,v,id) {
    if id == "scene.messages" { render_component("gallery/assistant",#{key:"assistant"}) }
    else { () }
}
