//! Real GPUI layout/editor/focus evidence for the shared retained-page policy.
use gpui::{IntoElement, ParentElement, Render, Styled};
use gpui::{ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{MotionPreference, NavigateBack, UiValue};
use gpui_rhai_registry::{BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID};

use crate::support::{
    Mount, ReplaceOnce, Step, bounds, by_test_id, dispatch, last_error, node_by_test_id, settle,
    snapshot, text_of,
};

const RETAINED: &str = r#"
import "components/textarea" as textarea;
import "components/tabs" as tabs;
fn state_schema() { #{ fields: #{
    show: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
    draft: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } }
} } }
fn toggle(ctx, event) { ctx.set_state("show", !ctx.get_state("show")); }
fn changed(ctx, value) { ctx.set_state("draft", value); }
fn noop(ctx, event) {}
fn page_content(ctx) {
    let lines = [];
    for i in 0..30 { lines.push(text(`Line ${i}`).with_key(`line:${i}`).test_id(`line:${i}`).with_style(style().height(px(24)))); }
    column([
            textarea::Textarea(#{ key: "draft", label: "Draft", value: ctx.get_state("draft"), rows: 3, on_change: Fn("changed") })
                .with_style(style().width(px(300))),
            column(lines).with_key("scroll").accessibility_role("group").accessibility_label("Scroll")
                .with_style(style().height(px(120)).width(px(300)).overflow_y_scroll()),
            text("Hidden action").with_key("inside").test_id("inside").on_click(Fn("noop")),
    ])
}
fn view(ctx) {
    let use_tabs = false;
    let content = if use_tabs {
        tabs::Tabs(#{ key: "tabs", label: "Workspace", value: if ctx.get_state("show") { "draft" } else { "other" }, keep_mounted: true,
            tabs: [#{ value: "draft", label: "Draft", content: page_content(ctx) }, #{ value: "other", label: "Other", content: text("Other page") }], on_change: Fn("toggle") })
    } else {
        column([page_content(ctx).with_key("page").retained_page(ctx.get_state("show")), text("Other page").with_key("other").retained_page(!ctx.get_state("show"))])
    };
    column([
        text("Toggle").with_key("toggle").test_id("toggle").on_click(Fn("toggle")),
        text(ctx.get_state("draft")).test_id("value"),
        content,
    ]).with_style(style().width(px(400)).height(px(500)))
}
"#;

fn action(visual: &mut VisualTestContext, view: &gpui_rhai::ScriptViewHandle, id: &str) {
    dispatch(visual, view, by_test_id(id), "click").unwrap();
    settle(visual, Step::Frame, 3);
}

fn source_file(path: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../registry")
            .join(path),
    )
    .unwrap()
}

#[gpui::test]
fn retained_page_preserves_native_scroll_selection_undo_and_identity(cx: &mut TestAppContext) {
    native_continuity(cx, RETAINED);
}

#[gpui::test]
fn tabs_keep_mounted_uses_the_same_native_continuity_policy(cx: &mut TestAppContext) {
    native_continuity(
        cx,
        &RETAINED.replace_once("let use_tabs = false;", "let use_tabs = true;"),
    );
}

fn native_continuity(cx: &mut TestAppContext, source: &str) {
    let (mut visual, view) = Mount::components(source)
        .motion(MotionPreference::None)
        .mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let editor = bounds(&mut visual, &view, "text_field", "Draft");
    crate::support::click_center(&mut visual, editor);
    visual.simulate_input("abcd");
    visual.simulate_keystrokes("shift-left shift-left");
    let focus = visual.update(|window, cx| window.focused(cx)).unwrap();
    let viewport = bounds(&mut visual, &view, "group", "Scroll");
    let unscrolled = node_by_test_id(&mut visual, &view, "line:10")
        .geometry
        .unwrap()
        .visual
        .y;
    visual.simulate_event(ScrollWheelEvent {
        position: crate::support::center(viewport),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-180.0))),
        ..ScrollWheelEvent::default()
    });
    settle(&mut visual, Step::Frame, 2);
    let before = node_by_test_id(&mut visual, &view, "line:10");
    let y = before.geometry.unwrap().visual.y;
    assert!(
        y < unscrolled - 100.0,
        "precondition: wheel moved the native handle"
    );
    action(&mut visual, &view, "toggle");
    assert!(
        snapshot(&mut visual, &view)
            .find_by_test_id("line:10")
            .next()
            .is_none()
    );
    assert!(
        snapshot(&mut visual, &view)
            .find_by_test_id("inside")
            .next()
            .is_none()
    );
    assert_ne!(
        visual.update(|window, cx| window.focused(cx)),
        Some(focus.clone()),
        "hidden editor must lose focus"
    );
    action(&mut visual, &view, "toggle");
    let after = node_by_test_id(&mut visual, &view, "line:10");
    assert_eq!(after.id, before.id);
    assert_eq!(
        after.geometry.unwrap().visual.y,
        y,
        "zero-size layout must not clamp scrolling"
    );
    visual.update(|window, cx| focus.focus(window, cx));
    visual.simulate_input("Q");
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(
        text_of(&mut visual, &view, "value"),
        "abQ",
        "native caret/selection survived"
    );
    visual.simulate_keystrokes("cmd-z");
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(
        text_of(&mut visual, &view, "value"),
        "abcd",
        "native undo history survived"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

const NAVIGATION: &str = r#"
import "patterns/navigation_stack" as nav;
fn state_schema() { #{ fields: #{
    depth: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 1 } },
    source: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } }
} } }
fn push(ctx, event) { ctx.set_state("depth", ctx.get_state("depth") + 1); }
fn pop(ctx, p) { ctx.set_state("depth", p.depth); ctx.set_state("source", p.source); }
fn view(ctx) {
    let pages = [];
    for i in 0..ctx.get_state("depth") {
        pages.push(#{ key: `visit:${i}`, title: `Visit ${i}`, content: text("Open record 42 again").with_key("open").test_id(`open:${i}`).on_click(Fn("push")) });
    }
    column([
        nav::NavigationStack(#{ key: "visits", label: "Records", stack: pages, on_pop: Fn("pop"), header: "breadcrumb" }),
        text(ctx.get_state("source")).test_id("source"),
    ]).with_style(style().width(px(480)).height(px(400)))
}
"#;

#[gpui::test]
fn navigation_action_proposes_pop_and_restores_actual_opener(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(NAVIGATION)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
        .module(
            "patterns/navigation_stack",
            source_file("patterns/navigation_stack.rhai"),
        )
        .motion(MotionPreference::None)
        .mount(cx);
    settle(&mut visual, Step::Frame, 4);
    let opener = node_by_test_id(&mut visual, &view, "open:0")
        .geometry
        .unwrap()
        .visual;
    crate::support::click_center(&mut visual, opener);
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_test_id("open:0")
            .next()
            .is_none()
    );
    assert!(
        snapshot(&mut visual, &view)
            .find_by_test_id("open:1")
            .next()
            .is_some()
    );
    visual.update(|window, cx| window.dispatch_action(Box::new(NavigateBack), cx));
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(text_of(&mut visual, &view, "source"), "keyboard");
    assert!(
        snapshot(&mut visual, &view)
            .find_by_test_id("open:0")
            .next()
            .is_some()
    );
    // Enter on the restored opener pushes, instead of activating the heading.
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Frame, 4);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_test_id("open:1")
            .next()
            .is_some()
    );
    let value = node_by_test_id(&mut visual, &view, "source");
    assert!(value.value.is_none() || value.value == Some(UiValue::String("keyboard".to_owned())));
}

#[gpui::test]
fn navigation_gallery_pages_mount_real_stateful_components(cx: &mut TestAppContext) {
    for page in ["retained_tabs", "navigation_stack"] {
        let main = format!(
            r#"import "gallery/pages/navigation" as page;
fn noop(ctx, event) {{}}
fn init(ctx) {{ ctx.register_action("gallery.toggle_source", Fn("noop")); }}
fn view(ctx) {{ page::view(ctx, #{{}}, #{{}}, "{page}") }}"#
        );
        let (mut visual, view) = Mount::components(main)
            .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
            .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
            .module(
                "patterns/navigation_stack",
                source_file("patterns/navigation_stack.rhai"),
            )
            .module(
                "gallery/pages/navigation",
                source_file("gallery/pages/navigation.rhai"),
            )
            .module("gallery/kit", source_file("gallery/kit.rhai"))
            .module(
                "gallery/retained_page_panel",
                source_file("gallery/retained_page_panel.rhai"),
            )
            .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
            .motion(MotionPreference::None)
            .mount(cx);
        settle(&mut visual, Step::Tick, 5);
        assert_eq!(last_error(&mut visual, &view), None, "gallery {page}");
        assert!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("text_field", "Checkout service draft")
                .next()
                .is_some()
                || snapshot(&mut visual, &view)
                    .find_by_role_and_name("text_field", "checkout-api draft")
                    .next()
                    .is_some()
        );
        let draft_label = if page == "retained_tabs" {
            "Checkout service draft"
        } else {
            "checkout-api draft"
        };
        let draft = bounds(&mut visual, &view, "text_field", draft_label);
        crate::support::click_center(&mut visual, draft);
        visual.simulate_input("Keep this local draft");
        settle(&mut visual, Step::Frame, 3);
        if page == "retained_tabs" {
            let tickets = bounds(&mut visual, &view, "tab", "Tickets");
            crate::support::click_center(&mut visual, tickets);
            settle(&mut visual, Step::Frame, 3);
            assert!(
                snapshot(&mut visual, &view)
                    .find_by_role_and_name("text_field", "Ticket 4821 draft")
                    .next()
                    .is_some()
            );
            let overview = bounds(&mut visual, &view, "tab", "Overview");
            crate::support::click_center(&mut visual, overview);
        } else {
            let open = bounds(&mut visual, &view, "button", "Open Ticket 4821");
            crate::support::click_center(&mut visual, open);
            settle(&mut visual, Step::Frame, 3);
            assert!(
                snapshot(&mut visual, &view)
                    .find_by_role_and_name("text_field", "Ticket 4821 draft")
                    .next()
                    .is_some()
            );
            visual.update(|window, cx| window.dispatch_action(Box::new(NavigateBack), cx));
        }
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("text_field", draft_label)
                .next()
                .unwrap()
                .value,
            Some(UiValue::String("Keep this local draft".to_owned()))
        );
        assert_eq!(
            last_error(&mut visual, &view),
            None,
            "interactive gallery {page}"
        );
    }
}

#[gpui::test]
fn navigation_breadcrumb_overflow_and_pointer_back_emit_controlled_depths(cx: &mut TestAppContext) {
    let source = NAVIGATION.replace_once("value: 1 }", "value: 6 }");
    let (mut visual, view) = Mount::components(source)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
        .module(
            "patterns/navigation_stack",
            source_file("patterns/navigation_stack.rhai"),
        )
        .motion(MotionPreference::None)
        .mount(cx);
    settle(&mut visual, Step::Frame, 4);
    let overflow = bounds(&mut visual, &view, "button", "…");
    crate::support::click_center(&mut visual, overflow);
    settle(&mut visual, Step::Frame, 4);
    let earlier = bounds(&mut visual, &view, "menuitem", "Visit 1");
    crate::support::click_center(&mut visual, earlier);
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(text_of(&mut visual, &view, "source"), "breadcrumb");
    let current = node_by_test_id(&mut visual, &view, "open:1")
        .geometry
        .unwrap()
        .visual;
    let at = crate::support::center(current);
    visual.simulate_mouse_down(
        at,
        gpui::MouseButton::Navigate(gpui::NavigationDirection::Back),
        gpui::Modifiers::none(),
    );
    visual.simulate_mouse_up(
        at,
        gpui::MouseButton::Navigate(gpui::NavigationDirection::Back),
        gpui::Modifiers::none(),
    );
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(text_of(&mut visual, &view, "source"), "pointer");
    assert!(
        snapshot(&mut visual, &view)
            .find_by_test_id("open:0")
            .next()
            .is_some()
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn revealing_an_unchanged_inner_stack_keeps_keyboard_focus_on_the_tab_list(
    cx: &mut TestAppContext,
) {
    let main = r#"import "components/tabs" as tabs;
import "patterns/navigation_stack" as nav;
fn state_schema() { #{ fields: #{ tab: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "one" } } } } }
fn changed(ctx, value) { ctx.set_state("tab", value); }
fn view(ctx) {
    let entries = [];
    for key in ["one", "two"] {
        let content = nav::NavigationStack(#{ key: key, label: key, header: "none", stack: [#{ key: "root", title: key, content: text(key).test_id(`body:${key}`) }] });
        entries.push(#{ value: key, label: key, content: content });
    }
    tabs::Tabs(#{ key: "tabs", label: "Nested stacks", value: ctx.get_state("tab"), keep_mounted: true, tabs: entries, on_change: Fn("changed") })
        .with_style(style().width(px(480)).height(px(300)))
}"#;
    let (mut visual, view) = Mount::components(main)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
        .module(
            "patterns/navigation_stack",
            source_file("patterns/navigation_stack.rhai"),
        )
        .motion(MotionPreference::None)
        .mount(cx);
    settle(&mut visual, Step::Frame, 4);
    visual.simulate_keystrokes("shift-tab");
    settle(&mut visual, Step::Frame, 2);
    let list_focus = visual.update(|window, cx| window.focused(cx));
    visual.simulate_keystrokes("right");
    settle(&mut visual, Step::Frame, 4);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_test_id("body:two")
            .next()
            .is_some()
    );
    assert!(
        snapshot(&mut visual, &view)
            .find_by_test_id("body:one")
            .next()
            .is_none()
    );
    assert_eq!(
        visual.update(|window, cx| window.focused(cx)),
        list_focus,
        "tab arrows must remain usable when each tab owns a stack"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

const SCOPES: &str = r#"
fn state_schema() { #{ fields: #{
    inner: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 2 } },
    outer: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 2 } }
} } }
fn inner_pop(ctx, proposal) { ctx.set_state("inner", proposal.depth); }
fn outer_pop(ctx, proposal) { ctx.set_state("outer", proposal.depth); }
fn noop(ctx, event) {}
fn view(ctx) {
    let inner = column([text("Focus this stack").with_key("focus").test_id("focus").on_click(Fn("noop"))])
        .with_key("inner").navigation_scope(ctx.get_state("inner")).accessibility_label("Inner");
    if ctx.get_state("inner") > 1 { inner = inner.on("navigation_back", Fn("inner_pop")); }
    let root = column([inner, text(`inner:${ctx.get_state("inner")} outer:${ctx.get_state("outer")}`).test_id("depth")])
        .with_key("outer").navigation_scope(ctx.get_state("outer")).accessibility_label("Outer")
        .with_style(style().width(px(240)).height(px(200)));
    if ctx.get_state("outer") > 1 { root.on("navigation_back", Fn("outer_pop")) } else { root }
}
"#;

struct Pair {
    host: gpui_rhai::ScriptViewHost,
    views: Vec<gpui_rhai::ScriptViewHandle>,
}
impl Render for Pair {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        self.host.container(
            gpui::div()
                .flex()
                .children(self.views.iter().map(|view| view.element().unwrap())),
        )
    }
}

#[gpui::test]
fn host_back_prefers_inner_scope_and_never_another_views_equal_local_key(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let prepared = [
        Mount::new(SCOPES).prepare().unwrap(),
        Mount::new(SCOPES).prepare().unwrap(),
    ];
    let saved = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = gpui_rhai::ScriptViewHost::new("navigation-pair", cx).unwrap();
        let views = prepared
            .into_iter()
            .zip(["one", "two"])
            .map(|(prepared, id)| {
                prepared
                    .mount(
                        gpui_rhai::ScriptViewConfig::new(id),
                        host.clone(),
                        window,
                        cx,
                    )
                    .unwrap()
            })
            .collect::<Vec<_>>();
        *capture.borrow_mut() = Some((host.clone(), views.clone()));
        Pair { host, views }
    });
    let (host, views) = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 3);
    let target = node_by_test_id(&mut visual, &views[0], "focus")
        .geometry
        .unwrap()
        .visual;
    crate::support::click_center(&mut visual, target);
    let scopes = visual.update(|window, cx| host.navigation_stacks(window, cx));
    assert_eq!(
        scopes
            .iter()
            .filter(|scope| scope.focused && scope.view_id == "one")
            .count(),
        2
    );
    assert!(
        scopes
            .iter()
            .filter(|scope| scope.view_id == "two")
            .all(|scope| !scope.focused)
    );
    assert!(visual.update(|window, cx| host.navigate_back(window, cx)));
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(text_of(&mut visual, &views[0], "depth"), "inner:1 outer:2");
    assert_eq!(text_of(&mut visual, &views[1], "depth"), "inner:2 outer:2");
    assert!(visual.update(|window, cx| host.navigate_back(window, cx)));
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(text_of(&mut visual, &views[0], "depth"), "inner:1 outer:1");
    assert!(
        !visual.update(|window, cx| host.navigate_back(window, cx)),
        "outer Host owns Back now; view two must not intercept"
    );
}
