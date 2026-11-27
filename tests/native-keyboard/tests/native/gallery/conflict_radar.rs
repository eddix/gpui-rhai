use std::time::{Duration, Instant};

use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle, UiValue};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, bounds, center, snapshot};

fn mount(cx: &mut TestAppContext, theme: &str) -> (VisualTestContext, ScriptViewHandle) {
    let (mut v, view) = gallery(cx, "conflict_radar", theme);
    v.simulate_resize(size(px(1380.), px(1050.)));
    ready(&mut v, &view);
    (v, view)
}

fn button(v: &mut VisualTestContext, view: &ScriptViewHandle, label: &str) {
    click(v, view, "button", label);
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn conflict_radar_path_roles_keep_actual_font_and_neutral_file_ink(cx: &mut TestAppContext) {
    use crate::support::glyphs_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("conflict_radar_filenames"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (preset, tone) in [
        ("modern-light", 0x6b6b6bff),
        ("modern-dark", 0xa1a1a1ff),
        ("clean-light", 0x0000009c),
        ("clean-dark", 0xffffff9e),
    ] {
        let (mut v, view) = mount(&mut real, preset);
        let full_file = node(&mut v, &view, "rowheader", "examples/local/config.rhai").id;
        for narrow in [false, true] {
            if narrow {
                button(&mut v, &view, "Narrow matrix");
            }
            assert_eq!(
                node(&mut v, &view, "rowheader", "examples/local/config.rhai").id,
                full_file
            );
            let name = bounds(&mut v, &view, "text", "config.rhai");
            let file = bounds(&mut v, &view, "rowheader", "examples/local/config.rhai");
            assert!(
                (name.height - 20.).abs() <= 0.5 && name.x + name.width <= file.x + file.width,
                "{preset}/{narrow}: {name:?} {file:?}"
            );
            let actual_glyphs = glyphs_with_ink(&mut v, name, tone);
            assert!(
                actual_glyphs >= 10,
                "{preset}/{narrow}: {actual_glyphs} glyphs, {name:?} in {file:?}"
            );
        }
    }
}

#[derive(Debug, PartialEq)]
struct WritingBand {
    x: f32,
    width: f32,
    clip: gpui::Bounds<gpui::ScaledPixels>,
    base: gpui::Bounds<gpui::ScaledPixels>,
    opacity: f32,
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn writing_bands(scene: &gpui::Scene, bands: &mut Vec<WritingBand>) {
    for fill in &scene.layered_fills {
        if let [front, tint, _card] = &*fill.layers
            && front.stops.len() == 3
            && (front.origin.size.width.0 / front.clip.size.width.0 - 0.4375).abs() < 0.001
        {
            bands.push(WritingBand {
                x: front.origin.left().0 - front.clip.left().0,
                width: front.origin.size.width.0,
                clip: front.clip,
                base: tint.origin,
                opacity: fill.opacity,
            });
        }
    }
    for group in &scene.isolated_groups {
        writing_bands(group.source(), bands);
    }
    #[cfg(feature = "gpui-fast")]
    for child in scene
        .layers
        .frames
        .iter()
        .flat_map(|layer| layer.content.scenes())
    {
        writing_bands(child, bands);
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn partial_reveals(scene: &gpui::Scene) -> usize {
    let count: usize = scene
        .isolated_groups
        .iter()
        .map(|group| {
            let [_, _, w, h, r, enabled] = group.rounded_reveal();
            if enabled != 0. {
                assert!(w > 0. && h > 0. && (0.0..=w.min(h) * 0.5).contains(&r));
            }
            usize::from(enabled != 0.) + partial_reveals(group.source())
        })
        .sum();
    #[cfg(feature = "gpui-fast")]
    let count = count
        + scene
            .layers
            .frames
            .iter()
            .flat_map(|layer| layer.content.scenes())
            .map(partial_reveals)
            .sum::<usize>();
    count
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn conflict_radar_moves_only_the_writing_band_without_rescheduling_rhai(cx: &mut TestAppContext) {
    for (key, policy) in [
        ("radar-normal", MotionPreference::Normal),
        ("radar-reduced", MotionPreference::Reduced),
        ("radar-disabled", MotionPreference::None),
    ] {
        let clock = ManualRuntimeClock::new(Instant::now());
        let prepared = acceptance::view(&AcceptanceLaunch {
            page: "conflict_radar".into(),
            theme: "clean-dark".into(),
            ..Default::default()
        })
        .unwrap()
        .runtime_clock(clock.clock())
        .motion_preference(policy)
        .prepare()
        .unwrap();
        let (win, view) =
            super::mount_prepared(cx, prepared, (key, "gallery"), false, |host, view| Host {
                host,
                view,
            });
        let mut v = VisualTestContext::from_window(*win, cx);
        v.update(|w, _| w.activate_window());
        v.simulate_resize(size(px(1380.), px(1050.)));
        ready(&mut v, &view);
        let first = node(&mut v, &view, "gridcell", "Atlas: Conflict");
        let bands = |v: &mut VisualTestContext| {
            v.update(|w, _| {
                w.visual_probe_with_painted_scene(|scene| {
                    let mut all = Vec::new();
                    writing_bands(scene, &mut all);
                    all
                })
            })
        };
        let reveals = |v: &mut VisualTestContext| {
            v.update(|w, _| w.visual_probe_with_painted_scene(partial_reveals))
        };
        let hidden = bands(&mut v);
        let outside =
            |band: &WritingBand| band.x <= -band.width + 0.01 || band.x >= band.clip.size.width.0;
        if policy == MotionPreference::Normal {
            assert!(hidden.is_empty(), "the untouched paint is hidden at entry");
        } else {
            assert_eq!(hidden.len(), 1);
            assert!(outside(&hidden[0]), "{hidden:?}");
        }
        v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        clock.advance(Duration::from_millis(260));
        ready(&mut v, &view);
        let initial = bands(&mut v);
        assert_eq!(initial.len(), 1);
        assert!((initial[0].clip.size.width.0 / initial[0].width - 32. / 14.).abs() < 0.001);
        if policy == MotionPreference::Normal {
            assert!(
                reveals(&mut v) > 0,
                "writing paints inside the rounded reveal"
            );
        } else {
            assert_eq!(reveals(&mut v), 0);
        }
        clock.advance(Duration::from_millis(390));
        ready(&mut v, &view);
        let mid = bands(&mut v);
        assert_eq!(reveals(&mut v), 0, "ordinary paint resumes after entry");
        assert_eq!(mid.len(), 1);
        if policy == MotionPreference::Normal {
            assert!(
                (mid[0].x / mid[0].width - 13. / 14.).abs() < 0.01,
                "{mid:?}"
            );
        } else {
            assert!(outside(&mid[0]), "{mid:?}");
        }
        assert_eq!(
            (mid[0].clip, mid[0].base, mid[0].opacity),
            (initial[0].clip, initial[0].base, 1.)
        );
        let last = node(&mut v, &view, "gridcell", "Atlas: Conflict");
        assert_eq!((first.id, first.geometry), (last.id, last.geometry));
        assert!(
            v.update(|_, cx| view.take_performance_snapshot(cx).unwrap())
                .timings
                .is_empty()
        );
        let cell = bounds(&mut v, &view, "gridcell", "Atlas: Conflict");
        v.simulate_mouse_move(center(cell), None, Modifiers::none());
        ready(&mut v, &view);
        v.executor().advance_clock(Duration::from_millis(450));
        ready(&mut v, &view);
        node(
            &mut v,
            &view,
            "tooltip",
            "Atlas: Conflict · Lines 18–24 · Lines 70–84",
        );
        button(&mut v, &view, "Finish agents");
        assert!(
            bands(&mut v).is_empty(),
            "Host completion removes the moving image"
        );
        assert_eq!(
            reveals(&mut v),
            0,
            "an edited bar is retained when writing stops"
        );
    }
}

fn acknowledge(v: &mut VisualTestContext, view: &ScriptViewHandle) {
    v.executor().advance_clock(Duration::from_millis(440));
    ready(v, view);
}

fn choose(v: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) {
    click(v, view, "radio", name);
    node(v, view, "status", "Choice proposed");
    assert!(node(v, view, "radio", name).disabled);
    acknowledge(v, view);
}

#[gpui::test]
fn conflict_radar_requires_both_distinct_bands_and_completed_agents(cx: &mut TestAppContext) {
    let (mut v, view) = mount(cx, "modern-light");
    assert!(node(&mut v, &view, "button", "2 overlaps to settle").disabled);
    node(&mut v, &view, "status", "release.md · Lines 20–24");
    choose(&mut v, &view, "Atlas");
    node(&mut v, &view, "status", "Host kept atlas");
    node(&mut v, &view, "gridcell", "Atlas: Kept");
    node(&mut v, &view, "gridcell", "Nova: Conflict");
    assert!(node(&mut v, &view, "button", "1 overlap to settle").disabled);
    button(&mut v, &view, "Next overlap");
    node(&mut v, &view, "status", "release.md · Lines 74–82");
    choose(&mut v, &view, "Nova");
    assert!(node(&mut v, &view, "button", "Waiting for 1 agent").disabled);
    button(&mut v, &view, "Finish agents");
    assert!(!node(&mut v, &view, "button", "Merge 3 branches").disabled);
    button(&mut v, &view, "Merge 3 branches");
    node(&mut v, &view, "status", "Merge proposed");
    assert!(node(&mut v, &view, "button", "Merge 3 branches").disabled);
    acknowledge(&mut v, &view);
    node(&mut v, &view, "status", "Merge accepted by Host");
}

#[gpui::test]
fn conflict_radar_rejection_and_revised_same_lines_cannot_authorize_merge(cx: &mut TestAppContext) {
    let (mut v, view) = mount(cx, "clean-dark");
    button(&mut v, &view, "Finish agents");
    button(&mut v, &view, "Refuse next proposal");
    choose(&mut v, &view, "Scribe");
    node(&mut v, &view, "status", "Proposal declined by Host");
    assert_eq!(
        node(&mut v, &view, "radio", "Scribe").checked,
        Some(UiValue::Bool(false))
    );
    choose(&mut v, &view, "Scribe");
    button(&mut v, &view, "Next overlap");
    choose(&mut v, &view, "Atlas");
    assert!(!node(&mut v, &view, "button", "Merge 3 branches").disabled);
    button(&mut v, &view, "Revise first edit");
    assert!(node(&mut v, &view, "button", "1 overlap to settle").disabled);
    assert_eq!(
        node(&mut v, &view, "radio", "Atlas").checked,
        Some(UiValue::Bool(true))
    );
    button(&mut v, &view, "Previous overlap");
    node(&mut v, &view, "status", "release.md · Lines 20–24");
    assert_eq!(
        node(&mut v, &view, "radio", "Scribe").checked,
        Some(UiValue::Bool(false))
    );
}

#[gpui::test]
fn conflict_radar_narrow_layout_keeps_row_identity_and_orders_the_choices(cx: &mut TestAppContext) {
    let (mut v, view) = mount(cx, "modern-dark");
    let before = node(&mut v, &view, "button", "docs/release.md").id;
    let a = bounds(&mut v, &view, "radio", "Atlas");
    let b = bounds(&mut v, &view, "radio", "Scribe");
    assert!((a.y - b.y).abs() < 2. && b.x > a.x);
    button(&mut v, &view, "Narrow matrix");
    let a = bounds(&mut v, &view, "radio", "Atlas");
    let b = bounds(&mut v, &view, "radio", "Scribe");
    assert!(b.y > a.y && (a.x - b.x).abs() < 2.);
    assert_eq!(node(&mut v, &view, "button", "docs/release.md").id, before);
    assert!(
        snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.name == "Atlas: Conflict" && n.description.contains("Lines 18–24"))
    );
}

#[gpui::test]
fn conflict_radar_reveals_actual_edit_ranges_on_native_hover(cx: &mut TestAppContext) {
    let (mut v, view) = mount(cx, "clean-light");
    let a = bounds(&mut v, &view, "gridcell", "Atlas: Conflict");
    v.simulate_mouse_move(center(a), None, Modifiers::none());
    ready(&mut v, &view);
    v.executor().advance_clock(Duration::from_millis(450));
    ready(&mut v, &view);
    node(
        &mut v,
        &view,
        "tooltip",
        "Atlas: Conflict · Lines 18–24 · Lines 70–84",
    );
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
