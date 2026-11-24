use gpui::{
    Modifiers, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px, size,
};
use gpui_rhai::{GeometryBounds, ManualRuntimeClock, MotionPreference, ScriptViewHandle, UiValue};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};
use std::time::{Duration, Instant};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, PresetRisks, Step, bounds, center, glyphs_with_ink, settle, snapshot};

#[gpui::test]
#[cfg(target_os = "macos")]
fn agent_cursors_overflow_badge_uses_avatar_paper_and_keeps_native_disclosure(
    cx: &mut TestAppContext,
) {
    use crate::support::live_fills;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("agent_cursors_more_badge"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut visual, view) = gallery(&mut native, "agent_cursors", "modern-light");
    visual.simulate_resize(size(px(1340.), px(960.)));
    ready(&mut visual, &view);
    let initial = node(&mut visual, &view, "button", "More agents").id;
    let mut risks = PresetRisks::new();
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    for (family, appearance, height, surface, ink) in [
        ("Modern", "Light", 28., 0xffffffff, 0x8a8a8aff),
        ("Modern", "Dark", 28., 0x1f1f22ff, 0x7c7c82ff),
        ("Clean", "Light", 24., 0xffffffff, 0x00000073),
        ("Clean", "Dark", 24., 0x1c1c1fff, 0xffffff73),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut visual, &view);
        let more = node(&mut visual, &view, "button", "More agents");
        let rect = more.geometry.unwrap().visual;
        assert_eq!(more.id, initial);
        assert_eq!(more.expanded, Some(false));
        assert!(
            (rect.width - 24.).abs() <= 0.6 && (rect.height - height).abs() <= 0.6,
            "{family}/{appearance}: {rect:?}"
        );
        assert!(
            live_fills(&mut visual, surface).into_iter().any(|paint| [
                f64::from(paint.x) - rect.x,
                f64::from(paint.y) - rect.y,
                f64::from(paint.width) - rect.width,
                f64::from(paint.height) - rect.height,
            ]
            .into_iter()
            .all(|delta| delta.abs() <= 0.6)),
            "{family}/{appearance}: the badge paints its own avatar surface"
        );
        assert!(
            glyphs_with_ink(&mut visual, rect, ink) >= 2,
            "{family}/{appearance}: +N ink"
        );
        let (stock, findings) = visual.update(|_, cx| {
            (
                view.theme_snapshot(cx).unwrap().variant,
                view.composition_audit_with(&rules, cx).unwrap(),
            )
        });
        let raw = findings
            .iter()
            .find(|finding| {
                finding
                    .path
                    .ends_with("/header-stack/header/faces/more/label-entry/label")
            })
            .unwrap_or_else(|| panic!("{family}/{appearance}: missing exact overflow warning"));
        assert!(risks.record(&stock, raw), "{family}/{appearance}: {raw:?}");
        let mut wrong = raw.clone();
        wrong.contrast.as_mut().unwrap().background_srgb = Some([0.5; 3]);
        assert!(!risks.record(&stock, &wrong));
    }
    click(&mut visual, &view, "button", "More agents");
    let expanded = node(&mut visual, &view, "button", "More agents");
    assert_eq!(expanded.id, initial);
    assert_eq!(expanded.expanded, Some(true));
    visual.simulate_keystrokes("space");
    ready(&mut visual, &view);
    let collapsed = node(&mut visual, &view, "button", "More agents");
    assert_eq!(collapsed.id, initial);
    assert_eq!(collapsed.expanded, Some(false));
    risks.assert_reported();
}

#[gpui::test]
fn agent_cursors_review_tab_and_header_risks_keep_their_exact_stock_paint(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_cursors", "modern-light");
    visual.simulate_resize(size(px(1300.), px(950.)));
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for (family, appearance) in [
        ("Modern", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Clean", "Dark"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        for density in ["comfortable", "compact"] {
            super::acceptance::action(
                &mut visual,
                &view,
                "gallery.set_density",
                UiValue::String(density.to_owned()),
            );
            ready(&mut visual, &view);
            let (theme, findings) = visual.update(|_, cx| {
                (
                    view.theme_snapshot(cx).unwrap().variant,
                    view.composition_audit_with(&rules, cx).unwrap(),
                )
            });
            for tail in [
                "/review/tabs/tab:checks/0",
                "/review/review-surface/sheet/header-stack/header/title",
                "/review/review-surface/sheet/header-stack/header/summary",
            ] {
                let finding = findings
                    .iter()
                    .find(|finding| finding.path.ends_with(tail))
                    .unwrap_or_else(|| panic!("{family}/{appearance}/{density}: missing {tail}"));
                assert!(
                    risks.record(&theme, finding),
                    "{family}/{appearance}/{density}: {finding:?}"
                );
                let mut moved = finding.clone();
                moved.path = finding.path.replace("/cursors-demo/", "/unrelated-demo/");
                assert!(!risks.record(&theme, &moved));
                let mut wrong_paper = finding.clone();
                wrong_paper.contrast.as_mut().unwrap().background_srgb = Some([0.5; 3]);
                assert!(!risks.record(&theme, &wrong_paper));
            }
        }
    }
    risks.assert_reported();
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn agent_cursors_review_tab_rest_and_hover_match_all_four_appearances(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("agent_cursors_review_tab_paint"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, resting, hover) in [
        ("modern-light", 0x8a8a8aff, 0x171717ff),
        ("modern-dark", 0x7c7c82ff, 0xedededff),
        ("clean-light", 0x00000073, 0x000000e0),
        ("clean-dark", 0xffffff73, 0xffffffe6),
    ] {
        // The shared fixture disables motion. A hover should settle on its
        // real authored endpoint immediately, without changing selection.
        let (mut v, view) = gallery(&mut native, "agent_cursors", theme);
        v.simulate_resize(size(px(1300.), px(950.)));
        ready(&mut v, &view);
        let entry = node(&mut v, &view, "tab", "Checklist");
        let area = entry.geometry.unwrap().visual;
        assert_eq!(entry.selected, Some(false));
        assert!(glyphs_with_ink(&mut v, area, resting) >= 5, "{theme}: idle");
        v.simulate_mouse_move(center(area), None, Modifiers::none());
        ready(&mut v, &view);
        assert!(glyphs_with_ink(&mut v, area, hover) >= 5, "{theme}: hover");
        let hovered = node(&mut v, &view, "tab", "Checklist");
        assert_eq!(hovered.id, entry.id);
        assert_eq!(hovered.geometry.unwrap().visual, area);
        assert_eq!(hovered.selected, Some(false));
        v.simulate_mouse_move(point(px(1.), px(1.)), None, Modifiers::none());
        ready(&mut v, &view);
        assert!(
            glyphs_with_ink(&mut v, area, resting) >= 5,
            "{theme}: pointer exit restores authored ink"
        );
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn agent_cursors_review_tab_hover_retargets_from_the_painted_midpoint(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("agent_cursors_review_tab_motion"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view, clock) = gallery_clock(&mut native);
    v.update(|_, cx| view.select_theme("Modern", "Dark", cx))
        .unwrap();
    ready(&mut v, &view);
    clock.advance(Duration::from_millis(200));
    ready(&mut v, &view);
    let area = bounds(&mut v, &view, "tab", "Checklist");
    let resting = 0x7c7c82ff;
    let hover = 0xedededff;
    // Half of the authored CSS ease's time is approximately 0.8024 in
    // progress. These Modern Dark endpoints are both opaque.
    let midway = 0xd7d7d8ff;
    assert!(glyphs_with_ink(&mut v, area, resting) >= 5);
    v.simulate_mouse_move(center(area), None, Modifiers::none());
    ready(&mut v, &view);
    if !gpui_rhai::EXTENDED_RENDERING {
        // Public GPUI applies both authored endpoints immediately.
        assert!(glyphs_with_ink(&mut v, area, hover) >= 5);
        v.simulate_mouse_move(point(px(1.), px(1.)), None, Modifiers::none());
        ready(&mut v, &view);
        assert!(glyphs_with_ink(&mut v, area, resting) >= 5);
        return;
    }
    assert!(glyphs_with_ink(&mut v, area, resting) >= 5, "start");
    clock.advance(Duration::from_millis(80));
    ready(&mut v, &view);
    assert!(glyphs_with_ink(&mut v, area, midway) >= 5, "midpoint");
    assert_eq!(glyphs_with_ink(&mut v, area, hover), 0);

    // Reverse on the same frame clock, before reaching the hover endpoint.
    v.simulate_mouse_move(point(px(1.), px(1.)), None, Modifiers::none());
    ready(&mut v, &view);
    assert!(glyphs_with_ink(&mut v, area, midway) >= 5, "reverse start");
    clock.advance(Duration::from_millis(160));
    ready(&mut v, &view);
    assert!(glyphs_with_ink(&mut v, area, resting) >= 5, "reverse end");
}

#[gpui::test]
fn held_reviewer_flag_uses_its_paper_not_its_gradient_edge_for_contrast(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_cursors", "modern-light");
    for (family, variant) in [
        ("Modern", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Clean", "Dark"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, variant, cx))
            .unwrap();
        for density in ["comfortable", "compact"] {
            super::acceptance::action(
                &mut visual,
                &view,
                "gallery.set_density",
                UiValue::String(density.to_owned()),
            );
            ready(&mut visual, &view);
            let flag = node(
                &mut visual,
                &view,
                "status",
                "Sora · Reviewing the timeline",
            );
            assert!(flag.geometry.is_some_and(|g| g.visual.width > 0.));
            let findings = visual.update(|_, cx| view.composition_audit(cx).unwrap());
            let flag_findings = findings
                .iter()
                .filter(|f| {
                    f.rule == gpui_rhai::AuditRule::LowContrastText
                        && f.path.contains("/held:sora/flag:sora/")
                })
                .collect::<Vec<_>>();
            assert!(
                flag_findings.is_empty(),
                "{family}/{variant}/{density}: {flag_findings:#?}"
            );
        }
    }
}

#[gpui::test]
fn agent_cursors_refusal_and_acceptance_use_the_host_snapshot(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_cursors", "modern-light");
    visual.simulate_resize(size(px(1220.0), px(950.0)));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Mira · Proposing a date");
    click(&mut visual, &view, "button", "Refuse next review request");
    click(&mut visual, &view, "button", "Inspect suggestion · cadence");
    click(&mut visual, &view, "button", "Accept suggestion");
    settle(&mut visual, Step::Tick, 14);
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The Host refused the review request.",
    );
    node(&mut visual, &view, "button", "Inspect suggestion · cadence");
    click(&mut visual, &view, "button", "Accept suggestion");
    settle(&mut visual, Step::Tick, 14);
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted document revision 1");
    let published = snapshot(&mut visual, &view);
    assert!(
        published
            .nodes()
            .any(|n| n.name.contains("early access group on Friday"))
    );
    assert!(
        !published
            .nodes()
            .any(|n| n.name == "Inspect suggestion · cadence")
    );
}

#[gpui::test]
fn agent_cursors_table_commands_and_comment_reply_are_live(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_cursors", "clean-dark");
    visual.simulate_resize(size(px(1190.0), px(920.0)));
    ready(&mut visual, &view);
    let presence = bounds(&mut visual, &view, "status", "Mira · Proposing a date");
    assert!(
        presence.width > 16.0 && (10.0..=64.0).contains(&presence.height),
        "{presence:?}"
    );
    click(&mut visual, &view, "button", "Open comment · notice/thread");
    click(&mut visual, &view, "text_field", "Reply to comment");
    visual.simulate_input("Include the shortcuts.");
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Tick, 14);
    ready(&mut visual, &view);
    let tree = snapshot(&mut visual, &view);
    assert!(tree.nodes().any(|n| n.name == "Include the shortcuts."));
    node(&mut visual, &view, "status", "Accepted document revision 1");
    click(&mut visual, &view, "tab", "Checklist");
    settle(&mut visual, Step::Tick, 14);
    ready(&mut visual, &view);
    node(&mut visual, &view, "heading", "Release checklist");
    node(&mut visual, &view, "status", "Mira · Checking the date");
    node(&mut visual, &view, "status", "Jules · Confirming scope");
    click(&mut visual, &view, "button", "+ Add row");
    settle(&mut visual, Step::Tick, 14);
    ready(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Follow-up 0")
    );
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn agent_cursors_away_follows_the_host_without_losing_its_group_fade(cx: &mut TestAppContext) {
    use crate::support::{logical_paint_rect, walk_live_paint};
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "agent_cursors", theme);
        v.simulate_resize(size(px(1340.), px(960.)));
        ready(&mut v, &view);
        let original = node(&mut v, &view, "button", "Follow Tess").id;
        for (name, pressed) in [("Follow Tess", false), ("Stop following Tess", true)] {
            if pressed {
                click(&mut v, &view, "button", "Refuse next review request");
                click(&mut v, &view, "button", "Follow Tess");
                settle(&mut v, Step::Tick, 14);
                ready(&mut v, &view);
                node(
                    &mut v,
                    &view,
                    "status",
                    "The Host refused the review request.",
                );
                let rejected = node(&mut v, &view, "button", "Follow Tess");
                assert_eq!(rejected.id, original);
                assert_eq!(rejected.pressed, Some(false));
                click(&mut v, &view, "button", "Follow Tess");
                settle(&mut v, Step::Tick, 14);
                ready(&mut v, &view);
                node(&mut v, &view, "status", "Accepted document revision 1");
            }
            let face = node(&mut v, &view, "button", name);
            let area = face.geometry.unwrap().visual;
            let groups = v.update(|window, _| {
                let scale = window.scale_factor();
                window.visual_probe_with_painted_scene(|scene| {
                    let mut groups = Vec::new();
                    walk_live_paint(
                        scene,
                        gpui::TransformationMatrix::unit(),
                        &mut |scene, pose| {
                            for group in &scene.isolated_groups {
                                let rect = logical_paint_rect(group.bounds, pose, scale);
                                let x = area.x + area.width / 2.;
                                let y = area.y + area.height / 2.;
                                if f64::from(rect.x) <= x
                                    && f64::from(rect.right()) >= x
                                    && f64::from(rect.y) <= y
                                    && f64::from(rect.bottom()) >= y
                                    && f64::from(rect.width) < area.width + 12.
                                    && f64::from(rect.height) < area.height + 12.
                                {
                                    groups.push(group.opacity());
                                }
                            }
                        },
                    );
                    groups
                })
            });
            assert!(
                groups.iter().any(|a| (*a - 0.4).abs() < 0.001),
                "{theme}/{name}: {groups:?}"
            );
            assert_eq!(face.id, original);
            assert_eq!(face.pressed, Some(pressed));
            let (stock, findings) = v.update(|_, cx| {
                (
                    view.theme_snapshot(cx).unwrap().variant,
                    view.composition_audit_with(&rules, cx).unwrap(),
                )
            });
            for id in ["tess", "kim"] {
                let tail = format!("/faces/follow:{id}/0");
                let risk = findings.iter().find(|f| f.path.ends_with(&tail));
                if pressed && id == "tess" {
                    // Following changes the destination inside the fading
                    // group. It is no longer this exact idle-palette pair.
                    if let Some(risk) = risk {
                        assert!(
                            !risks.record(&stock, risk),
                            "{theme}: active paint {risk:?}"
                        );
                    }
                } else {
                    let risk = risk.unwrap_or_else(|| panic!("{theme}: missing away risk {id}"));
                    assert!(risks.record(&stock, risk), "{theme}: {risk:?}");
                    let mut wrong = risk.clone();
                    wrong.contrast.as_mut().unwrap().element_opacity = 0.55;
                    assert!(!risks.record(&stock, &wrong));
                }
            }
        }
        click(&mut v, &view, "button", "Stop following Tess");
        settle(&mut v, Step::Tick, 14);
        ready(&mut v, &view);
        node(&mut v, &view, "status", "Accepted document revision 2");
        assert_eq!(node(&mut v, &view, "button", "Follow Tess").id, original);
    }
    risks.assert_reported();
}

#[gpui::test]
fn agent_cursors_follow_reveals_a_later_target_without_a_document_edit(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_cursors", "clean-light");
    visual.simulate_resize(size(px(1150.0), px(920.0)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Follow Mira");
    settle(&mut visual, Step::Tick, 14);
    ready(&mut visual, &view);
    node(&mut visual, &view, "button", "Stop following Mira");
    let before = bounds(&mut visual, &view, "heading", "Launch brief");
    click(&mut visual, &view, "button", "Move reviewers");
    node(&mut visual, &view, "status", "Mira · Checking the sign-off");
    let shifted = bounds(&mut visual, &view, "heading", "Launch brief");
    assert!(shifted.y < before.y - 40.0, "{before:?} -> {shifted:?}");
    click(&mut visual, &view, "button", "Stop following Mira");
    settle(&mut visual, Step::Tick, 14);
    ready(&mut visual, &view);
    node(&mut visual, &view, "button", "Follow Mira");
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn native_fades(scene: &gpui::Scene) -> usize {
    let groups = scene
        .isolated_groups
        .iter()
        .map(|group| usize::from(group.alpha_ramps()[3] > 0.) + native_fades(group.source()))
        .sum::<usize>();
    #[cfg(feature = "gpui-fast")]
    let groups = groups
        + scene
            .layers
            .frames
            .iter()
            .filter_map(|layer| layer.content.scene())
            .map(native_fades)
            .sum::<usize>();
    groups
}

#[gpui::test]
fn agent_cursors_page_fade_preserves_scroll_identity_and_review_hits(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_cursors", "modern-dark");
    visual.simulate_resize(size(px(1150.0), px(940.0)));
    ready(&mut visual, &view);
    #[cfg(feature = "vendor-gpui")]
    let inspect = |visual: &mut VisualTestContext| {
        visual.update(|window, _| window.visual_probe_with_painted_scene(native_fades))
    };
    #[cfg(feature = "vendor-gpui")]
    assert_eq!(inspect(&mut visual), 1);
    let heading = node(&mut visual, &view, "heading", "Launch brief");
    let before = heading.geometry.unwrap().visual;
    visual.simulate_event(ScrollEvent {
        position: center(before),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-30.0))),
        ..Default::default()
    });
    ready(&mut visual, &view);
    let scrolled = bounds(&mut visual, &view, "heading", "Launch brief");
    assert!(scrolled.y < before.y - 15.0);
    click(&mut visual, &view, "button", "Disable page fade");
    #[cfg(feature = "vendor-gpui")]
    assert_eq!(inspect(&mut visual), 0);
    let real = node(&mut visual, &view, "heading", "Launch brief");
    assert_eq!(real.id, heading.id);
    assert!((real.geometry.unwrap().visual.y - scrolled.y).abs() < 1.0);
    click(&mut visual, &view, "button", "Enable page fade");
    #[cfg(feature = "vendor-gpui")]
    assert_eq!(inspect(&mut visual), 1);
    assert!((bounds(&mut visual, &view, "heading", "Launch brief").y - scrolled.y).abs() < 1.0);
    click(&mut visual, &view, "button", "Open comment · notice/thread");
    node(&mut visual, &view, "text_field", "Reply to comment");
}

fn gallery_clock(
    cx: &mut TestAppContext,
) -> (VisualTestContext, ScriptViewHandle, ManualRuntimeClock) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "agent_cursors".into(),
        theme: "clean-dark".into(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::Normal)
    .runtime_clock(clock.clock())
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("gallery-motion", "gallery-motion"),
        false,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, _| window.activate_window());
    visual.simulate_resize(size(px(1150.0), px(950.0)));
    clock.advance(Duration::from_secs(1));
    ready(&mut visual, &view);
    (visual, view, clock)
}

#[gpui::test]
fn agent_cursors_motion_glides_and_retargets_from_the_displayed_point(cx: &mut TestAppContext) {
    let (mut visual, view, clock) = gallery_clock(cx);
    let cue = "Mira · Proposing a date";
    let old = bounds(&mut visual, &view, "status", cue).x;
    click(&mut visual, &view, "button", "Move typing cursor");
    let start = bounds(&mut visual, &view, "status", cue).x;
    assert!((start - old).abs() < 1.0, "{old} -> {start}");
    clock.advance(Duration::from_millis(55));
    ready(&mut visual, &view);
    let center = bounds(&mut visual, &view, "status", cue).x;
    assert!(center < start - 20.0, "{start} -> {center}");
    click(&mut visual, &view, "button", "Move typing cursor");
    let reverse = bounds(&mut visual, &view, "status", cue).x;
    assert!((reverse - center).abs() < 1.0, "{center} -> {reverse}");
    clock.advance(Duration::from_millis(55));
    ready(&mut visual, &view);
    let moving_home = bounds(&mut visual, &view, "status", cue).x;
    assert!(moving_home > reverse + 12.0 && moving_home < old - 12.0);
    clock.advance(Duration::from_millis(140));
    ready(&mut visual, &view);
    assert!((bounds(&mut visual, &view, "status", cue).x - old).abs() < 1.0);
}

fn held_image(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> GeometryBounds {
    let image = bounds(visual, view, "group", "Preview of the rollout timeline");
    let flag = bounds(visual, view, "status", "Sora · Reviewing the timeline");
    assert!((10.0..=64.0).contains(&flag.height), "{flag:?}");
    assert!(
        (flag.x - (image.x + image.width - 144.0)).abs() < 1.0,
        "{image:?} {flag:?}"
    );
    assert!(
        (flag.y - (image.y + image.height - 44.0)).abs() < 1.0,
        "{image:?} {flag:?}"
    );
    image
}

#[gpui::test]
fn agent_cursors_leading_float_keeps_paragraph_owners_across_narrow_resize(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "agent_cursors", "modern-light");
    visual.simulate_resize(size(px(1140.), px(950.)));
    ready(&mut visual, &view);
    let paragraph = |visual: &mut VisualTestContext, prefix: &str| {
        snapshot(visual, &view)
            .nodes()
            .find(|n| n.name.starts_with(prefix))
            .unwrap_or_else(|| panic!("missing paragraph {prefix}"))
            .clone()
    };
    let first = paragraph(&mut visual, "Start with a small audience.");
    let second = paragraph(&mut visual, "Keep an eye on the reports");
    let wide = bounds(
        &mut visual,
        &view,
        "group",
        "Preview of the rollout timeline",
    );
    let first_box = first.geometry.unwrap().visual;
    let second_box = second.geometry.unwrap().visual;
    if gpui_rhai::EXTENDED_RENDERING {
        assert!((first_box.y - wide.y).abs() < 1.0, "{wide:?} {first_box:?}");
        assert!(second_box.y < wide.y + wide.height);
    } else {
        assert!(first_box.y > wide.y + wide.height, "{wide:?} {first_box:?}");
    }
    assert!(first_box.y < second_box.y);
    visual.simulate_resize(size(px(700.), px(950.)));
    ready(&mut visual, &view);
    let narrow = bounds(
        &mut visual,
        &view,
        "group",
        "Preview of the rollout timeline",
    );
    let first_narrow = paragraph(&mut visual, "Start with a small audience.");
    assert_eq!(first.id, first_narrow.id);
    assert_eq!(
        second.id,
        paragraph(&mut visual, "Keep an eye on the reports").id
    );
    assert!(
        first_narrow.geometry.unwrap().visual.y > narrow.y + narrow.height,
        "{narrow:?}"
    );
    visual.simulate_resize(size(px(1140.), px(950.)));
    ready(&mut visual, &view);
    let wide_again = paragraph(&mut visual, "Start with a small audience.");
    assert_eq!(wide_again.id, first.id);
    assert!((wide_again.geometry.unwrap().visual.y - first_box.y).abs() < 1.0);
    if gpui_rhai::EXTENDED_RENDERING {
        assert!(
            (wide.x - first_box.x).abs() < 1.0,
            "logical start must be left in LTR"
        );
    }
    visual.update(|_, cx| view.select_locale("ar", cx)).unwrap();
    ready(&mut visual, &view);
    let rtl_text = paragraph(&mut visual, "Start with a small audience.");
    let rtl_media = bounds(
        &mut visual,
        &view,
        "group",
        "Preview of the rollout timeline",
    );
    let rtl_box = rtl_text.geometry.unwrap().visual;
    assert_eq!(rtl_text.id, first.id);
    if gpui_rhai::EXTENDED_RENDERING {
        assert!((rtl_media.y - rtl_box.y).abs() < 1.0);
        assert!(
            (rtl_media.x + rtl_media.width - rtl_box.x - rtl_box.width).abs() < 1.0,
            "logical start must be right in RTL: {rtl_media:?} {rtl_box:?}"
        );
    } else {
        assert!(rtl_box.y > rtl_media.y + rtl_media.height);
    }
}

#[gpui::test]
fn agent_cursors_motion_resizes_real_figure_and_its_held_corner(cx: &mut TestAppContext) {
    let (mut visual, view, clock) = gallery_clock(cx);
    let start = held_image(&mut visual, &view);
    click(&mut visual, &view, "button", "Resize illustration");
    if !gpui_rhai::EXTENDED_RENDERING {
        let resized = held_image(&mut visual, &view);
        assert!((resized.width - start.width).abs() > 20.0);
        assert!((resized.width / resized.height - 1.875).abs() < 0.03);
        click(&mut visual, &view, "button", "Resize illustration");
        let restored = held_image(&mut visual, &view);
        assert!((restored.width - resized.width).abs() > 20.0);
        assert!((restored.width / restored.height - 1.875).abs() < 0.03);
        return;
    }
    assert!((held_image(&mut visual, &view).width - start.width).abs() < 1.0);
    clock.advance(Duration::from_millis(130));
    ready(&mut visual, &view);
    let middle = held_image(&mut visual, &view);
    assert!(middle.width < start.width - 20.0 && middle.width > start.width * 0.60);
    assert!((middle.width / middle.height - 1.875).abs() < 0.03);
    click(&mut visual, &view, "button", "Resize illustration");
    assert!((held_image(&mut visual, &view).width - middle.width).abs() < 1.0);
    clock.advance(Duration::from_millis(130));
    ready(&mut visual, &view);
    assert!(held_image(&mut visual, &view).width > middle.width + 20.0);
    visual
        .update(|_, cx| view.set_motion_preference(MotionPreference::Reduced, cx))
        .unwrap();
    ready(&mut visual, &view);
    let end = held_image(&mut visual, &view);
    assert!((end.width / start.width - 0.65 / 0.52).abs() < 0.018);
}

#[gpui::test]
fn agent_cursors_leading_float_reflows_real_review_anchors(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_cursors", "modern-light");
    visual.simulate_resize(size(px(1140.), px(950.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Follow Imani");
    settle(&mut visual, Step::Tick, 14);
    ready(&mut visual, &view);
    let label = "Inspect suggestion · rollout_note";
    let pin = node(&mut visual, &view, "button", label);
    let initial = pin.geometry.unwrap().visual;
    click(&mut visual, &view, "button", "Resize illustration");
    let resized_pin = node(&mut visual, &view, "button", label);
    assert_eq!(resized_pin.id, pin.id);
    let reflow = resized_pin.geometry.unwrap().visual;
    if gpui_rhai::EXTENDED_RENDERING {
        assert!(
            (initial.x - reflow.x).abs() + (initial.y - reflow.y).abs() > 14.,
            "the actual suggestion pin should move with the text: {initial:?} {reflow:?}"
        );
    }
    click(&mut visual, &view, "button", label);
    node(&mut visual, &view, "group", "Suggested edit");
    click(&mut visual, &view, "button", "Accept suggestion");
    settle(&mut visual, Step::Tick, 14);
    ready(&mut visual, &view);
    let published = snapshot(&mut visual, &view);
    assert!(
        published
            .nodes()
            .any(|n| n.name.contains("One editor checks the wording")),
        "{:?}",
        published
            .nodes()
            .filter(|n| n.role == "status"
                || n.name.starts_with("Keep an eye")
                || n.name == "Accept suggestion")
            .map(|n| (&n.name, n.geometry))
            .collect::<Vec<_>>()
    );
    assert!(!published.nodes().any(|n| n.name == label));
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
