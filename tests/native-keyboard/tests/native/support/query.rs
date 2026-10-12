//! Reading a mounted view (accessibility nodes, geometry, errors) and acting on it (pointer
//! clicks at a node's centre, automation dispatch).

use gpui::{Modifiers, Pixels, Point, VisualTestContext, point, px};
use gpui_rhai::{
    AccessibilityNode, AccessibilityTree, AutomationCommand, AutomationLocator, AutomationResult,
    GeometryBounds, ScriptViewError, ScriptViewHandle,
};

/// The view's accessibility snapshot.
pub(crate) fn snapshot(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> AccessibilityTree {
    visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap())
}

/// Every accessibility node, in node order.
pub(crate) fn nodes(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> Vec<AccessibilityNode> {
    snapshot(visual, view).nodes().cloned().collect()
}

/// The first node with `test_id`.
#[track_caller]
pub(crate) fn node_by_test_id(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    test_id: &str,
) -> AccessibilityNode {
    snapshot(visual, view)
        .find_by_test_id(test_id)
        .next()
        .unwrap_or_else(|| panic!("no node with test id {test_id:?}"))
        .clone()
}

/// The accessible name of the first node with `test_id`, as it is (not trimmed).
#[track_caller]
pub(crate) fn text_of(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    test_id: &str,
) -> String {
    node_by_test_id(visual, view, test_id).name
}

/// The visual bounds of the first node with `role` and `name`.
#[track_caller]
pub(crate) fn bounds(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    snapshot(visual, view)
        .find_by_role_and_name(role, name)
        .next()
        .unwrap_or_else(|| panic!("no {role} named {name:?}"))
        .geometry
        .unwrap_or_else(|| panic!("the {role} named {name:?} has no geometry"))
        .visual
}

/// The view's last error.
pub(crate) fn last_error(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> Option<String> {
    visual.update(|_, cx| view.last_error(cx).unwrap())
}

/// The centre of `bounds`, as a window position.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn center(bounds: GeometryBounds) -> Point<Pixels> {
    point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    )
}

/// A primary click (no modifiers) at the centre of `bounds`; GPUI runs until parked after
/// each event.
pub(crate) fn click_center(visual: &mut VisualTestContext, bounds: GeometryBounds) {
    visual.simulate_click(center(bounds), Modifiers::none());
}

/// Locates the node with `test_id`.
pub(crate) fn by_test_id(test_id: &str) -> AutomationLocator {
    AutomationLocator::TestId {
        id: test_id.to_owned(),
    }
}

/// Locates the node with `role` and `name`.
pub(crate) fn by_role(role: &str, name: &str) -> AutomationLocator {
    AutomationLocator::RoleName {
        role: role.to_owned(),
        name: name.to_owned(),
    }
}

/// Dispatches `event` without a payload to the node `locator` finds, through the mounted
/// handlers (automation, not user input). Does not run until parked.
pub(crate) fn dispatch(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    locator: AutomationLocator,
    event: &str,
) -> Result<AutomationResult, ScriptViewError> {
    visual.update(|window, cx| {
        view.automate(
            AutomationCommand::Dispatch {
                locator,
                event: event.to_owned(),
                payload: None,
            },
            window,
            cx,
        )
    })
}
