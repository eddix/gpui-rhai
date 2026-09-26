import "components/button" as button;
import "components/badge" as badge;
import "components/command" as command;
import "components/command_dialog" as command_dialog;
import "components/dialog" as dialog;
import "components/sheet" as sheet;
import "components/input" as input;
import "components/table" as table;
import "components/pagination" as pagination;
import "components/code_viewer" as code_viewer;
import "components/diff_viewer" as diff_viewer;
import "components/progress" as progress;
import "components/toast" as toast;
import "components/alert" as alert;
import "components/empty" as empty;
import "components/skeleton" as skeleton;
import "components/spinner" as spinner;
import "components/title_bar" as title_bar;
import "components/status_bar" as status_bar;
import "charts/chart" as chart;

define_component(#{
    metadata: #{ id: "stories/operations_workbench", "export": "OperationsWorkbench",
        version: "0.1.6", runtime_api: #{ min_inclusive: 2, max_exclusive: 3 },
        dependencies: ["components/button", "components/badge", "components/command",
            "components/command_dialog", "components/dialog", "components/sheet",
            "components/input", "components/table", "components/pagination", "components/code_viewer",
            "components/diff_viewer", "components/progress", "components/toast", "components/alert",
            "components/empty", "components/skeleton", "components/spinner", "components/title_bar",
            "components/status_bar", "charts/chart"],
        capabilities: #{ "gallery.operations": "*" } },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{
        fixture_case: #{ schema: #{ type: "string", allowed: ["normal", "loading", "empty", "failure", "streaming", "large"] },
            "default": #{ type: "string", value: __OPERATIONS_CASE__ } },
        page: #{ schema: #{ type: "string", allowed: ["dashboard", "hosts", "configurations", "deployments", "settings"] },
            "default": #{ type: "string", value: __OPERATIONS_PAGE__ } },
        selected_hosts: #{ schema: #{ type: "array", max_items: 2, items: #{ type: "string" } },
            "default": #{ type: "array", value: [#{ type: "string", value: "edge-01" }] } },
        host_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        host_page: #{ schema: #{ type: "integer", min: 1 },
            "default": #{ type: "integer", value: 1 } },
        host_page_size: #{ schema: #{ type: "integer", min: 1 },
            "default": #{ type: "integer", value: 25 } },
        host_sort_key: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "host" } },
        host_sort_direction: #{ schema: #{ type: "string", allowed: ["ascending", "descending"] },
            "default": #{ type: "string", value: "ascending" } },
        collapsed_regions: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "string" } },
            "default": #{ type: "array", value: [] } },
        deploy_target: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "edge-01" } },
        release_channel: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "stable" } },
        confirm_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        command_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        sheet_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        deployment_phase: #{ schema: #{ type: "string", allowed: ["idle", "running", "complete", "failed"] },
            "default": #{ type: "string", value: "idle" } },
        deployment_progress: #{ schema: #{ type: "number", min: 0.0, max: 100.0 },
            "default": #{ type: "float", value: 0.0 } },
        toast_visible: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        command_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        command_active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "dashboard" } },
        stream_revision: #{ schema: #{ type: "integer", min: 0 },
            "default": #{ type: "integer", value: 0 } },
        } },
        events: #{}, slots: #{}, parts: [], effects: ["deployment", "fixture_stream"],
    },
    render: Fn("render_OperationsWorkbench")
});

fn OperationsWorkbench(props) { render_component("stories/operations_workbench", props) }

fn navigate(page, ctx, payload) { ctx.set_state("page", page); }
fn hosts_selected(ctx, values) {
    let accepted = [];
    for value in values { if accepted.len < 2 { accepted.push(value); } }
    ctx.set_state("selected_hosts", accepted);
    if accepted.len > 0 {
        ctx.set_state("deploy_target", accepted[0]);
        ctx.set_state("sheet_open", true);
    }
}
fn host_query_changed(ctx, value) {
    ctx.set_state("host_query", value);
    ctx.set_state("host_page", 1);
}
fn hosts_sorted(ctx, value) {
    if value != () {
        ctx.set_state("host_sort_key", value.key);
        ctx.set_state("host_sort_direction", value.direction);
        ctx.set_state("host_page", 1);
    }
}
fn host_page_changed(ctx, value) {
    ctx.set_state("host_page", value.current_page);
    ctx.set_state("host_page_size", value.page_size);
}
fn host_group_toggled(ctx, value) {
    let collapsed = ctx.get_state("collapsed_regions");
    if value in collapsed {
        let next = [];
        for region in collapsed { if region != value { next.push(region); } }
        ctx.set_state("collapsed_regions", next);
    } else {
        collapsed.push(value);
        ctx.set_state("collapsed_regions", collapsed);
    }
}
fn channel_changed(ctx, value) { ctx.set_state("release_channel", value); }
fn open_confirm(ctx, payload) { ctx.set_state("confirm_open", true); }
fn confirm_open_changed(ctx, value) { ctx.set_state("confirm_open", value); }
fn cancel_deployment(ctx, payload) { ctx.set_state("confirm_open", false); }
fn confirm_deployment(ctx, payload) {
    ctx.set_state("confirm_open", false);
    ctx.set_state("deployment_phase", "running");
    ctx.set_state("deployment_progress", 0.0);
    ctx.set_state("toast_visible", false);
    ctx.set_state("page", "deployments");
}
fn deployment_update(ctx, value) {
    if value < 0 {
        ctx.set_state("deployment_phase", "failed");
        ctx.set_state("toast_visible", true);
    } else {
        ctx.set_state("deployment_progress", value.to_float());
        if value >= 100 {
            ctx.set_state("deployment_phase", "complete");
            ctx.set_state("toast_visible", true);
        }
    }
}
fn deployment_error(ctx, error) {
    ctx.set_state("deployment_phase", "failed");
    ctx.set_state("toast_visible", true);
}
fn start_deployment(ctx, deps) {
    if deps.active {
        ctx.start_subscription("gallery.operations", "deploy", deps.fail,
            Fn("deployment_update"), Fn("deployment_error"), #{ delivery: "all" });
    }
}
fn stop_deployment(ctx, deps) { () }
fn stream_update(ctx, value) { ctx.set_state("stream_revision", value); }
fn stream_error(ctx, error) { ctx.set_state("stream_revision", 0); }
fn start_fixture_stream(ctx, deps) {
    if deps.active {
        ctx.start_subscription("gallery.operations", "stream", (),
            Fn("stream_update"), Fn("stream_error"), #{ delivery: "latest", throttle_ms: 16 });
    }
}
fn stop_fixture_stream(ctx, deps) { () }
fn dismiss_toast(ctx, payload) { ctx.set_state("toast_visible", false); }
fn command_query_changed(ctx, value) { ctx.set_state("command_query", value); }
fn command_active_changed(ctx, value) { ctx.set_state("command_active", value); }
fn command_open_changed(ctx, value) { ctx.set_state("command_open", value); }
fn open_command(ctx, payload) { ctx.set_state("command_open", true); }
fn command_action(ctx, value) {
    ctx.set_state("page", value);
    ctx.set_state("command_open", false);
}
fn sheet_open_changed(ctx, value) { ctx.set_state("sheet_open", value); }
fn open_host_sheet(ctx, payload) { ctx.set_state("sheet_open", true); }
fn compare_from_sheet(ctx, payload) {
    ctx.set_state("sheet_open", false);
    ctx.set_state("page", "configurations");
}

fn nav_button(ctx, page, label) {
    button::Button(#{ text: label, size: "sm",
        variant: if ctx.get_state("page") == page { "secondary" } else { "ghost" },
        on_click: Fn("navigate").curry(page) })
        .with_style(style().width(relative(1.0)).justify_start())
}

fn page_header(title, description) {
    column([
        text(title).with_style(style().typography("heading")),
        text(description).with_style(style().typography("body_small").text_color(theme_color("text_muted"))),
    ]).with_style(style().gap(theme_spacing("xxs")).padding_bottom(theme_spacing("sm"))
        .border_bottom(px(1)).border_color(theme_color("border")))
}

fn dashboard(ctx) {
    let fixture = ctx.get_state("fixture_case");
    if fixture == "loading" {
        return column([
            page_header("Dashboard", "Loading a deterministic Host snapshot."),
            row([spinner::Spinner(#{ key: "dashboard-loading", label: "Loading dashboard" }),
                text("Loading host health and metrics…")])
                .with_style(style().gap(theme_spacing("sm")).items_center()),
            skeleton::Skeleton(#{ key: "dashboard-skeleton-a", width: 640, height: 72 }),
            skeleton::Skeleton(#{ key: "dashboard-skeleton-b", width: 640, height: 280 }),
        ]).with_style(style().gap(theme_spacing("md")));
    }
    if fixture == "empty" {
        return column([
            page_header("Dashboard", "The fixture contains no registered hosts."),
            empty::Empty(#{ title: "No hosts yet",
                description: "Add a Host capability in your application to populate this workspace.",
                actions: [button::Button(#{ text: "Open Settings", variant: "outline",
                    on_click: Fn("navigate").curry("settings") })] })
        ]).with_style(style().gap(theme_spacing("md")));
    }
    let metrics = ctx.get_native_chart_data("operations_metrics");
    let host_count = if fixture == "large" { "1,000" } else if fixture == "failure" { "4" } else { "3" };
    let healthy_count = if fixture == "large" { "998" } else { "2" };
    let attention_count = if fixture == "failure" { "2" } else { "1" };
    let fixture_notice = if fixture == "failure" {
        alert::Alert(#{ title: "Partial Host failure",
            description: "edge-04 is unreachable; healthy hosts and historical metrics remain available.",
            variant: "warning" })
    } else if fixture == "streaming" {
        alert::Alert(#{ title: `Streaming revision ${ctx.get_state("stream_revision")}`,
            description: "A bounded Rust subscription updates this session without Rhai polling.",
            variant: "info" })
    } else { column([]) };
    column([
        page_header("Dashboard", "Host health and latency from Rust-owned chart data."),
        fixture_notice,
        row([
            column([text(host_count).with_style(style().typography("display")), text("Hosts")])
                .with_style(style().flex_grow().padding(theme_spacing("md")).border(px(1)).border_color(theme_color("border"))),
            column([text(healthy_count).with_style(style().typography("display")), text("Healthy")])
                .with_style(style().flex_grow().padding(theme_spacing("md")).border(px(1)).border_color(theme_color("border"))),
            column([text(attention_count).with_style(style().typography("display")), text("Needs attention")])
                .with_style(style().flex_grow().padding(theme_spacing("md")).border(px(1)).border_color(theme_color("border"))),
        ]).with_style(style().gap(theme_spacing("sm"))),
        chart::Chart(#{ key: "operations-latency", data: metrics, key_dimension: "id",
            spec: #{ interaction: #{ wheel_zoom: "modifier" }, title: "Latency",
                legend: #{ visible: false }, series: [#{ key: "latency", kind: "line",
                    encode: #{ x: "minute", y: "latency" } }] }
        }).with_style(style().width(relative(1.0)).height(px(280)))
    ]).with_style(style().gap(theme_spacing("md")))
}

fn hosts(ctx) {
    let fixture = ctx.get_state("fixture_case");
    if fixture == "loading" {
        return column([page_header("Hosts", "Loading the Rust NativeCollection."),
            skeleton::Skeleton(#{ key: "hosts-loading", width: 640, height: 320 })])
            .with_style(style().gap(theme_spacing("md")));
    }
    if fixture == "empty" {
        return column([page_header("Hosts", "No rows are available in this fixture."),
            empty::Empty(#{ title: "No hosts", description: "The virtualized collection is empty." })])
            .with_style(style().gap(theme_spacing("md")));
    }
    column([page_header("Hosts", if fixture == "large" { "1,000 Rust-owned rows; only the visible window is realized." }
            else { "Search, sort, and select the Rust-owned Host collection." }),
        row([
            input::Input(#{ key: "host-search", label: "Search hosts",
                value: ctx.get_state("host_query"), placeholder: "Host, region, version, or status",
                autofocus: true, on_change: Fn("host_query_changed") }).test_id("host-search"),
            button::Button(#{ text: `Inspect ${ctx.get_state("deploy_target")}`,
                variant: "outline", on_click: Fn("open_host_sheet") }).test_id("open-host-sheet"),
        ]).with_style(style().gap(theme_spacing("sm")).items_center()),
        table::Table(#{ key: "operations-hosts", label: "Operations hosts", row_key: "id",
        rows: ctx.get_native_collection("operations_hosts"), fill_height: true,
        query: ctx.get_state("host_query"),
        search_fields: ["host", "region", "version", "status"],
        page: ctx.get_state("host_page"), page_size: ctx.get_state("host_page_size"),
        columns: [
            #{ key: "host", title: "Host", width: #{ kind: "flex", value: 1 }, sortable: true,
                adornments: [#{ text_key: "status", variant_key: "status_variant", dot: true }] },
            #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 140 }, sortable: true },
            #{ key: "version", title: "Version", width: #{ kind: "fixed", value: 100 } },
            #{ key: "status", title: "Status", width: #{ kind: "fixed", value: 120 }, sortable: true },
        ], selection_mode: "multiple", selected_keys: ctx.get_state("selected_hosts"),
        sort: #{ key: ctx.get_state("host_sort_key"), direction: ctx.get_state("host_sort_direction") },
        group_by: "region", collapsed_groups: ctx.get_state("collapsed_regions"),
        on_selection_change: Fn("hosts_selected"), on_sort_change: Fn("hosts_sorted"),
        on_group_toggle: Fn("host_group_toggled") }),
        pagination::Pagination(#{ key: "host-pages", label: "Host pages",
            total_items: ctx.get_native_collection("operations_hosts").len,
            current_page: ctx.get_state("host_page"), page_size: ctx.get_state("host_page_size"),
            on_change: Fn("host_page_changed") })
    ]).with_style(style().height(relative(1.0)).gap(theme_spacing("md")))
}

fn configurations(ctx) {
    let selected = ctx.get_state("selected_hosts");
    let left_host = if selected.len > 0 { selected[0] } else { "edge-01" };
    let right_host = if selected.len > 1 { selected[1] } else { "edge-02" };
    let left = `server {\n  host = \"${left_host}\"\n  channel = \"stable\"\n  port = 443\n}\n`;
    let right = `server {\n  host = \"${right_host}\"\n  channel = \"${ctx.get_state("release_channel")}\"\n  port = 8443\n}\n`;
    column([
        page_header("Configurations", "Compare two hosts, then stage a deterministic deployment."),
        input::Input(#{ key: "release-channel", label: "Release channel",
            value: ctx.get_state("release_channel"), on_change: Fn("channel_changed") }),
        row([
            code_viewer::CodeViewer(#{ key: "config-source", label: "Target configuration",
                file_name: "server.rhai", language: "rhai", source: right,
                style: style().height(px(210)) }),
            diff_viewer::DiffViewer(#{ key: "config-diff", mode: "split",
                left: #{ source: left, label: left_host, file_name: "server.rhai", language: "rhai" },
                right: #{ source: right, label: right_host, file_name: "server.rhai", language: "rhai" },
                style: style().height(px(210)) }),
        ]).with_style(style().gap(theme_spacing("sm"))),
        button::Button(#{ text: `Deploy ${ctx.get_state("deploy_target")}`,
            variant: "primary", on_click: Fn("open_confirm") }).test_id("stage-deploy")
    ]).with_style(style().gap(theme_spacing("md")))
}

fn deployments(ctx) {
    let phase = ctx.get_state("deployment_phase");
    let complete = phase == "complete";
    let failed = phase == "failed";
    let running = phase == "running";
    column([
        page_header("Deployments", "The accepted deployment updates the same session model."),
        row([badge::Badge(#{ text: if complete { "Complete" } else if failed { "Failed" }
                    else if running { "Running" } else { "Idle" },
                variant: if complete { "success" } else if failed { "danger" }
                    else if running { "accent" } else { "neutral" }, dot: true }),
            text(`${ctx.get_state("deploy_target")} · ${ctx.get_state("release_channel")}`)])
            .with_style(style().gap(theme_spacing("sm")).items_center()),
        progress::Progress(#{ key: "deployment-progress", label: "Deployment progress",
            value: ctx.get_state("deployment_progress"), max: 100.0 }),
        text(if complete { "Configuration verified and activated." }
            else if failed { "Deployment failed; the previous Host configuration remains active." }
            else if running { "Applying the staged configuration through a bounded Host subscription…" }
            else { "Choose a host and stage a deployment from Configurations." })
    ]).with_style(style().gap(theme_spacing("md")))
}

fn settings(ctx) {
    column([
        page_header("Settings", "Theme changes preserve Workbench business state."),
        row([
            button::Button(#{ text: "Default Dark", variant: "outline",
                on_click: Fn("set_default_dark") }),
            button::Button(#{ text: "Default Light", variant: "outline",
                on_click: Fn("set_default_light") }),
        ]).with_style(style().gap(theme_spacing("sm"))),
        text(`Selected hosts: ${ctx.get_state("selected_hosts").len}`),
        text(`Fixture: ${ctx.get_state("fixture_case")}`)
    ]).with_style(style().gap(theme_spacing("md")))
}
fn set_default_dark(ctx, payload) { ctx.set_theme("Default", "Dark"); }
fn set_default_light(ctx, payload) { ctx.set_theme("Default", "Light"); }

fn page(ctx) {
    let current = ctx.get_state("page");
    if current == "dashboard" { dashboard(ctx) }
    else if current == "hosts" { hosts(ctx) }
    else if current == "configurations" { configurations(ctx) }
    else if current == "deployments" { deployments(ctx) }
    else { settings(ctx) }
}

fn render_OperationsWorkbench(ctx, props) {
    let fixture = ctx.get_state("fixture_case");
    effect("deployment", #{ active: ctx.get_state("deployment_phase") == "running",
        fail: fixture == "failure" }, Fn("start_deployment"), Fn("stop_deployment"));
    effect("fixture_stream", #{ active: fixture == "streaming" },
        Fn("start_fixture_stream"), Fn("stop_fixture_stream"));
    let commands = [
        #{ value: "dashboard", label: "Open Dashboard", group: "Navigate" },
        #{ value: "hosts", label: "Open Hosts", group: "Navigate" },
        #{ value: "configurations", label: "Open Configurations", group: "Navigate" },
        #{ value: "deployments", label: "Open Deployments", group: "Navigate" },
        #{ value: "settings", label: "Open Settings", group: "Navigate" },
    ];
    let failed = ctx.get_state("deployment_phase") == "failed";
    let toast_items = if ctx.get_state("toast_visible") {
        [#{ id: if failed { "deployment-failed" } else { "deployment-complete" },
            title: if failed { "Deployment failed" } else { "Deployment complete" },
            message: if failed { "The previous configuration remains active." }
                else { `${ctx.get_state("deploy_target")} now runs ${ctx.get_state("release_channel")}` },
            variant: if failed { "danger" } else { "success" }, duration_ms: 60000 }]
    } else { [] };
    let children = [
        title_bar::TitleBar(#{ label: "Operations Workbench", title: "OPERATIONS WORKBENCH",
            subtitle: "Deterministic local fixture", inset_start: 70 }),
        row([
            column([
                nav_button(ctx, "dashboard", "Dashboard"), nav_button(ctx, "hosts", "Hosts"),
                nav_button(ctx, "configurations", "Configurations"), nav_button(ctx, "deployments", "Deployments"),
                nav_button(ctx, "settings", "Settings"),
                button::Button(#{ text: "Command palette", variant: "outline",
                    on_click: Fn("open_command") }).test_id("open-command")
            ]).with_style(style().width(px(230)).height(relative(1.0)).padding(theme_spacing("sm"))
                .gap(theme_spacing("xs")).border_end(px(1)).border_color(theme_color("border"))),
            column([page(ctx)]).with_style(style().flex_grow().min_width(px(0)).min_height(px(0))
                .padding(theme_spacing("lg")).overflow_y_scroll())
        ]).with_style(style().flex_grow().min_height(px(0))),
        status_bar::StatusBar(#{ label: "Operations status",
            start: [text(ctx.get_state("page"))], center: [text(ctx.get_state("deployment_phase"))],
            end: [text(ctx.get_state("deploy_target"))] }),
    ];
    if ctx.get_state("confirm_open") {
        children.push(dialog::Dialog(#{ key: "deployment-confirm", open: true,
            title: "Confirm deployment", content: text(`Deploy ${ctx.get_state("release_channel")} to ${ctx.get_state("deploy_target")}?`),
            actions: [button::Button(#{ text: "Cancel", variant: "outline", on_click: Fn("cancel_deployment") })
                    .test_id("cancel-deploy"),
                button::Button(#{ text: "Deploy", variant: "primary", on_click: Fn("confirm_deployment") })
                    .test_id("confirm-deploy")],
            on_open_change: Fn("confirm_open_changed") }));
    }
    if ctx.get_state("command_open") {
        children.push(command_dialog::CommandDialog(#{ key: "operations-command", open: true,
            label: "Navigate", query: ctx.get_state("command_query"),
            active_value: ctx.get_state("command_active"), items: commands, max_visible: 5,
            on_query_change: Fn("command_query_changed"),
            on_active_change: Fn("command_active_changed"), on_action: Fn("command_action"),
            on_open_change: Fn("command_open_changed") }));
    }
    if ctx.get_state("sheet_open") {
        children.push(sheet::Sheet(#{ key: "host-details", open: true, side: "end",
            title: "Host details", content: column([
                text(`Target: ${ctx.get_state("deploy_target")}`),
                text(`Selected hosts: ${ctx.get_state("selected_hosts").len}`),
                badge::Badge(#{ text: "Rust NativeCollection", variant: "accent" })
            ]).with_style(style().gap(theme_spacing("sm"))),
            actions: [button::Button(#{ text: "Compare configuration", variant: "primary",
                on_click: Fn("compare_from_sheet") }).test_id("compare-from-sheet")],
            on_open_change: Fn("sheet_open_changed") }));
    }
    if ctx.get_state("toast_visible") {
        children.push(toast::Toast(#{ key: "operations-toast", items: toast_items,
            on_dismiss: Fn("dismiss_toast") }));
    }
    column(children).with_style(style().width(relative(1.0)).height(relative(1.0))
        .background(theme_color("surface")))
}

fn view(ctx) { OperationsWorkbench(#{ key: "operations-workbench" }) }
