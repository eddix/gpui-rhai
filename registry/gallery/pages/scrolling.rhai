// Shared scrollbar and growing-content acceptance page. The formal specimen
// owns its state, commands and callback scope; Gallery owns only navigation.
import "gallery/kit" as kit;
import "components/button" as button;
import "components/scroll_area" as scroll_area;
import "components/code_viewer" as code_viewer;

fn pages() { [#{ id: "scrolling", title: "Scrolling", group: "Interaction" }] }

fn view(ctx, h, v, id) {
    if id != "scrolling" { return (); }
    kit::page(h, v, id, "Scrolling", "One themed scrollbar for native documents, retained overflow and growing virtual rows.", [
        kit::sec("Growing transcript", render_component("gallery/scrolling_demo", #{ key: "transcript-demo" })),
        kit::sec("Horizontal and vertical", retained_specimen()),
        kit::sec("Native document", code_viewer::CodeViewer(#{ key: "scrolling-code", label: "Scrolling source",
            source: source_sample(), language: "rhai", scrollbar: "always" })
            .with_style(style().height(px(180)))),
    ])
}

define_component(#{
    metadata: #{ id: "gallery/scrolling_demo", "export": "ScrollingDemo", version: "0.0.1",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: ["components/button"], capabilities: #{},
        tokens: ["metrics.inset", "spacing.xs", "spacing.sm", "typography.body", "typography.caption",
            "text_primary", "text_muted", "border", "focus_ring"], environment: ["density"] },
    schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{
            count: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 40 } },
            first: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
            chunks: #{ schema: #{ type: "map", values: #{ type: "integer" } }, "default": #{ type: "map", value: #{} } },
            following: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
            unseen: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
        } }, events: #{}, slots: #{}, parts: ["root"] }, render: Fn("render_demo")
});

fn demo_append(ctx, value) { ctx.set_state("count", ctx.get_state("count") + 1); }
fn demo_prepend(ctx, value) { ctx.set_state("first", ctx.get_state("first") - 1); }
fn demo_delete(ctx, value) {
    if ctx.get_state("first") < ctx.get_state("count") - 1 { ctx.set_state("first", ctx.get_state("first") + 1); }
}
fn demo_grow(ctx, value) {
    let chunks = ctx.get_state("chunks");
    let key = `event-${ctx.get_state("count") - 1}`;
    chunks[key] = if key in chunks { chunks[key] + 4 } else { 4 };
    ctx.set_state("chunks", chunks);
}
fn demo_latest(ctx, value) { ctx.scroll_collection_to_end("transcript"); }
fn demo_start(ctx, value) { ctx.scroll_collection_to_start("transcript"); }
fn demo_page_up(ctx, value) { ctx.scroll_collection_by("transcript", -200); }
fn demo_page_down(ctx, value) { ctx.scroll_collection_by("transcript", 200); }
fn demo_follow(ctx, value) { ctx.set_state("following", value.following); ctx.set_state("unseen", value.unseen); }

fn demo_row(ctx, payload) {
    let item = payload.item;
    let children = [text(item.label).with_style(style().typography("caption").text_color(theme_color("text_muted"))),
        text(item.body).with_style(style().typography("body").text_color(theme_color("text_primary")))];
    for index in 0..item.growth {
        children.push(text(`Stream chunk ${index + 1}: variable height stays native. 中文、English and wrapping share the same reader anchor.`)
            .with_style(style().typography("body")));
    }
    column(children).with_key(item.key).accessibility_role("listitem").accessibility_label(item.label)
        .with_style(style().width(relative(1.0)).min_width(px(0)).padding(theme_length("metrics.inset"))
            .gap(theme_spacing("xs")).border_bottom(px(1)).border_color(theme_color("border")))
}

fn render_demo(ctx, props) {
    let count = ctx.get_state("count");
    let first = ctx.get_state("first");
    let following = ctx.get_state("following");
    let unseen = ctx.get_state("unseen");
    let chunks = ctx.get_state("chunks");
    let data = [];
    for index in first..count {
        data.push(#{ key: `event-${index}`, label: `Event ${index + 1}`,
            body: if index % 3 == 0 { "A longer event wraps naturally across lines. Read upward, then append or prepend: the row under your eye stays where it was. 这是一条自然换行的事件。" }
                else { "A short event." },
            growth: if `event-${index}` in chunks { chunks[`event-${index}`] } else { 0 } });
    }
    let actions = kit::line([
        button::Button(#{ key: "append", text: "Append event", variant: "secondary", on_click: Fn("demo_append") }),
        button::Button(#{ key: "stream", text: "Stream chunk", variant: "ghost", on_click: Fn("demo_grow") }),
        button::Button(#{ key: "prepend", text: "Prepend history", variant: "ghost", on_click: Fn("demo_prepend") }),
        button::Button(#{ key: "delete", text: "Remove first", variant: "ghost", on_click: Fn("demo_delete") }),
        button::Button(#{ key: "start", text: "Start", variant: "ghost", on_click: Fn("demo_start") }),
        button::Button(#{ key: "latest", text: if following { "Following latest" } else { `Jump to latest (${unseen})` },
            variant: "outline", on_click: Fn("demo_latest") }),
    ]);
    let transcript = virtual_collection(#{ key: "transcript", label: "Growing transcript", data: data,
        estimated_height: 72, height: 300, overdraw_pixels: 160, alignment: "bottom", follow_tail: true,
        on_follow_change: Fn("demo_follow") }, Fn("demo_row"))
        .scrollbars("hidden", "always").accessibility_role("list").accessibility_label("Growing transcript")
        .tab_stop(true).with_style(style().border(px(2)).border_color(theme_color("border"))
            .focus(style().border_color(theme_color("focus_ring"))))
        .on_key_value("end", Fn("demo_latest"), ()).on_key_value("home", Fn("demo_start"), ())
        .on_key_value("pageup", Fn("demo_page_up"), ()).on_key_value("pagedown", Fn("demo_page_down"), ());
    column([actions, transcript,
        kit::note(if following { `${count - first} events. Wheel up or Page Up pauses; reaching the bottom or End resumes.` }
            else { `${unseen} unseen events. Reading position is preserved while content changes.` })
            .accessibility_role("status")
            .accessibility_label(if following { `${count - first} events, following latest` } else { `${unseen} unseen events, reading paused` })])
        .with_style(style().flex_col().gap(theme_spacing("sm")).min_width(px(0)))
}

fn retained_specimen() {
    let rows = [];
    for index in 0..20 { rows.push(text(`Retained row ${index}: a wide content line that also scrolls horizontally.`)
        .with_style(style().typography("body").width(px(960)).padding(theme_length("metrics.inset")))); }
    scroll_area::ScrollArea(#{ key: "scrolling-retained", label: "Retained scrolling surface", axis: "both",
        height: px(180), horizontal_scrollbar: "always", vertical_scrollbar: "always", content: column(rows) })
}

fn source_sample() {
    let source = "// Native uniform-list document viewport\n";
    for index in 0..60 { source += `let event_${index} = "The same themed scrollbar, without a second position engine";` + "\n"; }
    source
}
