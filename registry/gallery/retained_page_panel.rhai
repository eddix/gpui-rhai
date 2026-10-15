// An independently owned local page: no state is hoisted to the Gallery root.
import "components/input" as input;
import "components/textarea" as textarea;
import "components/scroll_area" as scroll_area;

define_component(#{
    metadata: #{ id: "gallery/retained_page_panel", "export": "PagePanel", version: "0.2.2",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/input", "components/textarea", "components/scroll_area"], capabilities: #{},
        tokens: ["space.related", "typography.caption", "text_muted"], environment: [] },
    schema: #{ props: #{
        label: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Visit label." },
        actions: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false, doc: "Visit-local caller-owned actions, retained with the page." },
    }, state: #{ fields: #{
        filter: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        draft: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
    } }, events: #{}, slots: #{}, parts: ["root"] }, render: Fn("render_PagePanel")
});
fn PagePanel(props) { render_component("gallery/retained_page_panel", props) }
fn panel_filter(ctx, value) { ctx.set_state("filter", value); }
fn panel_draft(ctx, value) { ctx.set_state("draft", value); }
fn render_PagePanel(ctx, props) {
    let rows = [];
    let filter = ctx.get_state("filter");
    for index in 1..31 {
        let line = `${props.label} · instance edge-${index} · ready`;
        if filter == "" || line.contains(filter) { rows.push(text(line).with_key(`instance:${index}`)); }
    }
    let children = [
        input::Input(#{ key: "filter", label: `${props.label} filter`, value: filter, placeholder: "Filter instances", on_change: Fn("panel_filter") }),
        scroll_area::ScrollArea(#{ key: "instances", label: `${props.label} instances`, height: px(144), content: column(rows) }),
        textarea::Textarea(#{ key: "notes", label: `${props.label} draft`, placeholder: "Draft handoff notes — switch pages, then undo", value: ctx.get_state("draft"), rows: 3, on_change: Fn("panel_draft") }),
        text(`Presentation visible: ${ctx.is_visible()}. Business work is application-owned.`)
            .with_style(style().typography("caption").text_color(theme_color("text_muted"))),
    ];
    if "actions" in props && props.actions != () { children.push(props.actions); }
    column(children).with_style(ctx.component_style("root", style().gap(theme_length("space.related")).min_width(px(0))))
}
