//! Acceptance tests for the L2 layouts and patterns: a full AppShell composition
//! passes the productivity audit and its regions take keyboard focus with F6.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle, rgba,
};
use gpui_rhai::*;

const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");
const PRODUCTIVITY: &str = include_str!("../../../registry/profiles/productivity.rhai");

const SHELL: &str = r#"
import "patterns/app_shell" as app_shell;
import "patterns/data_view" as data_view;
import "patterns/section" as section;
import "patterns/description_list" as description_list;
import "patterns/stat" as stat;
import "patterns/form_layout" as form_layout;
import "layouts/stack" as stack;
import "layouts/region" as region;
import "components/tree" as tree;
import "components/table" as table;
import "components/button" as button;
import "components/input" as input;
import "components/badge" as badge;
import "components/tag" as tag;

fn noop(ctx, payload) {}

fn nav() {
    let items = [
        #{ key: "fleet", label: "Fleet" }, #{ key: "hosts", parent: "fleet", label: "Hosts" },
        #{ key: "deploys", parent: "fleet", label: "Deployments" },
    ];
    let list = tree::Tree(#{ key: "nav", label: "Navigation", fill_height: true, active_key: "hosts",
        items: items, expanded: ["fleet"], selected_keys: ["hosts"], on_selection_change: Fn("noop"),
        on_expanded_change: Fn("noop"), on_active_change: Fn("noop") });
    region::Region(#{ label: "Navigation", body: list, inset: false })
}

fn hosts_table() {
    let columns = [
        #{ key: "host", title: "Host", width: #{ kind: "flex", value: 2.0 } },
        #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 120.0 } },
        #{ key: "cpu", title: "CPU %", width: #{ kind: "fixed", value: 90.0 }, numeric: true },
    ];
    let rows = [
        #{ id: "a", host: "api-01", region: "us-east", cpu: "42.1" },
        #{ id: "b", host: "worker-07", region: "eu-west", cpu: "98.6" },
    ];
    table::Table(#{ key: "hosts-table", label: "Hosts", row_key: "id", fill_height: true,
        columns: columns, rows: rows, selected_keys: ["b"], selection_mode: "single",
        on_selection_change: Fn("noop") })
}

fn main_view() {
    let filter = input::Input(#{ key: "filter", label: "Filter hosts", value: "", placeholder: "Filter",
        on_change: Fn("noop") }).with_style(style().width(px(200)));
    let toolbar = #{
        context: [tag::Tag(#{ facet: "site", text: "i18n" })],
        filters: [filter],
        actions: [button::Button(#{ key: "export", text: "Export", variant: "ghost", on_click: Fn("noop") })],
        primary: button::Button(#{ key: "add", text: "Add host", variant: "primary", on_click: Fn("noop") }),
    };
    let footer = #{ status: badge::Badge(#{ text: "1 degraded", variant: "warning" }),
        count: "2 hosts", updated: "Updated 12:04" };
    data_view::DataView(#{ key: "hosts", label: "Hosts", title: "Hosts", body: hosts_table(),
        toolbar: toolbar, footer: footer })
}

fn scale_form() {
    let replicas = input::Input(#{ key: "replicas", label: "Replicas", value: "3", on_change: Fn("noop") });
    form_layout::FormLayout(#{ key: "scale", label: "Scale", label_width: px(96),
        fields: [#{ label: "Replicas", control: replicas, required: true,
            description: "Running copies of the service." }],
        submit: [button::Button(#{ key: "apply", text: "Apply", on_click: Fn("noop") })] })
}

fn inspector() {
    let details = description_list::DescriptionList(#{ label: "Details", items: [
        #{ label: "Region", value: "eu-west" }, #{ label: "Cores", value: "32", numeric: true },
    ] });
    let body = stack::Stack(#{ gap: "section", children: [
        stat::stats([#{ label: "CPU", value: "98.6", unit: "%", delta: "+31 in 1 h", tone: "warning" }]),
        section::Section(#{ title: "Details", content: details }),
        section::Section(#{ title: "Scale", content: scale_form() }),
    ] });
    region::Region(#{ label: "Inspector", title: "worker-07", body: body })
}

fn view(ctx) {
    app_shell::AppShell(#{ key: "shell", label: "Fleet console",
        title_bar: #{ title: "Fleet console" },
        sidebar: nav(), main: main_view(), inspector: inspector(),
        status: #{ start: [text("Connected")], end: [text("v0.2.0")] } })
}
"#;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn bundled_sources(entry: &ModuleId, script: &str) -> EmbeddedScriptSource {
    let mut sources = BTreeMap::from([(entry.clone(), script.to_owned())]);
    for (id, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(gpui_rhai_registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
        .chain(gpui_rhai_registry::BUNDLED_PATTERN_SOURCES_BY_ID)
    {
        sources.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    EmbeddedScriptSource::new(sources)
}

fn bundled_assets() -> Vec<(String, AssetData)> {
    gpui_rhai_registry::BUNDLED_ASSET_SOURCES
        .iter()
        .map(|(id, svg)| {
            (
                id.trim_end_matches(".svg").to_owned(),
                AssetData {
                    mime_type: "image/svg+xml".to_owned(),
                    bytes: svg.as_bytes().to_vec(),
                },
            )
        })
        .collect()
}

fn mount(cx: &mut TestAppContext, script: &str) -> (WindowHandle<Host>, ScriptViewHandle) {
    let entry = ModuleId::parse("main").unwrap();
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([("focus_ring".to_owned(), Rgba8::from_rgba_hex(0x00ff00ff))]),
        ..Default::default()
    };
    let prepared =
        EmbeddedScriptView::new(entry.clone(), bundled_sources(&entry, script), DEFAULT_DARK)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .profile_source(PRODUCTIVITY)
            .asset_sources(bundled_assets())
            .theme_token_overrides(overrides)
            .motion_preference(MotionPreference::None)
            .prepare()
            .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("l2", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("l2"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    (window, view)
}

#[gpui::test]
fn app_shell_composition_passes_the_productivity_audit(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx, SHELL);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    // The test platform ships no system fonts, so font resolution is checked on
    // real platforms by the Gallery gate instead.
    let rules = AuditRules::from_ids(
        AuditRule::ALL
            .into_iter()
            .filter(|rule| *rule != AuditRule::UnresolvedFont)
            .map(AuditRule::id),
    )
    .unwrap();
    let findings = visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
    assert!(
        findings.is_empty(),
        "{}",
        findings
            .iter()
            .map(|finding| format!(
                "{}: {} at {}",
                finding.rule.id(),
                finding.message,
                finding.path
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[gpui::test]
fn f6_moves_keyboard_focus_between_shell_regions(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx, SHELL);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let region_width = |name: &str| {
        tree.find_by_role_and_name("region", name)
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
            .width
    };
    let (navigation, hosts, inspector) = (
        region_width("Navigation"),
        region_width("Hosts"),
        region_width("Inspector"),
    );
    let framed_width = |visual: &mut VisualTestContext| {
        let color: gpui::Hsla = rgba(0x00ff00ff).into();
        visual.update(|window, _| {
            let scale = window.scale_factor();
            window
                .painted_quads()
                .iter()
                .filter(|quad| quad.border_widths.left.0 > 0.0 && quad.border_color == color)
                .map(|quad| f64::from(quad.bounds.size.width.0 / scale))
                .fold(0.0_f64, f64::max)
        })
    };
    // Enter the shell from the first control, then cycle regions.
    visual.update(|window, cx| view.focus(window, cx)).unwrap();
    visual.simulate_keystrokes("tab");
    let mut seen = Vec::new();
    for _ in 0..3 {
        visual.simulate_keystrokes("f6");
        visual.update(|window, _| window.refresh());
        visual.run_until_parked();
        seen.push(framed_width(&mut visual));
    }
    let matches = |width: f64, region: f64| (width - region).abs() < 1.5;
    assert!(
        seen.iter().any(|width| matches(*width, hosts))
            && seen.iter().any(|width| matches(*width, inspector))
            && seen.iter().any(|width| matches(*width, navigation)),
        "F6 must frame every region in turn: seen={seen:?}, regions={navigation}/{hosts}/{inspector}"
    );
    visual.simulate_keystrokes("shift-f6");
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    let back = framed_width(&mut visual);
    assert!(
        matches(back, seen[1]),
        "Shift+F6 must return to the previous region: back={back}, seen={seen:?}"
    );
}
