//! The Tabs bar's all-tabs menu trigger fills the bar's height (#128): it asked for
//! `height(relative(1.0))` inside the Menu root, which has no definite height, so it
//! collapsed to the 16px chevron at the bar's top edge. It now takes `metrics.control`.
//! Its hover fill has a control's corners, as IconButton's (feedback #37).

use std::collections::BTreeMap;

use gpui::{Modifiers, TestAppContext};
use gpui_rhai::{MotionPreference, Rgba8, ThemeTokenOverrides};

use super::tabs_collection::{mount, script};
use crate::support::{Mount, Step, bounds, filled, settle};

/// `surface_hover`, overridden so the trigger's hover fill is unambiguous.
const HOVER: u32 = 0x0000_ffff;

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

#[gpui::test]
fn the_overflow_menu_trigger_hover_has_rounded_corners(cx: &mut TestAppContext) {
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([("surface_hover".to_owned(), Rgba8::from_rgba_hex(HOVER))]),
        ..Default::default()
    };
    let (mut visual, view) = Mount::components(script(3, 360, ",overflow_menu:true"))
        .motion(MotionPreference::None)
        .with(move |view| view.theme_token_overrides(overrides))
        .id("tabs")
        .mount(cx);
    settle(&mut visual, Step::Tick, 3);
    let trigger = bounds(&mut visual, &view, "button", "All tabs");
    let center = gpui::point(
        gpui::px((trigger.x + trigger.width / 2.0) as f32),
        gpui::px((trigger.y + trigger.height / 2.0) as f32),
    );
    visual.simulate_mouse_move(center, None, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    let fills = filled(&mut visual, HOVER);
    let fill = fills
        .iter()
        .find(|quad| (quad.bounds.width - trigger.width as f32).abs() < 0.5)
        .unwrap_or_else(|| panic!("no hover fill the trigger's width: {fills:?}"));
    assert!(
        fill.corner_radii.iter().all(|radius| *radius > 0.0),
        "{fill:?}"
    );
}
