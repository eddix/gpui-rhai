//! The Tabs bar's all-tabs menu trigger fills the bar's height (#128): it asked for
//! `height(relative(1.0))` inside the Menu root, which has no definite height, so it
//! collapsed to the 16px chevron at the bar's top edge. It now takes `metrics.control`.

use gpui::TestAppContext;

use super::tabs_collection::{mount, script};
use crate::support::bounds;

#[gpui::test]
fn the_overflow_menu_trigger_fills_the_bar(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(3, 360, ",overflow_menu:true"));
    let tab = bounds(&mut visual, &view, "tab", "Tab number 0");
    let trigger = bounds(&mut visual, &view, "button", "All tabs");
    assert!(
        (trigger.height - tab.height).abs() < 0.5 && (trigger.y - tab.y).abs() < 0.5,
        "the trigger should span the bar like the tabs: tab {tab:?}, trigger {trigger:?}"
    );
}
