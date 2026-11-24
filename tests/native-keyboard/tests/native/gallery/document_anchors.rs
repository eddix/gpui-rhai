use gpui::{ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::ScriptViewHandle;

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, center, snapshot};

fn flag(v: &mut VisualTestContext, view: &ScriptViewHandle) -> gpui_rhai::GeometryBounds {
    bounds(v, view, "status", "Live document cursor")
}

fn has_flag(v: &mut VisualTestContext, view: &ScriptViewHandle) -> bool {
    snapshot(v, view)
        .find_by_role_and_name("status", "Live document cursor")
        .next()
        .is_some()
}

#[gpui::test]
fn document_anchors_follow_a_new_plan_even_at_the_same_revision(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "scene.document_anchors", "modern-light");
    v.simulate_resize(size(px(1220.), px(950.)));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Document revision 1");
    let start = flag(&mut v, &view);
    click(
        &mut v,
        &view,
        "button",
        "Move cursor without changing revision",
    );
    node(&mut v, &view, "status", "Document revision 1");
    let moved = flag(&mut v, &view);
    assert!(
        (moved.x - start.x).abs() + (moved.y - start.y).abs() > 20.0,
        "{start:?} {moved:?}"
    );

    click(&mut v, &view, "button", "Narrow document");
    let wrapped = flag(&mut v, &view);
    assert!(wrapped.y > moved.y + 5.0, "{moved:?} {wrapped:?}");
    click(&mut v, &view, "button", "Open anchored comment");
    node(&mut v, &view, "heading", "Anchored comment");
    click(&mut v, &view, "button", "Accept text suggestion");
    node(&mut v, &view, "status", "Document revision 2");
    node(&mut v, &view, "status", "Text anchors ready");
    flag(&mut v, &view);
}

#[gpui::test]
fn document_anchors_follow_transform_clip_and_source_lifecycle(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "scene.document_anchors", "clean-dark");
    v.simulate_resize(size(px(1220.), px(950.)));
    ready(&mut v, &view);
    let upright = flag(&mut v, &view);
    click(&mut v, &view, "button", "Tilt document text");
    let rotated = flag(&mut v, &view);
    if gpui_rhai::EXTENDED_RENDERING {
        assert!(rotated.y > upright.y + 5.0, "{upright:?} {rotated:?}");
    } else {
        assert!(
            (rotated.y - upright.y).abs() < 1.0,
            "{upright:?} {rotated:?}"
        );
    }
    let viewport = bounds(&mut v, &view, "group", "Document viewport");
    v.simulate_event(ScrollEvent {
        position: center(viewport),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-28.0))),
        ..Default::default()
    });
    ready(&mut v, &view);
    let wheeled = flag(&mut v, &view);
    assert!(wheeled.y < rotated.y - 12.0, "{rotated:?} {wheeled:?}");
    click(&mut v, &view, "button", "Scroll document");
    assert!(
        !has_flag(&mut v, &view),
        "a clipped caster must not leave a floating flag"
    );
    click(&mut v, &view, "button", "Restore document scroll");
    assert!((flag(&mut v, &view).y - rotated.y).abs() < 1.0);

    click(&mut v, &view, "button", "Remove text source");
    node(&mut v, &view, "status", "Awaiting text anchors");
    assert!(!has_flag(&mut v, &view));
    click(&mut v, &view, "button", "Restore text source");
    node(&mut v, &view, "status", "Text anchors ready");
    assert!((flag(&mut v, &view).y - rotated.y).abs() < 1.0);
}
