//! Repainting is not contract invalidation: native gestures survive layout and
//! unrelated script work, but cannot submit captured controlled/constraint or
//! callback inputs after a successful transaction replaces them.

use gpui::{Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{
    AutomationCommand, ExecutionOperation, ManualRuntimeClock, MotionPreference, ScriptViewHandle,
};

use crate::support::{
    Mount, ReplaceOnce, Step, bounds, by_test_id, dispatch, last_error, settle, text_of,
};

const SPLIT: &str = r#"
import "components/split_pane" as split;
fn state_schema(){#{fields:#{
    ratio:#{schema:#{type:"number"},"default":#{type:"float",value:0.5}},
    commits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    stamp:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    max:#{schema:#{type:"number"},"default":#{type:"float",value:16384.0}},
    disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    alternate:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    show:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}
}}}
fn resized(ctx,p){ctx.set_state("ratio",p);ctx.set_state("commits",ctx.get_state("commits")+1);}
fn other_resize(ctx,p){ctx.set_state("commits",ctx.get_state("commits")+100);}
fn changed(ctx,p){
    switch p {
        "unrelated"=>ctx.set_state("stamp",ctx.get_state("stamp")+1),
        "source"=>ctx.set_state("ratio",0.25),
        "constraint"=>ctx.set_state("max",150.0),
        "disabled"=>ctx.set_state("disabled",true),
        "callback"=>ctx.set_state("alternate",true),
        "unmount"=>ctx.set_state("show",false),
        "reject"=>{ctx.set_state("ratio",0.25);throw "rejected source update";},
    }
}
fn view(ctx){
    let children=[];
    for change in ["unrelated","source","constraint","disabled","callback","unmount","reject"] {
        children.push(text(change).test_id(change).on_click_value(Fn("changed"),change));
    }
    children.push(text(`${ctx.get_state("commits")}`).test_id("commits"));
    children.push(text(`${ctx.get_state("stamp")}`).test_id("stamp"));
    if ctx.get_state("show") {
        children.push(split::SplitPane(#{key:"split",label:"Resize panels",orientation:"horizontal",
            size:ctx.get_state("ratio"),min_start:0.0,min_end:0.0,max_start:ctx.get_state("max"),
            disabled:ctx.get_state("disabled"),start:text("A"),end:text("B"),
            handle:box([]),on_resize:if ctx.get_state("alternate"){Fn("other_resize")}else{Fn("resized")},
            part_styles:#{handle:style().width(px(8))}})
            .with_style(style().width(px(420)).height(px(180))));
    }
    column(children)
}
"#;

#[allow(clippy::cast_possible_truncation)]
fn press_split(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> (Point<Pixels>, f64) {
    settle(visual, Step::Tick, 6);
    let before = bounds(visual, view, "separator", "Resize panels");
    let at = point(
        px((before.x + before.width / 2.0) as f32),
        px((before.y + before.height / 2.0 + 40.0) as f32),
    );
    visual.simulate_mouse_move(at, None, Modifiers::none());
    settle(visual, Step::Tick, 4);
    visual.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    settle(visual, Step::Tick, 4);
    (at, before.x)
}

fn move_split(visual: &mut VisualTestContext, at: Point<Pixels>, delta: f32) {
    visual.simulate_mouse_move(
        at + point(px(delta), px(0.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    settle(visual, Step::Tick, 4);
}

fn release_split(visual: &mut VisualTestContext, at: Point<Pixels>, delta: f32) {
    visual.simulate_mouse_up(
        at + point(px(delta), px(0.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    settle(visual, Step::Tick, 4);
}

#[gpui::test]
fn split_grip_continues_across_presented_frames_with_thin_or_default_separator(
    cx: &mut TestAppContext,
) {
    for thickness in [1, 8, 16] {
        let source = SPLIT.replace_once("width(px(8))", &format!("width(px({thickness}))"));
        let (mut visual, view) = Mount::components(source)
            .motion(MotionPreference::None)
            .id(&format!("contract-split-{thickness}"))
            .mount(cx);
        let (at, before) = press_split(&mut visual, &view);
        for delta in [20.0_f32, 40.0, 60.0] {
            move_split(&mut visual, at, delta);
            let actual = bounds(&mut visual, &view, "separator", "Resize panels").x - before;
            assert!(
                (actual - f64::from(delta)).abs() < 1.0,
                "{thickness}px: {actual} != {delta}"
            );
        }
        release_split(&mut visual, at, 60.0);
        assert_eq!(text_of(&mut visual, &view, "commits"), "1");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn unrelated_script_state_update_keeps_active_native_preview(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(SPLIT).id("contract-unrelated").mount(cx);
    let (at, before) = press_split(&mut visual, &view);
    move_split(&mut visual, at, 20.0);
    dispatch(&mut visual, &view, by_test_id("unrelated"), "click").unwrap();
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(text_of(&mut visual, &view, "stamp"), "1");
    move_split(&mut visual, at, 60.0);
    assert!(
        (bounds(&mut visual, &view, "separator", "Resize panels").x - before - 60.0).abs() < 1.0
    );
    release_split(&mut visual, at, 60.0);
    assert_eq!(text_of(&mut visual, &view, "commits"), "1");
}

#[gpui::test]
fn rolled_back_source_update_does_not_cancel_active_native_preview(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(SPLIT).id("contract-rollback").mount(cx);
    let (at, _) = press_split(&mut visual, &view);
    move_split(&mut visual, at, 20.0);
    assert!(dispatch(&mut visual, &view, by_test_id("reject"), "click").is_err());
    settle(&mut visual, Step::Tick, 4);
    move_split(&mut visual, at, 60.0);
    release_split(&mut visual, at, 60.0);
    assert_eq!(text_of(&mut visual, &view, "commits"), "1");
}

#[gpui::test]
fn accepted_behavior_or_forwarded_callback_change_cancels_only_the_old_gesture(
    cx: &mut TestAppContext,
) {
    for change in ["source", "constraint", "disabled", "callback", "unmount"] {
        let (mut visual, view) = Mount::components(SPLIT)
            .id(&format!("contract-{change}"))
            .mount(cx);
        let (at, _) = press_split(&mut visual, &view);
        move_split(&mut visual, at, 20.0);
        dispatch(&mut visual, &view, by_test_id(change), "click").unwrap();
        settle(&mut visual, Step::Tick, 4);
        move_split(&mut visual, at, 60.0);
        release_split(&mut visual, at, 60.0);
        assert_eq!(
            text_of(&mut visual, &view, "commits"),
            "0",
            "change: {change}"
        );
        assert_eq!(last_error(&mut visual, &view), None, "change: {change}");
    }
}

const ASYNC_DRAG: &str = r#"
import "components/draggable" as draggable;
fn state_schema(){#{fields:#{
    armed:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    axes:#{schema:#{type:"string"},"default":#{type:"string",value:"both"}},
    stamp:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    commits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
}}}
fn arm(ctx,p){ctx.set_state("armed",true);}
fn update(ctx,p){UPDATE}
fn reject(ctx,p){throw "later async delivery failed";}
fn moved(ctx,p){ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){
    timeout("a-update",10,!ctx.get_state("armed"),Fn("update"),());
    timeout("b-reject",10,!ctx.get_state("armed"),Fn("reject"),());
    column([text("Arm").test_id("arm").on_click(Fn("arm")),
        text(`${ctx.get_state("commits")}`).test_id("commits"),
        text(`${ctx.get_state("axes")}:${ctx.get_state("stamp")}`).test_id("state"),
        draggable::Draggable(#{key:"card",label:"Move card",position:#{x:0,y:0},threshold:0,
            axes:ctx.get_state("axes"),on_move:Fn("moved"),
            content:text("Card").with_style(style().width(px(100)).height(px(50)))})
            .with_style(style().width(px(420)).height(px(180)))])
}
"#;

fn async_drag_probe(cx: &mut TestAppContext, change_axes: bool) {
    let update = if change_axes {
        r#"ctx.set_state("axes","vertical");"#
    } else {
        r#"ctx.set_state("stamp",1);"#
    };
    let source = ASYNC_DRAG.replace_once("UPDATE", update);
    let clock = ManualRuntimeClock::new(std::time::Instant::now()).clock();
    let (mut visual, view) = Mount::components(source)
        .with(move |view| view.runtime_clock(clock))
        .id(&format!("async-contract-{change_axes}"))
        .mount(cx);
    settle(&mut visual, Step::Tick, 4);
    let surface = bounds(&mut visual, &view, "group", "Move card");
    let at = point(px((surface.x + 20.0) as f32), px((surface.y + 20.0) as f32));
    visual.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    move_split(&mut visual, at, 20.0);
    dispatch(&mut visual, &view, by_test_id("arm"), "click").unwrap();
    // The Rhai RuntimeClock is independent of GPUI's executor clock. Keep
    // both deadlines pending until this single foreground poll, so `update`
    // commits and the following `reject` fails in the same async batch.
    let _ = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    let advance = visual.update(|window, cx| {
        view.automate(AutomationCommand::AdvanceTime { millis: 10 }, window, cx)
    });
    assert!(
        advance
            .unwrap_err()
            .to_string()
            .contains("later async delivery failed")
    );
    let performance = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    let updates = performance
        .timings
        .iter()
        .filter(|timing| timing.operation == ExecutionOperation::Callback("update".to_owned()))
        .collect::<Vec<_>>();
    let rejections = performance
        .timings
        .iter()
        .filter(|timing| timing.operation == ExecutionOperation::Callback("reject".to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(
        updates.len(),
        1,
        "update must execute exactly once: {performance:?}"
    );
    assert!(updates[0].succeeded);
    assert_eq!(
        rejections.len(),
        1,
        "reject must execute exactly once: {performance:?}"
    );
    assert!(!rejections[0].succeeded);
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        text_of(&mut visual, &view, "state"),
        if change_axes { "vertical:0" } else { "both:1" }
    );
    move_split(&mut visual, at, 60.0);
    release_split(&mut visual, at, 60.0);
    assert_eq!(
        text_of(&mut visual, &view, "commits"),
        if change_axes { "0" } else { "1" }
    );
}

#[gpui::test]
fn successful_async_behavior_change_survives_later_delivery_rollback(cx: &mut TestAppContext) {
    async_drag_probe(cx, true);
}

#[gpui::test]
fn unrelated_async_commit_and_later_failure_do_not_cancel_gesture(cx: &mut TestAppContext) {
    async_drag_probe(cx, false);
}
