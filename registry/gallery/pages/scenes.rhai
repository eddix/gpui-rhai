// Gallery scenes: realistic compositions that must be completable by keyboard alone.

import "gallery/kit" as kit;
import "layouts/region" as region;
import "layouts/toolbar" as toolbar;
import "layouts/stack" as stack;
import "patterns/section" as section;
import "patterns/description_list" as description_list;
import "patterns/stat" as stat;
import "patterns/form_layout" as form_layout;
import "patterns/inline_state" as inline_state;
import "patterns/data_view" as data_view;
import "patterns/list_detail" as list_detail;
import "components/button" as button;
import "components/input" as input;
import "components/select" as select;
import "components/checkbox" as checkbox;
import "components/radio_group" as radio_group;
import "components/date_picker" as date_picker;
import "components/switch" as switch_control;
import "components/slider" as slider;
import "components/tabs" as tabs;
import "components/table" as table;
import "components/tag" as tag;
import "components/badge" as badge;
import "components/toggle" as toggle;
import "components/pagination" as pagination;

fn pages() {
    [
        #{ id: "scene.operations", title: "Operations workbench", group: "Scenes" },
        #{ id: "scene.data", title: "Data browser", group: "Scenes" },
        #{ id: "scene.form", title: "Form", group: "Scenes" },
        #{ id: "scene.settings", title: "Settings", group: "Scenes" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "scene.operations" { operations(h, v) }
    else if id == "scene.data" { data_browser(h, v) }
    else if id == "scene.form" { form(h, v) }
    else if id == "scene.settings" { settings(h, v) }
    else { () }
}

// ---- operations workbench -------------------------------------------------------------

fn fleet() {
    [
        #{ id: "edge-01", host: "edge-01", region: "sg1", cpu: "41.0", version: "1.4.2", uptime: "12d", state: "Online", tone: "neutral" },
        #{ id: "edge-02", host: "edge-02", region: "sg1", cpu: "37.5", version: "1.4.2", uptime: "12d", state: "Online", tone: "neutral" },
        #{ id: "edge-03", host: "edge-03", region: "fra2", cpu: "97.8", version: "1.4.1", uptime: "3h", state: "Degraded", tone: "warning" },
    ]
}

fn filtered(filter) {
    let rows = [];
    let needle = filter.to_lower();
    for row in fleet() { if needle == "" || row.host.contains(needle) { rows.push(row); } }
    rows
}

fn host_detail(h, v, host) {
    if host == () {
        return inline_state::InlineState(#{ key: "ops-none", state: "empty", title: "No host selected",
            description: "Select a host to see its details." });
    }
    let restarted = kit::val(v, "ops.restarted", "");
    let facts = description_list::DescriptionList(#{ label: `${host.host} facts`, items: [
        #{ label: "Region", value: host.region }, #{ label: "Version", value: host.version, identifier: true },
        #{ label: "Uptime", value: host.uptime },
    ] });
    let children = [facts];
    if host.state == "Degraded" {
        children.push(inline_state::InlineState(#{ key: "ops-health", state: "error", title: "Health check failed",
            detail: "GET /healthz 503 after 2004 ms" }));
    }
    if restarted == host.id {
        children.push(badge::Badge(#{ text: `Restart requested for ${host.host}`, variant: "accent" }));
    }
    let restart = button::Button(#{ key: "ops-restart", text: "Restart", size: "sm",
        on_click: kit::put(h, "ops.restarted", host.id) });
    region::Region(#{ label: `${host.host} detail`, title: host.host, actions: [restart],
        body: stack::Stack(#{ gap: "group", children: children }), fill: true })
}

fn operations(h, v) {
    let filter = kit::val(v, "ops.filter", "");
    let rows = filtered(filter);
    let selected = kit::val(v, "ops.selected", []);
    let current = ();
    for row in rows { if row.id in selected { current = row; } }
    let hosts = table::Table(#{ key: "ops-hosts", label: "Hosts", row_key: "id", fill_height: true, rows: rows,
        columns: [#{ key: "host", title: "Host", width: #{ kind: "flex", value: 2.0 } },
            #{ key: "cpu", title: "CPU %", width: #{ kind: "fixed", value: 96.0 }, numeric: true },
            #{ key: "status", title: "Status", width: #{ kind: "fixed", value: 140.0 },
                adornments: [#{ text_key: "state", variant_key: "tone", dot: true }] }],
        selected_keys: selected, selection_mode: "single", empty_text: "No host matches the filter",
        on_selection_change: kit::store(h, "ops.selected") });
    let bar = toolbar::Toolbar(#{ label: "Hosts toolbar",
        context: [tag::Tag(#{ facet: "site", text: "i18n" }), tag::Tag(#{ facet: "env", text: "prod", variant: "danger" })],
        filters: [input::Input(#{ key: "ops-filter", label: "Filter hosts", value: filter, placeholder: "Filter hosts",
            on_change: kit::store(h, "ops.filter") }).with_style(style().width(px(220)))],
        actions: [button::Button(#{ key: "ops-export", text: "Export", variant: "ghost", on_click: h.noop }),
            button::Button(#{ key: "ops-refresh", text: "Refresh", on_click: h.noop })],
        primary: button::Button(#{ key: "ops-deploy", text: "Deploy", variant: "primary", on_click: h.noop }) });
    let figures = stat::stats([#{ label: "Hosts", value: "3" }, #{ label: "Healthy", value: "2" },
        #{ label: "Needs attention", value: "1", tone: "warning", delta: "edge-03" }]);
    let split = list_detail::ListDetail(#{ label: "Hosts and detail", list: hosts, detail: host_detail(h, v, current),
        list_size: px(420) });
    region::Region(#{ label: "Operations workbench", title: kit::heading("Operations workbench",
        "Filter, select by keyboard, act on the detail."), toolbar: bar,
        body: stack::Stack(#{ gap: "group", fill: true, children: [figures, split] }),
        footer: [badge::Badge(#{ text: "1 warning", variant: "warning" }), text(`${rows.len} hosts · fetched 09:20`)] })
        .with_key("page-scene.operations")
}

// ---- data browser ---------------------------------------------------------------------

fn generated_rows() {
    let rows = [];
    let regions = ["us-east", "eu-west", "ap-sg", "sa-east"];
    for index in 0..120 {
        let cpu = (index * 37) % 100;
        rows.push(#{ id: `host-${index}`, host: `host-${index + 100}`, region: regions[index % 4],
            cpu: `${cpu}.0`, cpu_value: cpu, mem: `${(index * 13) % 64}` });
    }
    rows
}

fn by_cpu(a, b) { if a.cpu_value < b.cpu_value { -1 } else if a.cpu_value > b.cpu_value { 1 } else { 0 } }

fn data_browser(h, v) {
    let sort = kit::val(v, "data.sort", ());
    let rows = generated_rows();
    if sort != () && sort.key == "cpu" {
        rows.sort(Fn("by_cpu"));
        if sort.direction == "descending" { rows.reverse(); }
    }
    let paging = kit::val(v, "data.page", #{ current_page: 1, page_size: 20 });
    let compact = kit::val(v, "data.compact", false);
    let grid = table::Table(#{ key: "data-table", label: "Hosts", row_key: "id", fill_height: true, rows: rows,
        columns: [#{ key: "host", title: "Host", width: #{ kind: "flex", value: 2.0 } },
            #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 120.0 } },
            #{ key: "cpu", title: "CPU %", width: #{ kind: "fixed", value: 96.0 }, numeric: true, sortable: true },
            #{ key: "mem", title: "Mem GiB", width: #{ kind: "fixed", value: 96.0 }, numeric: true }],
        sort: sort, page: paging.current_page, page_size: paging.page_size,
        selected_keys: kit::val(v, "data.selected", []), selection_mode: "multiple",
        on_selection_change: kit::store(h, "data.selected"), on_sort_change: kit::store(h, "data.sort") });
    let pager = pagination::Pagination(#{ key: "data-pages", label: "Host pages", total_items: rows.len,
        current_page: paging.current_page, page_size: paging.page_size, show_summary: false,
        on_change: kit::store(h, "data.page") });
    let density_toggle = toggle::Toggle(#{ key: "data-compact", text: "Compact", pressed: compact,
        on_pressed_change: kit::store(h, "data.compact") });
    let view = data_view::DataView(#{ key: "data-view-scene", label: "Data browser",
        title: "Data browser", body: grid, toolbar: #{ actions: [density_toggle] },
        footer: #{ status: pager, count: `${rows.len} hosts`, selection: `${kit::val(v, "data.selected", []).len} selected` } });
    let node = column([view]).with_style(style().flex_col().flex_grow().min_height(px(0)))
        .with_key("page-scene.data");
    if compact { node.env(#{ density: "compact" }) } else { node }
}

// ---- form -------------------------------------------------------------------------------

fn form_submit(h, attempt) { kit::put(h, "form.submitted", attempt) }

fn form(h, v) {
    let name = kit::val(v, "form.name", "");
    let submitted = kit::val(v, "form.submitted", 0);
    let invalid = submitted != 0 && name == "";
    let attempt = submitted + 1;
    let name_input = input::Input(#{ key: "form-name", label: "Service name", value: name, placeholder: "api-gateway",
        error: invalid, on_change: kit::store(h, "form.name"), on_submit: form_submit(h, attempt) })
        .with_style(style().width(px(260)));
    let region_select = select::Select(#{ key: "form-region", label: "Region", value: kit::val(v, "form.region", "eu-west"),
        open: kit::val(v, "form.region_open", false), query: "", width: px(200),
        options: [#{ value: "eu-west", label: "eu-west" }, #{ value: "us-east", label: "us-east" }, #{ value: "ap-sg", label: "ap-sg" }],
        on_change: kit::store(h, "form.region"), on_open_change: kit::store(h, "form.region_open") });
    let expiry = date_picker::DatePicker(#{ key: "form-expiry", label: "Expires", value: kit::val(v, "form.expiry", ()),
        placeholder: "Never", width: px(200), on_change: kit::store(h, "form.expiry") });
    let alerts = checkbox::Checkbox(#{ checked: kit::val(v, "form.alerts", true), label: "Page the owner on failures",
        on_change: kit::checked(h, "form.alerts") });
    let visibility = radio_group::RadioGroup(#{ value: kit::val(v, "form.visibility", "team"), label: "Visibility",
        orientation: "horizontal", options: [#{ value: "team", label: "Team" }, #{ value: "org", label: "Organization" }],
        on_change: kit::store(h, "form.visibility") });
    let identity = #{ title: "Identity", fields: [
        #{ label: "Service name", control: name_input, required: true,
            description: "Lowercase letters, digits and dashes.", error: if invalid { "Enter a service name." } else { () } },
        #{ label: "Visibility", control: visibility },
    ] };
    let access = #{ title: "Access", fields: [
        #{ label: "Region", control: region_select }, #{ label: "Expires", control: expiry },
        #{ label: "Alerts", control: alerts },
    ] };
    let create = button::Button(#{ key: "form-create", text: "Create service", variant: "primary",
        on_click: form_submit(h, attempt) });
    let layout = form_layout::FormLayout(#{ key: "scene-form", label: "New service", groups: [identity, access],
        submit: [create] });
    let children = [layout];
    if submitted != 0 && name != "" {
        children.insert(0, inline_state::InlineState(#{ key: "form-done", state: "refreshing",
            title: `Service ${name} created` }));
    }
    region::Region(#{ label: "New service", title: kit::heading("New service", "Tab through the fields; Enter submits."),
        body: column([stack::Stack(#{ gap: "group", children: children })])
            .with_style(style().flex_col().flex_grow().min_height(px(0)).overflow_y_scroll()) })
        .with_key("page-scene.form")
}

// ---- settings ---------------------------------------------------------------------------

fn setting_row(control) { control }

fn general_settings(h, v) {
    stack::Stack(#{ gap: "section", children: [
        section::Section(#{ title: "Updates", content: stack::Stack(#{ children: [
            switch_control::Switch(#{ checked: kit::val(v, "settings.updates", true), label: "Install updates automatically",
                on_change: kit::store(h, "settings.updates") }),
            switch_control::Switch(#{ checked: kit::val(v, "settings.beta", false), label: "Join the beta channel",
                on_change: kit::store(h, "settings.beta") }),
        ] }) }),
        section::Section(#{ title: "Editor", content: stack::Stack(#{ children: [
            slider::Slider(#{ key: "settings-font", value: kit::val(v, "settings.font", 14.0), min: 12.0, max: 20.0, step: 1.0,
                label: "Font size", on_change: kit::store(h, "settings.font") }),
        ] }) }),
        section::Section(#{ title: "Managed", description: "Set by your organization.",
            content: stack::Stack(#{ children: [
                switch_control::Switch(#{ checked: true, label: "Send diagnostics", on_change: h.noop }),
                switch_control::Switch(#{ checked: false, label: "Allow extensions", on_change: h.noop }),
            ] }).disabled(true) }),
    ] })
}

fn notification_settings(h, v) {
    section::Section(#{ title: "Channels", content: stack::Stack(#{ children: [
        switch_control::Switch(#{ checked: kit::val(v, "settings.email", true), label: "Email", on_change: kit::store(h, "settings.email") }),
        switch_control::Switch(#{ checked: kit::val(v, "settings.chat", false), label: "飞书", on_change: kit::store(h, "settings.chat") }),
    ] }) })
}

fn settings(h, v) {
    let tab = kit::val(v, "settings.tab", "general");
    let switcher = tabs::Tabs(#{ label: "Settings sections", value: tab, panel: false, on_change: kit::store(h, "settings.tab"),
        tabs: [#{ value: "general", label: "General" }, #{ value: "notifications", label: "Notifications" }] });
    let body = if tab == "notifications" { notification_settings(h, v) } else { general_settings(h, v) };
    region::Region(#{ label: "Settings", title: kit::heading("Settings", "Tabs switch the view; disabled sections inherit."),
        toolbar: switcher, body: column([body]).with_style(style().flex_col().flex_grow().min_height(px(0)).overflow_y_scroll()) })
        .with_key("page-scene.settings")
}
