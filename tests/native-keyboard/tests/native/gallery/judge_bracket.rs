use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, bounds, snapshot};

fn button(v: &mut VisualTestContext, view: &ScriptViewHandle, label: &str) {
    click(v, view, "button", label);
}

#[gpui::test]
fn judge_bracket_answer_metadata_uses_the_quiet_11px_recipe(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("judge_answer_metadata_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut visual, view) = gallery(&mut real, "judge_bracket", "modern-light");
    visual.simulate_resize(size(px(1430.), px(980.)));
    for (family, appearance, ink) in [
        ("Modern", "Light", 0xb8b8b8ff),
        ("Modern", "Dark", 0x4a4a50ff),
        ("Clean", "Light", 0x00000045),
        ("Clean", "Dark", 0xffffff47),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut visual, &view);
        let meta = bounds(&mut visual, &view, "text", "Outline · 3 sections");
        assert!(
            crate::support::glyphs_with_ink(&mut visual, meta, ink) >= 9,
            "{family}/{appearance}: use the answer's faint ink"
        );
        assert!(
            (meta.height - 16.).abs() <= 1.,
            "{family}/{appearance}: answer metadata uses a 16px line box, {meta:?}"
        );
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn active_glyph_ink(scene: &gpui::Scene, positions: &mut Vec<f32>) {
    for group in &scene.isolated_groups {
        let ink = group.glyph_ink();
        if ink[8] == 1. {
            assert!(
                ink[3] > ink[7],
                "the band can reduce opacity, not merely brighten text"
            );
            positions.push(group.transform().translation[0] + group.alpha_ramps()[2]);
        }
        active_glyph_ink(group.source(), positions);
    }
    #[cfg(feature = "gpui-fast")]
    for child in scene
        .layers
        .frames
        .iter()
        .flat_map(|layer| layer.content.scenes())
    {
        active_glyph_ink(child, positions);
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn judge_bracket_replaces_ink_only_in_the_active_match(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    for (key, policy) in [
        ("ink-normal", MotionPreference::Normal),
        ("ink-reduced", MotionPreference::Reduced),
    ] {
        let mut native = TestAppContext::build_with_text_system(
            cx.dispatcher.clone(),
            Some(key),
            std::sync::Arc::new(apple::AppleTextSystem::new()),
        );
        let clock = ManualRuntimeClock::new(Instant::now());
        let prepared = acceptance::view(&AcceptanceLaunch {
            page: "judge_bracket".into(),
            theme: "clean-dark".into(),
            ..Default::default()
        })
        .unwrap()
        .runtime_clock(clock.clock())
        .motion_preference(policy)
        .prepare()
        .unwrap();
        let (win, view) = super::mount_prepared(
            &mut native,
            prepared,
            (key, "gallery"),
            false,
            |host, view| Host { host, view },
        );
        let mut v = VisualTestContext::from_window(*win, &native);
        v.update(|w, _| w.activate_window());
        v.simulate_resize(size(px(1380.), px(1050.)));
        ready(&mut v, &view);
        let first = node(&mut v, &view, "button", "Semifinal 1");
        let positions = |v: &mut VisualTestContext| {
            let mut all = v.update(|w, _| {
                w.visual_probe_with_painted_scene(|scene| {
                    let mut points = Vec::new();
                    active_glyph_ink(scene, &mut points);
                    points
                })
            });
            all.sort_by(f32::total_cmp);
            all
        };
        assert!(positions(&mut v).is_empty());
        // The single-line names occupy the leading portion of a wide flex
        // box. Pick two phases that overlap real glyphs, then a third whose
        // band has moved into the remaining whitespace.
        clock.advance(Duration::from_millis(220));
        ready(&mut v, &view);
        let left = positions(&mut v);
        v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        clock.advance(Duration::from_millis(120));
        ready(&mut v, &view);
        let right = positions(&mut v);
        if policy == MotionPreference::Normal {
            assert_eq!(left.len(), 2);
            assert_eq!(right.len(), 2);
            assert!(left.iter().zip(right).all(|(a, b)| b > a + 10.));
        } else {
            assert!(left.is_empty() && right.is_empty());
        }
        clock.advance(Duration::from_millis(195));
        ready(&mut v, &view);
        assert!(
            positions(&mut v).is_empty(),
            "a band in whitespace needs no glyph capture"
        );
        let same = node(&mut v, &view, "button", "Semifinal 1");
        assert_eq!((first.id, first.geometry), (same.id, same.geometry));
        assert!(
            v.update(|_, cx| view.take_performance_snapshot(cx).unwrap())
                .timings
                .is_empty()
        );
        button(&mut v, &view, "Record next verdict");
        assert!(
            positions(&mut v).is_empty(),
            "previous band stops; the next match starts outside its text"
        );
    }
}

fn no_champion(v: &mut VisualTestContext, view: &ScriptViewHandle) {
    assert!(
        snapshot(v, view)
            .find_by_role_and_name("button", "Use Diagram draft")
            .next()
            .is_none()
    );
}

#[gpui::test]
fn judge_bracket_uses_accepted_verdicts_and_only_proposes_the_choice(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "judge_bracket", "clean-dark");
    v.simulate_resize(size(px(1380.), px(1050.)));
    ready(&mut v, &view);
    button(&mut v, &view, "Final");
    assert!(
        node(&mut v, &view, "button", "Final")
            .description
            .contains("Waiting for an earlier match")
    );
    no_champion(&mut v, &view);
    for _ in 0..2 {
        button(&mut v, &view, "Record next verdict");
        no_champion(&mut v, &view);
    }
    button(&mut v, &view, "Record next verdict");
    node(&mut v, &view, "status", "Diagram draft wins");
    node(&mut v, &view, "document", "Answer of Diagram draft");
    button(&mut v, &view, "Refuse next choice");
    button(&mut v, &view, "Use Diagram draft");
    node(&mut v, &view, "status", "Choice proposed: Diagram draft");
    assert!(node(&mut v, &view, "button", "Use Diagram draft").disabled);
    v.executor().advance_clock(Duration::from_millis(600));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Choice declined by Host");
    assert!(!node(&mut v, &view, "button", "Use Diagram draft").disabled);
    button(&mut v, &view, "Use Diagram draft");
    v.executor().advance_clock(Duration::from_millis(600));
    ready(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "Choice accepted by Host: Diagram draft",
    );
    button(&mut v, &view, "Replace first verdict");
    node(
        &mut v,
        &view,
        "status",
        "An earlier verdict did not match this pairing.",
    );
    assert!(
        node(&mut v, &view, "button", "Final")
            .description
            .contains("Checklist draft")
    );
    no_champion(&mut v, &view);
}

#[gpui::test]
fn judge_bracket_bye_navigation_and_match_identity_survive_reflow(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "judge_bracket", "modern-dark");
    v.simulate_resize(size(px(1380.), px(1050.)));
    ready(&mut v, &view);
    button(&mut v, &view, "Show odd field");
    let bye = node(&mut v, &view, "button", "Semifinal 2");
    assert!(bye.description.contains("Advanced without an opponent"));
    assert!(
        node(&mut v, &view, "button", "Final")
            .description
            .contains("Waiting for an earlier match")
    );
    let first = bounds(&mut v, &view, "button", "Semifinal 1");
    let final_round = bounds(&mut v, &view, "button", "Final");
    assert!(final_round.x > first.x + first.width);
    button(&mut v, &view, "Semifinal 1");
    v.simulate_keystrokes("right enter");
    ready(&mut v, &view);
    node(&mut v, &view, "heading", "Final");
    button(&mut v, &view, "Narrow bracket");
    assert_eq!(node(&mut v, &view, "button", "Semifinal 2").id, bye.id);
    let first = bounds(&mut v, &view, "button", "Semifinal 1");
    let final_round = bounds(&mut v, &view, "button", "Final");
    assert!((first.x - final_round.x).abs() <= 1. && first.y < final_round.y);
    button(&mut v, &view, "Semifinal 1");
    v.simulate_keystrokes("down enter");
    ready(&mut v, &view);
    node(&mut v, &view, "heading", "Semifinal 2");
    button(&mut v, &view, "Record next verdict");
    assert!(
        node(&mut v, &view, "button", "Final")
            .description
            .contains("Review panel is comparing")
    );
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn winner_path_extent(scene: &gpui::Scene) -> f32 {
    let h = scene
        .paths
        .iter()
        .filter_map(|p| {
            let c = p.color.as_solid()?.to_rgb();
            ((c.r - 131. / 255.).abs() < 0.009
                && (c.g - 199. / 255.).abs() < 0.009
                && c.b > 0.98
                && c.a > 0.95)
                .then_some(p.bounds.size.width.0 + p.bounds.size.height.0)
        })
        .sum::<f32>();
    #[cfg(feature = "gpui-fast")]
    let h = h + scene
        .layers
        .frames
        .iter()
        .filter(|l| l.tile_size == 0)
        .flat_map(|layer| layer.content.scenes())
        .map(winner_path_extent)
        .sum::<f32>();
    h
}

#[gpui::test]
#[cfg(feature = "vendor-gpui")]
fn judge_bracket_draws_the_accepted_champion_path_on_the_runtime_clock(cx: &mut TestAppContext) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "judge_bracket".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .runtime_clock(clock.clock())
    .motion_preference(MotionPreference::Normal)
    .prepare()
    .unwrap();
    let (win, view) =
        super::mount_prepared(cx, prepared, ("bracket", "gallery"), false, |host, view| {
            Host { host, view }
        });
    let mut v = VisualTestContext::from_window(*win, cx);
    v.update(|win, _| win.activate_window());
    v.simulate_resize(size(px(1380.), px(1050.)));
    ready(&mut v, &view);
    let measure = |v: &mut VisualTestContext| {
        v.update(|window, _| window.visual_probe_with_painted_scene(winner_path_extent))
    };
    for _ in 0..2 {
        button(&mut v, &view, "Record next verdict");
        assert_eq!(measure(&mut v), 0.);
    }
    button(&mut v, &view, "Record next verdict");
    clock.advance(Duration::from_millis(100));
    ready(&mut v, &view);
    let part = measure(&mut v);
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    let end = measure(&mut v);
    assert!(part > 0. && end > part + 7., "{part} to {end}");
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
