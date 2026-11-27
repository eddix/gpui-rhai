//! An ordinary Button follows its parent immediately; generic position motion stays explicit.

use crate::support::{Mount, Step, bounds, click_center, last_error, settle, snapshot};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{ElementGeometry, ManualRuntimeClock, MotionPreference, ScriptViewHandle};
use std::time::{Duration, Instant};

fn geometry(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> ElementGeometry {
    snapshot(visual, view)
        .find_by_role_and_name(role, name)
        .next()
        .unwrap()
        .geometry
        .unwrap()
}

#[gpui::test]
fn parent_reflow_does_not_turn_button_content_motion_into_a_position_tween(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (mut visual,view)=Mount::components(r#"
            import "components/button" as button;
            fn state_schema(){#{fields:#{moved:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
            fn noop(ctx,value){}
            fn moved(ctx,value){ctx.set_state("moved",true);}
            fn view(ctx){column([
                button::Button(#{key:"action",text:"Action",on_click:Fn("noop")})
                    .with_style(style().width(px(160))),
                box([]).with_key("explicit-motion").accessibility_role("group").accessibility_label("Explicit position motion")
                    .with_style(style().width(px(160)).height(px(40)).background(theme_color("accent")))
                    .layout_motion(480,"linear"),
                text("Move").accessibility_role("button").accessibility_label("Move").on_click(Fn("moved"))
            ]).with_style(style().items_start().width(px(600)).gap(px(8)).padding_y(px(20))
                .padding_x(px(if ctx.get_state("moved"){64}else{32})))}
        "#).theme(theme).motion(MotionPreference::Normal)
            .with(move|view|view.runtime_clock(clock)).id("button-reflow").mount(cx);
        settle(&mut visual, Step::Refresh, 3);
        let before = geometry(&mut visual, &view, "button", "Action");
        let trigger = bounds(&mut visual, &view, "button", "Move");
        click_center(&mut visual, trigger);
        settle(&mut visual, Step::Refresh, 3);
        let after = geometry(&mut visual, &view, "button", "Action");
        assert!(
            (after.layout.x - before.layout.x - 32.0).abs() < 0.5,
            "{name}: parent must actually move, {before:?}, {after:?}"
        );
        assert_eq!(
            after.visual.x, after.layout.x,
            "{name}: unchanged button content must not lag its parent"
        );
        let explicit = geometry(&mut visual, &view, "group", "Explicit position motion");
        assert!(
            (explicit.visual.x - before.visual.x).abs() < 0.5
                && explicit.layout.x > explicit.visual.x + 31.0,
            "{name}: explicit generic position motion must still run, {explicit:?}"
        );
        visual
            .update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap();
        manual.advance(Duration::from_millis(240));
        settle(&mut visual, Step::Refresh, 3);
        let middle = geometry(&mut visual, &view, "group", "Explicit position motion");
        assert!(
            (middle.visual.x - before.visual.x - 16.0).abs() < 0.5,
            "{name}: {middle:?}"
        );
        let timings = visual
            .update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings;
        assert!(
            timings.is_empty(),
            "native position sampling must not run Rhai: {timings:?}"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn layout_size_motion_reflows_siblings_on_both_axes(cx: &mut TestAppContext) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::components(r#"
        fn state_schema(){#{fields:#{expanded:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn toggle(ctx,value){ctx.set_state("expanded",!ctx.get_state("expanded"));}
        fn view(ctx){
            let large=ctx.get_state("expanded");
            let marker=large.to_string();
            column([
                row([
                    box([]).with_key("width").accessibility_role("group").accessibility_label("Width")
                        .with_style(style().width(relative(if large{0.60}else{0.25})).max_width(px(240))
                            .height(px(40)).flex_shrink(false)).layout_size_motion(marker,"width",400,"linear"),
                    text("Horizontal neighbor").accessibility_role("group").accessibility_label("Horizontal neighbor"),
                ]).with_style(style().width(px(600))),
                box([]).with_key("height").accessibility_role("group").accessibility_label("Height")
                    .with_style(style().height(px(if large{160}else{80})).width(px(160)).flex_shrink(false))
                    .layout_size_motion(marker,"height",400,"linear"),
                text("Vertical neighbor").accessibility_role("group").accessibility_label("Vertical neighbor"),
                text("Toggle").accessibility_role("button").on_click(Fn("toggle")),
            ]).with_style(style().items_start());
        }
    "#).motion(MotionPreference::Normal)
        .with(move |view| view.runtime_clock(clock))
        .mount(cx);
    settle(&mut visual, Step::Refresh, 3);
    let h = bounds(&mut visual, &view, "group", "Horizontal neighbor").x;
    let v = bounds(&mut visual, &view, "group", "Vertical neighbor").y;
    let button = bounds(&mut visual, &view, "button", "Toggle");
    click_center(&mut visual, button);
    settle(&mut visual, Step::Refresh, 3);
    let tween = gpui_rhai::EXTENDED_RENDERING;
    let (first_w, first_h) = if tween { (150., 80.) } else { (240., 160.) };
    assert!((bounds(&mut visual, &view, "group", "Width").width - first_w).abs() < 0.5);
    assert!((bounds(&mut visual, &view, "group", "Height").height - first_h).abs() < 0.5);
    manual.advance(Duration::from_millis(200));
    settle(&mut visual, Step::Refresh, 3);
    let mid_w = bounds(&mut visual, &view, "group", "Width").width;
    let mid_h = bounds(&mut visual, &view, "group", "Height").height;
    let (expected_w, expected_h) = if tween { (195., 120.) } else { (240., 160.) };
    assert!(
        (mid_w - expected_w).abs() < 0.5 && (mid_h - expected_h).abs() < 0.5,
        "{mid_w} {mid_h}"
    );
    assert!(
        (bounds(&mut visual, &view, "group", "Horizontal neighbor").x - h - (expected_w - 150.))
            .abs()
            < 0.5
    );
    assert!(
        (bounds(&mut visual, &view, "group", "Vertical neighbor").y - v - (expected_h - 80.)).abs()
            < 0.5
    );
    assert_eq!(last_error(&mut visual, &view), None);
}
