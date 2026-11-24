use std::time::{Duration, Instant};

use gpui::{Modifiers, TestAppContext, point, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, UiValue};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Mount, bounds, center, filled_including_rounded_clips, snapshot};

const MONTH_INFERENCE: &str = "Inference, $8,420, 60%";
const MONTH_STORAGE: &str = "Storage, $1,740, 12%";
const WEEK_INFERENCE: &str = "Inference, $642.5, 41%";
const WEEK_STORAGE: &str = "Storage, $720, 46%";

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn compact_breakdown_delta_uses_the_signed_surface_and_an_isolated_affine_entry(
    cx: &mut TestAppContext,
) {
    use crate::support::{live_fills, walk_live_paint};
    use gpui::{Background, TransformationMatrix, rgba};
    let clock = ManualRuntimeClock::new(Instant::now());
    let source = r#"
        import "patterns/compact_breakdown" as cost;
        fn view(ctx) {
            cost::CompactBreakdown(#{key:"delta",series:"costs",revision:0,delta:-6.8,
                items:[#{id:"work",label:"Work",value:85.0}]})
        }
    "#;
    let driver = clock.clock();
    let (mut v, view) = Mount::new(source)
        .registry("patterns/compact_breakdown")
        .theme(gpui_rhai_registry::CLEAN_DARK_THEME)
        .motion(MotionPreference::Normal)
        .with(move |view| view.runtime_clock(driver))
        .mount(cx);
    v.simulate_resize(size(px(520.), px(440.)));
    ready(&mut v, &view);
    clock.advance(Duration::from_millis(50));
    ready(&mut v, &view);
    let middle = bounds(&mut v, &view, "text", "-6.8%");
    let tint: Background = rgba(0xff6b5f24).into();
    let group_alpha = v
        .update(|window, _| {
            window.visual_probe_with_painted_scene(|scene| {
                let mut alpha = None;
                walk_live_paint(scene, TransformationMatrix::default(), &mut |scene, _| {
                    for group in &scene.isolated_groups {
                        if group
                            .source()
                            .quads
                            .iter()
                            .any(|quad| quad.background == tint)
                        {
                            alpha = Some(group.opacity());
                        }
                    }
                });
                alpha
            })
        })
        .expect("the signed badge paints into one isolated group");
    assert!(group_alpha > 0.05 && group_alpha < 0.9, "{group_alpha}");
    clock.advance(Duration::from_millis(420));
    ready(&mut v, &view);
    let final_line = bounds(&mut v, &view, "text", "-6.8%");
    assert!(
        middle.width < final_line.width && middle.width > final_line.width * 0.84,
        "{middle:?} -> {final_line:?}"
    );
    assert!(
        live_fills(&mut v, 0xff6b5f24)
            .into_iter()
            .any(|quad| (quad.height - 20.).abs() <= 0.5
                && f64::from(quad.x) < final_line.x
                && f64::from(quad.right()) >= final_line.x + final_line.width)
    );
}

#[gpui::test]
fn compact_breakdown_narrow_caption_preserves_the_painted_share(cx: &mut TestAppContext) {
    use crate::support::glyph_rects_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("compact_breakdown_caption"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    const LABEL: &str = "WWWWWWWWWWWWWWWWWW";
    let script = format!(
        r#"
import "patterns/compact_breakdown" as breakdown;
fn view(ctx){{
    breakdown::CompactBreakdown(#{{key:"cap",series:"usage",revision:0,
        items:[#{{id:"all",label:"{LABEL}",value:10.0}}]}})
}}
"#
    );
    let (mut v, view) = Mount::new(&script)
        .registry("patterns/compact_breakdown")
        .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
        .motion(MotionPreference::None)
        .mount(&mut real);
    let caption = format!("{LABEL} · 100% of total");
    let row = format!("{LABEL}, 10, 100%");
    let measure = |v: &mut gpui::VisualTestContext, width| {
        v.simulate_resize(size(px(width), px(420.)));
        ready(v, &view);
        let target = center(bounds(v, &view, "listitem", &row));
        v.simulate_mouse_move(target, None, Modifiers::none());
        ready(v, &view);
        let region = bounds(v, &view, "text", &caption);
        let mut ink = glyph_rects_with_ink(v, region, 0x8f8f8fff);
        ink.sort_by(|a, b| a.x.total_cmp(&b.x));
        (region, ink)
    };
    let (wide, full) = measure(&mut v, 378.);
    let (narrow, truncated) = measure(&mut v, 198.);
    assert!(narrow.width <= wide.width - 70., "{wide:?} {narrow:?}");
    assert!(
        truncated.len() < full.len(),
        "name should visually truncate"
    );
    // The last ten glyphs are inside the share phrase, not in the name.
    assert!(truncated.len() >= 10, "share phrase must remain visible");
    for (wide_glyph, narrow_glyph) in full.iter().rev().zip(truncated.iter().rev()).take(10) {
        assert!(
            (wide_glyph.width - narrow_glyph.width).abs() <= 1.
                && (wide_glyph.height - narrow_glyph.height).abs() <= 1.,
            "share was clipped or replaced: {wide_glyph:?} {narrow_glyph:?}",
        );
    }
}

#[gpui::test]
fn compact_breakdown_native_flex_distributes_gaps_and_keeps_a_small_segment(
    cx: &mut TestAppContext,
) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let script = r#"
import "patterns/compact_breakdown" as breakdown;
fn view(ctx){
    breakdown::CompactBreakdown(#{key:"test",series:"counts",revision:0,title:"Measured categories",
        items:[#{id:"main",label:"Major",value:998.9,color:0x9b44da},
               #{id:"tiny",label:"Tiny",value:1.0,color:0xdd8844},
               #{id:"dust",label:"Dust",value:0.1,color:0x19cd8c},
               #{id:"empty",label:"Zero",value:0.0,color:0xfd90ee}]})
}
"#;
    let bound_clock = clock.clock();
    let (mut v, view) = Mount::new(script)
        .registry("patterns/compact_breakdown")
        .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
        .motion(MotionPreference::Normal)
        .with(move |view| view.runtime_clock(bound_clock))
        .mount(cx);
    v.simulate_resize(size(px(315.), px(540.)));
    ready(&mut v, &view);
    let major = |v: &mut gpui::VisualTestContext| {
        filled_including_rounded_clips(v, 0x9b44daff)
            .into_iter()
            .map(|q| q.visible())
            .find(|q| q.width > 100. && (q.height - 8.).abs() <= 0.5)
    };
    assert!(
        major(&mut v).is_none(),
        "zero motion sample doesn't lay out a slot"
    );
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    clock.advance(Duration::from_millis(680));
    ready(&mut v, &view);
    let first = major(&mut v).expect("native motion must lay out the positive lane");
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty(),
        "a frame change must not rerun the script"
    );
    for width in [315., 235., 315.] {
        v.simulate_resize(size(px(width), px(540.)));
        ready(&mut v, &view);
        let large = major(&mut v).unwrap();
        let tiny = filled_including_rounded_clips(&mut v, 0xdd8844ff)
            .into_iter()
            .map(|q| q.visible())
            .find(|q| (q.y - large.y).abs() <= 0.5 && (q.height - 8.).abs() <= 0.5)
            .expect("0.1% remains visible");
        assert!((tiny.width - 2.).abs() <= 0.5, "{width}: {tiny:?}");
        assert!(
            (tiny.x - large.right() - 2.).abs() <= 0.5,
            "{width}: {large:?} {tiny:?}"
        );
        for absent in [0x19cd8cff, 0xfd90eeff] {
            assert!(
                filled_including_rounded_clips(&mut v, absent)
                    .into_iter()
                    .all(|q| (q.bounds.y - large.y).abs() > 0.5)
            );
        }
        let card = bounds(&mut v, &view, "group", "Measured categories");
        assert!(
            (card.x + card.width - f64::from(tiny.right()) - (f64::from(first.x) - card.x)).abs()
                <= 1.,
            "{width}: {card:?} {tiny:?}"
        );
    }
}

#[gpui::test]
fn compact_breakdown_period_refusal_then_acceptance_and_keyboard(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "compact_breakdown", "modern-light");
    visual.simulate_resize(size(px(1240.), px(1000.)));
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "radio", "Month").checked,
        Some(UiValue::Bool(true))
    );
    let original = bounds(&mut visual, &view, "listitem", MONTH_INFERENCE);
    assert!(original.y < bounds(&mut visual, &view, "listitem", MONTH_STORAGE).y);
    let segment = filled_including_rounded_clips(&mut visual, 0x4fb0_feff)
        .into_iter()
        .map(|quad| quad.bounds)
        .find(|b| b.width > 140. && b.height == 8.)
        .expect("the native stacked bar paints the largest segment");
    visual.simulate_mouse_move(
        point(px(segment.x + segment.width / 2.), px(segment.y + 4.)),
        None,
        Modifiers::none(),
    );
    ready(&mut visual, &view);
    node(&mut visual, &view, "text", "Inference · 60% of total");
    click(&mut visual, &view, "button", "Refuse next period");
    click(&mut visual, &view, "radio", "Week");
    node(&mut visual, &view, "status", "Waiting for the Host…");
    let focus = visual.update(|window, cx| window.focused(cx)).unwrap();
    assert_eq!(
        bounds(&mut visual, &view, "listitem", MONTH_INFERENCE),
        original
    );
    visual.executor().advance_clock(Duration::from_millis(350));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The Host refused the period change.",
    );
    assert_eq!(
        visual.update(|window, cx| window.focused(cx)),
        Some(focus.clone())
    );
    assert_eq!(
        node(&mut visual, &view, "radio", "Month").checked,
        Some(UiValue::Bool(true))
    );
    visual.simulate_keystrokes("space");
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Waiting for the Host…");
    visual.executor().advance_clock(Duration::from_millis(350));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted period: week");
    assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(focus));
    assert_eq!(
        node(&mut visual, &view, "radio", "Week").checked,
        Some(UiValue::Bool(true))
    );
    visual.executor().advance_clock(Duration::from_millis(850));
    ready(&mut visual, &view);
    assert!(
        bounds(&mut visual, &view, "listitem", WEEK_STORAGE).y
            < bounds(&mut visual, &view, "listitem", WEEK_INFERENCE).y
    );
    visual.simulate_keystrokes("end");
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Waiting for the Host…");
    visual.executor().advance_clock(Duration::from_millis(350));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted period: year");
}

#[gpui::test]
fn compact_breakdown_hover_peeks_and_narrow_keeps_accessible_shares(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "compact_breakdown", "clean-dark");
    visual.simulate_resize(size(px(1240.), px(1000.)));
    ready(&mut visual, &view);
    let period = bounds(&mut visual, &view, "radio", "Month");
    let inference = bounds(&mut visual, &view, "listitem", MONTH_INFERENCE);
    visual.simulate_mouse_move(center(inference), None, Modifiers::none());
    ready(&mut visual, &view);
    node(&mut visual, &view, "text", "Inference · 60% of total");
    assert_eq!(
        bounds(&mut visual, &view, "listitem", MONTH_INFERENCE),
        inference
    );
    visual.simulate_mouse_move(center(period), None, Modifiers::none());
    ready(&mut visual, &view);
    node(&mut visual, &view, "text", "Total spending");
    click(&mut visual, &view, "button", "Narrow card");
    node(&mut visual, &view, "listitem", MONTH_INFERENCE);
    node(&mut visual, &view, "listitem", "Archive, $40, <1%");
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("text", "60%")
            .next()
            .is_none(),
        "the narrow layout must not allocate a visual column for shares"
    );
    assert!(bounds(&mut visual, &view, "listitem", MONTH_INFERENCE).width < inference.width);
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
