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

fn bounds_of(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    tree.nodes()
        .find(|node| node.role == role && node.name.contains(name))
        .and_then(|node| node.geometry)
        .unwrap_or_else(|| panic!("no {role} named {name}"))
        .visual
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
    let error = visual.update(|_, cx| view.last_error(cx).unwrap());
    assert_eq!(error, None);
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
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let value = tree
        .nodes()
        .find(|node| node.role == "slider" && node.name == "Rotate arm")
        .and_then(|node| node.value.clone());
    assert_eq!(value, Some(UiValue::Float(30.0)));
}

fn theme_mount(cx: &mut TestAppContext, theme: &str) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let launch = AcceptanceLaunch {
        theme: theme.to_owned(),
        ..AcceptanceLaunch::default()
    };
    let (window, view) = mount(cx, &launch);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    (visual, view)
}

#[gpui::test]
fn gallery_launches_any_bundled_theme_and_toggles_within_its_family(cx: &mut TestAppContext) {
    let (mut visual, view) = theme_mount(cx, "catppuccin-latte");
    assert!(shows(&mut visual, &view, "THEME Catppuccin Latte"));
    action(&mut visual, &view, "gallery.toggle_mode", UiValue::Null);
    assert!(shows(&mut visual, &view, "THEME Catppuccin Mocha"));
    // A family with one variant has nothing to toggle to.
    let (mut visual, view) = theme_mount(cx, "nord");
    action(&mut visual, &view, "gallery.toggle_mode", UiValue::Null);
    assert!(shows(&mut visual, &view, "THEME Nord Dark"));
    no_error(&mut visual, &view);
}

fn click_at(visual: &mut VisualTestContext, bounds: GeometryBounds) {
    use gpui::{Modifiers, point, px};
    #[allow(clippy::cast_possible_truncation)]
    let center = point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    );
    visual.simulate_click(center, Modifiers::default());
    for _ in 0..3 {
        visual.update(|window, _| window.refresh());
        visual.run_until_parked();
    }
}

#[gpui::test]
fn gallery_theme_picker_lists_loaded_themes_and_switches(cx: &mut TestAppContext) {
    let (mut visual, view) = theme_mount(cx, "default-dark");
    let picker = bounds_of(&mut visual, &view, "combobox", "Theme");
    click_at(&mut visual, picker);
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let options = tree
        .nodes()
        .filter(|node| node.role == "option")
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    // The list is virtual: the first rows, in family and variant order.
    assert_eq!(
        options[..4],
        [
            "Aetheria",
            "Catppuccin · Latte",
            "Catppuccin · Mocha",
            "Default · Dark"
        ],
        "{options:?}"
    );
    let mocha = bounds_of(&mut visual, &view, "option", "Catppuccin · Mocha");
    click_at(&mut visual, mocha);
    no_error(&mut visual, &view);
    assert!(shows(&mut visual, &view, "THEME Catppuccin Mocha"));
}

/// Visible quads `width` x `height` logical pixels painted within `area`.
fn quads_sized(
    visual: &mut VisualTestContext,
    area: GeometryBounds,
    width: f32,
    height: f32,
) -> usize {
    visual.update(|window, _| {
        let scale = window.scale_factor();
        window
            .painted_quads()
            .iter()
            .filter(|quad| {
                let x = f64::from(quad.bounds.origin.x.0 / scale);
                quad.background
                    .as_solid()
                    .is_some_and(|color| color.a > 0.0)
                    && x >= area.x - 8.0
                    && x <= area.x + area.width + 8.0
                    && (quad.bounds.size.width.0 / scale - width).abs() < 0.01
                    && (quad.bounds.size.height.0 / scale - height).abs() < 0.01
            })
            .count()
    })
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
    assert_eq!(
        quads_sized(&mut visual, divider, 4.0, 24.0),
        0,
        "a bar at rest"
    );
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
    assert_eq!(
        quads_sized(&mut visual, divider, 4.0, 24.0),
        1,
        "no bar under the pointer"
    );
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
