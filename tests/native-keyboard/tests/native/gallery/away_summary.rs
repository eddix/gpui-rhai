use std::time::Duration;

use gpui::{TestAppContext, px, size};
use gpui_rhai::UiValue;

use super::acceptance::action;
use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, snapshot};

#[gpui::test]
fn away_summary_footer_buttons_share_the_reference_height_in_every_preset(cx: &mut TestAppContext) {
    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "away_summary", theme);
        v.simulate_resize(size(px(1250.), px(970.)));
        for density in ["comfortable", "compact"] {
            action(
                &mut v,
                &view,
                "gallery.set_density",
                UiValue::String(density.into()),
            );
            for (scale, expected) in [("100", 28.), ("150", 42.)] {
                action(
                    &mut v,
                    &view,
                    "gallery.set_scale",
                    UiValue::String(scale.into()),
                );
                ready(&mut v, &view);
                let more = bounds(&mut v, &view, "button", "Show 3 more");
                let done = bounds(&mut v, &view, "button", "Got it");
                let context = format!("{theme}/{density}/{scale}");
                assert!(
                    (more.height - expected).abs() <= 0.5,
                    "{context}: more {more:?}"
                );
                assert!(
                    (done.height - expected).abs() <= 0.5,
                    "{context}: done {done:?}"
                );
                assert!(
                    (more.y - done.y).abs() <= 0.5,
                    "{context}: {more:?} {done:?}"
                );
            }
        }
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn away_summary_keeps_caption_and_action_inks_distinct_through_folding(cx: &mut TestAppContext) {
    use crate::support::{PresetRisks, center, glyphs_with_ink, live_fills};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui::{Modifiers, point};
    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("away_summary_painted_inks"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let low_contrast = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    for (theme, caption, action_ink, active, wash) in [
        (
            "modern-light",
            0x8a8a8aff,
            0x6b6b6bff,
            0x171717ff,
            0xebebebff,
        ),
        (
            "modern-dark",
            0x7c7c82ff,
            0xa1a1a6ff,
            0xedededff,
            0x2c2c30ff,
        ),
        (
            "clean-light",
            0x00000073,
            0x0000009c,
            0x000000e0,
            0x0000000d,
        ),
        ("clean-dark", 0xffffff73, 0xffffff9e, 0xffffffe6, 0xffffff12),
    ] {
        let (mut v, view) = gallery(&mut native, "away_summary", theme);
        v.simulate_resize(size(px(1250.), px(970.)));
        ready(&mut v, &view);
        let count = bounds(&mut v, &view, "text", "1 failed");
        let done = node(&mut v, &view, "button", "Got it");
        let done_area = done.geometry.unwrap().visual;
        let more = bounds(&mut v, &view, "button", "Show 3 more");
        assert!(
            glyphs_with_ink(&mut v, count, caption) >= 4,
            "{theme}: header caption"
        );
        assert!(
            glyphs_with_ink(&mut v, done_area, action_ink) >= 4,
            "{theme}: acknowledge at rest"
        );
        assert!(
            glyphs_with_ink(&mut v, more, action_ink) >= 7,
            "{theme}: more at rest"
        );
        let (preset, findings) = v.update(|_, cx| {
            (
                view.theme_snapshot(cx).unwrap().variant,
                view.composition_audit_with(&low_contrast, cx).unwrap(),
            )
        });
        let summary_risks: Vec<_> = findings
            .iter()
            .filter(|f| f.path.contains("/away-demo/width/summary/"))
            .collect();
        let mut policy = PresetRisks::new();
        assert_eq!(summary_risks.len(), 7, "{theme}: {summary_risks:#?}");
        for risk in summary_risks {
            assert!(policy.record(&preset, risk), "{theme}: {risk:#?}");
        }
        policy.assert_reported();
        v.simulate_mouse_move(center(done_area), None, Modifiers::none());
        ready(&mut v, &view);
        assert!(
            glyphs_with_ink(&mut v, done_area, active) >= 4,
            "{theme}: acknowledge on hover"
        );
        assert!(
            live_fills(&mut v, wash).into_iter().any(|rect| {
                (f64::from(rect.x) - done_area.x).abs() < 1.
                    && (f64::from(rect.y) - done_area.y).abs() < 1.
                    && (f64::from(rect.height) - done_area.height).abs() < 1.
            }),
            "{theme}: acknowledge hover fill"
        );
        click(&mut v, &view, "button", "Got it");
        v.simulate_mouse_move(point(px(4.), px(9.)), None, Modifiers::none());
        ready(&mut v, &view);
        let state = snapshot(&mut v, &view);
        let folded = state
            .nodes()
            .find(|n| n.role == "button" && n.name.starts_with("You were away"))
            .unwrap();
        let folded_area = folded.geometry.unwrap().visual;
        assert_eq!(folded.id, done.id);
        assert_eq!(folded.expanded, Some(false));
        assert!(
            glyphs_with_ink(&mut v, folded_area, caption) >= 12,
            "{theme}: collapsed summary"
        );
    }
}

#[gpui::test]
fn away_summary_keeps_ranks_across_accepted_results_and_restores_focus(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "away_summary", "clean-light");
    v.simulate_resize(size(px(1250.), px(970.)));
    ready(&mut v, &view);
    let id = node(&mut v, &view, "listitem", "Choose metadata").id;
    let before = bounds(&mut v, &view, "listitem", "Choose metadata");
    assert!(before.y < bounds(&mut v, &view, "listitem", "Migrate pipeline").y);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("listitem", "Prepare release")
            .next()
            .is_none()
    );
    node(&mut v, &view, "status", "2 need you · 1 failed · 3 done");
    click(&mut v, &view, "button", "Refuse next request");
    click(&mut v, &view, "button", "Decide: Choose metadata");
    v.executor().advance_clock(Duration::from_millis(390));
    ready(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "The Host kept the current snapshot.",
    );
    v.simulate_keystrokes("space");
    ready(&mut v, &view);
    v.executor().advance_clock(Duration::from_millis(390));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "1 needs you · 1 failed · 4 done");
    assert_eq!(node(&mut v, &view, "listitem", "Choose metadata").id, id);
    assert_eq!(bounds(&mut v, &view, "listitem", "Choose metadata"), before);
    let completed = node(&mut v, &view, "button", "Open: Choose metadata").id;
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("listitem", "Choose metadata")
            .next()
            .is_none()
    );
    let tree = snapshot(&mut v, &view);
    let folded = tree
        .nodes()
        .find(|n| n.role == "button" && n.name.starts_with("You were away"))
        .unwrap()
        .id;
    let focus = v
        .update(|win, cx| view.debug_frame_digest(win, cx))
        .unwrap()
        .focus_path;
    assert_eq!(focus.first(), Some(&folded));
    v.simulate_keystrokes("space");
    ready(&mut v, &view);
    let first = bounds(&mut v, &view, "listitem", "Choose metadata");
    assert!(first.y < bounds(&mut v, &view, "listitem", "Migrate pipeline").y);
    let opened = node(&mut v, &view, "button", "Open: Choose metadata").id;
    let focus = v
        .update(|win, cx| view.debug_frame_digest(win, cx))
        .unwrap()
        .focus_path;
    assert_eq!(focus.first(), Some(&opened), "old action {completed}");
    click(&mut v, &view, "button", "Show 3 more");
    node(&mut v, &view, "listitem", "Prepare release");
    click(&mut v, &view, "button", "Show less");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("listitem", "Prepare release")
            .next()
            .is_none()
    );
    click(&mut v, &view, "button", "Show clock time");
    assert!(
        snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.role == "text" && n.name.starts_with("since "))
    );
}
