//! Hidden-page continuity must retain the actual native virtual list, not
//! reconstruct one from a remembered approximate pixel offset.
use std::{cell::RefCell, rc::Rc};

use gpui::{ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{
    ScriptViewExtension, ScriptViewHandle, UiRuntimeState, VirtualCollectionMetrics,
    VirtualRequestRegistry,
};

use crate::support::{
    ActiveHost, Mount, ReplaceOnce, Step, by_test_id, dispatch, last_error, settle, snapshot,
};

const HISTORY: &str = r#"
import "components/tabs" as tabs;
import "patterns/navigation_stack" as nav;
fn state_schema() { #{ fields: #{
    show: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
    present: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
    count: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 100 } }
} } }
fn toggle(ctx, p) { ctx.set_state("show", !ctx.get_state("show")); }
fn choose(ctx, p) { ctx.set_state("show", p == "list"); }
fn append(ctx, p) { ctx.set_state("count", ctx.get_state("count") + 10); }
fn remove(ctx, p) { ctx.set_state("present", !ctx.get_state("present")); }
fn history_row(ctx, p) {
    text(`Row ${p.item.number}`).with_key(p.key)
        .accessibility_role("listitem").accessibility_label(`Row ${p.item.number}`)
        .with_style(style().height(px(32)))
}
fn view(ctx) {
    let content = text("Removed");
    if ctx.get_state("present") {
        let data = [];
        for n in 0..ctx.get_state("count") { data.push(#{ key: `row-${n}`, number: n }); }
        let history = virtual_collection(#{ key: "history", label: "History", data: data,
            height: 220, estimated_height: 32, alignment: "bottom", follow_tail: true }, Fn("history_row"));
        content = PAGE;
    }
    column([
        text("Toggle").test_id("toggle").on_click(Fn("toggle")),
        text("Append").test_id("append").on_click(Fn("append")),
        text("Remove").test_id("remove").on_click(Fn("remove")),
        content,
    ]).with_style(style().width(px(420)).height(px(420)))
}
"#;

#[derive(Clone, Copy, Debug)]
enum Pages {
    Direct,
    Tabs,
    Navigation,
}

fn source(pages: Pages) -> String {
    let page = match pages {
        Pages::Direct => {
            r#"column([
            history.with_key("page").retained_page(ctx.get_state("show")),
            text("Other page").with_key("other").retained_page(!ctx.get_state("show")),
        ])"#
        }
        Pages::Tabs => {
            r#"tabs::Tabs(#{ key: "pages", label: "Pages",
            value: if ctx.get_state("show") { "list" } else { "other" },
            keep_mounted: true, on_change: Fn("choose"), tabs: [
                #{ value: "list", label: "List", content: history },
                #{ value: "other", label: "Other", content: text("Other page") },
            ] })"#
        }
        Pages::Navigation => {
            r#"{
            let stack = [#{ key: "reader", title: "Reader", content: history }];
            if !ctx.get_state("show") {
                stack.push(#{ key: "detail", title: "Detail", content: text("Other page") });
            }
            nav::NavigationStack(#{ key: "pages", label: "Pages", stack: stack, header: "none" })
        }"#
        }
    };
    HISTORY.replace_once("PAGE", page)
}

#[derive(Clone)]
struct CaptureRegistry(Rc<RefCell<Option<VirtualRequestRegistry>>>);

impl ScriptViewExtension for CaptureRegistry {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        *self.0.borrow_mut() = Some(runtime.virtual_requests.clone());
        Ok(())
    }
}

fn mount(
    cx: &mut TestAppContext,
    pages: Pages,
) -> (VisualTestContext, ScriptViewHandle, VirtualRequestRegistry) {
    mount_source(cx, source(pages))
}

fn mount_source(
    cx: &mut TestAppContext,
    source: String,
) -> (VisualTestContext, ScriptViewHandle, VirtualRequestRegistry) {
    let registry = Rc::new(RefCell::new(None));
    let capture = registry.clone();
    let (window, view) = Mount::components(source)
        .registry("patterns/navigation_stack")
        .with(move |view| view.extension(CaptureRegistry(capture)))
        .mount_root(cx, |host, view| ActiveHost { host, view });
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Tick, 10);
    assert_eq!(last_error(&mut visual, &view), None);
    let registry = registry.borrow().clone().unwrap();
    (visual, view, registry)
}

fn metrics(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> VirtualCollectionMetrics {
    assert_eq!(last_error(visual, view), None);
    visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
        .virtual_collections[0]
        .clone()
}

fn visible_rows(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    let viewport = visual.debug_bounds("virtual-list:history").unwrap();
    snapshot(visual, view)
        .nodes()
        .filter_map(|node| {
            let geometry = node.geometry?.visual;
            (node.role == "listitem"
                && geometry.y < f64::from(viewport.bottom())
                && geometry.y + geometry.height > f64::from(viewport.top()))
            .then(|| node.name.clone())
        })
        .collect()
}

fn action(visual: &mut VisualTestContext, view: &ScriptViewHandle, id: &str) {
    dispatch(visual, view, by_test_id(id), "click").unwrap();
    settle(visual, Step::Tick, 8);
    assert_eq!(last_error(visual, view), None);
}

fn pause_reader(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> VirtualCollectionMetrics {
    let initial = metrics(visual, view);
    let viewport = visual.debug_bounds("virtual-list:history").unwrap();
    visual.simulate_event(ScrollWheelEvent {
        position: point(viewport.left() + px(60.0), viewport.top() + px(100.0)),
        delta: ScrollDelta::Pixels(point(px(0.0), px(400.0))),
        ..ScrollWheelEvent::default()
    });
    settle(visual, Step::Tick, 10);
    let paused = metrics(visual, view);
    assert!(paused.scroll_item < initial.scroll_item);
    assert!(!paused.is_following_tail);
    paused
}

fn reader_roundtrip(cx: &mut TestAppContext, pages: Pages, append: bool) {
    let (mut visual, view, _) = mount(cx, pages);
    let paused = pause_reader(&mut visual, &view);
    let rows = visible_rows(&mut visual, &view);
    action(&mut visual, &view, "toggle");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .all(|node| node.role != "listitem")
    );
    if append {
        action(&mut visual, &view, "append");
    }
    action(&mut visual, &view, "toggle");
    let restored = metrics(&mut visual, &view);
    assert_eq!(restored.scroll_item, paused.scroll_item, "{pages:?}");
    assert_eq!(restored.scroll_offset, paused.scroll_offset, "{pages:?}");
    assert!(!restored.is_following_tail, "{pages:?}");
    assert_eq!(visible_rows(&mut visual, &view), rows, "{pages:?}");
    assert_eq!(restored.unseen_count, if append { 10 } else { 0 });
}

#[gpui::test]
fn retained_page_keeps_virtual_list_reader(cx: &mut TestAppContext) {
    reader_roundtrip(cx, Pages::Direct, false);
}

#[gpui::test]
fn tabs_keep_mounted_keeps_virtual_list_reader(cx: &mut TestAppContext) {
    reader_roundtrip(cx, Pages::Tabs, false);
}

#[gpui::test]
fn navigation_stack_keeps_covered_virtual_list_reader(cx: &mut TestAppContext) {
    reader_roundtrip(cx, Pages::Navigation, false);
}

#[gpui::test]
fn paused_virtual_reader_accumulates_hidden_appends(cx: &mut TestAppContext) {
    reader_roundtrip(cx, Pages::Direct, true);
}

#[gpui::test]
fn tabs_paused_virtual_reader_accumulates_hidden_appends(cx: &mut TestAppContext) {
    reader_roundtrip(cx, Pages::Tabs, true);
}

#[gpui::test]
fn following_virtual_reader_follows_hidden_appends(cx: &mut TestAppContext) {
    let (mut visual, view, _) = mount(cx, Pages::Direct);
    let initial = metrics(&mut visual, &view);
    assert!(initial.is_following_tail);
    action(&mut visual, &view, "toggle");
    action(&mut visual, &view, "append");
    action(&mut visual, &view, "toggle");
    let restored = metrics(&mut visual, &view);
    assert_eq!(restored.item_count, 110);
    assert!(restored.is_following_tail);
    assert_eq!(restored.unseen_count, 0);
    assert!(restored.scroll_item > initial.scroll_item);
    assert!(visible_rows(&mut visual, &view).contains(&"Row 109".to_owned()));
}

#[gpui::test]
fn removed_hidden_virtual_list_releases_native_state(cx: &mut TestAppContext) {
    let (mut visual, view, registry) = mount(cx, Pages::Direct);
    let paused = pause_reader(&mut visual, &view);
    action(&mut visual, &view, "toggle");
    assert!(registry.scroll_handle(&paused.id).is_some());
    action(&mut visual, &view, "remove");
    assert!(registry.scroll_handle(&paused.id).is_none());
    action(&mut visual, &view, "toggle");
    action(&mut visual, &view, "remove");
    let remounted = metrics(&mut visual, &view);
    assert!(
        remounted.is_following_tail,
        "a real remount must not revive the paused reader"
    );
    assert!(remounted.scroll_item > paused.scroll_item);
}

#[gpui::test]
fn disposing_view_releases_hidden_virtual_list(cx: &mut TestAppContext) {
    let (mut visual, view, registry) = mount(cx, Pages::Direct);
    let id = metrics(&mut visual, &view).id;
    action(&mut visual, &view, "toggle");
    assert!(registry.scroll_handle(&id).is_some());
    visual.update(|_, cx| view.dispose(cx)).unwrap();
    settle(&mut visual, Step::Tick, 8);
    assert!(registry.scroll_handle(&id).is_none());
}

#[gpui::test]
fn failed_hidden_list_removal_restores_native_capabilities(cx: &mut TestAppContext) {
    let source = source(Pages::Direct)
        .replace_once(
            "fn view(ctx) {",
            r#"
            define_component(#{
                metadata: #{ id: "test/failure", "export": "Failure", version: "0.2.2",
                    runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
                schema: #{ props: #{
                    key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    enabled: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
                }, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: [], effects: ["fail"] },
                render: Fn("render_failure")
            });
            fn start_failure(ctx, enabled) { if enabled { throw "rejected removal"; } }
            fn cleanup_failure(ctx, enabled) {}
            fn render_failure(ctx, props) {
                effect("fail", props.enabled, Fn("start_failure"), Fn("cleanup_failure"));
                text("Validation")
            }
            fn view(ctx) {
            "#,
        )
        .replace_once(
            "        content,",
            r#"        content,
                render_component("test/failure", #{ key: "failure", enabled: !ctx.get_state("present") }),"#,
        );
    let (mut visual, view, registry) = mount_source(cx, source);
    let paused = pause_reader(&mut visual, &view);
    action(&mut visual, &view, "toggle");
    assert!(registry.scroll_handle(&paused.id).is_some());
    assert!(dispatch(&mut visual, &view, by_test_id("remove"), "click").is_err());
    settle(&mut visual, Step::Tick, 8);
    assert!(
        last_error(&mut visual, &view)
            .unwrap()
            .contains("rejected removal")
    );
    let restored = registry
        .scroll_handle(&paused.id)
        .expect("last-good hidden native capability");
    assert!(!restored.status().following);
}

#[gpui::test]
fn changing_collection_id_under_a_stable_node_key_retires_previous_native_entity(
    cx: &mut TestAppContext,
) {
    let source = source(Pages::Direct)
        .replace_once(
            "key: \"history\", label: \"History\"",
            r#"key: if ctx.get_state("count") == 100 { "history" } else { `history-${ctx.get_state("count")}` }, label: "History""#,
        )
        .replace_once(
            "Fn(\"history_row\"));",
            r#"Fn("history_row")).with_key("viewport").accessibility_role("list").accessibility_label("Viewport").test_id("viewport");"#,
        );
    let (mut visual, view, registry) = mount_source(cx, source);
    let mut previous = pause_reader(&mut visual, &view).id;
    let original_node = snapshot(&mut visual, &view)
        .find_by_test_id("viewport")
        .next()
        .unwrap()
        .id;
    for _ in 0..3 {
        assert!(format!("{registry:?}").contains("retained_entities: Some(1)"));
        action(&mut visual, &view, "append");
        let current = metrics(&mut visual, &view);
        assert_ne!(current.id, previous);
        assert!(registry.scroll_handle(&previous).is_none());
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_test_id("viewport")
                .next()
                .unwrap()
                .id,
            original_node,
            "the declarative node deliberately survives the source identity change"
        );
        assert!(
            format!("{registry:?}").contains("retained_entities: Some(1)"),
            "old native entities must not accumulate behind a live NodeId: {registry:?}"
        );
        assert!(
            current.is_following_tail,
            "a new identity starts a new native list"
        );
        previous = current.id;
    }
}
