//! The Gallery's overlay pages open from their triggers. The Menu page once passed a Button
//! whose `on_click` did nothing: the Button kept the click, so the menu never opened.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::mount_prepared;
use crate::support::{Host, Step, click_center, last_error, settle, snapshot};

fn mount(cx: &mut TestAppContext, page: &str) -> (VisualTestContext, ScriptViewHandle) {
    let launch = AcceptanceLaunch {
        page: page.to_owned(),
        density: "comfortable".to_owned(),
        theme: "default-dark".to_owned(),
        locale: "en".to_owned(),
        motion: MotionPreference::None,
    };
    let prepared = acceptance::view(&launch)
        .unwrap()
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
    let (window, view) =
        mount_prepared(cx, prepared, ("gallery", "gallery"), false, |host, view| {
            Host { host, view }
        });
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 3);
    (visual, view)
}

fn click_button(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) {
    let bounds = snapshot(visual, view)
        .find_by_role_and_name("button", name)
        .next()
        .and_then(|node| node.geometry)
        .unwrap_or_else(|| panic!("no button {name:?}"))
        .visual;
    click_center(visual, bounds);
    settle(visual, Step::Frame, 3);
}

fn menu_items(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> usize {
    snapshot(visual, view)
        .nodes()
        .filter(|node| node.role == "menuitem")
        .count()
}

#[gpui::test]
fn the_menu_page_opens_its_menu(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, "menu");
    assert_eq!(menu_items(&mut visual, &view), 0);
    click_button(&mut visual, &view, "File");
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(
        menu_items(&mut visual, &view) > 0,
        "clicking File opens the menu"
    );
}

#[gpui::test]
fn the_popover_page_opens_its_popover(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, "popover");
    let shown = |visual: &mut VisualTestContext| {
        snapshot(visual, &view)
            .nodes()
            .any(|node| node.name == "Only hosts with alerts in the last hour.")
    };
    assert!(!shown(&mut visual));
    click_button(&mut visual, &view, "Filters");
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(shown(&mut visual), "clicking Filters opens the popover");
}
