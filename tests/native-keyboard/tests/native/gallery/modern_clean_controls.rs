//! The canonical Gallery owns the same controlled loading UI users exercise.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ThemeMode};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use crate::support::{
    Host, ReplaceOnce, Step, bounds, click_center, last_error, painted_quads, settle, snapshot,
};

#[gpui::test]
fn gallery_icon_actions_accept_real_clicks_and_keep_state_through_all_presets(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount_command_page(cx, "icon_button");
    let search = snapshot(&mut visual, &view)
        .find_by_role_and_name("button", "Search records")
        .next()
        .unwrap()
        .id;
    let details = snapshot(&mut visual, &view)
        .find_by_role_and_name("button", "Toggle details")
        .next()
        .unwrap()
        .id;
    let area = bounds(&mut visual, &view, "button", "Search records");
    click_center(&mut visual, area);
    settle(&mut visual, Step::Tick, 4);
    let area = bounds(&mut visual, &view, "button", "Toggle details");
    click_center(&mut visual, area);
    settle(&mut visual, Step::Tick, 4);
    for (family, appearance) in [
        ("Modern", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Clean", "Dark"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Tick, 4);
        let tree = snapshot(&mut visual, &view);
        let busy = tree
            .find_by_role_and_name("button", "Search records")
            .next()
            .unwrap();
        assert_eq!(busy.id, search);
        assert!(busy.disabled);
        assert_eq!(
            busy.pressed, None,
            "an ordinary icon action is not a toggle"
        );
        let selected = tree
            .find_by_role_and_name("button", "Toggle details")
            .next()
            .unwrap();
        assert_eq!(selected.id, details);
        assert_eq!(selected.pressed, Some(true));
        assert_eq!(
            crate::support::text_of(&mut visual, &view, "icon-button-search-status"),
            "Search running"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
    let area = bounds(&mut visual, &view, "button", "Finish icon search");
    click_center(&mut visual, area);
    settle(&mut visual, Step::Tick, 4);
    assert!(!snapshot(&mut visual, &view).node(search).unwrap().disabled);
    assert_eq!(
        crate::support::text_of(&mut visual, &view, "icon-button-search-status"),
        "Search idle"
    );
    let area = bounds(&mut visual, &view, "button", "Toggle details");
    click_center(&mut visual, area);
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(
        snapshot(&mut visual, &view).node(details).unwrap().pressed,
        Some(false)
    );
    visual.simulate_keystrokes("space");
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(
        snapshot(&mut visual, &view).node(details).unwrap().pressed,
        Some(true)
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn gallery_empty_and_invalid_selects_accept_choices_and_retain_their_values(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount_command_page(cx, "select");
    for (label, option_label, display_value) in [
        ("Zone", "us-west", "us-west"),
        ("Tier", "Dedicated", "Dedicated"),
    ] {
        let before = snapshot(&mut visual, &view)
            .find_by_role_and_name("combobox", label)
            .next()
            .unwrap()
            .id;
        let area = bounds(&mut visual, &view, "combobox", label);
        click_center(&mut visual, area);
        settle(&mut visual, Step::Tick, 4);
        assert_eq!(
            snapshot(&mut visual, &view).node(before).unwrap().expanded,
            Some(true),
            "{label} must really open"
        );
        let option = bounds(&mut visual, &view, "option", option_label);
        click_center(&mut visual, option);
        settle(&mut visual, Step::Tick, 4);
        let tree = snapshot(&mut visual, &view);
        let chosen = tree.node(before).unwrap();
        // Native accessibility exposes the human-readable label. Reopening
        // below also verifies the raw business key selected its real option.
        assert_eq!(
            chosen.value,
            Some(gpui_rhai::UiValue::String(display_value.into()))
        );
        assert_eq!(chosen.expanded, Some(false));
        assert!(
            !chosen.invalid,
            "accepting a required value clears its validation state"
        );
        visual
            .update(|_, cx| view.select_theme("Clean", "Dark", cx))
            .unwrap();
        settle(&mut visual, Step::Tick, 3);
        assert_eq!(
            snapshot(&mut visual, &view).node(before).unwrap().value,
            Some(gpui_rhai::UiValue::String(display_value.into()))
        );
        let area = bounds(&mut visual, &view, "combobox", label);
        click_center(&mut visual, area);
        settle(&mut visual, Step::Tick, 3);
        let tree = snapshot(&mut visual, &view);
        assert_eq!(
            tree.find_by_role_and_name("option", option_label)
                .next()
                .unwrap()
                .selected,
            Some(true),
            "the raw business key must select its option, not merely render its label as an unknown value"
        );
        visual.simulate_keystrokes("escape");
        settle(&mut visual, Step::Tick, 3);
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn gallery_empty_date_picker_keeps_the_calendar_choice_after_closing(cx: &mut TestAppContext) {
    let (mut visual, view) = mount_command_page(cx, "date_picker");
    let before = snapshot(&mut visual, &view)
        .find_by_role_and_name("combobox", "Freeze date")
        .next()
        .unwrap()
        .id;
    let area = bounds(&mut visual, &view, "combobox", "Freeze date");
    click_center(&mut visual, area);
    settle(&mut visual, Step::Tick, 4);
    let tree = snapshot(&mut visual, &view);
    let day = tree
        .nodes()
        .find(|node| {
            node.role == "gridcell"
                && !node.disabled
                && node.value.is_some()
                && node.geometry.is_some()
        })
        .unwrap();
    let date = day.value.clone();
    click_center(&mut visual, day.geometry.unwrap().visual);
    settle(&mut visual, Step::Tick, 4);
    let tree = snapshot(&mut visual, &view);
    let chosen = tree.node(before).unwrap();
    assert_eq!(chosen.value, date);
    assert_eq!(chosen.expanded, Some(false));
    visual
        .update(|_, cx| view.select_theme("Clean", "Dark", cx))
        .unwrap();
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        snapshot(&mut visual, &view).node(before).unwrap().value,
        date
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn gallery_editable_input_sizes_and_error_state_keep_real_edits(cx: &mut TestAppContext) {
    let (mut visual, view) = mount_command_page(cx, "input");
    let mut edited = Vec::new();
    for (label, value) in [
        ("Extra small", "xs · edited"),
        ("Small", "sm · 已编辑"),
        ("Medium", "md · edited"),
        ("Large", "lg · edited"),
        ("Replicas", "3"),
    ] {
        let field = snapshot(&mut visual, &view)
            .find_by_role_and_name("text_field", label)
            .next()
            .unwrap()
            .clone();
        let area = field.geometry.unwrap().visual;
        assert!(
            area.y >= 0.0 && area.y + area.height < 900.0,
            "real visible target: {label}: {area:?}"
        );
        click_center(&mut visual, area);
        visual.simulate_keystrokes(&crate::support::primary_keys("secondary-a"));
        visual.simulate_input(value);
        settle(&mut visual, Step::Tick, 4);
        assert_eq!(
            snapshot(&mut visual, &view).node(field.id).unwrap().value,
            Some(gpui_rhai::UiValue::String(value.into())),
            "{label} must accept the edit, not only paint a transient draft"
        );
        edited.push((field.id, value));
    }
    for (family, appearance) in [
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Clean", "Dark"),
        ("Modern", "Light"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Tick, 4);
        let tree = snapshot(&mut visual, &view);
        for (id, value) in &edited {
            assert_eq!(
                tree.node(*id).unwrap().value,
                Some(gpui_rhai::UiValue::String((*value).into()))
            );
        }
        for (label, initial) in [
            ("Extra small", "xs"),
            ("Small", "sm"),
            ("Medium", "md"),
            ("Large", "lg"),
        ] {
            // The other density specimens have independent controlled values.
            let peers = tree
                .find_by_role_and_name("text_field", label)
                .collect::<Vec<_>>();
            assert_eq!(
                peers.len(),
                3,
                "size, comfortable and compact specimens all exist"
            );
            for other in peers.into_iter().skip(1) {
                assert_eq!(
                    other.value,
                    Some(gpui_rhai::UiValue::String(initial.into()))
                );
            }
        }
        assert_eq!(last_error(&mut visual, &view), None);
    }
    visual.simulate_keystrokes(&crate::support::primary_keys("secondary-z"));
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("text_field", "Replicas")
            .next()
            .unwrap()
            .value,
        Some(gpui_rhai::UiValue::String("-1".into()))
    );
}

#[gpui::test]
fn gallery_editable_error_textarea_keeps_multiline_edits_through_all_presets(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount_command_page(cx, "textarea");
    let field = snapshot(&mut visual, &view)
        .find_by_role_and_name("text_field", "Reason")
        .next()
        .unwrap()
        .clone();
    click_center(&mut visual, field.geometry.unwrap().visual);
    visual.simulate_input("Pending approval\n等待确认");
    settle(&mut visual, Step::Tick, 4);
    for (family, appearance) in [
        ("Modern", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Clean", "Dark"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Tick, 4);
        let tree = snapshot(&mut visual, &view);
        let reason = tree.node(field.id).unwrap();
        assert_eq!(
            reason.value,
            Some(gpui_rhai::UiValue::String(
                "Pending approval\n等待确认".into()
            ))
        );
        assert!(
            reason.invalid,
            "this remains the explicit error-state specimen"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

fn main_pager(
    visual: &mut VisualTestContext,
    view: &gpui_rhai::ScriptViewHandle,
) -> Vec<gpui_rhai::AccessibilityNode> {
    let tree = snapshot(visual, view);
    let pager = tree
        .find_by_role_and_name("navigation", "Hosts pages")
        .next()
        .unwrap();
    let mut pending = pager.children.clone();
    let mut result = Vec::new();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        pending.extend(&node.children);
        result.push(node.clone());
    }
    result
}

fn mount_command_page(
    cx: &mut TestAppContext,
    page: &str,
) -> (VisualTestContext, gpui_rhai::ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: page.into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("command-responsive-gallery", "command-responsive-gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Tick, 8);
    (visual, view)
}

#[gpui::test]
fn gallery_width_controls_resize_the_same_live_command(cx: &mut TestAppContext) {
    let (mut visual, view) = mount_command_page(cx, "command");
    let input = snapshot(&mut visual, &view)
        .find_by_role_and_name("text_field", "Commands")
        .next()
        .unwrap()
        .id;
    for (button, width, height) in [("Narrow panel", 360.0, 44.0), ("Wide panel", 560.0, 40.0)] {
        let button = bounds(&mut visual, &view, "button", button);
        click_center(&mut visual, button);
        settle(&mut visual, Step::Tick, 10);
        let panel = crate::support::node_by_test_id(&mut visual, &view, "command-panel-demo")
            .geometry
            .unwrap()
            .visual;
        assert!((panel.width - width).abs() < 0.6);
        assert_eq!(
            bounds(&mut visual, &view, "option", "Deploy service").height,
            height
        );
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("text_field", "Commands")
                .next()
                .unwrap()
                .id,
            input
        );
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn gallery_image_failure_and_restore_keep_the_same_accessible_image(cx: &mut TestAppContext) {
    let (mut visual, view) = mount_command_page(cx, "icon");
    let id = snapshot(&mut visual, &view)
        .find_by_role_and_name("image", "Recoverable logo")
        .next()
        .unwrap()
        .id;
    for label in ["Simulate image failure", "Restore image"] {
        let button = bounds(&mut visual, &view, "button", label);
        click_center(&mut visual, button);
        settle(&mut visual, Step::Tick, 6);
        assert!(last_error(&mut visual, &view).is_none());
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("image", "Recoverable logo")
                .next()
                .unwrap()
                .id,
            id
        );
    }
    assert_eq!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("button", "Simulate image failure")
            .count(),
        1
    );
}

#[gpui::test]
fn gallery_settings_switches_accept_pointer_and_keyboard_changes_with_one_owner(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount_command_page(cx, "switch");
    let tree = snapshot(&mut visual, &view);
    let background = tree
        .find_by_role_and_name("switch", "Background sync")
        .next()
        .unwrap();
    let id = background.id;
    assert_eq!(background.checked, Some(gpui_rhai::UiValue::Bool(true)));
    assert!(!background.description.is_empty());
    let area = bounds(&mut visual, &view, "switch", "Background sync");
    click_center(&mut visual, area);
    settle(&mut visual, Step::Tick, 5);
    let tree = snapshot(&mut visual, &view);
    let background = tree
        .find_by_role_and_name("switch", "Background sync")
        .next()
        .unwrap();
    assert_eq!(background.id, id);
    assert_eq!(background.checked, Some(gpui_rhai::UiValue::Bool(false)));
    let quiet = bounds(&mut visual, &view, "switch", "Quiet hours");
    click_center(&mut visual, quiet);
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("switch", "Quiet hours")
            .next()
            .unwrap()
            .checked,
        Some(gpui_rhai::UiValue::Bool(true))
    );
    visual.simulate_keystrokes("space");
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("switch", "Quiet hours")
            .next()
            .unwrap()
            .checked,
        Some(gpui_rhai::UiValue::Bool(false))
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn gallery_command_dialog_mounts_the_shared_rich_items_and_closes_on_real_activation(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount_command_page(cx, "command_dialog");
    let open = bounds(&mut visual, &view, "button", "Open palette");
    click_center(&mut visual, open);
    settle(&mut visual, Step::Tick, 10);
    let tree = snapshot(&mut visual, &view);
    assert_eq!(tree.find_by_role_and_name("dialog", "Commands").count(), 1);
    let deploy = tree
        .find_by_role_and_name("option", "Deploy service")
        .next()
        .unwrap();
    assert_eq!(deploy.description, "Production");
    assert_eq!(deploy.geometry.unwrap().visual.height, 44.0);
    let logs = bounds(&mut visual, &view, "option", "Open logs");
    click_center(&mut visual, logs);
    settle(&mut visual, Step::Tick, 10);
    assert_eq!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("dialog", "Commands")
            .count(),
        0
    );
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn gallery_command_actions_and_host_footer_are_observable_after_a_design_switch(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_rhai::install);
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "command".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("command-gallery", "command-gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(gpui::size(gpui::px(1280.0), gpui::px(900.0)));
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 4);
    let logs = bounds(&mut visual, &view, "option", "Open logs");
    click_center(&mut visual, logs);
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(
        crate::support::text_of(&mut visual, &view, "command-last-action"),
        "Last action: logs"
    );
    visual
        .update(|_, cx| view.select_theme("Clean", "Dark", cx))
        .unwrap();
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(
        crate::support::text_of(&mut visual, &view, "command-last-action"),
        "Last action: logs"
    );
    // Scroll the page gutter, not either command list. This is a real pointer
    // route through the Gallery shell rather than dispatching an offscreen callback.
    visual.simulate_event(gpui::ScrollEvent {
        position: gpui::point(gpui::px(1100.0), gpui::px(650.0)),
        delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(-2000.0))),
        ..Default::default()
    });
    settle(&mut visual, Step::Frame, 3);
    let clear = bounds(&mut visual, &view, "button", "Clear command feedback");
    assert!(
        clear.y >= 50.0 && clear.y + clear.height < 870.0,
        "the Gallery footer control is reachable: {clear:?}"
    );
    click_center(&mut visual, clear);
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(
        crate::support::text_of(&mut visual, &view, "command-last-action"),
        "Last action: none"
    );
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(
        crate::support::text_of(&mut visual, &view, "command-host-action"),
        "Host action: none"
    );
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn gallery_command_chrome_has_no_structural_audit_regressions_and_keeps_raw_color_risks(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_rhai::install);
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "command".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("command-audit", "command-audit"),
        false,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    let rules = gpui_rhai::AuditRules::from_ids(
        gpui_rhai::AuditRule::ALL
            .into_iter()
            .filter(|rule| *rule != gpui_rhai::AuditRule::UnresolvedFont)
            .map(gpui_rhai::AuditRule::id),
    )
    .unwrap();
    let mut risks = 0;
    for (family, appearance) in [
        ("Modern", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Clean", "Dark"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Frame, 4);
        let findings = visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
        let structural = findings
            .iter()
            .filter(|finding| finding.rule != gpui_rhai::AuditRule::LowContrastText)
            .collect::<Vec<_>>();
        assert!(
            structural.is_empty(),
            "{family}/{appearance}: {structural:?}"
        );
        risks += findings.len();
        for finding in &findings {
            eprintln!("Original color risk, not a WCAG pass: {family}/{appearance}: {finding}");
        }
        assert!(last_error(&mut visual, &view).is_none());
    }
    assert!(
        risks > 0,
        "the strict audit must keep reporting the unmodified original soft inks"
    );
}

#[gpui::test]
fn gallery_checkbox_mixed_and_unchecked_samples_commit_real_controlled_values(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_rhai::install);
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "checkbox".to_owned(),
        theme: "modern-light".to_owned(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("checkbox-gallery", "checkbox-gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 4);
    let state = |visual: &mut VisualTestContext, name: &str| {
        snapshot(visual, &view)
            .find_by_role_and_name("checkbox", name)
            .next()
            .unwrap()
            .checked
            .clone()
    };
    assert_eq!(
        state(&mut visual, "Some regions"),
        Some(gpui_rhai::UiValue::String("mixed".to_owned()))
    );
    for expected in [true, false, true] {
        let area = bounds(&mut visual, &view, "checkbox", "Some regions");
        click_center(&mut visual, area);
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(
            state(&mut visual, "Some regions"),
            Some(gpui_rhai::UiValue::Bool(expected))
        );
    }
    let area = bounds(&mut visual, &view, "checkbox", "Unchecked");
    click_center(&mut visual, area);
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(
        state(&mut visual, "Unchecked"),
        Some(gpui_rhai::UiValue::Bool(true))
    );
    visual
        .update(|_, cx| view.select_theme("Clean", "Dark", cx))
        .unwrap();
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(
        state(&mut visual, "Unchecked"),
        Some(gpui_rhai::UiValue::Bool(true))
    );
    assert_eq!(
        state(&mut visual, "Some regions"),
        Some(gpui_rhai::UiValue::Bool(true))
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn gallery_pagination_controls_are_interactive_and_survive_a_design_switch(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_rhai::install);
    let launch = AcceptanceLaunch {
        page: "pagination".to_owned(),
        theme: "modern-light".to_owned(),
        ..Default::default()
    };
    let prepared = acceptance::view(&launch)
        .unwrap()
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("pagination-gallery", "pagination-gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(gpui::size(gpui::px(1440.0), gpui::px(1100.0)));
    settle(&mut visual, Step::Frame, 4);
    for name in ["Hide page numbers", "Disable main pager"] {
        let target = bounds(&mut visual, &view, "button", name);
        click_center(&mut visual, target);
        settle(&mut visual, Step::Frame, 4);
    }
    visual
        .update(|_, cx| view.select_theme("Clean", "Dark", cx))
        .unwrap();
    settle(&mut visual, Step::Frame, 4);
    let pager = main_pager(&mut visual, &view);
    assert!(
        !pager
            .iter()
            .any(|node| node.role == "button" && node.name.parse::<i64>().is_ok())
    );
    assert!(
        pager
            .iter()
            .filter(|node| node.role == "button" || node.role == "combobox")
            .all(|node| node.disabled)
    );
    assert!(
        pager
            .iter()
            .any(|node| node.role == "text" && node.name == "21–30 of 128")
    );
    for name in ["Enable main pager", "Show page numbers"] {
        let target = bounds(&mut visual, &view, "button", name);
        click_center(&mut visual, target);
        settle(&mut visual, Step::Frame, 4);
    }
    let fifth = main_pager(&mut visual, &view)
        .into_iter()
        .find(|node| node.role == "button" && node.name == "5")
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    click_center(&mut visual, fifth);
    settle(&mut visual, Step::Frame, 4);
    let pager = main_pager(&mut visual, &view);
    assert!(
        pager
            .iter()
            .any(|node| node.current.as_deref() == Some("page") && node.name == "5")
    );
    assert!(
        pager
            .iter()
            .any(|node| node.role == "text" && node.name == "41–50 of 128")
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn gallery_theme_selection_handles_names_containing_the_old_separator(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let palette = |family: &str, name: &str| {
        gpui_rhai_registry::MODERN_DARK_THEME
            .replace_once("family: \"Modern\"", &format!("family: \"{family}\""))
            .replace_once("name: \"Dark\"", &format!("name: \"{name}\""))
    };
    let prepared = acceptance::view(&AcceptanceLaunch::default())
        .unwrap()
        .theme_sources([
            ("first.rhai".to_owned(), palette("A|B", "C")),
            ("second.rhai".to_owned(), palette("A", "B|C")),
        ])
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("theme-key-gallery", "theme-key-gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 2);
    for (family, name) in [("A|B", "C"), ("A", "B|C")] {
        let trigger = bounds(&mut visual, &view, "combobox", "Theme");
        click_center(&mut visual, trigger);
        settle(&mut visual, Step::Frame, 2);
        let option = bounds(&mut visual, &view, "option", family);
        click_center(&mut visual, option);
        settle(&mut visual, Step::Frame, 2);
        let selected = visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant);
        assert_eq!(
            (selected.family.as_str(), selected.name.as_str()),
            (family, name)
        );
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn gallery_scroll_gutters_leave_the_first_controls_exterior_focus_ring_visible(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_rhai::install);
    for &(name, _) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let launch = AcceptanceLaunch {
            page: "button".to_owned(),
            theme: name.trim_end_matches(".rhai").replace('_', "-"),
            ..Default::default()
        };
        let prepared = acceptance::view(&launch)
            .unwrap()
            .motion_preference(MotionPreference::None)
            .prepare()
            .unwrap();
        let (window, view) = super::mount_prepared(
            cx,
            prepared,
            ("focus-gallery", "focus-gallery"),
            false,
            |host, view| Host { host, view },
        );
        let mut visual = VisualTestContext::from_window(*window, cx);
        settle(&mut visual, Step::Frame, 3);
        let deploy = bounds(&mut visual, &view, "button", "Deploy");
        click_center(&mut visual, deploy);
        visual.simulate_keystrokes("right");
        settle(&mut visual, Step::Frame, 3);
        let rings = painted_quads(&mut visual)
            .into_iter()
            .filter(|quad| {
                quad.border_widths == [2.0; 4]
                    && (f64::from(quad.bounds.width) - deploy.width - 8.0).abs() < 0.5
                    && (f64::from(quad.bounds.x) - deploy.x + 4.0).abs() < 0.5
            })
            .collect::<Vec<_>>();
        let b = rings.first().expect("focused specimen ring").bounds;
        // Rounded ancestor clips may emit several rectangular slices of the
        // same quad. Each edge midpoint must survive the union of those masks.
        for (x, y) in [
            (b.x + 1.0, b.y + b.height / 2.0),
            (b.right() - 1.0, b.y + b.height / 2.0),
            (b.x + b.width / 2.0, b.y + 1.0),
            (b.x + b.width / 2.0, b.bottom() - 1.0),
        ] {
            assert!(
                rings.iter().any(|ring| x >= ring.mask.x
                    && x <= ring.mask.right()
                    && y >= ring.mask.y
                    && y <= ring.mask.bottom()),
                "{name}: focus edge ({x},{y}) is clipped: {rings:?}"
            );
        }
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn gallery_loading_completes_after_live_design_and_appearance_switches(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    for (name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let key = name.trim_end_matches(".rhai").replace('_', "-");
        let launch = AcceptanceLaunch {
            page: "button".to_owned(),
            theme: key,
            ..Default::default()
        };
        let prepared = acceptance::view(&launch)
            .unwrap()
            .motion_preference(MotionPreference::None)
            .prepare()
            .unwrap();
        let (window, view) = super::mount_prepared(
            cx,
            prepared,
            ("loading-gallery", "loading-gallery"),
            false,
            |host, view| Host { host, view },
        );
        let mut visual = VisualTestContext::from_window(*window, cx);
        settle(&mut visual, Step::Frame, 3);
        let run = bounds(&mut visual, &view, "button", "Run query");
        click_center(&mut visual, run);
        settle(&mut visual, Step::Frame, 2);
        assert!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("status", "Query is running")
                .next()
                .is_some(),
            "{name}"
        );
        let expected =
            gpui_rhai::load_theme_source(gpui_rhai::RuntimeEngine::new().engine(), name, source)
                .unwrap();
        let other_family = if expected.family == "Modern" {
            "Clean"
        } else {
            "Modern"
        };
        let other_mode = if expected.mode == ThemeMode::Light {
            "Dark"
        } else {
            "Light"
        };
        visual
            .update(|_, cx| view.select_theme(other_family, other_mode, cx))
            .unwrap();
        settle(&mut visual, Step::Frame, 3);
        assert!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("status", "Query is running")
                .next()
                .is_some(),
            "design switch must preserve controlled state"
        );
        let finish = bounds(&mut visual, &view, "button", "Finish query");
        click_center(&mut visual, finish);
        settle(&mut visual, Step::Frame, 2);
        assert!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("status", "Query is idle")
                .next()
                .is_some(),
            "the retained completion callback must still work"
        );
        assert!(last_error(&mut visual, &view).is_none());
    }
}
