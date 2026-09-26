import "components/button" as button;
import "components/badge" as badge;
import "components/command" as command;
import "components/dialog" as dialog;
import "components/input" as input;
import "components/table" as table;
import "components/code_viewer" as code_viewer;
import "components/diff_viewer" as diff_viewer;
import "components/progress" as progress;
import "components/toast" as toast;
import "components/title_bar" as title_bar;
import "components/status_bar" as status_bar;
import "charts/chart" as chart;

fn state_schema() {
    #{ fields: #{
        page: #{ schema: #{ type: "string", allowed: ["dashboard", "hosts", "configurations", "deployments", "settings"] },
            "default": #{ type: "string", value: __OPERATIONS_PAGE__ } },
        selected_hosts: #{ schema: #{ type: "array", max_items: 2, items: #{ type: "string" } },
            "default": #{ type: "array", value: [#{ type: "string", value: "edge-01" }] } },
        deploy_target: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "edge-01" } },
        release_channel: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "stable" } },
        confirm_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        deployment_phase: #{ schema: #{ type: "string", allowed: ["idle", "complete"] },
            "default": #{ type: "string", value: "idle" } },
        deployment_progress: #{ schema: #{ type: "number", min: 0.0, max: 100.0 },
            "default": #{ type: "float", value: 0.0 } },
        toast_visible: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        command_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        command_active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "dashboard" } },
    } }
}

fn navigate(page, ctx, payload) { ctx.set_state("page", page); }
fn hosts_selected(ctx, values) {
    ctx.set_state("selected_hosts", values);
    if values.len > 0 { ctx.set_state("deploy_target", values[0]); }
}
fn channel_changed(ctx, value) { ctx.set_state("release_channel", value); }
fn open_confirm(ctx, payload) { ctx.set_state("confirm_open", true); }
fn confirm_open_changed(ctx, value) { ctx.set_state("confirm_open", value); }
fn cancel_deployment(ctx, payload) { ctx.set_state("confirm_open", false); }
fn confirm_deployment(ctx, payload) {
    ctx.set_state("confirm_open", false);
    ctx.set_state("deployment_phase", "complete");
    ctx.set_state("deployment_progress", 100.0);
    ctx.set_state("toast_visible", true);
    ctx.set_state("page", "deployments");
}
fn dismiss_toast(ctx, payload) { ctx.set_state("toast_visible", false); }
fn command_query_changed(ctx, value) { ctx.set_state("command_query", value); }
fn command_active_changed(ctx, value) { ctx.set_state("command_active", value); }
fn command_action(ctx, value) { ctx.set_state("page", value); }

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
    let metrics = ctx.get_native_chart_data("operations_metrics");
    column([
        page_header("Dashboard", "Host health and latency from Rust-owned chart data."),
        row([
            column([text("3").with_style(style().typography("display")), text("Hosts")])
                .with_style(style().flex_grow().padding(theme_spacing("md")).border(px(1)).border_color(theme_color("border"))),
            column([text("2").with_style(style().typography("display")), text("Healthy")])
                .with_style(style().flex_grow().padding(theme_spacing("md")).border(px(1)).border_color(theme_color("border"))),
            column([text("1").with_style(style().typography("display")), text("Needs attention")])
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
    table::Table(#{ key: "operations-hosts", label: "Operations hosts", row_key: "id",
        rows: ctx.get_native_collection("operations_hosts"), fill_height: true,
        columns: [
            #{ key: "host", title: "Host", width: #{ kind: "flex", value: 1 }, sortable: true,
                adornments: [#{ text_key: "status", variant_key: "status_variant", dot: true }] },
            #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 140 }, sortable: true },
            #{ key: "version", title: "Version", width: #{ kind: "fixed", value: 100 } },
        ], selection_mode: "multiple", selected_keys: ctx.get_state("selected_hosts"),
        on_selection_change: Fn("hosts_selected") })
}

fn configurations(ctx) {
    let left = "server {\n  host = \"edge-01\"\n  channel = \"stable\"\n  port = 443\n}\n";
    let right = `server {\n  host = \"edge-02\"\n  channel = \"${ctx.get_state("release_channel")}\"\n  port = 8443\n}\n`;
    column([
        page_header("Configurations", "Compare two hosts, then stage a deterministic deployment."),
        input::Input(#{ key: "release-channel", label: "Release channel",
            value: ctx.get_state("release_channel"), on_change: Fn("channel_changed") }),
        row([
            code_viewer::CodeViewer(#{ key: "config-source", label: "Target configuration",
                file_name: "server.rhai", language: "rhai", source: right,
                style: style().height(px(210)) }),
            diff_viewer::DiffViewer(#{ key: "config-diff", mode: "split",
                left: #{ source: left, label: "edge-01", file_name: "server.rhai", language: "rhai" },
                right: #{ source: right, label: "edge-02", file_name: "server.rhai", language: "rhai" },
                style: style().height(px(210)) }),
        ]).with_style(style().gap(theme_spacing("sm"))),
        button::Button(#{ text: `Deploy ${ctx.get_state("deploy_target")}`,
            variant: "primary", on_click: Fn("open_confirm") }).test_id("stage-deploy")
    ]).with_style(style().gap(theme_spacing("md")))
}

fn deployments(ctx) {
    let complete = ctx.get_state("deployment_phase") == "complete";
    column([
        page_header("Deployments", "The accepted deployment updates the same session model."),
        row([badge::Badge(#{ text: if complete { "Complete" } else { "Idle" },
                variant: if complete { "success" } else { "neutral" }, dot: true }),
            text(`${ctx.get_state("deploy_target")} · ${ctx.get_state("release_channel")}`)])
            .with_style(style().gap(theme_spacing("sm")).items_center()),
        progress::Progress(#{ key: "deployment-progress", label: "Deployment progress",
            value: ctx.get_state("deployment_progress"), max: 100.0 }),
        text(if complete { "Configuration verified and activated." }
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
        text(`Selected hosts: ${ctx.get_state("selected_hosts").len}`)
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

fn view(ctx) {
    let commands = [
        #{ value: "dashboard", label: "Open Dashboard", group: "Navigate" },
        #{ value: "hosts", label: "Open Hosts", group: "Navigate" },
        #{ value: "configurations", label: "Open Configurations", group: "Navigate" },
        #{ value: "deployments", label: "Open Deployments", group: "Navigate" },
        #{ value: "settings", label: "Open Settings", group: "Navigate" },
    ];
    let toast_items = if ctx.get_state("toast_visible") {
        [#{ id: "deployment-complete", title: "Deployment complete",
            message: `${ctx.get_state("deploy_target")} now runs ${ctx.get_state("release_channel")}`,
            variant: "success", duration_ms: 60000 }]
    } else { [] };
    column([
        title_bar::TitleBar(#{ label: "Operations Workbench", title: "OPERATIONS WORKBENCH",
            subtitle: "Deterministic local fixture", inset_start: 70 }),
        row([
            column([
                nav_button(ctx, "dashboard", "Dashboard"), nav_button(ctx, "hosts", "Hosts"),
                nav_button(ctx, "configurations", "Configurations"), nav_button(ctx, "deployments", "Deployments"),
                nav_button(ctx, "settings", "Settings"),
                command::Command(#{ key: "operations-command", label: "Navigate",
                    query: ctx.get_state("command_query"), active_value: ctx.get_state("command_active"),
                    items: commands, max_visible: 5, on_query_change: Fn("command_query_changed"),
                    on_active_change: Fn("command_active_changed"), on_action: Fn("command_action") })
            ]).with_style(style().width(px(230)).height(relative(1.0)).padding(theme_spacing("sm"))
                .gap(theme_spacing("xs")).border_end(px(1)).border_color(theme_color("border"))),
            column([page(ctx)]).with_style(style().flex_grow().min_width(px(0)).min_height(px(0))
                .padding(theme_spacing("lg")).overflow_y_scroll())
        ]).with_style(style().flex_grow().min_height(px(0))),
        status_bar::StatusBar(#{ label: "Operations status",
            start: [text(ctx.get_state("page"))], center: [text(ctx.get_state("deployment_phase"))],
            end: [text(ctx.get_state("deploy_target"))] }),
        dialog::Dialog(#{ key: "deployment-confirm", open: ctx.get_state("confirm_open"),
            title: "Confirm deployment", content: text(`Deploy ${ctx.get_state("release_channel")} to ${ctx.get_state("deploy_target")}?`),
            actions: [button::Button(#{ text: "Cancel", variant: "outline", on_click: Fn("cancel_deployment") })
                    .test_id("cancel-deploy"),
                button::Button(#{ text: "Deploy", variant: "primary", on_click: Fn("confirm_deployment") })
                    .test_id("confirm-deploy")],
            on_open_change: Fn("confirm_open_changed") }),
        toast::Toast(#{ key: "operations-toast", items: toast_items, on_dismiss: Fn("dismiss_toast") })
    ]).with_style(style().width(relative(1.0)).height(relative(1.0)).background(theme_color("surface")))
}
