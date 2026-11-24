use std::time::{Duration, Instant};

use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, Mount, bounds, center, snapshot};

#[gpui::test]
#[cfg(target_os = "macos")]
fn fallback_chain_endpoint_ink_tracks_the_accepted_result(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("fallback_endpoint_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut native, "fallback_chain", "modern-light");
    v.simulate_resize(size(px(1380.), px(1080.)));
    for (design, appearance, pending, answered, failed) in [
        ("Modern", "Light", 0x8a8a8aff, 0x1e8a4fff, 0xb25e00ff),
        ("Modern", "Dark", 0x8a8a8aff, 0x5fc88aff, 0xffb35cff),
        ("Clean", "Light", 0x00000073, 0x1e8a4fff, 0xc4320aff),
        ("Clean", "Dark", 0xffffff73, 0x5fc88aff, 0xff8a7aff),
    ] {
        v.update(|_, cx| view.select_theme(design, appearance, cx))
            .unwrap();
        click(&mut v, &view, "button", "New request");
        let endpoint = node(&mut v, &view, "group", "Response").id;
        let waiting = bounds(&mut v, &view, "group", "Response");
        assert!(
            crate::support::glyphs_with_ink(&mut v, waiting, pending) >= 5,
            "{design}/{appearance}: pending stays neutral"
        );
        click(&mut v, &view, "button", "Deliver answer");
        assert_eq!(node(&mut v, &view, "group", "Response").id, endpoint);
        let result = bounds(&mut v, &view, "group", "Response");
        assert!(
            crate::support::glyphs_with_ink(&mut v, result, answered) >= 5,
            "{design}/{appearance}: answered subtitle must paint success ink"
        );
        click(&mut v, &view, "button", "Fail the chain");
        assert_eq!(node(&mut v, &view, "group", "No answer").id, endpoint);
        let failed_endpoint = bounds(&mut v, &view, "group", "No answer");
        assert!(
            crate::support::glyphs_with_ink(&mut v, failed_endpoint, failed) >= 5,
            "{design}/{appearance}: failed subtitle must paint failure ink"
        );
    }
}

#[gpui::test]
fn fallback_chain_progress_fits_the_native_strip_and_preserves_the_fraction(
    cx: &mut TestAppContext,
) {
    use super::acceptance::action;
    use crate::support::paint::live_painted_quads;
    use gpui_rhai::UiValue;
    let (mut v, view) = gallery(cx, "fallback_chain", "modern-dark");
    v.simulate_resize(size(px(1380.), px(1080.)));
    ready(&mut v, &view);
    for narrow in [false, true] {
        if narrow {
            click(&mut v, &view, "button", "Narrow route");
        }
        for (scale, factor) in [("100", 1.), ("150", 1.5)] {
            action(
                &mut v,
                &view,
                "gallery.set_scale",
                UiValue::String(scale.into()),
            );
            ready(&mut v, &view);
            let rules = gpui_rhai::AuditRules::from_ids(["mixed-type-in-row"]).unwrap();
            let findings = v.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
            assert!(
                !findings
                    .iter()
                    .any(|finding| finding.path.contains("/attempt-content")),
                "model, provider and bill roles at {scale}% (narrow: {narrow}): {findings:#?}"
            );
            let card = bounds(&mut v, &view, "group", "Nova Mini");
            let chip = 5. * factor;
            let gap = 2. * factor;
            let left = card.x + 46. * factor;
            let right = card.x + card.width - 12. * factor;
            let expected = ((right - left + gap) / (chip + gap) + 0.001).floor();
            let mut dots = live_painted_quads(&mut v)
                .into_iter()
                .filter_map(|q| {
                    let rgb = q.background.as_solid()?.to_rgb();
                    let active = rgb.b > 0.95
                        && (rgb.g - 176. / 255.).abs() < 0.012
                        && (rgb.r - 79. / 255.).abs() < 0.012;
                    let idle = (rgb.r - 44. / 255.).abs() < 0.012
                        && (rgb.g - 44. / 255.).abs() < 0.012
                        && (rgb.b - 48. / 255.).abs() < 0.012;
                    let rect = q.bounds;
                    ((active || idle)
                        && (f64::from(rect.height) - chip).abs() <= 0.55
                        && (f64::from(rect.width) - chip).abs() <= 0.55
                        && f64::from(rect.y) >= card.y + card.height * 0.5
                        && f64::from(rect.bottom()) <= card.y + card.height + 0.5
                        && f64::from(rect.x) >= left - 0.5
                        && q.visible().width > 0.)
                        .then_some((rect, q.mask, active))
                })
                .collect::<Vec<_>>();
            dots.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
            assert!(
                (f64::from(u32::try_from(dots.len()).unwrap()) - expected).abs() < 0.1,
                "{narrow}/{scale}: {card:?}, {left}..{right}: {dots:#?}"
            );
            assert_eq!(
                dots.iter().filter(|x| x.2).count(),
                dots.len().div_ceil(2),
                "the Host's 2 s of 4 s stays half filled at {narrow}/{scale}"
            );
            assert!(
                dots.iter().all(
                    |(dot, clip, _)| dot.right() <= clip.right() + 0.5 && dot.x >= clip.x - 0.5
                ),
                "{narrow}/{scale}: clipped block"
            );
            assert!(
                (f64::from(dots.last().unwrap().0.right()) - right).abs() <= 0.5,
                "{narrow}/{scale}: the track ends at its own content edge"
            );
        }
    }
}

#[gpui::test]
#[cfg(feature = "vendor-gpui")]
fn fallback_chain_running_numbers_sweep_then_rest_without_advancing_the_host(
    cx: &mut TestAppContext,
) {
    use super::collaboration_stream::glyph_band_center;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("fallback_glyph_sweep"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (id, preference) in [
        ("normal", MotionPreference::Normal),
        ("reduced", MotionPreference::Reduced),
        ("disabled", MotionPreference::None),
    ] {
        let clock = ManualRuntimeClock::new(Instant::now());
        let prepared = acceptance::view(&AcceptanceLaunch {
            page: "fallback_chain".into(),
            theme: "modern-dark".into(),
            ..Default::default()
        })
        .unwrap()
        .runtime_clock(clock.clock())
        .motion_preference(preference)
        .prepare()
        .unwrap();
        let (window, view) = super::mount_prepared(
            &mut native,
            prepared,
            (id, "gallery"),
            false,
            |host, view| Host { host, view },
        );
        let mut v = VisualTestContext::from_window(*window, &native);
        v.update(|w, _| w.activate_window());
        v.simulate_resize(size(px(1380.), px(1050.)));
        ready(&mut v, &view);
        let sample = |v: &mut VisualTestContext| {
            v.update(|w, _| w.visual_probe_with_painted_scene(glyph_band_center))
        };
        let original = node(&mut v, &view, "group", "Nova Mini");
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap();
        clock.advance(Duration::from_millis(550));
        ready(&mut v, &view);
        let left = sample(&mut v);
        clock.advance(Duration::from_millis(290));
        ready(&mut v, &view);
        let right = sample(&mut v);
        if preference == MotionPreference::Normal {
            assert!(right.unwrap() > left.unwrap() + 5., "{left:?} -> {right:?}");
        } else {
            assert_eq!((left, right), (None, None));
        }
        node(&mut v, &view, "status", "Attempt 2 of 3 · 3.8s · $0.010");
        clock.advance(Duration::from_millis(820));
        ready(&mut v, &view);
        assert_eq!(
            sample(&mut v),
            None,
            "rest portion of the same native cycle"
        );
        assert!(
            v.update(|_, cx| view.take_performance_snapshot(cx))
                .unwrap()
                .timings
                .is_empty()
        );
        click(&mut v, &view, "button", "Deliver answer");
        assert_eq!(node(&mut v, &view, "group", "Nova Mini").id, original.id);
        clock.advance(Duration::from_millis(1500));
        ready(&mut v, &view);
        assert_eq!(sample(&mut v), None, "accepted answer ends the sweep");
        node(&mut v, &view, "status", "Answered · 7.5s · $0.013");
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn fallback_chain_running_summary_uses_muted_text_not_the_dot_accent(cx: &mut TestAppContext) {
    use crate::support::glyphs_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("fallback_summary_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "fallback_chain", "modern-light");
    v.simulate_resize(size(px(1290.), px(940.)));
    ready(&mut v, &view);
    let label = bounds(&mut v, &view, "text", "Attempt 2 of 3");
    assert!(glyphs_with_ink(&mut v, label, 0x8a8a8aff) > 6);
    assert_eq!(glyphs_with_ink(&mut v, label, 0x4fb0feff), 0);
}

#[gpui::test]
fn fallback_chain_shows_live_costs_and_waits_for_an_accepted_answer(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "fallback_chain", "modern-light");
    v.simulate_resize(size(px(1290.), px(940.)));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Attempt 2 of 3 · 3.8s · $0.010");
    assert!(
        node(&mut v, &view, "group", "Atlas Pro")
            .description
            .contains("Rate limited · 429")
    );
    assert!(
        node(&mut v, &view, "group", "Sprout")
            .description
            .starts_with("Up next")
    );
    let end = bounds(&mut v, &view, "group", "Response");
    v.simulate_mouse_move(center(end), None, Modifiers::none());
    v.executor().advance_clock(Duration::from_millis(250));
    ready(&mut v, &view);
    node(&mut v, &view, "group", "Response details");
    assert!(
        !snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.name.contains("Your request has been reviewed"))
    );
    click(&mut v, &view, "button", "Advance clock");
    node(&mut v, &view, "status", "Attempt 2 of 3 · 9.8s · $0.010");
    assert!(
        node(&mut v, &view, "group", "Nova Mini")
            .description
            .starts_with("Trying")
    );
    click(&mut v, &view, "button", "Deliver answer");
    node(&mut v, &view, "status", "Answered · 7.5s · $0.013");
    assert_eq!(
        node(&mut v, &view, "group", "Response").description,
        "412 tokens"
    );
    let end = bounds(&mut v, &view, "group", "Response");
    v.simulate_mouse_move(center(end), None, Modifiers::none());
    v.executor().advance_clock(Duration::from_millis(250));
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.name.contains("Your request has been reviewed"))
    );
    click(&mut v, &view, "button", "New request");
    v.simulate_keystrokes("tab");
    v.executor().advance_clock(Duration::from_millis(250));
    ready(&mut v, &view);
    node(&mut v, &view, "group", "Request details");
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert!(
        !snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.name == "Request details")
    );
}

#[gpui::test]
fn fallback_chain_retry_needs_host_acceptance_and_the_narrow_tree_keeps_response(
    cx: &mut TestAppContext,
) {
    for theme in ["modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "fallback_chain", theme);
        v.simulate_resize(size(px(1280.), px(940.)));
        ready(&mut v, &view);
        let source = node(&mut v, &view, "group", "Nova Mini");
        let wide = bounds(&mut v, &view, "group", "Fallback route");
        click(&mut v, &view, "button", "Narrow route");
        let narrow = bounds(&mut v, &view, "group", "Fallback route");
        assert!(narrow.width < wide.width && narrow.height > wide.height);
        assert_eq!(node(&mut v, &view, "group", "Nova Mini").id, source.id);
        assert!(
            bounds(&mut v, &view, "group", "Response").y
                > bounds(&mut v, &view, "group", "Nova Mini").y
        );
        click(&mut v, &view, "button", "Fail the chain");
        click(&mut v, &view, "button", "Refuse next retry");
        click(&mut v, &view, "button", "Retry");
        node(&mut v, &view, "status", "The Host declined the retry");
        node(&mut v, &view, "status", "No model answered · 7.5s · $0.010");
        v.simulate_keystrokes("space");
        ready(&mut v, &view);
        node(&mut v, &view, "status", "The Host started a new request");
        node(&mut v, &view, "status", "Attempt 2 of 3 · 3.8s · $0.010");
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn fallback_chain_pulse_uses_native_frames_and_not_a_second_script_clock(cx: &mut TestAppContext) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "fallback_chain".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .runtime_clock(clock.clock())
    .motion_preference(MotionPreference::Normal)
    .prepare()
    .unwrap();
    let (win, view) =
        super::mount_prepared(cx, prepared, ("chain", "gallery"), false, |host, view| {
            Host { host, view }
        });
    let mut v = VisualTestContext::from_window(*win, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1270.), px(940.)));
    ready(&mut v, &view);
    let x = |v: &mut VisualTestContext| {
        fn light(s: &gpui::Scene, scale: f32) -> Option<f32> {
            for path in &s.paths {
                if let Some(color) = path.color.as_solid() {
                    let c = color.to_rgb();
                    let w = path.bounds.size.width.0 / scale;
                    if (4.5..6.0).contains(&w)
                        && (0.26..0.36).contains(&c.r)
                        && (0.65..0.75).contains(&c.g)
                        && c.b > 0.95
                        && c.a > 0.95
                    {
                        return Some(path.bounds.center().x.0 / scale);
                    }
                }
            }
            #[cfg(feature = "gpui-fast")]
            for layer in s.layers.frames.iter().filter(|l| l.tile_size == 0) {
                if let Some(v) = layer.content.scenes().find_map(|s| light(s, scale)) {
                    return Some(v);
                }
            }
            None
        }
        v.update(|win, _| {
            win.visual_probe_with_painted_scene(|scene| light(scene, win.scale_factor()))
        })
        .expect("native attempt pulse head")
    };
    clock.advance(Duration::from_millis(160));
    ready(&mut v, &view);
    let first = x(&mut v);
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    clock.advance(Duration::from_millis(385));
    ready(&mut v, &view);
    let later = x(&mut v);
    assert!(later > first + 4.0, "{first} → {later}");
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
}

#[gpui::test]
fn fallback_chain_24_attempts_fit_an_embedded_mount(cx: &mut TestAppContext) {
    let (mut v,view)=Mount::new(r#"
        import "patterns/fallback_chain" as f;
        fn view(ctx){
            let attempts=[];
            for i in 0..24{
                attempts.push(#{id:"id/"+i.to_string(),model:"Model "+i.to_string(),status:"waiting"});
            }
            f::FallbackChain(#{key:"many",run:"long",attempts:attempts})
        }
    "#).registry("patterns/fallback_chain").motion(MotionPreference::None).mount(cx);
    ready(&mut v, &view);
    assert_eq!(
        snapshot(&mut v, &view)
            .nodes()
            .filter(|n| n.role == "group" && n.name.starts_with("Model "))
            .count(),
        24
    );
    node(&mut v, &view, "status", "Waiting · 0ms · $0");
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
