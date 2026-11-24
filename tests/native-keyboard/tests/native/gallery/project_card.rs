use std::time::Duration;

use gpui::{Modifiers, TestAppContext, point, px, size};
use gpui_rhai::{GeometryBounds, UiValue};

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, center, click_center, snapshot};

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn has_inset_shadow(
    visual: &mut gpui::VisualTestContext,
    owner: GeometryBounds,
    rgba: u32,
) -> bool {
    let expected = gpui::rgba(rgba);
    visual.update(|window, _| {
        let device_scale = window.scale_factor();
        window.visual_probe_with_painted_scene(|scene| {
            let mut present = false;
            crate::support::walk_live_paint(
                scene,
                gpui::TransformationMatrix::default(),
                &mut |scene, placement| {
                    present |= scene.shadows.iter().any(|shadow| {
                        let color = shadow.color.to_rgb();
                        let rect = crate::support::logical_paint_rect(
                            shadow.element_bounds,
                            placement,
                            device_scale,
                        );
                        shadow.inset == 1
                            && [
                                color.r - expected.r,
                                color.g - expected.g,
                                color.b - expected.b,
                                color.a - expected.a,
                            ]
                            .into_iter()
                            .all(|delta| delta.abs() <= 1. / 255.)
                            && [
                                f64::from(rect.x) - owner.x,
                                f64::from(rect.y) - owner.y,
                                f64::from(rect.width) - owner.width,
                                f64::from(rect.height) - owner.height,
                            ]
                            .into_iter()
                            .all(|delta| delta.abs() < 0.75)
                    });
                },
            );
            present
        })
    })
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn project_status_and_picker_use_distinct_native_paints_in_each_design(cx: &mut TestAppContext) {
    use crate::support::{glyphs_with_ink, live_fills};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("project_status_paint"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "project_card", "modern-light");
    v.simulate_resize(size(px(1250.), px(950.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Release logistics");
    let trigger_id = node(&mut v, &view, "button", "In progress").id;
    for (family, appearance, ink, wash, option_ink, selected_fill) in [
        (
            "Modern", "Light", 0xb25e00ff, 0xe8b00024, 0xb25e00ff, 0xe8b0003d,
        ),
        (
            "Modern", "Dark", 0xe6c24dff, 0xe8b00024, 0xe6c24dff, 0xe8b0003d,
        ),
        (
            "Clean", "Light", 0x8f6c00ff, 0xe8b0001a, 0x000000e0, 0x0000000f,
        ),
        (
            "Clean", "Dark", 0xe6c24dff, 0xe6c24d24, 0xffffffe6, 0xffffff14,
        ),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut v, &view);
        let trigger = bounds(&mut v, &view, "button", "In progress");
        assert_eq!(node(&mut v, &view, "button", "In progress").id, trigger_id);
        let fills_owner = |quads: Vec<crate::support::Rect>, owner: GeometryBounds| {
            quads.into_iter().any(|quad| {
                [
                    f64::from(quad.x) - owner.x,
                    f64::from(quad.y) - owner.y,
                    f64::from(quad.width) - owner.width,
                    f64::from(quad.height) - owner.height,
                ]
                .iter()
                .all(|error| error.abs() <= 0.75)
            })
        };
        assert!(
            (trigger.height - 20.).abs() < 0.75,
            "{family}/{appearance}: {trigger:?}"
        );
        assert!(glyphs_with_ink(&mut v, trigger, ink) >= 5);
        assert!(
            fills_owner(live_fills(&mut v, wash), trigger),
            "{family}/{appearance}: status wash"
        );

        click(&mut v, &view, "button", "In progress");
        let option = bounds(&mut v, &view, "radio", "In progress");
        let outlined_owner = if family == "Clean" { trigger } else { option };
        let outline = (ink & 0xffff_ff00) | if family == "Clean" { 0x59 } else { 0x57 };
        assert!(
            has_inset_shadow(&mut v, outlined_owner, outline),
            "{family}/{appearance}: dynamic inset must resolve from the real themed text ink"
        );
        assert!(
            (option.height - 24.).abs() < 0.75,
            "{family}/{appearance}: {option:?}"
        );
        assert!(
            glyphs_with_ink(&mut v, option, option_ink) >= 5,
            "{family}/{appearance}: selected option text"
        );
        assert!(
            fills_owner(live_fills(&mut v, selected_fill), option),
            "{family}/{appearance}: selected option surface"
        );
        let has_dot = live_fills(&mut v, ink).into_iter().any(|dot| {
            (dot.width - 6.).abs() < 0.75
                && (dot.height - 6.).abs() < 0.75
                && (f64::from(dot.y) + 3. - option.y - option.height / 2.).abs() < 0.75
                && f64::from(dot.x) >= option.x
                && f64::from(dot.right()) <= option.x + option.width
        });
        assert_eq!(has_dot, family == "Clean", "{family}/{appearance}");
        v.simulate_keystrokes("escape");
        ready(&mut v, &view);
        assert_eq!(node(&mut v, &view, "button", "In progress").id, trigger_id);
    }
}

#[gpui::test]
fn authored_project_status_and_due_date_remain_in_the_raw_contrast_audit(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("project_status_audit"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "project_card", "modern-light");
    v.simulate_resize(size(px(1250.), px(950.)));
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = crate::support::PresetRisks::new();
    for family in ["Modern", "Clean"] {
        v.update(|_, cx| view.select_theme(family, "Light", cx))
            .unwrap();
        ready(&mut v, &view);
        let (stock, findings) = v.update(|_, cx| {
            (
                view.theme_snapshot(cx).unwrap().variant,
                view.composition_audit_with(&rules, cx).unwrap(),
            )
        });
        for role in ["status/label", "due"] {
            let suffix = format!("/release-card/paper/card/head/1/1/{role}");
            let finding = findings
                .iter()
                .find(|finding| finding.path.ends_with(&suffix));
            if family == "Clean" && role == "status/label" {
                // The independent amber wash restores contrast to 4.54:1.
                // The native paint test independently verifies both colors.
                assert!(
                    finding.is_none(),
                    "Clean amber status should pass: {finding:?}"
                );
                continue;
            }
            let finding = finding
                .unwrap_or_else(|| panic!("{family}: expected strict audit warning at {role}"));
            assert!(risks.record(&stock, finding), "{family}: {finding:?}");
            let mut altered = finding.clone();
            altered.contrast.as_mut().unwrap().background_srgb = Some([0.9; 3]);
            assert!(!risks.record(&stock, &altered));
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn project_card_proposes_only_hit_tasks_and_keeps_header_stable(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "project_card", "modern-light");
    v.simulate_resize(size(px(1250.), px(950.)));
    ready(&mut v, &view);
    let title = bounds(&mut v, &view, "button", "Release logistics");
    click(&mut v, &view, "button", "Release logistics");
    assert_eq!(bounds(&mut v, &view, "button", "Release logistics"), title);
    let tree = snapshot(&mut v, &view);
    let ring = tree
        .nodes()
        .find(|n| n.name == "Task progress")
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    assert_eq!(
        node(&mut v, &view, "checkbox", "Run acceptance checks").checked,
        Some(UiValue::Bool(false))
    );
    // Blank centre: the underlying Canvas must not translate its rectangle into a task.
    click_center(
        &mut v,
        GeometryBounds {
            x: ring.x + ring.width / 2. - 0.25,
            y: ring.y + ring.height / 2. - 0.25,
            width: 0.5,
            height: 0.5,
        },
    );
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("status", "Waiting for the Host…")
            .next()
            .is_none()
    );
    click(&mut v, &view, "button", "Refuse next request");
    // The second of four arcs is the bottom-right quadrant.
    let arc = GeometryBounds {
        x: ring.x + ring.width * 0.775 - 0.25,
        y: ring.y + ring.height * 0.775 - 0.25,
        width: 0.5,
        height: 0.5,
    };
    v.simulate_mouse_move(center(arc), None, Modifiers::none());
    ready(&mut v, &view);
    v.simulate_mouse_move(
        center(arc) + point(px(0.25), px(0.25)),
        None,
        Modifiers::none(),
    );
    ready(&mut v, &view);
    let status = snapshot(&mut v, &view);
    assert!(
        status
            .find_by_role_and_name("status", "Assigned to Lee Park")
            .next()
            .is_some(),
        "{ring:?} {arc:?} statuses: {:?}",
        status
            .nodes()
            .filter(|n| n.role == "status")
            .map(|n| n.name.clone())
            .collect::<Vec<_>>()
    );
    click_center(&mut v, arc);
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Waiting for the Host…");
    v.executor().advance_clock(Duration::from_millis(350));
    ready(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "The Host kept the current snapshot.",
    );
    assert_eq!(
        node(&mut v, &view, "checkbox", "Run acceptance checks").checked,
        Some(UiValue::Bool(false))
    );
    click_center(&mut v, arc);
    ready(&mut v, &view);
    v.executor().advance_clock(Duration::from_millis(350));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Accepted task: qa");
    assert_eq!(
        node(&mut v, &view, "checkbox", "Run acceptance checks").checked,
        Some(UiValue::Bool(true))
    );
    click(&mut v, &view, "button", "In progress");
    click(&mut v, &view, "radio", "In review");
    v.executor().advance_clock(Duration::from_millis(350));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Accepted project status");
    node(&mut v, &view, "button", "In review");
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "button", "Release logistics").expanded,
        Some(false)
    );
    assert_eq!(bounds(&mut v, &view, "button", "Release logistics"), title);
}

#[gpui::test]
fn project_card_hover_disclosure_and_pinned_member_are_distinct(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "project_card", "clean-dark");
    v.simulate_resize(size(px(1250.), px(950.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Open on hover");
    let title = bounds(&mut v, &view, "button", "Release logistics");
    v.simulate_mouse_move(center(title), None, Modifiers::none());
    v.executor().advance_clock(Duration::from_millis(150));
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "button", "Release logistics").expanded,
        Some(true)
    );
    click(&mut v, &view, "button", "Ava Chen");
    let at = point(
        px((title.x + title.width / 2.) as f32),
        px((title.y + 3.) as f32),
    );
    v.simulate_mouse_move(at, None, Modifiers::none());
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Ava Chen · 1 of 2 done");
    let outside = bounds(&mut v, &view, "button", "Open on click");
    v.simulate_mouse_move(center(outside), None, Modifiers::none());
    v.executor().advance_clock(Duration::from_millis(270));
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "button", "Release logistics").expanded,
        Some(false)
    );
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
