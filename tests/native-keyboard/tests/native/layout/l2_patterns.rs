//! Acceptance tests for the L2 layouts and patterns: a full AppShell composition
//! passes the productivity audit and its regions take keyboard focus with F6.

use std::collections::BTreeMap;
use std::{cell::RefCell, rc::Rc};

use gpui::{TestAppContext, VisualTestContext, WindowHandle, rgba};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID, BUNDLED_PROFILES,
};

use crate::support::{Host, Mount, bounds, last_error, painted_quads};

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

#[gpui::test]
fn navigation_back_uses_the_application_icon_namespace_in_both_directions(cx: &mut TestAppContext) {
    use crate::support::{Step, settle};
    fn image_assets(node: &UiNode, ids: &mut Vec<String>) {
        match node.kind() {
            UiNodeKind::Image {
                source: ImageSourceSpec::Asset(id),
            } => ids.push(id.as_str().to_owned()),
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                for child in children {
                    image_assets(child, ids);
                }
            }
            _ => {}
        }
    }
    for (locale, icon) in [("en", "chevron_left"), ("ar", "chevron_right")] {
        let source = format!(
            r#"
            import "patterns/navigation_stack" as nav;
            fn init(ctx){{ctx.set_locale("{locale}");}}
            fn popped(ctx, proposal){{}}
            fn view(ctx){{nav::NavigationStack(#{{key:"visits",label:"Visits",header:"back",
                stack:[#{{key:"root",title:"Library",content:text("Root")}},#{{key:"item",title:"Item",content:text("Item")}}],on_pop:Fn("popped")}})}}
        "#
        );
        let reports = Rc::new(RefCell::new(Vec::new()));
        let sink = reports.clone();
        let (mut visual, view) = Mount::components(source)
            .registry("patterns/navigation_stack")
            .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
            .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
            .config(move |config| {
                config.on_error(move |report, _| sink.borrow_mut().push(report.message.clone()))
            })
            .motion(MotionPreference::None)
            .mount(cx);
        settle(&mut visual, Step::Frame, 4);
        let ids = visual.update(|_, cx| {
            let mut ids = Vec::new();
            image_assets(&view.root(cx).unwrap().unwrap(), &mut ids);
            ids
        });
        assert_eq!(ids, vec![format!("app/icons/{icon}")]);
        assert_eq!(last_error(&mut visual, &view), None);
        assert!(
            reports.borrow().is_empty(),
            "{locale}: {:?}",
            reports.borrow()
        );
    }
}

fn mount(cx: &mut TestAppContext, script: &str) -> (WindowHandle<Host>, ScriptViewHandle) {
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([("focus_ring".to_owned(), Rgba8::from_rgba_hex(0x00ff00ff))]),
        ..Default::default()
    };
    let productivity = BUNDLED_PROFILES
        .iter()
        .find(|(id, _)| *id == "productivity")
        .unwrap()
        .1;
    Mount::components(script)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .with(move |view| {
            view.profile_source(productivity)
                .theme_token_overrides(overrides)
        })
        .id("l2")
        .mount_window(cx)
}

#[gpui::test]
fn app_shell_composition_passes_the_productivity_audit(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx, SHELL);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    assert_eq!(last_error(&mut visual, &view), None);
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
    let mut region_width = |name: &str| bounds(&mut visual, &view, "region", name).width;
    let (navigation, hosts, inspector) = (
        region_width("Navigation"),
        region_width("Hosts"),
        region_width("Inspector"),
    );
    let framed_width = |visual: &mut VisualTestContext| {
        let color: gpui::Hsla = rgba(0x00ff00ff).into();
        painted_quads(visual)
            .iter()
            .filter(|quad| quad.border_widths[3] > 0.0 && quad.border_color == color)
            .map(|quad| f64::from(quad.bounds.width))
            .fold(0.0_f64, f64::max)
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
