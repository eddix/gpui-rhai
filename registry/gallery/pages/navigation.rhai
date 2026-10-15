// Stateful acceptance specimens: local drafts and scroll belong to each page,
// not to the Gallery root. Navigation entries are visits to application data.
import "gallery/kit" as kit;
import "components/tabs" as tabs;
import "gallery/retained_page_panel" as panel;
import "components/button" as button;
import "patterns/navigation_stack" as navigation;

fn pages() {
    [
        #{ id: "retained_tabs", title: "Retained tabs", group: "Navigation" },
        #{ id: "navigation_stack", title: "NavigationStack", group: "Navigation" },
    ]
}
fn view(ctx, h, v, id) {
    if id == "retained_tabs" {
        kit::page(h, v, id, "Retained tabs", "Drafts, native selection/undo and scrolling survive a tab round trip; hidden pages have no input or accessibility presentation.", [
            kit::sec("Service workspace", render_component("gallery/navigation-demo", #{ key: "retained-tabs", mode: id })),
        ])
    } else {
        kit::page(h, v, id, "NavigationStack", "Follow a service → ticket → pipeline → instance, then Back to the actual opener. Each visit owns a separate draft.", [
            kit::sec("Deployment drill-down", render_component("gallery/navigation-demo", #{ key: "navigation", mode: id })),
        ])
    }
}

fn demo_tab(ctx, value) { ctx.set_state("tab", value); }
fn render_RetainedTabsDemo(ctx, props) {
    tabs::Tabs(#{ key: "workspace", label: "Service workspace", value: ctx.get_state("tab"), keep_mounted: true,
        tabs: [
            #{ value: "overview", label: "Overview", content: panel::PagePanel(#{ key: "overview", label: "Checkout service" }) },
            #{ value: "tickets", label: "Tickets", content: panel::PagePanel(#{ key: "tickets", label: "Ticket 4821" }) },
        ], on_change: Fn("demo_tab") }).with_style(ctx.component_style("root", style().width(relative(1.0))))
}

define_component(#{
    metadata: #{ id: "gallery/navigation-demo", "export": "NavigationDemo", version: "0.2.2",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: ["patterns/navigation_stack", "components/button", "components/tabs", "gallery/retained_page_panel"], capabilities: #{}, tokens: ["space.related", "space.group"], environment: [] },
    schema: #{ props: #{ mode: #{ schema: #{ type: "string" }, required: true, sensitive: false, doc: "Retained tabs or navigation-stack specimen." } }, state: #{ fields: #{
        tab: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "overview" } },
        depth: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 1 } },
        header: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "breadcrumb" } },
        refresh: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
    } }, events: #{}, slots: #{}, parts: ["root"] }, render: Fn("render_NavigationDemo")
});
fn demo_push(ctx, event) { if ctx.get_state("depth") < 8 { ctx.set_state("depth", ctx.get_state("depth") + 1); } }
fn demo_pop(ctx, proposal) { ctx.set_state("depth", proposal.depth); }
fn demo_refresh(ctx, event) { ctx.set_state("refresh", ctx.get_state("refresh") + 1); }
fn demo_header(ctx, value) { ctx.set_state("header", value); }
fn visit_title(index) {
    if index == 0 { "checkout-api" } else if index == 1 { "Ticket 4821" }
    else if index == 2 { "Pipeline deploy-prod" } else if index == 3 { "Instance edge-07" }
    else { `Ticket 4821 · visit ${index}` }
}
fn render_NavigationDemo(ctx, props) {
    if props.mode == "retained_tabs" { return render_RetainedTabsDemo(ctx, props); }
    let entries = [];
    for index in 0..ctx.get_state("depth") {
        let title = visit_title(index);
        let content = column([
            panel::PagePanel(#{ key: `panel:${index}`, label: title }),
            button::Button(#{ key: `open:${index}`, text: if index < 3 { `Open ${visit_title(index + 1)}` } else { "Visit ticket 4821 again" }, disabled: index >= 7, on_click: Fn("demo_push") }),
        ]).with_style(style().gap(theme_length("space.related")));
        entries.push(#{ key: `visit:${index}`, title: title, content: content });
    }
    column([
        tabs::Tabs(#{ key: "header-style", label: "Navigation header", panel: false, value: ctx.get_state("header"),
            tabs: [#{ value: "back", label: "Back" }, #{ value: "breadcrumb", label: "Breadcrumb" }, #{ value: "none", label: "None" }], on_change: Fn("demo_header") }),
        navigation::NavigationStack(#{ key: "deployments", label: "Deployment visits", stack: entries,
            header: ctx.get_state("header"), max_depth: 8, motion_key: "deployments", on_pop: Fn("demo_pop"),
            actions: [button::Button(#{ key: "refresh", text: `Refresh ${ctx.get_state("refresh")}`, variant: "ghost", on_click: Fn("demo_refresh") })] }),
    ]).with_style(ctx.component_style("root", style().width(relative(1.0)).gap(theme_length("space.group"))))
}
