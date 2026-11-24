//! Gallery acceptance gates: every page passes the productivity audit in both
//! densities, and the scenes can be completed by keyboard alone.

use gpui::{TestAppContext, VisualTestContext, WindowHandle};
use gpui_rhai::*;
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::mount_prepared;
use crate::support::{Host, PresetRisks, Step, click_center, last_error, settle, snapshot};

fn mount(
    cx: &mut TestAppContext,
    launch: &AcceptanceLaunch,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    let prepared = acceptance::view(launch)
        .unwrap()
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
    mount_prepared(cx, prepared, ("gallery", "gallery"), false, |host, view| {
        Host { host, view }
    })
}

#[gpui::test]
fn gallery_number_input_holds_and_releases_a_real_stepper(cx: &mut TestAppContext) {
    use crate::support::{bounds, center};
    use gpui::{Modifiers, MouseButton};
    use std::time::Duration;
    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let (window, view) = mount(
            cx,
            &AcceptanceLaunch {
                page: "number_input".into(),
                theme: theme.into(),
                ..Default::default()
            },
        );
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, _| window.activate_window());
        settle(&mut visual, Step::Frame, 4);
        let value = |visual: &mut VisualTestContext| {
            snapshot(visual, &view)
                .find_by_role_and_name("spinbutton", "Token budget")
                .next()
                .unwrap()
                .value
                .clone()
        };
        assert_eq!(value(&mut visual), Some(UiValue::Integer(4000)));
        let stepper = center(bounds(&mut visual, &view, "button", "Increase"));
        visual.simulate_mouse_move(stepper, None, Modifiers::none());
        visual.simulate_mouse_down(stepper, MouseButton::Left, Modifiers::none());
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(value(&mut visual), Some(UiValue::Float(4100.0)), "{theme}");
        for delay in [400, 120] {
            visual
                .executor()
                .advance_clock(Duration::from_millis(delay));
            settle(&mut visual, Step::Frame, 3);
        }
        assert_eq!(value(&mut visual), Some(UiValue::Float(4300.0)), "{theme}");
        visual.simulate_mouse_up(stepper, MouseButton::Left, Modifiers::none());
        visual.executor().advance_clock(Duration::from_millis(500));
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(value(&mut visual), Some(UiValue::Float(4300.0)), "{theme}");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn gallery_otp_input_reaches_verification_and_never_echoes_the_code_semantically(
    cx: &mut TestAppContext,
) {
    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let (window, view) = mount(
            cx,
            &AcceptanceLaunch {
                page: "otp_input".into(),
                theme: theme.into(),
                ..Default::default()
            },
        );
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, _| window.activate_window());
        settle(&mut visual, Step::Frame, 4);
        let editor = crate::support::bounds(&mut visual, &view, "text_field", "Verification code");
        click_center(&mut visual, editor);
        visual.simulate_input("０５٨321");
        settle(&mut visual, Step::Frame, 5);
        let tree = snapshot(&mut visual, &view);
        assert!(
            tree.find_by_role_and_name("status", "Verified")
                .next()
                .is_some(),
            "{theme}: expected Gallery completion"
        );
        assert!(
            !tree.nodes().any(|node| node.name.contains("058321")
                || node.value == Some(UiValue::String("058321".into()))),
            "{theme}: the code is not an audit label"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn gallery_date_range_picker_commits_the_comparison_workflow(cx: &mut TestAppContext) {
    use crate::support::bounds;
    use gpui::{px, size};

    let (window, view) = mount(
        cx,
        &AcceptanceLaunch {
            page: "date_range_picker".into(),
            theme: "modern-dark".into(),
            ..Default::default()
        },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, _| window.activate_window());
    visual.simulate_resize(size(px(1240.), px(790.)));
    settle(&mut visual, Step::Tick, 4);
    for (role, name) in [
        ("combobox", "Reporting period"),
        ("option", "Last 7 days"),
        ("button", "Previous period"),
    ] {
        let target = bounds(&mut visual, &view, role, name);
        click_center(&mut visual, target);
        settle(&mut visual, Step::Tick, 4);
        assert_eq!(last_error(&mut visual, &view), None);
    }
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Comparison off")
    );
    let apply = bounds(&mut visual, &view, "button", "Apply");
    click_center(&mut visual, apply);
    settle(&mut visual, Step::Tick, 4);
    let tree = snapshot(&mut visual, &view);
    let trigger = tree
        .find_by_role_and_name("combobox", "Reporting period")
        .next()
        .unwrap();
    assert_eq!(trigger.expanded, Some(false));
    assert!(
        tree.nodes()
            .any(|n| n.name == "Comparison: Oct 3, 2026 – Oct 9, 2026")
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn gallery_tag_input_accepts_paste_and_remove_in_the_real_page(cx: &mut TestAppContext) {
    use crate::support::bounds;
    use gpui::ClipboardItem;

    let (window, view) = mount(
        cx,
        &AcceptanceLaunch {
            page: "tag_input".into(),
            theme: "modern-dark".into(),
            ..Default::default()
        },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Tick, 4);
    let editor = bounds(&mut visual, &view, "text_field", "Project labels");
    click_center(&mut visual, editor);
    visual.update(|_, cx| {
        cx.write_to_clipboard(ClipboardItem::new_string("Review\nAccessibility".into()));
    });
    visual.simulate_keystrokes("cmd-v");
    settle(&mut visual, Step::Tick, 4);
    let remove = bounds(&mut visual, &view, "button", "Remove Review");
    click_center(&mut visual, remove);
    settle(&mut visual, Step::Tick, 4);
    visual.simulate_input("Release,");
    settle(&mut visual, Step::Tick, 4);
    let tree = snapshot(&mut visual, &view);
    for name in ["Remove Accessibility", "Remove Release"] {
        assert!(
            tree.find_by_role_and_name("button", name).next().is_some(),
            "{name}"
        );
    }
    assert!(
        tree.find_by_role_and_name("button", "Remove Review")
            .next()
            .is_none()
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn gallery_filter_menu_proposes_the_applied_filters_in_the_real_page(cx: &mut TestAppContext) {
    use crate::support::bounds;
    let (window, view) = mount(
        cx,
        &AcceptanceLaunch {
            page: "filter_menu".into(),
            theme: "clean-dark".into(),
            ..Default::default()
        },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|w, _| w.activate_window());
    settle(&mut visual, Step::Tick, 5);
    let target = bounds(&mut visual, &view, "checkbox", "Active");
    click_center(&mut visual, target);
    settle(&mut visual, Step::Tick, 5);
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Applied selections: 4")
    );
    let clear = bounds(&mut visual, &view, "button", "Clear all");
    click_center(&mut visual, clear);
    settle(&mut visual, Step::Tick, 5);
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Applied selections: 0")
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn gallery_stepper_form_validates_edits_and_saves_the_real_workflow(cx: &mut TestAppContext) {
    use crate::support::bounds;
    use gpui::{px, size};
    use std::time::Duration;

    let (window, view) = mount(
        cx,
        &AcceptanceLaunch {
            page: "stepper_form".into(),
            theme: "modern-light".into(),
            ..Default::default()
        },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, _| window.activate_window());
    visual.simulate_resize(size(px(1240.), px(820.)));
    settle(&mut visual, Step::Tick, 6);
    let press = |visual: &mut VisualTestContext, name: &str| {
        let target = bounds(visual, &view, "button", name);
        click_center(visual, target);
        settle(visual, Step::Tick, 6);
        assert_eq!(last_error(visual, &view), None);
    };
    press(&mut visual, "Continue");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "alert" && n.name == "Enter a workspace name.")
    );
    visual.simulate_input("Design");
    settle(&mut visual, Step::Tick, 6);
    press(&mut visual, "Continue");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "text_field" && n.name == "Invite teammates")
    );
    press(&mut visual, "Continue");
    press(&mut visual, "Continue");
    press(&mut visual, "Edit Workspace name");
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("Design Lab");
    settle(&mut visual, Step::Tick, 6);
    press(&mut visual, "Back to Review");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Design Lab")
    );
    press(&mut visual, "Finish");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Submitting…" && n.disabled)
    );
    visual.executor().advance_clock(Duration::from_millis(700));
    settle(&mut visual, Step::Tick, 6);
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "All set")
    );
    press(&mut visual, "Start over");
    let snapshot = snapshot(&mut visual, &view);
    let field = snapshot
        .find_by_role_and_name("text_field", "Workspace name")
        .next()
        .unwrap();
    assert_eq!(field.value, Some(UiValue::String(String::new())));
}

#[gpui::test]
fn gallery_tag_input_audit_checks_layout_and_keeps_stock_palette_findings(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "tag_input");
}

#[gpui::test]
fn gallery_filter_menu_audit_checks_layout_and_keeps_stock_palette_findings(
    cx: &mut TestAppContext,
) {
    audit_page_layout_matrix(cx, "filter_menu");
}

#[gpui::test]
fn gallery_stepper_form_audit_checks_layout_and_keeps_stock_palette_findings(
    cx: &mut TestAppContext,
) {
    audit_page_layout_matrix(cx, "stepper_form");
}

#[gpui::test]
fn gallery_progress_tracker_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "progress_tracker");
}

#[gpui::test]
fn gallery_setup_guide_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "setup_guide");
}

#[gpui::test]
fn gallery_reaction_bar_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "reaction_bar");
}

#[gpui::test]
fn gallery_message_actions_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "message_actions");
}

#[gpui::test]
fn gallery_model_picker_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "model_picker");
}

#[gpui::test]
fn gallery_cited_answer_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "cited_answer");
}

#[gpui::test]
fn gallery_ai_answer_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "ai_answer");
}

#[gpui::test]
fn gallery_thinking_loader_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "thinking_loader");
}

#[gpui::test]
fn gallery_thinking_state_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "thinking_state");
}

#[gpui::test]
fn gallery_run_controls_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "run_controls");
}

#[gpui::test]
fn gallery_prompt_library_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "prompt_library");
}

#[gpui::test]
fn gallery_integration_picker_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "integration_picker");
}

#[gpui::test]
fn gallery_integration_settings_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "integration_settings");
}

#[gpui::test]
fn gallery_comment_thread_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "comment_thread");
}

#[gpui::test]
fn gallery_comments_panel_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "comments_panel");
}

#[gpui::test]
fn gallery_writing_assistant_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "writing_assistant");
}

#[gpui::test]
fn gallery_approval_card_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "approval_card");
}

#[gpui::test]
fn gallery_permission_request_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "permission_request");
}

#[gpui::test]
fn gallery_decision_card_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "decision_card");
}

#[gpui::test]
fn gallery_decision_inbox_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "decision_inbox");
}

#[gpui::test]
fn gallery_decision_tree_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "decision_tree");
}

#[gpui::test]
fn gallery_review_center_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "review_center");
}

#[gpui::test]
fn gallery_delegate_board_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "delegate_board");
}

#[gpui::test]
fn gallery_agent_queue_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_queue");
}

#[gpui::test]
fn gallery_agent_lanes_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_lanes");
}

#[gpui::test]
fn gallery_agent_handoff_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_handoff");
}

#[gpui::test]
fn gallery_agent_inbox_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_inbox");
}

#[gpui::test]
fn gallery_agent_plan_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_plan");
}

#[gpui::test]
fn gallery_agent_checkpoints_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_checkpoints");
}

#[gpui::test]
fn gallery_agent_trail_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_trail");
}

#[gpui::test]
fn gallery_agent_tree_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_tree");
}

#[gpui::test]
fn gallery_agent_graph_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_graph");
}

#[gpui::test]
fn gallery_agent_fleet_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_fleet");
}

#[gpui::test]
fn gallery_agent_monitor_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_monitor");
}

#[gpui::test]
fn gallery_agent_steering_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "agent_steering");
}

#[gpui::test]
fn gallery_agent_recap_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix_with_motion(cx, "agent_recap", gpui_rhai::MotionPreference::None);
}

#[gpui::test]
fn gallery_agent_findings_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix_with_motion(cx, "agent_findings", gpui_rhai::MotionPreference::None);
}

#[gpui::test]
fn gallery_agent_ledger_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix_with_motion(cx, "agent_ledger", gpui_rhai::MotionPreference::None);
}

#[gpui::test]
fn gallery_trace_waterfall_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix_with_motion(cx, "trace_waterfall", gpui_rhai::MotionPreference::None);
}

#[gpui::test]
fn gallery_approval_gates_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "approval_gates");
}

#[gpui::test]
fn gallery_autonomy_dial_audit_at_all_scales(cx: &mut TestAppContext) {
    audit_page_layout_matrix(cx, "autonomy_dial");
}

fn audit_page_layout_matrix(cx: &mut TestAppContext, page: &str) {
    audit_page_layout_matrix_with_motion(cx, page, gpui_rhai::MotionPreference::Normal);
}

fn audit_page_layout_matrix_with_motion(
    cx: &mut TestAppContext,
    page: &str,
    motion: gpui_rhai::MotionPreference,
) {
    let mut risks = PresetRisks::new();
    let mut failures = Vec::new();
    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let (window, view) = mount(
            cx,
            &AcceptanceLaunch {
                page: page.into(),
                theme: theme.into(),
                motion,
                ..Default::default()
            },
        );
        let mut visual = VisualTestContext::from_window(*window, cx);
        for density in ["comfortable", "compact"] {
            action(
                &mut visual,
                &view,
                "gallery.set_density",
                UiValue::String(density.to_owned()),
            );
            for scale in ["100", "125", "150"] {
                action(
                    &mut visual,
                    &view,
                    "gallery.set_scale",
                    UiValue::String(scale.to_owned()),
                );
                assert_eq!(
                    last_error(&mut visual, &view),
                    None,
                    "{theme}/{density}/{scale}"
                );
                let variant = visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant);
                let findings =
                    visual.update(|_, cx| view.composition_audit_with(&rules(), cx).unwrap());
                let unexpected: Vec<_> = findings
                    .into_iter()
                    .filter(|finding| !risks.record(&variant, finding))
                    .collect();
                for finding in unexpected {
                    failures.push(format!(
                        "{theme}/{density}/{scale}: {finding}; evidence={:?}",
                        finding.contrast
                    ));
                }
                let unreachable =
                    visual.update(|_, cx| view.debug_unreachable_controls(cx).unwrap());
                for finding in unreachable {
                    failures.push(format!("{theme}/{density}/{scale}: {finding}"));
                }
            }
        }
    }
    risks.assert_reported();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn try_action(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    id: &str,
    payload: UiValue,
) -> Result<(), String> {
    let result = visual.update(|window, cx| {
        view.automate(
            AutomationCommand::Action {
                id: id.to_owned(),
                payload: Some(payload),
            },
            window,
            cx,
        )
    });
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    result.map(|_| ()).map_err(|error| error.to_string())
}

pub(super) fn action(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    id: &str,
    payload: UiValue,
) {
    try_action(visual, view, id, payload).unwrap();
}

/// Every rule except font resolution: the test platform ships no system fonts.
fn rules() -> AuditRules {
    AuditRules::from_ids(
        AuditRule::ALL
            .into_iter()
            .filter(|rule| *rule != AuditRule::UnresolvedFont)
            .map(AuditRule::id),
    )
    .unwrap()
}

#[gpui::test]
fn overview_metadata_risks_keep_the_real_compact_and_schedule_surfaces(cx: &mut TestAppContext) {
    use super::conversation::{gallery, ready};
    use gpui::{px, size};
    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for (page, presets, required) in [
        (
            "compact_breakdown",
            &["modern-light", "clean-light"][..],
            &["/figure/delta/1", "/panel/caption", "/footer/0"][..],
        ),
        (
            "scheduled_runs",
            &["modern-light", "modern-dark", "clean-light", "clean-dark"][..],
            &[
                "/task:audit/detail/trigger/summary",
                "/part:release-window/detail/trigger/summary",
            ][..],
        ),
    ] {
        for &preset in presets {
            let (mut v, view) = gallery(cx, page, preset);
            v.simulate_resize(size(px(1380.), px(980.)));
            ready(&mut v, &view);
            let (theme, findings) = v.update(|_, cx| {
                (
                    view.theme_snapshot(cx).unwrap().variant,
                    view.composition_audit_with(&rules, cx).unwrap(),
                )
            });
            for tail in required {
                let finding = findings
                    .iter()
                    .find(|finding| finding.path.ends_with(tail))
                    .unwrap_or_else(|| panic!("{page}/{preset} must still report {tail}"));
                assert!(
                    risks.record(&theme, finding),
                    "{page}/{preset}: {finding:#?}"
                );
                let mut altered = finding.clone();
                altered.contrast.as_mut().unwrap().background_srgb = Some([0.5; 3]);
                assert!(!risks.record(&theme, &altered));
            }
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn conflict_radar_directory_risks_require_their_actual_row_surfaces(cx: &mut TestAppContext) {
    use super::conversation::{gallery, ready};
    use gpui::{px, size};

    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for preset in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "conflict_radar", preset);
        v.simulate_resize(size(px(1280.), px(900.)));
        ready(&mut v, &view);
        let (theme, findings) = v.update(|_, cx| {
            (
                view.theme_snapshot(cx).unwrap().variant,
                view.composition_audit_with(&rules, cx).unwrap(),
            )
        });
        let subject: Vec<_> = findings
            .iter()
            .filter(|f| f.path.contains("/sample/branches/card/") && f.path.ends_with("/directory"))
            .collect();
        assert_eq!(subject.len(), 3, "{preset}: {subject:?}");
        for finding in subject {
            assert!(risks.record(&theme, finding), "{preset}: {finding:?}");
            let mut wrong_paper = finding.clone();
            wrong_paper.contrast.as_mut().unwrap().background_srgb = Some([0.5; 3]);
            assert!(!risks.record(&theme, &wrong_paper), "{preset}");
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn scheduling_tool_and_billing_risks_keep_their_stock_surface(cx: &mut TestAppContext) {
    use super::conversation::{gallery, ready};
    use gpui::{px, size};
    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for (page, field, appearances) in [
        (
            "agent_steering",
            "/note:accepted/copy/tag/1",
            &["modern-light", "clean-light"][..],
        ),
        (
            "scheduled_runs",
            "/head/heading-text/1",
            &["modern-light", "clean-light"][..],
        ),
        (
            "tool_access_map",
            "/sized/case/head/4",
            &["modern-light", "modern-dark", "clean-light", "clean-dark"][..],
        ),
        (
            "billing_schedule",
            "/totals/invoice/0",
            &["modern-light", "clean-light"][..],
        ),
        (
            "billing_schedule",
            "/details/summary/range/separator",
            &["clean-dark"][..],
        ),
    ] {
        for &preset in appearances {
            let (mut v, view) = gallery(cx, page, preset);
            v.simulate_resize(size(px(1280.), px(900.)));
            for density in ["comfortable", "compact"] {
                action(
                    &mut v,
                    &view,
                    "gallery.set_density",
                    UiValue::String(density.to_owned()),
                );
                ready(&mut v, &view);
                let (theme, findings) = v.update(|_, cx| {
                    (
                        view.theme_snapshot(cx).unwrap().variant,
                        view.composition_audit_with(&rules, cx).unwrap(),
                    )
                });
                let anchor = findings
                    .iter()
                    .find(|finding| finding.path.ends_with(field))
                    .unwrap_or_else(|| panic!("{page}/{preset}/{density}: missing raw warning"));
                assert!(
                    risks.record(&theme, anchor),
                    "{preset}/{density}: {anchor:#?}"
                );
                let mut altered = anchor.clone();
                altered.contrast.as_mut().unwrap().background_srgb = Some([0.5; 3]);
                assert!(!risks.record(&theme, &altered));
                if page == "tool_access_map" {
                    let unverified = findings
                        .iter()
                        .filter(|f| f.path.contains("/sized/case/") && !risks.record(&theme, f))
                        .collect::<Vec<_>>();
                    assert!(unverified.is_empty(), "{preset}/{density}: {unverified:#?}");
                }
            }
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn quota_and_diff_fixture_risks_have_independent_paint_evidence(cx: &mut TestAppContext) {
    use super::conversation::{gallery, ready};
    use gpui::{px, size};

    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    let mut unreviewed = Vec::new();
    for (page, specimen, witness) in [
        ("usage_meter", "/width/usage-card/", "/head/header-title/1"),
        ("diff_timeline", "/sized/timeline/", "/head/1/1/directory"),
    ] {
        let (mut v, view) = gallery(cx, page, "modern-light");
        v.simulate_resize(size(px(1280.), px(900.)));
        for (design, appearance) in [
            ("Modern", "Light"),
            ("Modern", "Dark"),
            ("Clean", "Light"),
            ("Clean", "Dark"),
        ] {
            v.update(|_, cx| view.select_theme(design, appearance, cx))
                .unwrap();
            for density in ["comfortable", "compact"] {
                action(
                    &mut v,
                    &view,
                    "gallery.set_density",
                    UiValue::String(density.to_owned()),
                );
                ready(&mut v, &view);
                let (theme, findings) = v.update(|_, cx| {
                    (
                        view.theme_snapshot(cx).unwrap().variant,
                        view.composition_audit_with(&rules, cx).unwrap(),
                    )
                });
                let rows = findings.iter().filter(|row| row.path.contains(specimen));
                if page == "diff_timeline" || appearance == "Light" {
                    let anchor = rows
                        .clone()
                        .find(|row| row.path.ends_with(witness))
                        .expect("authored paint still appears in the raw audit");
                    assert!(risks.record(&theme, anchor));
                    let mut other = anchor.clone();
                    other.contrast.as_mut().unwrap().background_srgb = Some([0.42; 3]);
                    assert!(!risks.record(&theme, &other));
                }
                for row in rows {
                    if !risks.record(&theme, row) {
                        unreviewed.push(format!(
                            "{page}/{design}/{appearance}/{density}: {row}, {:?}",
                            row.contrast
                        ));
                    }
                }
            }
        }
    }
    risks.assert_reported();
    assert!(unreviewed.is_empty(), "{}", unreviewed.join("\n"));
}

#[gpui::test]
fn memory_rate_and_judge_stock_captions_match_only_their_actual_paint(cx: &mut TestAppContext) {
    use super::conversation::{gallery, ready};
    use gpui::{px, size};

    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for (page, slot, palettes) in [
        (
            "memory_panel",
            "/sized/case/0/0/0/total/1",
            &["modern-light"][..],
        ),
        (
            "rate_limit_planner",
            "/sized/case/heading/0/1",
            &["modern-light", "clean-light"][..],
        ),
        (
            "judge_bracket",
            "/answers/answer:0/1",
            &["modern-light", "modern-dark", "clean-light", "clean-dark"][..],
        ),
    ] {
        for &preset in palettes {
            let (mut visual, view) = gallery(cx, page, preset);
            visual.simulate_resize(size(px(1450.), px(1040.)));
            for density in ["comfortable", "compact"] {
                action(
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
                let finding = findings
                    .iter()
                    .find(|f| f.path.ends_with(slot))
                    .unwrap_or_else(|| {
                        panic!("{page}/{preset}/{density}: fixture lost its raw warning")
                    });
                assert!(
                    risks.record(&theme, finding),
                    "{page}/{preset}/{density}: {finding:#?}"
                );
                let mut other_paper = finding.clone();
                other_paper.contrast.as_mut().unwrap().background_srgb = Some([0.5; 3]);
                assert!(!risks.record(&theme, &other_paper));
            }
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn fallback_and_collaboration_authored_risks_keep_their_exact_paint(cx: &mut TestAppContext) {
    use super::conversation::{gallery, ready};
    use gpui::{px, size};

    let mut risks = PresetRisks::new();
    let contrast = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    for (page, specimen, anchor) in [
        (
            "fallback_chain",
            "/sized/fallback-chain/",
            "/attempt:sprout/trigger/attempt-content/logo/0",
        ),
        (
            "collaboration_stream",
            "/sized/activity/",
            "/m:confirm/0/1/body/meta/at",
        ),
    ] {
        let (mut visual, view) = gallery(cx, page, "modern-light");
        visual.simulate_resize(size(px(1450.), px(1080.)));
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
                action(
                    &mut visual,
                    &view,
                    "gallery.set_density",
                    UiValue::String(density.to_owned()),
                );
                ready(&mut visual, &view);
                let (theme, findings) = visual.update(|_, cx| {
                    (
                        view.theme_snapshot(cx).unwrap().variant,
                        view.composition_audit_with(&contrast, cx).unwrap(),
                    )
                });
                let reported = findings.iter().filter(|f| f.path.contains(specimen));
                let example = reported
                    .clone()
                    .find(|f| f.path.ends_with(anchor))
                    .unwrap_or_else(|| {
                        panic!("{page}/{family}/{appearance}/{density}: missing raw warning")
                    });
                assert!(risks.record(&theme, example), "{example:?}");
                let mut wrong = example.clone();
                wrong.contrast.as_mut().unwrap().element_opacity *= 0.8;
                assert!(!risks.record(&theme, &wrong));
                let open = reported
                    .filter(|f| !risks.record(&theme, f))
                    .collect::<Vec<_>>();
                assert!(
                    open.is_empty(),
                    "{page}/{family}/{appearance}/{density}: {open:#?}"
                );
            }
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn gallery_equal_header_roles_keep_their_line_boxes_at_reading_scales(cx: &mut TestAppContext) {
    use super::conversation::{gallery, ready};
    use crate::support::bounds;
    use gpui::{px, size};

    for (page, labels, line_boxes) in [
        (
            "context_map",
            [("text", "Context"), ("text", "Atlas")],
            [20., 20.],
        ),
        (
            "judge_bracket",
            [
                ("heading", "Judge"),
                ("text", "Which draft explains the change best?"),
            ],
            [20., 20.],
        ),
        (
            "project_card",
            [("text", "In progress"), ("text", "Oct 24")],
            [20., 16.],
        ),
    ] {
        let (mut visual, view) = gallery(cx, page, "modern-dark");
        visual.simulate_resize(size(px(1400.), px(1080.)));
        let mixed = AuditRules::from_ids(["mixed-type-in-row"]).unwrap();
        for (design, appearance, line) in [
            ("Modern", "Dark", line_boxes[0]),
            ("Clean", "Light", line_boxes[1]),
        ] {
            visual
                .update(|_, cx| view.select_theme(design, appearance, cx))
                .unwrap();
            for (scale, factor) in [("100", 1.), ("125", 1.25), ("150", 1.5)] {
                action(
                    &mut visual,
                    &view,
                    "gallery.set_scale",
                    UiValue::String(scale.into()),
                );
                ready(&mut visual, &view);
                let [first, second] =
                    labels.map(|(role, name)| bounds(&mut visual, &view, role, name));
                assert!(
                    (first.height - line * factor).abs() <= 1.
                        && (first.height - second.height).abs() <= 1.
                        && (first.y - second.y).abs() <= 1.,
                    "{page}/{design}/{scale}: {first:?} {second:?}"
                );
                let findings =
                    visual.update(|_, cx| view.composition_audit_with(&mixed, cx).unwrap());
                assert!(findings.is_empty(), "{page}/{design}/{scale}: {findings:?}");
            }
        }
    }
}

#[gpui::test]
fn gallery_reviewed_text_aligns_in_every_preset_and_density(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx, &AcceptanceLaunch::default());
    let mut visual = VisualTestContext::from_window(*window, cx);
    let rules = AuditRules::from_ids(["mixed-type-in-row", "text-edge-misaligned"]).unwrap();
    let pages = [
        "usage_meter",
        "compact_breakdown",
        "memory_panel",
        "cost_waterfall",
        "fallback_chain",
        "profile_menu",
        "judge_bracket",
        "column_browser",
        "email_draft",
        "comment_thread",
    ];
    let mut findings = Vec::new();
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
            action(
                &mut visual,
                &view,
                "gallery.set_density",
                UiValue::String(density.into()),
            );
            for scale in ["100", "125", "150"] {
                action(
                    &mut visual,
                    &view,
                    "gallery.set_scale",
                    UiValue::String(scale.into()),
                );
                for page in pages {
                    action(
                        &mut visual,
                        &view,
                        "gallery.go",
                        UiValue::String(page.into()),
                    );
                    super::conversation::ready(&mut visual, &view);
                    for item in
                        visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap())
                    {
                        if item.path.contains(&format!("/page-{page}/")) {
                            findings.push(format!(
                                "{family}/{appearance}/{density}/{scale}/{page}: {item}"
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(findings.is_empty(), "{}", findings.join("\n"));
}

#[gpui::test]
fn every_gallery_page_passes_the_audit_in_all_four_presets_and_both_densities(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx, &AcceptanceLaunch::default());
    let mut visual = VisualTestContext::from_window(*window, cx);
    let rules = rules();
    let mut failures = Vec::new();
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
        settle(&mut visual, Step::Frame, 2);
        for density in ["comfortable", "compact"] {
            action(
                &mut visual,
                &view,
                "gallery.set_density",
                UiValue::String(density.to_owned()),
            );
            for page in acceptance::page_ids() {
                if let Err(error) = try_action(
                    &mut visual,
                    &view,
                    "gallery.go",
                    UiValue::String(page.clone()),
                ) {
                    failures.push(format!("{family}/{appearance}/{density}/{page}: {error}"));
                    continue;
                }
                if let Some(error) = visual.update(|_, cx| view.last_error(cx).unwrap()) {
                    failures.push(format!(
                        "{family}/{appearance}/{density}/{page}: render error: {error}"
                    ));
                    continue;
                }
                let theme = visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant);
                for finding in
                    visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap())
                {
                    if !risks.record(&theme, &finding) {
                        failures.push(format!(
                            "{family}/{appearance}/{density}/{page}: {finding}; evidence={:?}",
                            finding.contrast
                        ));
                    }
                }
            }
        }
    }
    risks.assert_reported();
    assert!(
        failures.is_empty(),
        "{} findings:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn offered_reading_scales(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    action(visual, view, "gallery.palette", UiValue::Null);
    typed(visual, "Reading scale");
    settle(visual, Step::Tick, 5);
    let mut values = snapshot(visual, view)
        .nodes()
        .filter_map(|node| {
            let value = node
                .name
                .strip_prefix("Reading scale ")?
                .strip_suffix('%')?;
            value.parse::<u16>().ok()
        })
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    keys(visual, "escape");
    values.into_iter().map(|n| n.to_string()).collect()
}

#[gpui::test]
fn gallery_palette_exposes_all_six_reading_scales(cx: &mut TestAppContext) {
    let (mut visual, view) = keyboard_mount(cx, "input");
    assert_eq!(
        offered_reading_scales(&mut visual, &view),
        ["100", "110", "125", "150", "175", "200"]
    );
    no_error(&mut visual, &view);
}

/// At every reading scale the Gallery offers, each page passes the audit and
/// keeps every control whole and apart: none cut off where nothing scrolls,
/// none laid over another.
#[gpui::test]
fn every_gallery_page_reads_at_each_scale(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx, &AcceptanceLaunch::default());
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| view.focus(window, cx)).unwrap();
    let scales = offered_reading_scales(&mut visual, &view);
    assert_eq!(scales, ["100", "110", "125", "150", "175", "200"]);
    // A focused local diagnostic can reproduce a single scale; ordinary
    // CI always exercises the entire presented set and every stock witness.
    let focused = std::env::var("GPUI_RHAI_AUDIT_READING_SCALE").ok();
    if let Some(scale) = &focused {
        assert!(
            scales.contains(scale),
            "unsupported diagnostic scale {scale}"
        );
        eprintln!("Focused Gallery audit: {scale}%");
    }
    let rules = rules();
    let mut failures = Vec::new();
    let mut risks = PresetRisks::new();
    for scale in scales
        .into_iter()
        .filter(|n| focused.as_ref().is_none_or(|s| s == n))
    {
        action(
            &mut visual,
            &view,
            "gallery.set_scale",
            UiValue::String(scale.clone()),
        );
        for page in acceptance::page_ids() {
            if let Err(error) = try_action(
                &mut visual,
                &view,
                "gallery.go",
                UiValue::String(page.clone()),
            ) {
                failures.push(format!("{scale}/{page}: {error}"));
                continue;
            }
            if let Some(error) = visual.update(|_, cx| view.last_error(cx).unwrap()) {
                failures.push(format!("{scale}/{page}: render error: {error}"));
                continue;
            }
            let theme = visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant);
            for finding in visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap()) {
                if !risks.record(&theme, &finding) {
                    failures.push(format!("{scale}/{page}: {finding}"));
                }
            }
            for finding in visual.update(|_, cx| view.debug_unreachable_controls(cx).unwrap()) {
                failures.push(format!("{scale}/{page}: {finding}"));
            }
        }
    }
    if focused.is_none() {
        risks.assert_reported();
    } else {
        risks.report_if_present();
    }
    assert!(
        failures.is_empty(),
        "{} findings:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn keyboard_mount(cx: &mut TestAppContext, page: &str) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let launch = AcceptanceLaunch {
        page: page.to_owned(),
        ..AcceptanceLaunch::default()
    };
    let prepared = acceptance::view(&launch)
        .unwrap()
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
    let (window, view) = mount_prepared(
        cx,
        prepared,
        ("gallery-keyboard", "gallery"),
        true,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| view.focus(window, cx)).unwrap();
    visual.run_until_parked();
    (visual, view)
}

fn keys(visual: &mut VisualTestContext, keystrokes: &str) {
    visual.simulate_keystrokes(keystrokes);
    settle(visual, Step::Refresh, 1);
}

fn typed(visual: &mut VisualTestContext, input: &str) {
    visual.simulate_input(input);
    settle(visual, Step::Refresh, 1);
}

fn shows(visual: &mut VisualTestContext, view: &ScriptViewHandle, needle: &str) -> bool {
    snapshot(visual, view).nodes().any(|node| {
        node.name.contains(needle)
            || matches!(&node.value, Some(UiValue::String(value)) if value.contains(needle))
    })
}

/// From the host, Tab enters the shell and F6 moves to the main region; Tab then
/// reaches the first control of the page.
fn enter_main(visual: &mut VisualTestContext) {
    keys(visual, "tab");
    keys(visual, "f6");
    keys(visual, "tab");
}

#[gpui::test]
fn form_scene_validates_and_submits_by_keyboard(cx: &mut TestAppContext) {
    let (mut visual, view) = keyboard_mount(cx, "scene.form");
    enter_main(&mut visual);
    keys(&mut visual, "enter");
    assert!(
        shows(&mut visual, &view, "Enter a service name."),
        "an empty submit shows the field error"
    );
    typed(&mut visual, "api");
    keys(&mut visual, "enter");
    assert!(
        shows(&mut visual, &view, "Service api created"),
        "Enter in the name field submits"
    );
}

#[gpui::test]
fn operations_scene_filters_selects_and_acts_by_keyboard(cx: &mut TestAppContext) {
    let (mut visual, view) = keyboard_mount(cx, "scene.operations");
    enter_main(&mut visual);
    typed(&mut visual, "edge-03");
    assert!(
        shows(&mut visual, &view, "1 hosts"),
        "the filter narrows the table"
    );
    // Export, Refresh, Deploy, then the table.
    for _ in 0..4 {
        keys(&mut visual, "tab");
    }
    keys(&mut visual, "down");
    assert!(
        shows(&mut visual, &view, "Health check failed"),
        "Down selects the host and opens its detail"
    );
    keys(&mut visual, "tab");
    keys(&mut visual, "enter");
    assert!(
        shows(&mut visual, &view, "Restart requested for edge-03"),
        "Restart runs from the keyboard"
    );
}

#[gpui::test]
fn settings_scene_switches_views_and_toggles_by_keyboard(cx: &mut TestAppContext) {
    let (mut visual, view) = keyboard_mount(cx, "scene.settings");
    enter_main(&mut visual);
    keys(&mut visual, "right");
    assert!(
        shows(&mut visual, &view, "飞书"),
        "Right moves the view switcher to Notifications"
    );
    keys(&mut visual, "tab");
    keys(&mut visual, "space");
    let tree = snapshot(&mut visual, &view);
    let email = tree
        .find_by_role_and_name("switch", "Email")
        .next()
        .unwrap();
    assert_eq!(
        email.checked,
        Some(UiValue::Bool(false)),
        "Space toggles the focused switch"
    );
}

#[gpui::test]
fn command_palette_navigates_by_keyboard(cx: &mut TestAppContext) {
    let (mut visual, view) = keyboard_mount(cx, "button");
    keys(&mut visual, "tab");
    keys(&mut visual, "cmd-k");
    let tree = snapshot(&mut visual, &view);
    assert!(
        tree.find_by_role_and_name("dialog", "Go to")
            .next()
            .is_some(),
        "Cmd+K opens the palette"
    );
    typed(&mut visual, "badge");
    keys(&mut visual, "enter");
    let tree = snapshot(&mut visual, &view);
    assert!(
        tree.find_by_role_and_name("region", "Badge")
            .next()
            .is_some(),
        "the palette opened the Badge page"
    );
}

#[gpui::test]
fn palette_theme_switch_in_a_narrow_window_preserves_editor_and_scroll(cx: &mut TestAppContext) {
    use gpui::{ScrollDelta, ScrollEvent, point, px, size};

    let (mut visual, view) = keyboard_mount(cx, "input");
    visual.update(|window, _| window.activate_window());
    visual.simulate_resize(size(px(590.), px(555.)));
    settle(&mut visual, Step::Tick, 6);
    let read = |v: &mut VisualTestContext| {
        snapshot(v, &view)
            .find_by_role_and_name("text_field", "Styled input")
            .next()
            .expect("the official Gallery's editor")
            .clone()
    };
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("combobox", "Theme")
            .next()
            .is_none(),
        "the compact title bar omits the direct theme picker"
    );
    let before_scroll = read(&mut visual).geometry.unwrap().visual;
    #[allow(clippy::cast_possible_truncation)]
    let dy = (before_scroll.y - 365.).max(100.) as f32;
    visual.simulate_event(ScrollEvent {
        position: point(px(565.), px(372.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(-dy))),
        ..Default::default()
    });
    settle(&mut visual, Step::Frame, 4);
    let scrolled = read(&mut visual);
    let near = scrolled.geometry.unwrap().visual;
    assert!(
        near.y > 65. && near.y + near.height <= 520. && near.y < before_scroll.y - 70.,
        "the wheel must bring the editor into the viewport: {before_scroll:?} -> {near:?}"
    );
    let old_value = scrolled.value.unwrap();
    click_center(&mut visual, near);
    settle(&mut visual, Step::Frame, 2);
    // A paste is one edit, unlike a sequence that first replaces a
    // selection and then types the remaining characters.
    visual.update(|_, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("alpha beta".to_owned()));
    });
    keys(&mut visual, "cmd-a cmd-v");
    keys(&mut visual, "shift-left shift-left shift-left shift-left");
    let editor_focus = visual.update(|window, cx| window.focused(cx)).unwrap();
    let stable = read(&mut visual);
    assert_eq!(stable.value, Some(UiValue::String("alpha beta".into())));

    keys(&mut visual, "cmd-k");
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("dialog", "Go to")
            .next()
            .is_some()
    );
    assert_ne!(
        visual.update(|window, cx| window.focused(cx)),
        Some(editor_focus.clone())
    );
    typed(&mut visual, "Clean Light");
    keys(&mut visual, "enter");
    assert!(
        shows(&mut visual, &view, "Theme Clean Light"),
        "the keyboard commands must select a different design"
    );
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("dialog", "Go to")
            .next()
            .is_none()
    );
    assert_eq!(
        visual.update(|window, cx| window.focused(cx)),
        Some(editor_focus.clone())
    );
    let after = read(&mut visual);
    assert_eq!(stable.id, after.id);
    let area = after.geometry.unwrap().visual;
    assert!(
        area.y > 65. && area.y + area.height <= 520. && area.y < before_scroll.y - 55.,
        "the design switch must keep the scrolled edit target visible: {before_scroll:?} -> {area:?}"
    );

    typed(&mut visual, "Z");
    assert_eq!(
        read(&mut visual).value,
        Some(UiValue::String("alpha Z".into()))
    );
    keys(&mut visual, "cmd-z");
    assert_eq!(
        read(&mut visual).value,
        Some(UiValue::String("alpha beta".into()))
    );
    keys(&mut visual, "cmd-z");
    assert_eq!(read(&mut visual).value, Some(old_value.clone()));
    keys(&mut visual, "cmd-k");
    typed(&mut visual, "Modern Dark");
    keys(&mut visual, "enter");
    assert!(shows(&mut visual, &view, "Theme Modern Dark"));
    assert_eq!(read(&mut visual).value, Some(old_value));
    assert_eq!(
        visual.update(|window, cx| window.focused(cx)),
        Some(editor_focus)
    );
    no_error(&mut visual, &view);
}

#[gpui::test]
fn gallery_pages_drawer_releases_main_width_and_preserves_edits(cx: &mut TestAppContext) {
    use gpui::{px, size};
    let (mut visual, view) = keyboard_mount(cx, "input");
    visual.update(|window, _| window.activate_window());
    visual.simulate_resize(size(px(590.), px(555.)));
    settle(&mut visual, Step::Tick, 6);
    action(
        &mut visual,
        &view,
        "gallery.set_scale",
        UiValue::String("150".into()),
    );
    settle(&mut visual, Step::Tick, 6);
    no_error(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("button", "Browse pages")
            .next()
            .is_some(),
        "the compact title bar needs a drawer trigger"
    );
    let main = bounds_of(&mut visual, &view, "region", "Input");
    assert!(
        main.width >= 500.,
        "the compact main pane needs a real reading column: {main:?}"
    );
    let catalog_visible = |v: &mut VisualTestContext| {
        snapshot(v, &view)
            .find_by_role_and_name("tree", "Gallery pages")
            .any(|node| {
                node.geometry
                    .is_some_and(|g| g.visual.width > 0. && g.visual.height > 0.)
            })
    };
    let drawer_open = |v: &mut VisualTestContext| {
        snapshot(v, &view)
            .find_by_role_and_name("dialog", "gpui-rhai Gallery sidebar")
            .next()
            .is_some()
    };
    let editor = |v: &mut VisualTestContext| {
        snapshot(v, &view)
            .find_by_role_and_name("text_field", "Name")
            .next()
            .unwrap()
            .clone()
    };
    assert!(!catalog_visible(&mut visual));
    let field = editor(&mut visual).geometry.unwrap().visual;
    click_at(&mut visual, field);
    typed(&mut visual, "Retain this edit");
    let edited = editor(&mut visual);
    assert_eq!(
        edited.value,
        Some(UiValue::String("Retain this edit".into()))
    );
    let focus = visual.update(|window, cx| window.focused(cx)).unwrap();

    action(&mut visual, &view, "gallery.browse_pages", UiValue::Null);
    settle(&mut visual, Step::Tick, 4);
    assert!(drawer_open(&mut visual));
    assert!(catalog_visible(&mut visual));
    keys(&mut visual, "escape");
    assert!(!drawer_open(&mut visual));
    assert_eq!(
        visual.update(|window, cx| window.focused(cx)),
        Some(focus.clone())
    );

    visual.simulate_resize(size(px(1120.), px(610.)));
    settle(&mut visual, Step::Tick, 6);
    assert!(
        catalog_visible(&mut visual),
        "wide windows still dock the catalog"
    );
    assert!(!drawer_open(&mut visual));
    assert_eq!(editor(&mut visual).id, edited.id);
    assert_eq!(editor(&mut visual).value, edited.value);
    assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(focus));

    visual.simulate_resize(size(px(820.), px(610.)));
    settle(&mut visual, Step::Tick, 6);
    assert!(
        !catalog_visible(&mut visual),
        "an enlarged medium window uses the drawer too"
    );
    let trigger = bounds_of(&mut visual, &view, "button", "Browse pages");
    click_at(&mut visual, trigger);
    assert!(drawer_open(&mut visual));
    keys(&mut visual, "down enter");
    assert!(
        !drawer_open(&mut visual),
        "choosing a page closes the drawer"
    );
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("region", "NumberInput")
            .next()
            .is_some()
    );
    no_error(&mut visual, &view);
}

fn bounds_of(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    snapshot(visual, view)
        .nodes()
        .find(|node| node.role == role && node.name.contains(name))
        .and_then(|node| node.geometry)
        .unwrap_or_else(|| panic!("no {role} named {name}"))
        .visual
}

fn status_presentation(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> gpui_rhai::ElementGeometry {
    snapshot(v, view)
        .find_by_role_and_name(role, name)
        .next()
        .and_then(|node| node.geometry)
        .unwrap_or_else(|| panic!("missing presented {role} {name}"))
}

#[gpui::test]
fn extreme_gallery_chrome_keeps_pointer_entry_points_and_locale_access(cx: &mut TestAppContext) {
    use gpui::{px, size};
    let (mut visual, view) = keyboard_mount(cx, "input");
    visual.update(|window, _| window.activate_window());
    visual.simulate_resize(size(px(390.), px(555.)));
    settle(&mut visual, Step::Tick, 6);
    action(
        &mut visual,
        &view,
        "gallery.set_scale",
        UiValue::String("200".into()),
    );
    settle(&mut visual, Step::Tick, 6);
    no_error(&mut visual, &view);
    let control = |v: &mut VisualTestContext, name: &str| {
        let g = status_presentation(v, &view, "button", name);
        assert!(
            has_full_status_width(g) && g.visual.x >= 0. && g.visual.x + g.visual.width <= 390.5,
            "{name} must have a full, on-screen pointer target: {g:?}"
        );
        g.visual
    };
    let pages = control(&mut visual, "Browse pages");
    let commands = control(&mut visual, "Commands");
    let title = status_presentation(&mut visual, &view, "text", "GPUI RHAI");
    assert!(
        has_full_status_width(title),
        "title text should elide within its own box: {title:?}"
    );
    assert!(
        (pages.width - pages.height).abs() <= 1. && (commands.width - commands.height).abs() <= 1.,
        "the tools remain square at large reading sizes: {pages:?} {commands:?}"
    );
    assert!(pages.x + pages.width < commands.x);
    let exposed_locale = |v: &mut VisualTestContext| {
        snapshot(v, &view)
            .find_by_role_and_name("combobox", "Locale")
            .any(|node| node.geometry.is_some_and(|g| g.visual.width > 0.))
    };
    assert!(
        !exposed_locale(&mut visual),
        "the optional picker cannot fit at 390px and 200%"
    );

    click_at(&mut visual, pages);
    let panel = status_presentation(&mut visual, &view, "dialog", "gpui-rhai Gallery sidebar");
    assert!(
        has_full_status_width(panel)
            && panel.visual.x >= 0.
            && panel.visual.x + panel.visual.width <= 390.5,
        "{panel:?}"
    );
    keys(&mut visual, "escape");
    click_at(&mut visual, commands);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("dialog", "Go to")
            .next()
            .is_some()
    );
    typed(&mut visual, "Arabic");
    keys(&mut visual, "enter");
    settle(&mut visual, Step::Tick, 6);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("dialog", "Go to")
            .next()
            .is_none()
    );
    let pages = control(&mut visual, "Browse pages");
    let commands = control(&mut visual, "Commands");
    assert!(
        commands.x + commands.width < pages.x,
        "the selected language mirrors the toolbar"
    );

    visual.simulate_resize(size(px(590.), px(555.)));
    settle(&mut visual, Step::Tick, 6);
    action(
        &mut visual,
        &view,
        "gallery.set_scale",
        UiValue::String("150".into()),
    );
    settle(&mut visual, Step::Tick, 6);
    let locale = status_presentation(&mut visual, &view, "combobox", "Locale");
    assert!(
        has_full_status_width(locale),
        "the selector returns when there is enough room"
    );
    assert!(shows(&mut visual, &view, "العربية"));
    no_error(&mut visual, &view);
}

fn has_full_status_width(geometry: gpui_rhai::ElementGeometry) -> bool {
    let visible = geometry.visual;
    visible.width > 0.
        && geometry.clip.is_none_or(|clip| {
            clip.x <= visible.x + 0.5 && clip.x + clip.width + 0.5 >= visible.x + visible.width
        })
}

#[gpui::test]
fn status_bar_uses_spare_end_space_without_changing_center_alignment(cx: &mut TestAppContext) {
    use crate::support::Mount;
    use gpui::{px, size};

    let (mut visual, view) = Mount::new(
        r#"
            import "components/status_bar" as status;
            fn fixed(label, width){
                text(label).with_style(style().width(px(width)).flex_shrink(false))
            }
            fn view(ctx) {
                column([
                    status::StatusBar(#{key:"split", label:"Uncentered",
                        start:[fixed("Long start",350)], end:[fixed("End action",92)]}),
                    status::StatusBar(#{key:"centered", label:"Centered",
                        start:[fixed("Short start",24)], center:[fixed("Center marker",104)],
                        end:[fixed("Longer end",120)]})
                ]).with_style(style().width(relative(1.0)))
            }
        "#,
    )
    .registry("components/status_bar")
    .theme(gpui_rhai_registry::CLEAN_LIGHT_THEME)
    .mount(cx);
    visual.simulate_resize(size(px(590.), px(270.)));
    settle(&mut visual, Step::Tick, 4);
    let long = status_presentation(&mut visual, &view, "text", "Long start");
    assert!(
        has_full_status_width(long),
        "the 350px item fits beside the 92px end: {long:?}"
    );
    let end = status_presentation(&mut visual, &view, "text", "End action");
    assert!(has_full_status_width(end));
    let bar = status_presentation(&mut visual, &view, "statusbar", "Centered").visual;
    let mid = status_presentation(&mut visual, &view, "text", "Center marker").visual;
    assert!(
        (mid.x + mid.width / 2. - bar.x - bar.width / 2.).abs() <= 0.5,
        "an occupied center keeps the visual center: {bar:?} {mid:?}"
    );

    visual.simulate_resize(size(px(390.), px(270.)));
    settle(&mut visual, Step::Tick, 4);
    assert!(!has_full_status_width(status_presentation(
        &mut visual,
        &view,
        "text",
        "Long start"
    )));
    assert!(has_full_status_width(status_presentation(
        &mut visual,
        &view,
        "text",
        "End action"
    )));
    no_error(&mut visual, &view);
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn gallery_status_bar_keeps_locale_and_the_audit_action_visible(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui::{px, size};
    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("gallery-status-width"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut visual, view) = theme_mount(&mut native, "clean-light");
    visual.simulate_resize(size(px(590.), px(555.)));
    settle(&mut visual, Step::Tick, 6);
    let locale = status_presentation(&mut visual, &view, "text", "Locale en");
    assert!(
        has_full_status_width(locale),
        "no avoidable locale clipping: {locale:?}"
    );
    let audit = status_presentation(&mut visual, &view, "button", "Open audit report");
    assert!(has_full_status_width(audit));

    action(
        &mut visual,
        &view,
        "gallery.set_scale",
        UiValue::String("150".into()),
    );
    settle(&mut visual, Step::Tick, 6);
    let enlarged = status_presentation(&mut visual, &view, "button", "Open audit report");
    assert!(
        has_full_status_width(enlarged),
        "the action keeps its width at 150%"
    );
    for required in ["Theme Clean Light", "Scale 150%"] {
        assert!(has_full_status_width(status_presentation(
            &mut visual,
            &view,
            "text",
            required
        )));
    }
    for optional in ["Density comfortable", "Locale en"] {
        let tree = snapshot(&mut visual, &view);
        let geometry = tree
            .find_by_role_and_name("text", optional)
            .next()
            .and_then(|node| node.geometry);
        assert!(
            geometry.is_none_or(|g| g.visual.width <= 0. || has_full_status_width(g)),
            "{optional} must be visible as a whole or left out: {geometry:?}"
        );
    }
    click_at(&mut visual, enlarged.visual);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("dialog", "Audit findings")
            .next()
            .is_some()
    );
    no_error(&mut visual, &view);
}

fn drag(visual: &mut VisualTestContext, from: (f64, f64), by: (f64, f64)) {
    use gpui::{Modifiers, MouseButton, point, px};
    #[allow(clippy::cast_possible_truncation)]
    let at = |t: f64| {
        point(
            px((from.0 + by.0 * t) as f32),
            px((from.1 + by.1 * t) as f32),
        )
    };
    visual.simulate_mouse_move(at(0.0), None, Modifiers::default());
    visual.simulate_mouse_down(at(0.0), MouseButton::Left, Modifiers::default());
    for step in 1..=4 {
        visual.simulate_mouse_move(
            at(f64::from(step) / 4.0),
            MouseButton::Left,
            Modifiers::default(),
        );
        visual.run_until_parked();
    }
    visual.simulate_mouse_up(at(1.0), MouseButton::Left, Modifiers::default());
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
}

fn no_error(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    assert_eq!(last_error(visual, view), None);
}

#[gpui::test]
fn resizable_page_stores_the_proposal_as_the_next_rect(cx: &mut TestAppContext) {
    let (mut visual, view) = keyboard_mount(cx, "resizable");
    let before = bounds_of(&mut visual, &view, "separator", "se resize handle");
    let center = (
        before.x + before.width / 2.0,
        before.y + before.height / 2.0,
    );
    drag(&mut visual, center, (60.0, 30.0));
    no_error(&mut visual, &view);
    let after = bounds_of(&mut visual, &view, "separator", "se resize handle");
    assert!(
        (after.x - before.x - 60.0).abs() < 1.0 && (after.y - before.y - 30.0).abs() < 1.0,
        "{before:?} -> {after:?}"
    );
}

#[gpui::test]
fn pan_zoom_page_drags_accumulate_and_reset(cx: &mut TestAppContext) {
    let (mut visual, view) = keyboard_mount(cx, "pan_zoom");
    let canvas = bounds_of(&mut visual, &view, "region", "Pan and zoom canvas");
    let start = (
        canvas.x + canvas.width / 2.0,
        canvas.y + canvas.height / 2.0,
    );
    drag(&mut visual, start, (80.0, 40.0));
    drag(&mut visual, (start.0 + 80.0, start.1 + 40.0), (20.0, 10.0));
    no_error(&mut visual, &view);
    assert!(shows(&mut visual, &view, "x 100 · y 50 · 100%"));
    if gpui_rhai::EXTENDED_RENDERING {
        let mirror = bounds_of(&mut visual, &view, "button", "Mirror canvas");
        click_at(&mut visual, mirror);
        assert!(shows(&mut visual, &view, "x 100 · y 50 · 100%"));
        assert!(shows(&mut visual, &view, "Restore orientation"));
        drag(&mut visual, start, (40.0, 20.0));
        assert!(shows(&mut visual, &view, "x 60 · y 70 · 100%"));
    } else {
        assert!(!shows(&mut visual, &view, "Mirror canvas"));
        drag(&mut visual, start, (40.0, 20.0));
        assert!(shows(&mut visual, &view, "x 140 · y 70 · 100%"));
    }
    no_error(&mut visual, &view);
    let reset = bounds_of(&mut visual, &view, "button", "Reset view");
    drag(&mut visual, (reset.x + 8.0, reset.y + 8.0), (0.0, 0.0));
    assert!(shows(&mut visual, &view, "x 0 · y 0 · 100%"));
}

#[gpui::test]
fn rotatable_page_turns_from_anywhere_in_the_area(cx: &mut TestAppContext) {
    let (mut visual, view) = keyboard_mount(cx, "rotatable");
    let area = bounds_of(&mut visual, &view, "slider", "Rotate arm");
    // The pivot sits at (100, 100) in the area; a drag well away from both the
    // arm and the old corner handle sweeps about 34 degrees, snapped to 30.
    let pivot = (area.x + 100.0, area.y + 100.0);
    drag(&mut visual, (pivot.0 + 80.0, pivot.1 + 94.0), (-70.0, 0.0));
    no_error(&mut visual, &view);
    let tree = snapshot(&mut visual, &view);
    let value = tree
        .nodes()
        .find(|node| node.role == "slider" && node.name == "Rotate arm")
        .and_then(|node| node.value.clone());
    assert_eq!(value, Some(UiValue::Float(30.0)));
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn backdrop_foundation_combines_parent_read_group_opacity_and_foreground_blur(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = keyboard_mount(cx, "foundations.backdrop_blur");
    let parent_reads = |visual: &mut VisualTestContext| {
        visual.update(|window, _| {
            window.visual_probe_with_painted_scene(|scene| {
                let mut reads = Vec::new();
                crate::support::walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::unit(),
                    &mut |scene, _| {
                        reads.extend(scene.isolated_groups.iter().filter_map(|group| {
                            group.parent_backdrop().map(|read| {
                                (read.sigma, group.source_blur_sigma(), group.opacity())
                            })
                        }));
                    },
                );
                reads
            })
        })
    };
    let live_affine = |visual: &mut VisualTestContext| {
        visual.update(|window, _| {
            window.visual_probe_with_painted_scene(|scene| {
                let mut found = false;
                crate::support::walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::unit(),
                    &mut |scene, _| {
                        found |= scene.isolated_groups.iter().any(|group| {
                            group.preserves_parent_backdrop() && group.needs_live_parent()
                        });
                    },
                );
                found
            })
        })
    };
    assert!(!live_affine(&mut visual));
    let rotate = bounds_of(&mut visual, &view, "button", "Rotate panel");
    click_at(&mut visual, rotate);
    assert!(shows(&mut visual, &view, "Straighten panel"));
    assert!(live_affine(&mut visual));
    assert!(
        parent_reads(&mut visual).is_empty(),
        "a normal read stays a paint operation, not a separate filter group"
    );
    let switch = bounds_of(&mut visual, &view, "button", "Combine filters");
    click_at(&mut visual, switch);
    for (family, appearance) in [("Modern", "Light"), ("Clean", "Dark")] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Tick, 3);
        let reads = parent_reads(&mut visual);
        assert!(
            reads.iter().any(|(sigma, foreground, opacity)| *sigma > 0.
                && *foreground > 0.
                && (*opacity - 0.55).abs() < 0.001),
            "{reads:?}"
        );
        assert!(live_affine(&mut visual));
        assert!(shows(&mut visual, &view, "Use only backdrop"));
        no_error(&mut visual, &view);
    }
    let sharp = bounds_of(&mut visual, &view, "button", "Sharpen backdrop");
    click_at(&mut visual, sharp);
    assert!(
        parent_reads(&mut visual).is_empty(),
        "zero backdrop clears the parent read"
    );
    assert!(
        !live_affine(&mut visual),
        "a sharp panel needs no live parent read"
    );
    assert!(shows(&mut visual, &view, "Use only backdrop"));
    no_error(&mut visual, &view);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[cfg(target_os = "macos")]
#[gpui::test]
fn backdrop_foundation_transmission_recovers_after_budget_failure(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast::metal_renderer::{GroupBudget, MetalRenderer};
    #[cfg(feature = "gpui-pre")]
    use apple_pre::metal_renderer::{GroupBudget, MetalRenderer};

    let (mut visual, view) = keyboard_mount(cx, "foundations.backdrop_blur");
    let rotate = bounds_of(&mut visual, &view, "button", "Rotate panel");
    click_at(&mut visual, rotate);
    visual.update(|window, _| {
        let extent = window.viewport_size();
        let scale = window.scale_factor();
        let extent = gpui::size(
            gpui::DevicePixels((f32::from(extent.width) * scale).ceil() as i32),
            gpui::DevicePixels((f32::from(extent.height) * scale).ceil() as i32),
        );
        window.visual_probe_with_painted_scene(|scene| {
            let mut budget = None;
            crate::support::walk_live_paint(
                scene,
                gpui::TransformationMatrix::unit(),
                &mut |scene, _| {
                    for group in scene
                        .isolated_groups
                        .iter()
                        .filter(|group| group.preserves_parent_backdrop())
                    {
                        let reads = &group.source().backdrop_blurs;
                        if let [read] = reads.as_slice() {
                            let extent = group.source_size();
                            let area = u64::from(extent.width.0 as u32)
                                * u64::from(extent.height.0 as u32);
                            let width = (read.bounds.right().0.ceil()
                                - read.bounds.origin.x.0.floor())
                                as u64;
                            let height = (read.bounds.bottom().0.ceil()
                                - read.bounds.origin.y.0.floor())
                                as u64;
                            let taps = ((4. * read.sigma).ceil() as u64).div_ceil(2);
                            budget = Some(width * height * (2 + 2 * (1 + 2 * taps)) + 4 * area);
                        }
                    }
                },
            );
            let mut renderer = MetalRenderer::new_headless(Default::default()).unwrap();
            let reference = renderer.render_scene_to_image(scene, extent).unwrap();
            renderer
                .set_group_budget(GroupBudget {
                    max_blur_samples: budget
                        .expect("the actual Gallery frame has one placed replacement"),
                    ..GroupBudget::default()
                })
                .unwrap();
            let error = renderer.render_scene_to_image(scene, extent).unwrap_err();
            assert!(
                format!("{error:#}").contains("shared blur sample budget exceeded"),
                "{error:#}"
            );
            renderer.set_group_budget(GroupBudget::default()).unwrap();
            let recovered = renderer.render_scene_to_image(scene, extent).unwrap();
            assert_eq!(
                recovered, reference,
                "the rejected frame must not leave a stale background prefix"
            );
        });
    });
}

fn theme_mount(cx: &mut TestAppContext, theme: &str) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let launch = AcceptanceLaunch {
        theme: theme.to_owned(),
        ..AcceptanceLaunch::default()
    };
    let (window, view) = mount(cx, &launch);
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Refresh, 1);
    (visual, view)
}

#[gpui::test]
fn gallery_launches_any_bundled_theme_and_toggles_within_its_family(cx: &mut TestAppContext) {
    let (mut visual, view) = theme_mount(cx, "catppuccin-latte");
    assert!(shows(&mut visual, &view, "Theme Catppuccin Latte"));
    action(&mut visual, &view, "gallery.toggle_mode", UiValue::Null);
    assert!(shows(&mut visual, &view, "Theme Catppuccin Mocha"));
    // A family with one variant has nothing to toggle to.
    let (mut visual, view) = theme_mount(cx, "nord");
    action(&mut visual, &view, "gallery.toggle_mode", UiValue::Null);
    assert!(shows(&mut visual, &view, "Theme Nord Dark"));
    no_error(&mut visual, &view);
}

fn click_at(visual: &mut VisualTestContext, bounds: GeometryBounds) {
    click_center(visual, bounds);
    settle(visual, Step::Refresh, 3);
}

#[gpui::test]
fn gallery_theme_picker_lists_loaded_themes_and_switches(cx: &mut TestAppContext) {
    let (mut visual, view) = theme_mount(cx, "default-dark");
    let picker = bounds_of(&mut visual, &view, "combobox", "Theme");
    click_at(&mut visual, picker);
    let tree = snapshot(&mut visual, &view);
    let options = tree
        .nodes()
        .filter(|node| node.role == "option")
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    // The list is virtual: the first rows, in family and variant order.
    assert_eq!(
        options[..4],
        [
            "Absolutely · Dark",
            "Absolutely · Light",
            "Aetheria",
            "Catppuccin · Latte"
        ],
        "{options:?}"
    );
    let mocha = bounds_of(&mut visual, &view, "option", "Catppuccin · Mocha");
    click_at(&mut visual, mocha);
    no_error(&mut visual, &view);
    assert!(shows(&mut visual, &view, "Theme Catppuccin Mocha"));
}

/// Visible quads `width` x `height` logical pixels over the hovered divider center.
fn quads_sized(
    visual: &mut VisualTestContext,
    area: GeometryBounds,
    width: f32,
    height: f32,
) -> Vec<crate::support::paint::PaintedQuad> {
    crate::support::painted_quads(visual)
        .into_iter()
        .filter(|quad| {
            let painted = quad.visible();
            let hover_x = area.x + area.width / 2.0;
            let hover_y = area.y + area.height / 2.0;
            quad.background
                .as_solid()
                .is_some_and(|color| color.a > 0.0)
                && f64::from(painted.x) <= hover_x
                && f64::from(painted.right()) > hover_x
                && f64::from(painted.y) <= hover_y
                && f64::from(painted.bottom()) > hover_y
                && (quad.bounds.width - width).abs() < 0.01
                && (quad.bounds.height - height).abs() < 0.01
        })
        .collect()
}

#[gpui::test]
fn the_source_sits_beside_the_page_and_resizes(cx: &mut TestAppContext) {
    use gpui::{Modifiers, point, px};
    let (mut visual, view) = keyboard_mount(cx, "button");
    action(&mut visual, &view, "gallery.toggle_source", UiValue::Null);
    let page = bounds_of(&mut visual, &view, "region", "Button");
    let source = bounds_of(&mut visual, &view, "region", "Source");
    let divider = bounds_of(&mut visual, &view, "separator", "Resize source");
    assert!(
        divider.x > page.x + page.width - 1.0 && source.x > divider.x,
        "page {page:?}, divider {divider:?}, source {source:?}"
    );
    // The divider's bar shows only under the pointer.
    let at_rest = quads_sized(&mut visual, divider, 4.0, 24.0);
    assert!(at_rest.is_empty(), "a bar at rest: {at_rest:?}");
    let center = (
        divider.x + divider.width / 2.0,
        divider.y + divider.height / 2.0,
    );
    #[allow(clippy::cast_possible_truncation)]
    visual.simulate_mouse_move(
        point(px(center.0 as f32), px(center.1 as f32)),
        None,
        Modifiers::default(),
    );
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    let hovered = quads_sized(&mut visual, divider, 4.0, 24.0);
    assert_eq!(hovered.len(), 1, "hovered divider marks: {hovered:?}");
    // Dragging it left widens the source; the page keeps its start edge.
    drag(&mut visual, center, (-120.0, 0.0));
    no_error(&mut visual, &view);
    let wider = bounds_of(&mut visual, &view, "region", "Source");
    let page_after = bounds_of(&mut visual, &view, "region", "Button");
    assert!(
        (wider.width - source.width - 120.0).abs() < 2.0 && (page_after.x - page.x).abs() < 0.5,
        "source {source:?} -> {wider:?}, page {page:?} -> {page_after:?}"
    );
    // Hiding and showing the source keeps its width.
    action(&mut visual, &view, "gallery.toggle_source", UiValue::Null);
    assert!(!shows(&mut visual, &view, "Resize source"));
    action(&mut visual, &view, "gallery.toggle_source", UiValue::Null);
    let again = bounds_of(&mut visual, &view, "region", "Source");
    assert!(
        (again.width - wider.width).abs() < 0.5,
        "{wider:?} -> {again:?}"
    );
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
