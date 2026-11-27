//! Canvas stays per-frame hot; unrelated cold siblings keep their cache policy.

use crate::support::{
    Mount, Step, bounds, by_role, center, click_center, dispatch, last_error, settle,
};
use gpui::{Modifiers, TestAppContext, point, px};
use gpui_rhai::{FocusIndicators, MotionPreference};

#[gpui::test]
fn canvas_context_refinements_do_not_execute_rhai_or_replace_retained_geometry(
    cx: &mut TestAppContext,
) {
    for islands in [false, true] {
        let (mut visual, view) =
            Mount::new(include_str!("../../fixtures/canvas_current_color.rhai"))
                .motion(MotionPreference::None)
                .config(move |config| {
                    config
                        .host_sets_width(true)
                        .render_islands(islands)
                        .focus_indicators(FocusIndicators::Always)
                })
                .mount(cx);
        visual.update(|window, _| window.activate_window());
        settle(&mut visual, Step::Refresh, 3);
        for _ in 0..4 {
            dispatch(&mut visual, &view, by_role("button", "Outside"), "click").unwrap();
            // Real notifications schedule these frames. A forced window
            // refresh would deliberately redraw every cached view as well.
            settle(&mut visual, Step::Frame, 3);
        }
        let cache = visual.update(|_, cx| view.take_render_statistics(cx).unwrap());
        if islands && cfg!(feature = "gpui-pre") {
            assert!(
                cache.island_hits > 0,
                "positive control must actually replay cached content: {cache:?}"
            );
        }
        let outside = bounds(&mut visual, &view, "button", "Outside");
        click_center(&mut visual, outside);
        settle(&mut visual, Step::Frame, 3);
        let revision = visual.update(|_, cx| view.committed_revision(cx).unwrap());
        let before = bounds(&mut visual, &view, "button", "Canvas");
        visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        for target in [
            center(bounds(&mut visual, &view, "group", "Parent")),
            center(before),
            point(px(470.0), px(210.0)),
        ] {
            visual.simulate_mouse_move(target, None, Modifiers::none());
            settle(&mut visual, Step::Refresh, 3);
            assert_eq!(bounds(&mut visual, &view, "button", "Canvas"), before);
            assert_eq!(
                visual.update(|_, cx| view.committed_revision(cx).unwrap()),
                revision
            );
        }
        visual.simulate_keystrokes("shift-tab");
        settle(&mut visual, Step::Refresh, 3);
        visual.simulate_keystrokes("tab");
        settle(&mut visual, Step::Refresh, 3);
        assert!(
            visual
                .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
                .timings
                .is_empty()
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}
