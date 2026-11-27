use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{MotionPreference, ScriptViewHandle, WallClock, WallClockSource};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Mount, bounds, snapshot};

fn has(v: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) -> bool {
    snapshot(v, view)
        .find_by_role_and_name(role, name)
        .next()
        .is_some()
}

fn acknowledge(v: &mut VisualTestContext, view: &ScriptViewHandle) {
    v.executor().advance_clock(Duration::from_millis(375));
    ready(v, view);
}

#[gpui::test]
fn tool_call_card_native_code_viewport_caps_and_copy_uses_full_json(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "tool_call_card", "modern-light");
    v.simulate_resize(size(px(1320.), px(1380.)));
    ready(&mut v, &view);
    assert!(!has(&mut v, &view, "document", "Result"));
    click(&mut v, &view, "button", "read_index");
    node(&mut v, &view, "document", "Arguments");
    let collapsed = bounds(&mut v, &view, "document", "Result");
    assert!(collapsed.height <= 130., "{collapsed:?}");
    click(&mut v, &view, "button", "Copy result");
    node(&mut v, &view, "status", "Copied result for index/task");
    let copied = v
        .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&copied).unwrap();
    assert_eq!(json["indexed"], 34);
    assert_eq!(json["documents"].as_array().unwrap().len(), 34);

    click(&mut v, &view, "button", "Show all");
    let expanded = bounds(&mut v, &view, "document", "Result");
    assert!(
        expanded.height >= collapsed.height + 200. && expanded.height <= 642.,
        "{collapsed:?} -> {expanded:?}"
    );
    click(&mut v, &view, "button", "Show less");
    assert!(bounds(&mut v, &view, "document", "Result").height <= 130.);
    click(&mut v, &view, "button", "Narrow card");
    let root = bounds(&mut v, &view, "group", "read_index tool call");
    let data = bounds(&mut v, &view, "document", "Result");
    assert!(
        root.width <= 342. && data.x >= root.x && data.x + data.width <= root.x + root.width + 1.,
        "{root:?} {data:?}"
    );
    click(&mut v, &view, "button", "read_index");
    assert!(!has(&mut v, &view, "document", "Result"));
}

#[gpui::test]
fn tool_call_card_retries_are_versioned_and_do_not_optimistically_clear_a_failure(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "tool_call_card", "clean-dark");
    v.simulate_resize(size(px(1310.), px(980.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Refuse next retry");
    click(&mut v, &view, "button", "Publish failure");
    assert_eq!(
        node(&mut v, &view, "button", "read_index").expanded,
        Some(true)
    );
    node(
        &mut v,
        &view,
        "group",
        "Cannot load the index. Try again after the cache warms.",
    );
    click(&mut v, &view, "button", "Retry");
    node(&mut v, &view, "status", "Waiting for retry approval");
    node(&mut v, &view, "status", "read_index Failed");
    acknowledge(&mut v, &view);
    node(&mut v, &view, "status", "Retry request refused");
    node(&mut v, &view, "status", "read_index Failed");
    click(&mut v, &view, "button", "Retry");
    acknowledge(&mut v, &view);
    node(&mut v, &view, "status", "Accepted retry: index/task");
    node(&mut v, &view, "status", "read_index Running");
    assert_eq!(
        node(&mut v, &view, "button", "read_index").expanded,
        Some(false)
    );
    let time = node(&mut v, &view, "button", "read_index").description;
    click(&mut v, &view, "button", "Advance clock");
    assert_ne!(
        node(&mut v, &view, "button", "read_index").description,
        time
    );
    click(&mut v, &view, "button", "read_index");
    assert!(has(&mut v, &view, "text", "Waiting for result…"));
}

#[gpui::test]
fn tool_call_group_derives_summary_and_only_mounts_children_while_open(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "tool_call_group", "clean-light");
    v.simulate_resize(size(px(1300.), px(1200.)));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Using tools… · 1 failed");
    assert!(!has(&mut v, &view, "button", "search_docs"));
    click(&mut v, &view, "button", "Using tools…");
    node(&mut v, &view, "button", "search_docs");
    node(&mut v, &view, "group", "Cache not ready.");
    click(&mut v, &view, "button", "Publish result");
    let header = node(&mut v, &view, "button", "Used 3 tools");
    assert_eq!(header.expanded, Some(true));
    assert_eq!(header.description, "1 failed");
    click(&mut v, &view, "button", "read_metadata");
    click(&mut v, &view, "button", "Copy result");
    node(&mut v, &view, "status", "Copied result for meta/task");
    click(&mut v, &view, "button", "Retry");
    acknowledge(&mut v, &view);
    node(&mut v, &view, "status", "Accepted retry: cache/task");
    node(&mut v, &view, "status", "load_cache Running");
    click(&mut v, &view, "button", "Using tools…");
    assert!(!has(&mut v, &view, "button", "read_metadata"));
    assert!(!has(&mut v, &view, "document", "Result"));
    click(&mut v, &view, "button", "Using tools…");
    assert_eq!(
        node(&mut v, &view, "button", "read_metadata").expanded,
        Some(true)
    );
    node(&mut v, &view, "document", "Result");
}

#[gpui::test]
fn tool_calls_group_samples_the_view_clock_and_shares_it_with_children(cx: &mut TestAppContext) {
    #[derive(Debug)]
    struct Epoch(Rc<Cell<i64>>);
    impl WallClockSource for Epoch {
        fn now_ms(&self) -> i64 {
            self.0.get()
        }
    }
    let epoch = Rc::new(Cell::new(1700));
    let wall = WallClock::from_source(Epoch(Rc::clone(&epoch)));
    let (mut v, view) = Mount::components(r#"
        import "patterns/tool_call_group" as group;
        fn view(ctx) { group::ToolCallGroup(#{key:"live",session:"test",revision:0,default_open:true,
            calls:[#{id:"read",name:"read",status:"running",started_ms:1000},
                   #{id:"find",name:"find",status:"running",started_ms:1500}]}) }
    "#)
    .registry("patterns/tool_call_group")
    .motion(MotionPreference::None)
    .with(move |v| v.wall_clock(wall))
    .mount(cx);
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "button", "Using tools…").description,
        "700ms"
    );
    epoch.set(3200);
    v.executor().advance_clock(Duration::from_millis(230));
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "button", "Using tools…").description,
        "2.2s"
    );
    assert!(
        node(&mut v, &view, "button", "find")
            .description
            .contains("1.7s")
    );
}
