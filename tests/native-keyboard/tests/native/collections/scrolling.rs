//! Regression controls for native following and keyed reader anchors (#162/163).

use gpui::{ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{ScriptViewHandle, VirtualCollectionMetrics};

use crate::support::{
    Mount, ReplaceOnce, Step, dispatch_script_button, last_error, settle, snapshot,
};

const COLLECTION: &str = r#"
fn state_schema() { #{ fields: #{
    count: #{ schema: #{type:"integer"}, "default": #{type:"integer", value:50} },
    prefix: #{ schema: #{type:"integer"}, "default": #{type:"integer", value:0} },
    chunks: #{ schema: #{type:"map", values:#{type:"integer"}}, "default": #{type:"map", value:#{}} },
    notices: #{ schema: #{type:"integer"}, "default": #{type:"integer", value:0} },
    following: #{ schema: #{type:"bool"}, "default": #{type:"bool", value:true} },
    unseen: #{ schema: #{type:"integer"}, "default": #{type:"integer", value:0} },
    reversed: #{ schema: #{type:"bool"}, "default": #{type:"bool", value:false} },
    bottom: #{ schema: #{type:"bool"}, "default": #{type:"bool", value:false} },
} } }
fn append(ctx, value) {ctx.set_state("count", ctx.get_state("count") + 1);}
fn prepend(ctx, value) {ctx.set_state("prefix", ctx.get_state("prefix") - 1);}
fn remove_first(ctx, value) {ctx.set_state("prefix", ctx.get_state("prefix") + 1);}
fn grow(ctx, value) {
    let chunks = ctx.get_state("chunks"); let key = `row-${ctx.get_state("count") - 1}`;
    chunks[key] = if key in chunks {chunks[key] + 12} else {12}; ctx.set_state("chunks", chunks);
}
fn latest(ctx, value) {ctx.scroll_collection_to_end("events");}
fn start(ctx, value) {ctx.scroll_collection_to_start("events");}
fn reverse_order(ctx, value) {ctx.set_state("reversed", !ctx.get_state("reversed"));}
fn rejected_latest(ctx, value) {ctx.scroll_collection_to_end("events"); throw "rejected scroll command";}
fn prepend_and_start(ctx, value) {ctx.set_state("prefix", ctx.get_state("prefix") - 1); ctx.scroll_collection_to_start("events");}
fn replace_and_end(ctx, value) {ctx.set_state("prefix", 100); ctx.set_state("count", 120); ctx.scroll_collection_to_end("events");}
fn recreate_source(ctx,value){ctx.set_state("bottom",!ctx.get_state("bottom"));}
fn followed(ctx, value) {
    ctx.set_state("following", value.following);
    ctx.set_state("unseen", value.unseen);
    ctx.set_state("notices", ctx.get_state("notices") + 1);
}
fn item(ctx, payload) {
    let lines = []; for index in 0..payload.item.lines {lines.push(text(`Line ${index}`));}
    column([text(payload.item.label), column(lines)])
        .accessibility_role("listitem").accessibility_label(payload.item.label)
        .with_style(style().min_height(px(40)))
}
fn control(label, callback) {
    text(label).with_key(label).accessibility_role("button").accessibility_label(label).on_click(callback)
}
fn view(ctx) {
    let data = [];
    let chunks = ctx.get_state("chunks");
    for index in ctx.get_state("prefix")..ctx.get_state("count") {
        data.push(#{key:`row-${index}`, label:`Row ${index}`,
            lines:if `row-${index}` in chunks {chunks[`row-${index}`]} else {0}});
    }
    if ctx.get_state("reversed") {data.reverse();}
    column([
        row([control("Append", Fn("append")), control("Prepend", Fn("prepend")),
            control("Remove first", Fn("remove_first")), control("Grow", Fn("grow")),
            control("Latest", Fn("latest")), control("Start", Fn("start")), control("Reverse", Fn("reverse_order")),
            control("Rejected latest", Fn("rejected_latest")), control("Prepend and start", Fn("prepend_and_start")),
            control("Replace and end", Fn("replace_and_end")), control("Recreate source",Fn("recreate_source"))]),
        text(`Notices ${ctx.get_state("notices")}`),
        virtual_collection(#{key:"events", label:"Events", data:data, estimated_height:40,
            height:200, overdraw_pixels:80, alignment:if ctx.get_state("bottom"){"bottom"}else{"top"}, follow_tail:FOLLOW,
            on_follow_change:Fn("followed")}, Fn("item"))
            .scrollbars("hidden", "always")
    ]).with_style(style().width(relative(1.0)).max_width(px(600)))
}
"#;

fn metrics(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> VirtualCollectionMetrics {
    assert!(
        last_error(visual, view).is_none(),
        "scrolling fixture must remain valid"
    );
    visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
        .virtual_collections[0]
        .clone()
}

fn wheel(visual: &mut VisualTestContext, delta: f32) {
    let viewport = visual
        .debug_bounds("virtual-list:events")
        .expect("mounted list viewport");
    visual.simulate_event(ScrollWheelEvent {
        position: point(
            viewport.origin.x + viewport.size.width / 2.0,
            viewport.origin.y + viewport.size.height / 2.0,
        ),
        delta: ScrollDelta::Pixels(point(px(0.0), px(delta))),
        ..Default::default()
    });
    settle(visual, Step::Tick, 10);
}

fn visible_rows(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<(String, f64)> {
    let viewport = visual.debug_bounds("virtual-list:events").unwrap();
    snapshot(visual, view)
        .nodes()
        .filter_map(|node| {
            let bounds = node.geometry?.visual;
            (node.role == "listitem"
                && bounds.y < f64::from(viewport.bottom())
                && bounds.y + bounds.height > f64::from(viewport.top()))
            .then(|| (node.name.clone(), bounds.y))
        })
        .collect()
}

#[gpui::test]
fn following_pauses_across_realization_and_resumes_at_the_native_tail(cx: &mut TestAppContext) {
    for follow in [false, true] {
        let (mut visual, view) =
            Mount::new(COLLECTION.replace_once("FOLLOW", if follow { "true" } else { "false" }))
                .id(&format!("follow-control-{follow}"))
                .mount(cx);
        settle(&mut visual, Step::Tick, 10);
        if !follow {
            dispatch_script_button(&mut visual, &view, "Latest");
            settle(&mut visual, Step::Tick, 10);
        }
        let before = metrics(&mut visual, &view);
        assert!(
            before.scroll_item > 30,
            "control starts at the end: {before:?}"
        );
        assert_eq!(before.is_following_tail, follow);
        wheel(&mut visual, 400.0);
        let paused = metrics(&mut visual, &view);
        assert!(
            paused.scroll_item < before.scroll_item,
            "reading up must survive realization: {before:?} -> {paused:?}"
        );
        assert!(!paused.is_following_tail);
        let rows = visible_rows(&mut visual, &view);
        dispatch_script_button(&mut visual, &view, "Append");
        settle(&mut visual, Step::Tick, 10);
        assert_eq!(
            visible_rows(&mut visual, &view),
            rows,
            "paused append preserves presented reader rows"
        );
        assert_eq!(
            metrics(&mut visual, &view).unseen_count,
            usize::from(follow)
        );
        dispatch_script_button(&mut visual, &view, "Latest");
        settle(&mut visual, Step::Tick, 10);
        let latest = metrics(&mut visual, &view);
        assert_eq!(latest.is_following_tail, follow);
        assert_eq!(latest.unseen_count, 0);
        assert!(latest.scroll_item > paused.scroll_item);
        wheel(&mut visual, 160.0);
        assert!(!metrics(&mut visual, &view).is_following_tail);
        wheel(&mut visual, -10000.0);
        assert_eq!(
            metrics(&mut visual, &view).is_following_tail,
            follow,
            "wheel to bottom resumes native following"
        );
    }
}

#[gpui::test]
fn keyed_append_insert_and_delete_keep_the_presented_reader_anchor(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(COLLECTION.replace_once("FOLLOW", "false"))
        .id("keyed-reader")
        .mount(cx);
    settle(&mut visual, Step::Tick, 10);
    wheel(&mut visual, -400.0);
    assert!(
        metrics(&mut visual, &view).scroll_item > 0,
        "the regression control must scroll before editing"
    );
    let before = visible_rows(&mut visual, &view);
    assert!(!before.is_empty());
    for command in ["Append", "Prepend", "Remove first", "Remove first"] {
        dispatch_script_button(&mut visual, &view, command);
        settle(&mut visual, Step::Tick, 10);
        assert_eq!(
            visible_rows(&mut visual, &view),
            before,
            "{command} must preserve key and intra-row offset"
        );
    }
}

#[gpui::test]
fn paused_follow_survives_reorder_without_a_false_native_tail_reengagement(
    cx: &mut TestAppContext,
) {
    for overdraw in ["80", "0"] {
        let main = COLLECTION
            .replace_once("FOLLOW", "true")
            .replace_once("overdraw_pixels:80", &format!("overdraw_pixels:{overdraw}"));
        let (mut visual, view) = Mount::new(main)
            .id(&format!("reorder-reader-{overdraw}"))
            .mount(cx);
        settle(&mut visual, Step::Tick, 10);
        wheel(&mut visual, 800.0);
        let before = visible_rows(&mut visual, &view);
        assert!(!metrics(&mut visual, &view).is_following_tail);
        let anchor = before.first().expect("visible reader row").clone();
        dispatch_script_button(&mut visual, &view, "Reverse");
        settle(&mut visual, Step::Tick, 10);
        assert!(
            !metrics(&mut visual, &view).is_following_tail,
            "zero-height unmeasured moved rows must not fool native tail reengagement"
        );
        assert!(
            visible_rows(&mut visual, &view)
                .iter()
                .any(|row| row == &anchor),
            "reorder preserves the reader's key and viewport coordinate"
        );
    }
}

#[gpui::test]
fn streaming_last_row_keeps_its_bottom_visible_and_follow_callbacks_settle(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = Mount::new(COLLECTION.replace_once("FOLLOW", "true"))
        .id("streaming-tail")
        .mount(cx);
    settle(&mut visual, Step::Tick, 10);
    for _ in 0..3 {
        dispatch_script_button(&mut visual, &view, "Grow");
        settle(&mut visual, Step::Tick, 10);
        assert!(metrics(&mut visual, &view).is_following_tail);
        let viewport = visual.debug_bounds("virtual-list:events").unwrap();
        let row = snapshot(&mut visual, &view)
            .nodes()
            .find(|node| node.role == "listitem" && node.name == "Row 49")
            .expect("last row is realized")
            .geometry
            .unwrap()
            .visual;
        assert!(row.height > f64::from(viewport.size.height));
        assert!(
            (row.y + row.height - f64::from(viewport.bottom())).abs() < 1.0,
            "tail is the bottom of a growing tall row, not its top: {row:?} in {viewport:?}"
        );
    }
    let old_viewport = visual.debug_bounds("virtual-list:events").unwrap();
    visual.simulate_resize(gpui::size(px(320.0), px(480.0)));
    settle(&mut visual, Step::Tick, 8);
    let viewport = visual.debug_bounds("virtual-list:events").unwrap();
    assert!(viewport.size.width < old_viewport.size.width);
    assert!(
        metrics(&mut visual, &view).is_following_tail,
        "width invalidation preserves native following"
    );
    let row = snapshot(&mut visual, &view)
        .nodes()
        .find(|node| node.role == "listitem" && node.name == "Row 49")
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    assert!(
        (row.y + row.height - f64::from(viewport.bottom())).abs() < 1.0,
        "the resized tail still shows the growing row's bottom"
    );
    let before = snapshot(&mut visual, &view);
    let notice_before = before
        .nodes()
        .find(|node| node.name.starts_with("Notices "))
        .unwrap()
        .name
        .clone();
    settle(&mut visual, Step::Tick, 12);
    let after = snapshot(&mut visual, &view);
    let notice_after = after
        .nodes()
        .find(|node| node.name.starts_with("Notices "))
        .unwrap()
        .name
        .clone();
    assert_eq!(
        notice_after, notice_before,
        "unchanged presented state must not emit follow callbacks forever"
    );
    let _ = metrics(&mut visual, &view);
    settle(&mut visual, Step::Tick, 4);
    let stable = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    assert!(!stable.pending_virtual_requests);
    assert!(
        stable.timings.iter().all(|timing| !matches!(
            timing.operation,
            gpui_rhai::ExecutionOperation::VirtualCollection(_)
        )),
        "stable frames execute no virtual row Rhai"
    );
}

#[gpui::test]
fn scrolling_gallery_module_mounts_and_exercises_its_real_controls(cx: &mut TestAppContext) {
    for density in ["comfortable", "compact"] {
        let main = r#"
import "gallery/pages/scrolling" as scrolling;
fn noop(ctx, value) {}
fn init(ctx) { ctx.register_action("gallery.toggle_source", Fn("noop")); }
fn view(ctx) {scrolling::view(ctx, #{}, #{}, "scrolling").env(#{density:"DENSITY"})}
"#
        .replace_once("DENSITY", density);
        let (mut visual, view) = Mount::components(main)
            .modules(gpui_rhai_registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
            .modules(gpui_rhai_registry::BUNDLED_PATTERN_SOURCES_BY_ID)
            .module(
                "gallery/kit",
                include_str!("../../../../../registry/gallery/kit.rhai"),
            )
            .module(
                "gallery/pages/scrolling",
                include_str!("../../../../../registry/gallery/pages/scrolling.rhai"),
            )
            .id(&format!("scrolling-gallery-{density}"))
            .mount(cx);
        settle(&mut visual, Step::Tick, 10);
        assert!(last_error(&mut visual, &view).is_none());
        let initial = metrics(&mut visual, &view);
        assert_eq!(initial.item_count, 40);
        assert!(initial.is_following_tail);
        for control in [
            "Append event",
            "Stream chunk",
            "Prepend history",
            "Remove first",
        ] {
            dispatch_script_button(&mut visual, &view, control);
            settle(&mut visual, Step::Tick, 8);
            assert!(
                last_error(&mut visual, &view).is_none(),
                "{density}/{control}"
            );
        }
        assert_eq!(metrics(&mut visual, &view).item_count, 41);
        dispatch_script_button(&mut visual, &view, "Start");
        settle(&mut visual, Step::Tick, 8);
        assert!(!metrics(&mut visual, &view).is_following_tail);
        dispatch_script_button(&mut visual, &view, "Append event");
        settle(&mut visual, Step::Tick, 8);
        assert_eq!(metrics(&mut visual, &view).unseen_count, 1);
        dispatch_script_button(&mut visual, &view, "Jump to latest (1)");
        settle(&mut visual, Step::Tick, 8);
        assert!(metrics(&mut visual, &view).is_following_tail);
        let viewport = visual.debug_bounds("virtual-list:transcript").unwrap();
        let position = point(
            viewport.origin.x + viewport.size.width / 2.0,
            viewport.origin.y + viewport.size.height / 2.0,
        );
        visual.simulate_mouse_down(
            position,
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );
        visual.simulate_mouse_up(
            position,
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );
        for (key, following) in [
            ("home", false),
            ("end", true),
            ("pageup", false),
            ("end", true),
        ] {
            visual.simulate_keystrokes(key);
            settle(&mut visual, Step::Tick, 6);
            assert_eq!(
                metrics(&mut visual, &view).is_following_tail,
                following,
                "{density}/{key} is owned by the transcript viewport"
            );
        }
    }
}

#[gpui::test]
fn edits_preserve_an_anchor_inside_a_tall_row_while_following_is_paused(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(COLLECTION.replace_once("FOLLOW", "true"))
        .id("tall-reader")
        .mount(cx);
    settle(&mut visual, Step::Tick, 8);
    for _ in 0..3 {
        dispatch_script_button(&mut visual, &view, "Grow");
        settle(&mut visual, Step::Tick, 8);
    }
    wheel(&mut visual, 260.0);
    let before = metrics(&mut visual, &view);
    assert!(!before.is_following_tail);
    assert!(
        before.scroll_offset > 20.0,
        "control is inside, not above, a tall row: {before:?}"
    );
    let reader = visible_rows(&mut visual, &view)
        .into_iter()
        .find(|row| row.0 == "Row 49")
        .unwrap();
    for command in ["Append", "Prepend", "Remove first", "Reverse"] {
        dispatch_script_button(&mut visual, &view, command);
        settle(&mut visual, Step::Tick, 8);
        let current = metrics(&mut visual, &view);
        assert!(
            !current.is_following_tail,
            "{command} must not reengage following inside a tall row"
        );
        assert!(
            (current.scroll_offset - before.scroll_offset).abs() < 1.0,
            "{command} preserves intra-row offset"
        );
        assert!(
            visible_rows(&mut visual, &view).contains(&reader),
            "{command} preserves presented tall-row geometry"
        );
    }
    let old_viewport = visual.debug_bounds("virtual-list:events").unwrap();
    visual.simulate_resize(gpui::size(px(320.0), px(480.0)));
    settle(&mut visual, Step::Tick, 8);
    let viewport = visual.debug_bounds("virtual-list:events").unwrap();
    assert!(viewport.size.width < old_viewport.size.width);
    let resized = metrics(&mut visual, &view);
    assert!(
        !resized.is_following_tail,
        "width invalidation cannot pull a paused tall-row reader to the tail"
    );
    assert!((resized.scroll_offset - before.scroll_offset).abs() < 1.0);
    let resized_row = visible_rows(&mut visual, &view)
        .into_iter()
        .find(|row| row.0 == reader.0)
        .unwrap();
    assert!(
        (resized_row.1 - f64::from(viewport.top()) - (reader.1 - f64::from(old_viewport.top())))
            .abs()
            < 1.0,
        "resize preserves the tall anchor relative to its viewport"
    );
}

#[gpui::test]
fn equal_local_collection_keys_in_different_views_do_not_share_native_scroll_state(
    cx: &mut TestAppContext,
) {
    let source = COLLECTION.replace_once("FOLLOW", "true");
    let (mut first, first_view) = Mount::new(source.clone())
        .id("collection-owner-a")
        .mount(cx);
    let (mut second, second_view) = Mount::new(source).id("collection-owner-b").mount(cx);
    settle(&mut first, Step::Tick, 6);
    settle(&mut second, Step::Tick, 6);
    let first_id = metrics(&mut first, &first_view).id;
    let second_id = metrics(&mut second, &second_view).id;
    assert_eq!(first_id.key, second_id.key);
    assert_ne!(first_id.component, second_id.component);
    second.update(|_, cx| {
        assert!(
            second_view
                .collection_scroll_handle(&first_id, cx)
                .unwrap()
                .is_none()
        )
    });
    first.update(|_, cx| {
        first_view
            .collection_scroll_handle(&first_id, cx)
            .unwrap()
            .unwrap()
            .scroll_to_start(cx)
    });
    settle(&mut first, Step::Tick, 6);
    settle(&mut second, Step::Tick, 6);
    assert!(!metrics(&mut first, &first_view).is_following_tail);
    assert!(metrics(&mut second, &second_view).is_following_tail);
}

#[gpui::test]
fn failed_callback_rolls_back_a_queued_collection_scroll_command(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(COLLECTION.replace_once("FOLLOW", "true"))
        .id("scroll-rollback")
        .mount(cx);
    settle(&mut visual, Step::Tick, 8);
    wheel(&mut visual, 400.0);
    let before = visible_rows(&mut visual, &view);
    let result = crate::support::dispatch(
        &mut visual,
        &view,
        crate::support::by_role("button", "Rejected latest"),
        "click",
    );
    assert!(result.is_err());
    visual.update(|_, cx| view.clear_error(cx)).unwrap();
    settle(&mut visual, Step::Tick, 8);
    assert!(
        !metrics(&mut visual, &view).is_following_tail,
        "a rejected transaction cannot resume native following"
    );
    assert_eq!(visible_rows(&mut visual, &view), before);
}

#[gpui::test]
fn explicit_collection_commands_apply_after_new_data_and_anchor_synchronization(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = Mount::new(COLLECTION.replace_once("FOLLOW", "false"))
        .id("late-scroll-priority")
        .mount(cx);
    settle(&mut visual, Step::Tick, 8);
    wheel(&mut visual, -400.0);
    assert!(metrics(&mut visual, &view).scroll_item > 0);
    dispatch_script_button(&mut visual, &view, "Prepend and start");
    settle(&mut visual, Step::Tick, 8);
    assert_eq!(
        metrics(&mut visual, &view).scroll_item,
        0,
        "explicit Start wins over preserving the old reader on this prepend"
    );
    assert!(
        visible_rows(&mut visual, &view)
            .iter()
            .any(|row| row.0 == "Row -1")
    );
    dispatch_script_button(&mut visual, &view, "Replace and end");
    settle(&mut visual, Step::Tick, 8);
    assert_eq!(metrics(&mut visual, &view).item_count, 20);
    assert!(
        visible_rows(&mut visual, &view)
            .iter()
            .any(|row| row.0 == "Row 119"),
        "End targets the newly committed replacement, not the previously presented handle"
    );
}

#[gpui::test]
fn nested_raw_virtual_row_follow_notification_uses_the_real_callback_owner(
    cx: &mut TestAppContext,
) {
    let main = r#"
fn state_schema(){#{fields:#{notified:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn changed(ctx, status){ctx.set_state("notified",ctx.get_state("notified")+1);}
fn inner_row(ctx, payload){text(payload.item.key).with_style(style().height(px(30)))}
fn outer_row(ctx, payload){
    let rows=[];for index in 0..20 {rows.push(#{key:`inner-${index}`});}
    virtual_collection(#{key:"inner",label:"Inner",data:rows,estimated_height:30,height:90,
        alignment:"bottom",follow_tail:true,on_follow_change:Fn("changed")},Fn("inner_row"))
}
fn view(ctx){column([text(`Notified ${ctx.get_state("notified")}`),
    virtual_collection(#{key:"outer",label:"Outer",data:[#{key:"first"}],estimated_height:90,height:160},Fn("outer_row"))
]).with_style(style().width(px(300)))}
"#;
    let (mut visual, view) = Mount::new(main).id("raw-follow-owner").mount(cx);
    settle(&mut visual, Step::Tick, 12);
    assert!(last_error(&mut visual, &view).is_none());
    assert!(
        crate::support::palette_texts(&mut visual, &view).contains(&"Notified 1".to_owned()),
        "notification context belongs to the executable caller, not synthetic Item state"
    );
}

#[gpui::test]
fn scroll_track_routes_unsupported_axes_and_only_boundary_residual_to_sdk_ancestors(
    cx: &mut TestAppContext,
) {
    let main = r#"
fn reset(ctx,value){ctx.scroll_to("outer",0,0);}
fn item(ctx,payload){text(payload.item.key).with_style(style().height(px(40)))}
fn view(ctx){
    let rows=[];for index in 0..30 {rows.push(#{key:`row-${index}`});}
    let collection=virtual_collection(#{key:"inner",label:"Inner",data:rows,estimated_height:40,height:100},Fn("item"))
        .with_style(style().width(px(180))).scrollbars("hidden","always");
    let content=column([text("Outer marker").test_id("outer-marker").with_style(style().height(px(20))),
        collection,box([]).with_style(style().width(px(800)).height(px(400)))])
        .with_style(style().width(px(800)));
    column([text("Reset outer").with_key("reset").accessibility_role("button").accessibility_label("Reset outer").on_click(Fn("reset")),
        box([content]).with_key("outer").with_ref(element_ref("outer"))
            .with_style(style().width(px(240)).height(px(160)).overflow_scroll())])
}

"#;
    let (mut visual, view) = Mount::new(main).id("residual-scroll-chain").mount(cx);
    settle(&mut visual, Step::Tick, 10);
    let track_wheel = |visual: &mut VisualTestContext, x: f32, y: f32| {
        let viewport = visual.debug_bounds("virtual-list:inner").unwrap();
        visual.simulate_event(ScrollWheelEvent {
            position: point(viewport.right() - px(6.0), viewport.top() + px(50.0)),
            delta: ScrollDelta::Pixels(point(px(x), px(y))),
            ..Default::default()
        });
        settle(visual, Step::Tick, 8);
    };
    let marker = crate::support::node_by_test_id(&mut visual, &view, "outer-marker")
        .geometry
        .unwrap()
        .visual;
    track_wheel(&mut visual, -30.0, 0.0);
    let moved = crate::support::node_by_test_id(&mut visual, &view, "outer-marker")
        .geometry
        .unwrap()
        .visual;
    assert!(
        (moved.x - marker.x + 30.0).abs() < 1.0,
        "horizontal delta passes the vertical ListState track to the outer horizontal axis once"
    );
    assert_eq!(metrics(&mut visual, &view).scroll_item, 0);
    dispatch_script_button(&mut visual, &view, "Reset outer");
    settle(&mut visual, Step::Tick, 8);
    let id = metrics(&mut visual, &view).id;
    visual.update(|_, cx| {
        view.collection_scroll_handle(&id, cx)
            .unwrap()
            .unwrap()
            .scroll_to_end(cx)
    });
    settle(&mut visual, Step::Tick, 8);
    let before = crate::support::node_by_test_id(&mut visual, &view, "outer-marker")
        .geometry
        .unwrap()
        .visual
        .y;
    track_wheel(&mut visual, 0.0, -35.0);
    let after = crate::support::node_by_test_id(&mut visual, &view, "outer-marker")
        .geometry
        .unwrap()
        .visual
        .y;
    assert!(
        (after - before + 35.0).abs() < 1.0,
        "at the inner end the outer gets the entire unconsumed vertical delta once"
    );
    dispatch_script_button(&mut visual, &view, "Reset outer");
    settle(&mut visual, Step::Tick, 8);
    visual.update(|_, cx| {
        view.collection_scroll_handle(&id, cx)
            .unwrap()
            .unwrap()
            .scroll_by(px(-10.0), cx)
    });
    settle(&mut visual, Step::Tick, 8);
    let before = crate::support::node_by_test_id(&mut visual, &view, "outer-marker")
        .geometry
        .unwrap()
        .visual
        .y;
    track_wheel(&mut visual, 0.0, -35.0);
    let after = crate::support::node_by_test_id(&mut visual, &view, "outer-marker")
        .geometry
        .unwrap()
        .visual
        .y;
    assert!(
        (after - before + 25.0).abs() < 1.0,
        "the inner consumes only its last 10px; parent receives 25px, not another full 35px"
    );
}

#[gpui::test]
fn collection_source_recreation_retires_a_captured_scrollbar_drag(cx: &mut TestAppContext) {
    let source=COLLECTION.replace_once("FOLLOW","false").replace_once(".scrollbars(\"hidden\", \"always\")",
        ".scrollbars(\"hidden\", \"always\").with_part_style(\"scrollbar_thumb\",style().background(rgba(0xea00eaff))).with_part_style(\"scrollbar_thumb_hover\",style().background(rgba(0xea00eaff)))");
    let (mut visual, view) = Mount::new(source)
        .id("collection-drag-source-replacement")
        .mount(cx);
    settle(&mut visual, Step::Tick, 8);
    let id = metrics(&mut visual, &view).id;
    let old = visual.update(|_, cx| view.collection_scroll_handle(&id, cx).unwrap().unwrap());
    let thumb = crate::support::filled(&mut visual, 0xea00_eaff)[0].visible();
    let position = point(
        px(thumb.x + thumb.width / 2.0),
        px(thumb.y + thumb.height / 2.0),
    );
    visual.simulate_mouse_down(
        position,
        gpui::MouseButton::Left,
        gpui::Modifiers::default(),
    );
    assert!(old.is_scrollbar_dragging());
    dispatch_script_button(&mut visual, &view, "Recreate source");
    settle(&mut visual, Step::Tick, 8);
    assert!(
        !old.is_scrollbar_dragging(),
        "changing alignment cancels the old native source's height freeze even under the same collection key"
    );
    let current = visual.update(|_, cx| view.collection_scroll_handle(&id, cx).unwrap().unwrap());
    assert!(!current.is_scrollbar_dragging());
}

#[gpui::test]
fn deferred_follow_notification_is_dropped_after_same_parent_collection_unmount(
    cx: &mut TestAppContext,
) {
    use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    struct Host {
        host: gpui_rhai::ScriptViewHost,
        view: ScriptViewHandle,
        armed: Rc<Cell<bool>>,
    }
    impl Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let view = self.view.clone();
            let armed = Rc::clone(&self.armed);
            self.host.container(
                gpui::div()
                    .child(
                        gpui::canvas(
                            move |_, window, cx| {
                                if !armed.replace(true) {
                                    window.defer(cx, move |window, cx| {
                                        view.automate(
                                            gpui_rhai::AutomationCommand::Action {
                                                id: "test.hide".to_owned(),
                                                payload: None,
                                            },
                                            window,
                                            cx,
                                        )
                                        .unwrap();
                                    });
                                }
                            },
                            |_, (), _, _| {},
                        )
                        .absolute()
                        .w(px(1.0))
                        .h(px(1.0)),
                    )
                    .child(self.view.element().unwrap()),
            )
        }
    }
    let source = r#"
fn state_schema(){#{fields:#{show:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}},
    notified:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn hide(ctx,value){ctx.set_state("show",false);}
fn changed(ctx,status){ctx.set_state("notified",ctx.get_state("notified")+1);}
fn init(ctx){ctx.register_action("test.hide",Fn("hide"));}
fn item(ctx,payload){text("Item").with_style(style().height(px(30)))}
fn view(ctx){let children=[text(`Notified ${ctx.get_state("notified")}`)];
    if ctx.get_state("show"){children.push(virtual_collection(#{key:"events",label:"Events",data:[#{key:"one"}],
        estimated_height:30,height:90,follow_tail:true,on_follow_change:Fn("changed")},Fn("item")));}
    column(children).with_style(style().width(px(300))) }
"#;
    let prepared = Mount::new(source).prepare().unwrap();
    let capture = Rc::new(RefCell::new(None));
    let slot = Rc::clone(&capture);
    let window = cx.add_window(move |window, cx| {
        let host = gpui_rhai::ScriptViewHost::new("follow-owner-expiry", cx).unwrap();
        let view = prepared
            .mount(
                gpui_rhai::ScriptViewConfig::new("follow-owner-expiry"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *slot.borrow_mut() = Some(view.clone());
        Host {
            host,
            view,
            armed: Rc::new(Cell::new(false)),
        }
    });
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = capture.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Tick, 10);
    assert!(last_error(&mut visual, &view).is_none());
    assert!(
        crate::support::palette_texts(&mut visual, &view).contains(&"Notified 0".to_owned()),
        "an obsolete collection cannot deliver into a still-mounted caller component"
    );
}
