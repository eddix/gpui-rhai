// Gallery pages: L2 layouts and patterns.

import "gallery/kit" as kit;
import "layouts/stack" as stack;
import "layouts/inline" as inline;
import "layouts/toolbar" as toolbar;
import "layouts/region" as region;
import "patterns/section" as section;
import "patterns/description_list" as description_list;
import "patterns/stat" as stat;
import "patterns/form_layout" as form_layout;
import "patterns/inline_state" as inline_state;
import "patterns/data_view" as data_view;
import "patterns/list_detail" as list_detail;
import "patterns/app_shell" as app_shell;
import "components/button" as button;
import "components/input" as input;
import "components/select" as select;
import "components/tag" as tag;
import "components/badge" as badge;
import "components/table" as table;
import "components/tree" as tree;

fn pages() {
    [
        #{ id: "stack", title: "Stack", group: "Layouts and patterns" },
        #{ id: "inline", title: "Inline", group: "Layouts and patterns" },
        #{ id: "toolbar", title: "Toolbar", group: "Layouts and patterns" },
        #{ id: "region", title: "Region", group: "Layouts and patterns" },
        #{ id: "section", title: "Section", group: "Layouts and patterns" },
        #{ id: "description_list", title: "DescriptionList", group: "Layouts and patterns" },
        #{ id: "stat", title: "Stat", group: "Layouts and patterns" },
        #{ id: "form_layout", title: "FormLayout", group: "Layouts and patterns" },
        #{ id: "inline_state", title: "InlineState", group: "Layouts and patterns" },
        #{ id: "data_view", title: "DataView", group: "Layouts and patterns" },
        #{ id: "list_detail", title: "ListDetail", group: "Layouts and patterns" },
        #{ id: "app_shell", title: "AppShell", group: "Layouts and patterns" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "stack" { stack_page(h, v) }
    else if id == "inline" { inline_page(h, v) }
    else if id == "toolbar" { toolbar_page(h, v) }
    else if id == "region" { region_page(h, v) }
    else if id == "section" { section_page(h, v) }
    else if id == "description_list" { description_list_page(h, v) }
    else if id == "stat" { stat_page(h, v) }
    else if id == "form_layout" { form_layout_page(h, v) }
    else if id == "inline_state" { inline_state_page(h, v) }
    else if id == "data_view" { data_view_page(h, v) }
    else if id == "list_detail" { list_detail_page(h, v) }
    else if id == "app_shell" { app_shell_page(h, v) }
    else { () }
}

fn block(label_text) {
    row([text(label_text)]).with_style(style().min_height(theme_length("metrics.row"))
        .padding_x(theme_length("metrics.inset")).items_center().background(theme_color("surface_raised")))
}

fn stack_page(h, v) {
    kit::page(h, v, "stack", "Stack", "A column with one spacing relationship. Prefer relationship names over raw steps.", [
        kit::sec("Related", stack::Stack(#{ children: [block("One"), block("Two"), block("Three")] })
            .with_style(style().width(px(320)))),
        kit::sec("Group", stack::Stack(#{ gap: "group", children: [block("Group A"), block("Group B")] })
            .with_style(style().width(px(320)))),
    ])
}

fn inline_page(h, v) {
    kit::page(h, v, "inline", "Inline", "A row with one relationship; `size` reaches every control inside.", [
        kit::sec("Small controls", inline::Inline(#{ size: "sm", children: [
            button::Button(#{ key: "inline-a", text: "Refresh", on_click: h.noop }),
            button::Button(#{ key: "inline-b", text: "Export", variant: "ghost", on_click: h.noop }),
            input::Input(#{ key: "inline-filter", label: "Filter", value: "", placeholder: "Filter", on_change: h.noop })
                .with_style(style().width(px(180))),
        ] })),
    ])
}

fn hosts_toolbar(h, v, key) {
    toolbar::Toolbar(#{ label: "Hosts toolbar",
        context: [tag::Tag(#{ facet: "site", text: "i18n" }), tag::Tag(#{ facet: "env", text: "prod", variant: "danger" })],
        filters: [input::Input(#{ key: `${key}-filter`, label: "Filter hosts", value: kit::val(v, "toolbar.filter", ""),
            placeholder: "Filter hosts", on_change: kit::store(h, "toolbar.filter") }).with_style(style().width(px(200)))],
        actions: [button::Button(#{ key: `${key}-export`, text: "Export", variant: "ghost", on_click: h.noop }),
            button::Button(#{ key: `${key}-refresh`, text: "Refresh", on_click: h.noop })],
        primary: button::Button(#{ key: `${key}-deploy`, text: "Deploy", variant: "primary", on_click: h.noop }) })
}

fn toolbar_page(h, v) {
    kit::page(h, v, "toolbar", "Toolbar", "Context and filters at the start, actions and one primary at the end.", [
        kit::sec("Hosts", hosts_toolbar(h, v, "toolbar")),
        kit::sec("Search fills the bar", toolbar::Toolbar(#{ label: "Logs toolbar",
            context: [tag::Tag(#{ facet: "psm", text: "api.gateway" })],
            fill: input::Input(#{ key: "toolbar-query", label: "Query logs", value: kit::val(v, "toolbar.query", ""),
                placeholder: "Query: level, trace id, message", on_change: kit::store(h, "toolbar.query") }),
            actions: [button::Button(#{ key: "toolbar-run", text: "Run", on_click: h.noop })] })),
    ])
}

fn region_page(h, v) {
    let sample = region::Region(#{ label: "Example region", title: "Deployments",
        actions: [button::Button(#{ key: "region-new", text: "New", size: "sm", on_click: h.noop })],
        body: kit::note("The body fills the remaining height."), footer: [text("3 deployments")], fill: false });
    let lines = [];
    for index in 1..=12 { lines.push(text(`Event ${index}: replica restarted`)); }
    let scrolling = region::Region(#{ label: "Events", title: "Events", scroll: true,
        body: column(lines).with_style(style().gap(theme_length("space.related"))), footer: [text("12 events")] });
    kit::page(h, v, "region", "Region", "Title and actions, toolbar, a body that fills, a footer pinned to the bottom.", [
        kit::sec("Example", column([sample]).with_style(style().height(px(200)).background(theme_color("surface_raised")))),
        kit::sec("Scrolling body", column([scrolling]).with_style(style().height(px(200)).background(theme_color("surface_raised")))),
    ])
}

fn section_page(h, v) {
    kit::page(h, v, "section", "Section", "A subtitle with actions on the end side, or the label voice for side panels.", [
        kit::sec("Subtitle", section::Section(#{ title: "Replicas", description: "Running copies of the service.",
            actions: [button::Button(#{ key: "section-scale", text: "Scale", size: "sm", on_click: h.noop })],
            content: kit::note("Three replicas in eu-west.") })),
    ])
}

fn facts() {
    [
        #{ label: "Region", value: "eu-west" }, #{ label: "Cores", value: "32", numeric: true },
        #{ label: "Address", value: "10.0.4.17", identifier: true }, #{ label: "负责人", value: "infra-oncall" },
    ]
}

fn description_list_page(h, v) {
    kit::page(h, v, "description_list", "DescriptionList", "Label and value pairs on one label column, metrics.row high.", [
        kit::sec("Columns", description_list::DescriptionList(#{ label: "Host facts", items: facts() })),
        kit::sec("Stacked", description_list::DescriptionList(#{ label: "Host facts stacked", items: facts(), layout: "stacked" })),
    ])
}

fn stat_page(h, v) {
    kit::page(h, v, "stat", "Stat", "A figure with its unit in muted text; only a deviating delta takes color.", [
        kit::sec("Row", stat::stats([
            #{ label: "Hosts", value: "128" }, #{ label: "CPU", value: "98.6", unit: "%", delta: "+31 in 1 h", tone: "warning" },
            #{ label: "Memory", value: "61", unit: "GiB", delta: "steady" },
        ])),
    ])
}

fn form_layout_page(h, v) {
    let name = input::Input(#{ key: "form-layout-name", label: "Service name", value: kit::val(v, "form_layout.name", ""),
        placeholder: "api-gateway", on_change: kit::store(h, "form_layout.name") }).with_style(style().width(px(240)));
    let region_select = select::Select(#{ key: "form-layout-region", label: "Region", value: kit::val(v, "form_layout.region", "eu-west"),
        open: kit::val(v, "form_layout.region_open", false), query: "", width: px(200),
        options: [#{ value: "eu-west", label: "eu-west" }, #{ value: "us-east", label: "us-east" }],
        on_change: kit::store(h, "form_layout.region"), on_open_change: kit::store(h, "form_layout.region_open") });
    kit::page(h, v, "form_layout", "FormLayout", "Labels on one column, fields `related`, groups `group`, submit on the field edge.", [
        kit::sec("Service", form_layout::FormLayout(#{ key: "form-layout", label: "Service", groups: [
            #{ title: "Identity", fields: [#{ label: "Service name", control: name, required: true,
                description: "Lowercase and dashes." }] },
            #{ title: "Placement", fields: [#{ label: "Region", control: region_select }] },
        ], submit: [button::Button(#{ key: "form-layout-create", text: "Create", variant: "primary", on_click: h.noop })] })),
    ])
}

fn inline_state_page(h, v) {
    kit::page(h, v, "inline_state", "InlineState", "Feedback where the content would appear.", [
        kit::sec("Loading", inline_state::InlineState(#{ key: "state-loading", state: "loading", title: "Loading hosts" })),
        kit::sec("Empty", inline_state::InlineState(#{ key: "state-empty", state: "empty", title: "No hosts yet",
            description: "Register a host to see it here." })),
        kit::sec("Error", inline_state::InlineState(#{ key: "state-error", state: "error", title: "Health check failed",
            detail: "connect ECONNREFUSED 10.0.4.17:443" })),
        kit::sec("Stale", inline_state::InlineState(#{ key: "state-stale", state: "stale", title: "Showing cached data",
            description: "The API did not answer." })),
    ])
}

fn host_rows() {
    [
        #{ id: "a", host: "api-01", region: "us-east", cpu: "42.1" },
        #{ id: "b", host: "worker-07", region: "eu-west", cpu: "98.6" },
        #{ id: "c", host: "缓存-03", region: "ap-sg", cpu: "13.25" },
    ]
}

fn host_table(h, v, key, height) {
    table::Table(#{ key: key, label: "Hosts", row_key: "id", height: height, rows: host_rows(),
        columns: [#{ key: "host", title: "Host", width: #{ kind: "flex", value: 2.0 } },
            #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 120.0 } },
            #{ key: "cpu", title: "CPU %", width: #{ kind: "fixed", value: 96.0 }, numeric: true }],
        selected_keys: kit::val(v, "data_view.selected", ["b"]), selection_mode: "single",
        on_selection_change: kit::store(h, "data_view.selected") })
}

fn data_view_page(h, v) {
    let view = data_view::DataView(#{ key: "data-view", label: "Hosts", title: "Hosts", body: host_table(h, v, "data-view-table", 160.0),
        toolbar: #{ filters: [input::Input(#{ key: "data-view-filter", label: "Filter", value: "", placeholder: "Filter",
            on_change: h.noop }).with_style(style().width(px(200)))],
            primary: button::Button(#{ key: "data-view-add", text: "Add host", variant: "primary", on_click: h.noop }) },
        footer: #{ status: badge::Badge(#{ text: "1 degraded", variant: "warning" }), count: "3 hosts", updated: "Updated 12:04" } });
    kit::page(h, v, "data_view", "DataView", "Toolbar, rows that bleed to the region edge, and a footer with count and data time.", [
        kit::sec("Hosts", column([view]).with_style(style().height(px(340)).background(theme_color("surface_raised")))),
    ])
}

fn list_detail_page(h, v) {
    let list = tree::Tree(#{ key: "list-detail-tree", label: "Hosts", fill_height: true,
        items: [#{ key: "api-01", label: "api-01" }, #{ key: "worker-07", label: "worker-07" }, #{ key: "cache-03", label: "缓存-03" }],
        expanded: [], selected_keys: kit::val(v, "list_detail.selected", ["worker-07"]),
        on_selection_change: kit::store(h, "list_detail.selected") });
    let detail = region::Region(#{ label: "Host", title: "worker-07", fill: false,
        body: description_list::DescriptionList(#{ label: "worker-07 facts", items: facts() }) });
    kit::page(h, v, "list_detail", "ListDetail", "A fixed list and a filling detail, separated by one hairline.", [
        kit::sec("Hosts", column([list_detail::ListDetail(#{ label: "Hosts", list: list, detail: detail, list_size: px(200) })])
            .with_style(style().height(px(260)))),
    ])
}

fn app_shell_page(h, v) {
    kit::page(h, v, "app_shell", "AppShell", "This Gallery is an AppShell: title bar, sidebar, main, inspector, status bar.", [
        kit::sec("Regions", kit::note("F6 and Shift+F6 move focus between the sidebar, the main area and the source inspector.")),
        kit::sec("Keyboard", description_list::DescriptionList(#{ label: "Shell keys", items: [
            #{ label: "F6", value: "Next region" }, #{ label: "Shift+F6", value: "Previous region" },
            #{ label: "⌘K", value: "Command palette" }, #{ label: "Tab", value: "Next control in a region" },
        ] })),
    ])
}
