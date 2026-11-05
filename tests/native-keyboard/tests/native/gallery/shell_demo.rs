//! The Gallery's responsive shell measures its own frame and places its side regions by
//! that width: docked at 900px, both drawers at 480px, where Details opens the inspector
//! drawer and Escape closes it. A size read reaches its reader on the view's frame poll,
//! so the steps advance the clock.

use super::mount_prepared;
use crate::support::{Host, Step, bounds, click_center, last_error, nodes, settle};
use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::MotionPreference;
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

fn shows(visual: &mut VisualTestContext, view: &gpui_rhai::ScriptViewHandle, name: &str) -> bool {
    nodes(visual, view).iter().any(|node| node.name == name)
}

#[gpui::test]
fn the_responsive_shell_follows_its_frame(cx: &mut TestAppContext) {
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "app_shell".into(),
        ..AcceptanceLaunch::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let (window, view) = mount_prepared(cx, prepared, ("shell", "shell-view"), true, |host, view| {
        Host { host, view }
    });
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(1600.0), px(1000.0)));
    settle(&mut visual, Step::Tick, 5);
    assert!(shows(
        &mut visual,
        &view,
        "900px wide: sidebar dock, inspector dock"
    ));

    let narrow = bounds(&mut visual, &view, "button", "480");
    click_center(&mut visual, narrow);
    settle(&mut visual, Step::Tick, 5);
    assert!(shows(
        &mut visual,
        &view,
        "480px wide: sidebar hidden, inspector hidden"
    ));
    let details = bounds(&mut visual, &view, "button", "Details");
    click_center(&mut visual, details);
    settle(&mut visual, Step::Tick, 5);
    assert!(shows(
        &mut visual,
        &view,
        "480px wide: sidebar hidden, inspector overlay"
    ));
    bounds(&mut visual, &view, "dialog", "Demo shell inspector");
    visual.simulate_keystrokes("escape");
    settle(&mut visual, Step::Tick, 5);
    assert!(shows(
        &mut visual,
        &view,
        "480px wide: sidebar hidden, inspector hidden"
    ));
    assert_eq!(last_error(&mut visual, &view), None);
}
