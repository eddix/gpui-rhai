//! Section actions do not squeeze the title (#125): the heading keeps at least a label
//! column beside the actions, and the actions wrap below it when both do not fit. A
//! title without actions is the control.

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

const TITLE: &str = "oec.affiliate.task_engine";

const SCRIPT: &str = r#"
fn noop(ctx, payload) { () }
fn actions() {
    [
        badge::Badge(#{ text: "Running", variant: "success" }),
        tag::Tag(#{ text: "Creator Onboarding" }),
        button::Button(#{ text: "在浏览器打开", on_click: Fn("noop") }),
    ]
}
fn scene(with_actions) {
    let props = #{ title: "oec.affiliate.task_engine", content: text("x") };
    if with_actions { props.actions = actions(); }
    let part = section::Section(props);
    box([part]).with_style(style().width(px(347)).flex_col())
}
"#;

fn mount(cx: &mut TestAppContext, view_body: &str) -> (VisualTestContext, ScriptViewHandle) {
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
import "components/badge" as badge;
import "components/button" as button;
import "components/tag" as tag;
import "patterns/section" as section;
{SCRIPT}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(800)).height(px(400))) }}
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
        let host = ScriptViewHost::new("section", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("section"), host.clone(), window, cx)
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

/// Layout bounds of the title heading and, when present, the Button action.
fn measure(cx: &mut TestAppContext, with_actions: bool) -> (Bounds, Option<Bounds>) {
    cx.update(gpui_rhai::install);
    let (mut visual, view) = mount(cx, &format!("scene({with_actions})"));
    let found = visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        let bounds = |role: &str, name: &str| {
            snapshot
                .find_by_role_and_name(role, name)
                .next()
                .map(|node| {
                    let g = node.geometry.unwrap().layout;
                    (g.x, g.y, g.width, g.height)
                })
        };
        (
            bounds("heading", TITLE).expect("no title heading"),
            bounds("button", "在浏览器打开"),
        )
    });
    eprintln!(
        "actions: {with_actions}: title (x, y, w, h) = {:?}, button = {:?}",
        found.0, found.1
    );
    found
}

/// The title's one-line height, measured without actions.
fn line_height(cx: &mut TestAppContext) -> f64 {
    measure(cx, false).0.3
}

#[gpui::test]
fn title_without_actions_is_one_line(cx: &mut TestAppContext) {
    let (title, _) = measure(cx, false);
    // Subtitle line height is well under 40px; one line means the title is not wrapped.
    assert!(
        title.3 < 40.0 && title.2 < 347.0,
        "control: the title should be a single line: {title:?}"
    );
}

#[gpui::test]
fn title_keeps_room_next_to_actions(cx: &mut TestAppContext) {
    // The one-line height, measured without actions in its own window.
    let line = line_height(cx);
    let (title, button) = measure(cx, true);
    let button = button.expect("no button action");
    let (_, ty, _, th) = title;
    let (_, by, _, _) = button;
    let at_most_two_lines = th <= line * 2.0 + 0.5;
    let actions_below_title = by >= ty + th - 0.5;
    assert!(
        at_most_two_lines || actions_below_title,
        "the title is squeezed beside the actions: {}x{} ({} lines of {line}px), button at y {by}",
        title.2,
        th,
        (th / line).round()
    );
}
