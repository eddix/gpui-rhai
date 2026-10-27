// Stateful acceptance specimens: local drafts and scroll belong to each page,
// not to the Gallery root. Navigation entries are visits to application data.
import "gallery/kit" as kit;
import "components/tabs" as tabs;
import "components/card" as card;
import "gallery/retained_page_panel" as panel;
import "components/button" as button;
import "components/tab_overflow_menu" as tab_overflow_menu;
import "gallery/tabs_demo" as tabs_demo;
import "patterns/navigation_stack" as navigation;

fn pages() {
    [
        #{ id: "tabs", title: "Tabs", group: "Navigation" },
        #{ id: "tab_overflow_menu", title: "TabOverflowMenu", group: "Navigation" },
        #{ id: "retained_tabs", title: "Retained tabs", group: "Navigation" },
        #{ id: "navigation_stack", title: "NavigationStack", group: "Navigation" },
    ]
}
fn view(ctx, h, v, id) {
    if id == "tabs" {
        tabs_page(h, v)
    } else if id == "tab_overflow_menu" {
        overflow_page(h, v)
    } else if id == "retained_tabs" {
        kit::page(h, v, id, "Retained tabs", "Drafts, native selection/undo and scrolling survive a tab round trip; hidden pages have no input or accessibility presentation.", [
            kit::sec("Service workspace", render_component("gallery/navigation-demo", #{ key: "retained-tabs", mode: id })),
        ])
    } else if id == "navigation_stack" {
        kit::page(h, v, id, "NavigationStack", "Follow a service → ticket → pipeline → instance, then Back to the actual opener. Command+[ backs the focused stack; each visit owns a separate draft.", [
            kit::sec("Deployment drill-down", render_component("gallery/navigation-demo", #{ key: "navigation", mode: id })),
        ])
    } else {
        ()
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
        let content = panel::PagePanel(#{ key: `panel:${index}`, label: title,
            actions: button::Button(#{ key: `open:${index}`, text: if index < 3 { `Open ${visit_title(index + 1)}` } else { "Visit ticket 4821 again" }, disabled: index >= 7, on_click: Fn("demo_push") }) });
        entries.push(#{ key: `visit:${index}`, title: title, content: content });
    }
    column([
        tabs::Tabs(#{ key: "header-style", label: "Navigation header", panel: false, value: ctx.get_state("header"),
            tabs: [#{ value: "back", label: "Back" }, #{ value: "breadcrumb", label: "Breadcrumb" }, #{ value: "none", label: "None" }], on_change: Fn("demo_header") }),
        navigation::NavigationStack(#{ key: "deployments", label: "Deployment visits", stack: entries,
            header: ctx.get_state("header"), max_depth: 8, motion_key: "deployments", back_key: "cmd+[", on_pop: Fn("demo_pop"),
            actions: [button::Button(#{ key: "refresh", text: `Refresh ${ctx.get_state("refresh")}`, variant: "ghost", on_click: Fn("demo_refresh") })] }),
    ]).with_style(ctx.component_style("root", style().width(relative(1.0)).gap(theme_length("space.group"))))
}

// ---- Tabs ----------------------------------------------------------------------------

fn tab_list() {
    [
        #{ value: "overview", label: "Overview", content: text("Service overview.") },
        #{ value: "logs", label: "Logs", content: text("Recent log lines.") },
        #{ value: "traces", label: "调用链", content: text("Trace samples.") },
        #{ value: "settings", label: "Settings", content: text("Configuration."), disabled: true },
    ]
}

fn settings_tabs() {
    [
        #{ value: "general", label: "General", content: text("Name, region and owner.") },
        #{ value: "alerts", label: "Alerts", content: text("Paging rules for this service.") },
        #{ value: "access", label: "Access", content: text("Who can deploy.") },
    ]
}

fn tabs_page(h, v) {
    let value = kit::val(v, "tabs.value", "overview");
    kit::page(h, v, "tabs", "Tabs", "One component, three looks: a track for a switcher among controls, underlined labels for the sections of a panel or a card, a bar of document tabs. Every look can count, close, add and reorder.", [
        kit::sec("Track with panel", tabs::Tabs(#{ label: "Service", value: value, tabs: tab_list(),
            on_change: kit::store(h, "tabs.value") })),
        kit::sec("Track switcher", tabs::Tabs(#{ label: "View", value: value, tabs: tab_list(), panel: false, size: "sm",
            on_change: kit::store(h, "tabs.value") })),
        kit::sec("Underline", tabs::Tabs(#{ label: "Service sections", value: value, variant: "underline",
            tabs: tab_list(), on_change: kit::store(h, "tabs.value") })),
        kit::sec("Underline in a card", card::Card(#{ header: text("Checkout service"),
            content: tabs::Tabs(#{ label: "Service settings", value: kit::val(v, "tabs.settings", "general"),
                variant: "underline", tabs: settings_tabs(), on_change: kit::store(h, "tabs.settings") }) })
            .with_style(style().width(px(360)))),
        kit::sec("Watched repositories", tabs_demo::TabsDemo(#{ key: "tabs-repos", mode: "repos" })),
        kit::sec("Bar of open files", tabs_demo::TabsDemo(#{ key: "tabs-files", mode: "files" })),
        kit::sec("Vertical", tabs_demo::TabsDemo(#{ key: "tabs-vertical", mode: "vertical" })),
    ])
}

fn overflow_page(h, v) {
    let value = kit::val(v, "tab_overflow_menu.value", "logs");
    let entries = [];
    for tab in tab_list() { entries.push(#{ value: tab.value, label: tab.label, disabled: tab_flag(tab) }); }
    kit::page(h, v, "tab_overflow_menu", "TabOverflowMenu", "A chevron whose menu lists every tab of a strip. It keeps the menu's own state, so the tabs beside it stay stateless; Tabs uses it for overflow_menu.", [
        kit::sec("Beside a track switcher", row([
            tabs::Tabs(#{ label: "View", value: value, tabs: tab_list(), panel: false,
                on_change: kit::store(h, "tab_overflow_menu.value") }),
            tab_overflow_menu::TabOverflowMenu(#{ key: "overflow-specimen", label: "All views", value: value,
                items: entries, on_pick: kit::store(h, "tab_overflow_menu.value") }),
        ]).with_style(style().items_center().gap(theme_spacing("xs")))),
    ])
}
fn tab_flag(tab) { "disabled" in tab && tab.disabled }
