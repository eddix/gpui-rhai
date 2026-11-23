use std::time::Duration;

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::ScriptViewHandle;

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, snapshot};

const CLOSED: &str = "Background tasks: 2 running, 1 needs you";

#[gpui::test]
fn running_now_trigger_paints_preset_ink_and_surface_without_a_second_button(
    cx: &mut TestAppContext,
) {
    use crate::support::{PresetRisks, glyphs_with_ink, live_fills};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("running_now_trigger_paint"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "running_now", "modern-light");
    v.simulate_resize(size(px(1250.), px(940.)));
    ready(&mut v, &view);
    let id = node(&mut v, &view, "button", CLOSED).id;
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for (family, appearance, height, paper, caption, separator) in [
        ("Modern", "Light", 30., 0xf0f0f0ff, 0x8a8a8aff, 0xb8b8b8ff),
        ("Modern", "Dark", 30., 0x19191cff, 0x7c7c82ff, 0x4a4a50ff),
        ("Clean", "Light", 28., 0xffffffff, 0x0000009c, 0x00000045),
        ("Clean", "Dark", 28., 0x1c1c1eff, 0xffffff9e, 0xffffff4d),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut v, &view);
        let trigger = bounds(&mut v, &view, "button", CLOSED);
        assert!(
            (trigger.height - height).abs() < 0.75,
            "{family}/{appearance}: {trigger:?}"
        );
        assert_eq!(node(&mut v, &view, "button", CLOSED).id, id);
        assert!(
            glyphs_with_ink(&mut v, trigger, caption) >= 12,
            "{family}/{appearance}: caption ink"
        );
        assert_eq!(
            glyphs_with_ink(&mut v, trigger, separator),
            1,
            "{family}/{appearance}: separator"
        );
        assert!(
            live_fills(&mut v, paper).iter().any(|quad| [
                f64::from(quad.x) - trigger.x,
                f64::from(quad.y) - trigger.y,
                f64::from(quad.width) - trigger.width,
                f64::from(quad.height) - trigger.height,
            ]
            .iter()
            .all(|delta| delta.abs() <= 0.75)),
            "{family}/{appearance}: trigger fill"
        );
        assert!(!snapshot(&mut v, &view).nodes().any(|n| n.name == "·"));
        let (stock, findings) = v.update(|_, cx| {
            (
                view.theme_snapshot(cx).unwrap().variant,
                view.composition_audit_with(&rules, cx).unwrap(),
            )
        });
        for suffix in [
            "/trigger/running/caption",
            "/trigger/dot",
            "/trigger/waiting/caption",
        ] {
            let actual = findings.iter().find(|f| f.path.ends_with(suffix));
            if suffix.ends_with("/dot") || family == "Modern" {
                let finding = actual
                    .unwrap_or_else(|| panic!("{family}/{appearance}: missing warning {suffix}"));
                assert!(
                    risks.record(&stock, finding),
                    "{family}/{appearance}: {finding:?}"
                );
                let mut wrong = finding.clone();
                wrong.path.push_str("/unreviewed");
                assert!(!risks.record(&stock, &wrong));
            } else {
                assert!(
                    actual.is_none(),
                    "{family}/{appearance}: unexpected {actual:?}"
                );
            }
        }
    }
    risks.assert_reported();
}

fn key(v: &mut VisualTestContext, view: &ScriptViewHandle, stroke: &str) {
    v.simulate_keystrokes(stroke);
    ready(v, view);
}

fn settled(v: &mut VisualTestContext, view: &ScriptViewHandle, millis: u64) {
    v.executor().advance_clock(Duration::from_millis(millis));
    ready(v, view);
}

fn task_opened(v: &mut VisualTestContext, view: &ScriptViewHandle, id: &str, stage: &str) {
    let tree = snapshot(v, view);
    let messages: Vec<_> = tree
        .nodes()
        .filter(|n| n.role == "status")
        .map(|n| n.name.as_str())
        .collect();
    assert!(
        messages.contains(&format!("Open task: {id}").as_str()),
        "{stage}: {messages:?}"
    );
}

#[gpui::test]
fn running_now_requests_changes_without_optimistic_status(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "running_now", "modern-light");
    v.simulate_resize(size(px(1220.), px(880.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Refuse next change");
    click(&mut v, &view, "button", CLOSED);
    node(&mut v, &view, "dialog", "Background tasks");
    let main = node(&mut v, &view, "button", "Index documents");
    assert!(main.description.contains("2 of 4 steps"));

    click(&mut v, &view, "button", "Pause Index documents");
    node(&mut v, &view, "status", "Waiting for the Host…");
    node(&mut v, &view, "button", CLOSED);
    assert_eq!(
        node(&mut v, &view, "button", "Pause Index documents").description,
        "Waiting for acknowledgement"
    );
    settled(&mut v, &view, 370);
    node(&mut v, &view, "status", "The Host refused the change.");
    node(&mut v, &view, "button", "Pause Index documents");

    click(&mut v, &view, "button", "Pause Index documents");
    settled(&mut v, &view, 370);
    node(&mut v, &view, "status", "Accepted pause for index");
    node(
        &mut v,
        &view,
        "button",
        "Background tasks: 1 running, 1 needs you",
    );
    let next = node(&mut v, &view, "button", "Index documents");
    assert_eq!(next.id, main.id);
    assert!(next.description.starts_with("Paused"));
    click(&mut v, &view, "button", "Resume Index documents");
    settled(&mut v, &view, 370);
    node(&mut v, &view, "status", "Accepted resume for index");
    node(&mut v, &view, "button", CLOSED);
    click(&mut v, &view, "button", "Open Prepare report");
    node(&mut v, &view, "status", "Open task: report");
    click(&mut v, &view, "button", "Open Tasks");
    node(&mut v, &view, "status", "Open full task list");
}

#[gpui::test]
fn running_now_keyboard_and_empty_popover_survive_live_theme(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "running_now", "clean-dark");
    v.simulate_resize(size(px(1220.), px(880.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", CLOSED);
    key(&mut v, &view, "end");
    key(&mut v, &view, "enter");
    task_opened(&mut v, &view, "export", "End in the popover");
    v.update(|_, cx| assert!(view.select_theme("Modern", "Light", cx).unwrap()));
    ready(&mut v, &view);
    key(&mut v, &view, "home");
    key(&mut v, &view, "enter");
    task_opened(&mut v, &view, "index", "Home after a theme change");
    key(&mut v, &view, "escape");
    assert_eq!(node(&mut v, &view, "button", CLOSED).expanded, Some(false));
    key(&mut v, &view, "up");
    settled(&mut v, &view, 45);
    assert_eq!(node(&mut v, &view, "button", CLOSED).expanded, Some(true));
    key(&mut v, &view, "enter");
    task_opened(&mut v, &view, "export", "ArrowUp opens at the last task");
    key(&mut v, &view, "escape");

    click(&mut v, &view, "button", "Finish indexing");
    node(&mut v, &view, "status", "Index documents: Done");
    click(&mut v, &view, "button", "Empty list");
    click(
        &mut v,
        &view,
        "button",
        "Background tasks: No tasks running",
    );
    let panel = bounds(&mut v, &view, "dialog", "Background tasks");
    assert!(panel.width <= 350., "{panel:?}");
    assert!(
        snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.name == "Nothing running right now")
    );
    assert_eq!(
        node(
            &mut v,
            &view,
            "button",
            "Background tasks: No tasks running"
        )
        .expanded,
        Some(true)
    );
    node(&mut v, &view, "button", "Open Tasks");
    key(&mut v, &view, "tab");
    assert_eq!(
        node(
            &mut v,
            &view,
            "button",
            "Background tasks: No tasks running"
        )
        .expanded,
        Some(false)
    );
}

#[gpui::test]
fn running_now_retained_popup_moves_focus_when_a_focused_task_is_removed(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "running_now", "clean-light");
    v.simulate_resize(size(px(1220.), px(880.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Remove first task on next open");
    click(&mut v, &view, "button", CLOSED);
    settled(&mut v, &view, 530);
    node(&mut v, &view, "dialog", "Background tasks");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Index documents")
            .next()
            .is_none()
    );
    key(&mut v, &view, "enter");
    task_opened(&mut v, &view, "report", "first surviving task");
}
