use super::mount_prepared;
use crate::support::{
    Host, Step, bounds, by_role, click_center, dispatch, last_error, nodes, settle,
};
use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::MotionPreference;
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

#[gpui::test]
fn cli_gallery_responsive_page_uses_clickable_module_local_callbacks(cx: &mut TestAppContext) {
    for locale in ["en", "ar"] {
        let prepared = acceptance::view(&AcceptanceLaunch {
            page: "foundations.responsive_grid".into(),
            locale: locale.into(),
            ..AcceptanceLaunch::default()
        })
        .unwrap()
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
        let (window, view) = mount_prepared(
            cx,
            prepared,
            (&format!("grid-{locale}"), &format!("grid-view-{locale}")),
            true,
            |host, view| Host { host, view },
        );
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.simulate_resize(size(px(1200.0), px(900.0)));
        settle(&mut visual, Step::Frame, 5);
        let first = bounds(&mut visual, &view, "button", "Select item 1");
        let second = bounds(&mut visual, &view, "button", "Select item 2");
        if locale == "ar" {
            assert!(first.x > second.x, "RTL row order: {first:?} {second:?}");
        } else {
            assert!(first.x < second.x);
        }
        click_center(&mut visual, second);
        settle(&mut visual, Step::Frame, 3);
        assert!(
            nodes(&mut visual, &view)
                .iter()
                .any(|node| node.name == "Selected item 2")
        );
        // This callback is defined in the plain page module and passed into
        // the formal Button. It must not be resolved in Gallery's entry AST.
        dispatch(
            &mut visual,
            &view,
            by_role("button", "Select first item"),
            "click",
        )
        .unwrap();
        settle(&mut visual, Step::Frame, 3);
        assert!(
            nodes(&mut visual, &view)
                .iter()
                .any(|node| node.name == "Selected item 1")
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}
