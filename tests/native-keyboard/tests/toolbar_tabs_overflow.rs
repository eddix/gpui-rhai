//! A view switcher in a Toolbar's `filters` shrinks with the bar and scrolls inside it
//! (#124): the Toolbar's groups may shrink below their content, and an empty end group
//! is left out, so it cannot wrap onto a second line. The `part_styles` cases are the
//! controls the reporter used before the fix.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_LAYOUT_SOURCES_BY_ID,
    BUNDLED_PATTERN_SOURCES_BY_ID, DEFAULT_THEME,
};

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

const SCRIPT: &str = r#"
fn noop(ctx, payload) { () }
fn tab(value, label) { #{ value: value, label: label, content: box([]) } }
fn switcher() {
    tabs::Tabs(#{ value: "overview", label: "View", panel: false, on_change: Fn("noop"), tabs: [
        tab("overview", "Overview"), tab("deployments", "Deployments"),
        tab("configuration", "Configuration"), tab("observability", "Observability"),
        tab("permissions", "Permissions"),
    ] })
}
"#;

fn mount(
    cx: &mut TestAppContext,
    extra: &str,
    view_body: &str,
) -> (VisualTestContext, ScriptViewHandle) {
    let mut modules = BTreeMap::new();
    let tables = [
        BUNDLED_COMPONENT_SOURCES_BY_ID,
        BUNDLED_LAYOUT_SOURCES_BY_ID,
        BUNDLED_PATTERN_SOURCES_BY_ID,
    ];
    for (id, source) in tables.iter().flat_map(|t| t.iter()) {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let main = format!(
        r#"
import "components/tabs" as tabs;
import "layouts/region" as region;
import "layouts/toolbar" as toolbar;
{SCRIPT}
{extra}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(900)).height(px(400))) }}
"#
    );
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), main);
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .motion_preference(MotionPreference::None)
            .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window: WindowHandle<Host> = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("toolbar", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("toolbar"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    (visual, view)
}

type Bounds = (f64, f64, f64, f64);

/// Layout bounds of the region, the toolbar and the tab list in a 450px box.
fn measure(
    cx: &mut TestAppContext,
    label: &str,
    toolbar_props: &str,
    tabs: &str,
) -> (Bounds, Bounds, Bounds) {
    cx.update(gpui_rhai::install);
    // Built in steps: one nested expression exceeds Rhai's default expression depth.
    let scene = format!(
        r#"fn scene() {{
    let list = {tabs};
    let bar = toolbar::Toolbar(#{{ label: "Filters", filters: [list] {toolbar_props} }});
    let area = region::Region(#{{ label: "Services", body: text("x"), toolbar: bar }});
    box([area]).with_style(style().width(px(450)).height(px(200)).flex_col())
}}"#
    );
    let (mut visual, view) = mount(cx, &scene, "scene()");
    let found = visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        let bounds = |role: &str, name: &str| {
            let g = snapshot
                .find_by_role_and_name(role, name)
                .next()
                .unwrap_or_else(|| panic!("no {role} {name:?}"))
                .geometry
                .unwrap()
                .layout;
            (g.x, g.y, g.width, g.height)
        };
        (
            bounds("region", "Services"),
            bounds("toolbar", "Filters"),
            bounds("tablist", "View"),
        )
    });
    eprintln!(
        "{label}: region {:?}, toolbar {:?}, tablist {:?}",
        found.0, found.1, found.2
    );
    found
}

fn natural_tablist_width(cx: &mut TestAppContext) -> f64 {
    cx.update(gpui_rhai::install);
    let (mut visual, view) = mount(cx, "", "switcher()");
    let width = visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        snapshot
            .find_by_role_and_name("tablist", "View")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .layout
            .width
    });
    eprintln!("tab list alone: {width}px wide");
    width
}

#[gpui::test]
fn tab_list_is_wider_than_the_region(cx: &mut TestAppContext) {
    let width = natural_tablist_width(cx);
    assert!(
        width > 450.0,
        "precondition: the tab list's natural width ({width}px) exceeds 450px"
    );
}

#[gpui::test]
fn tabs_in_a_narrow_toolbar_stay_inside_it(cx: &mut TestAppContext) {
    let (_, bar, list) = measure(cx, "default", "", "switcher()");
    let (bx, _, bw, _) = bar;
    let (lx, _, lw, _) = list;
    assert!(
        lx + lw <= bx + bw + 0.5,
        "the tab list ({lw}px wide) ends at {}, past the toolbar's right edge at {} ({bw}px wide)",
        lx + lw,
        bx + bw
    );
    // With no actions there is no end group to wrap onto a second line.
    let (_, _, _, bh) = bar;
    let (_, _, _, lh) = list;
    assert!(
        (bh - lh).abs() < 0.5,
        "the toolbar is one line high: toolbar {bh}px, tab list {lh}px"
    );
}

#[gpui::test]
fn tabs_stay_inside_with_min_width_zero(cx: &mut TestAppContext) {
    let (_, bar, list) = measure(
        cx,
        "start/tabs min_width(0)",
        r#", part_styles: #{ start: style().min_width(px(0)) }"#,
        "switcher().with_style(style().min_width(px(0)))",
    );
    let (bx, _, bw, _) = bar;
    let (lx, _, lw, _) = list;
    assert!(
        lx + lw <= bx + bw + 0.5,
        "control: the tab list ({lw}px wide) ends at {}, the toolbar at {}",
        lx + lw,
        bx + bw
    );
}

#[gpui::test]
fn tabs_stay_inside_with_start_min_width_zero_only(cx: &mut TestAppContext) {
    // Narrows the cause: the tab list already has min_width(px(0)) in its own style; the
    // start group is the item that refuses to shrink.
    let (_, bar, list) = measure(
        cx,
        "start min_width(0) only",
        r#", part_styles: #{ start: style().min_width(px(0)) }"#,
        "switcher()",
    );
    let (bx, _, bw, _) = bar;
    let (lx, _, lw, _) = list;
    assert!(
        lx + lw <= bx + bw + 0.5,
        "control: the tab list ({lw}px wide) ends at {}, the toolbar at {}",
        lx + lw,
        bx + bw
    );
}
