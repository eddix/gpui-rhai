//! Gallery acceptance gates: every page passes the productivity audit in both
//! densities, and the scenes can be completed by keyboard alone.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(
    cx: &mut TestAppContext,
    launch: &AcceptanceLaunch,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    let prepared = acceptance::view(launch)
        .unwrap()
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("gallery", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("gallery"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    (window, view)
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

fn action(visual: &mut VisualTestContext, view: &ScriptViewHandle, id: &str, payload: UiValue) {
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
fn every_gallery_page_passes_the_audit_in_both_densities(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx, &AcceptanceLaunch::default());
    let mut visual = VisualTestContext::from_window(*window, cx);
    let rules = rules();
    let mut failures = Vec::new();
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
                failures.push(format!("{density}/{page}: {error}"));
                continue;
            }
            if let Some(error) = visual.update(|_, cx| view.last_error(cx).unwrap()) {
                failures.push(format!("{density}/{page}: render error: {error}"));
                continue;
            }
            for finding in visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap()) {
                failures.push(format!("{density}/{page}: {finding}"));
            }
        }
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
    let bindings = prepared.key_bindings().to_vec();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("gallery-keyboard", cx).unwrap();
        host.bind_keys(bindings, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("gallery"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| view.focus(window, cx)).unwrap();
    visual.run_until_parked();
    (visual, view)
}

fn keys(visual: &mut VisualTestContext, keystrokes: &str) {
    visual.simulate_keystrokes(keystrokes);
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
}

fn typed(visual: &mut VisualTestContext, input: &str) {
    visual.simulate_input(input);
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
}

fn shows(visual: &mut VisualTestContext, view: &ScriptViewHandle, needle: &str) -> bool {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    tree.nodes().any(|node| {
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
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
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
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    assert!(
        tree.find_by_role_and_name("dialog", "Go to")
            .next()
            .is_some(),
        "Cmd+K opens the palette"
    );
    typed(&mut visual, "badge");
    keys(&mut visual, "enter");
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    assert!(
        tree.find_by_role_and_name("region", "Badge")
            .next()
            .is_some(),
        "the palette opened the Badge page"
    );
}
