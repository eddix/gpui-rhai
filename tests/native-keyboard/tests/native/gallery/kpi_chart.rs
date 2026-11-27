use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::ScriptViewHandle;

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, snapshot};

fn hover(v: &mut VisualTestContext, view: &ScriptViewHandle, chart: &str, frac: f64) {
    let rect = bounds(v, view, "group", chart);
    v.simulate_mouse_move(
        point(
            px((rect.x + frac * rect.width) as f32),
            px((rect.y + 0.65 * rect.height) as f32),
        ),
        None,
        Modifiers::none(),
    );
    ready(v, view);
}

#[gpui::test]
fn kpi_chart_pointer_keys_and_live_theme_share_one_sample(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "kpi_chart", "modern-light");
    v.simulate_resize(size(px(1240.), px(920.)));
    ready(&mut v, &view);
    click(&mut v, &view, "group", "Revenue chart");
    node(&mut v, &view, "status", "Thursday, Mar 7");
    let focus = v.update(|window, cx| window.focused(cx)).unwrap();
    v.simulate_keystrokes("left");
    ready(&mut v, &view);
    let details = node(&mut v, &view, "status", "Wednesday, Mar 6").description;
    assert!(details.contains("Direct: $3.7k"), "{details}");
    assert!(details.contains("Partners: No reading"), "{details}");

    hover(&mut v, &view, "Revenue chart", 0.99);
    node(&mut v, &view, "status", "Sunday, Mar 10");
    v.simulate_mouse_move(point(px(12.), px(12.)), None, Modifiers::none());
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Wednesday, Mar 6");

    v.update(|_, cx| assert!(view.select_theme("Clean", "Dark", cx).unwrap()));
    ready(&mut v, &view);
    assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
    v.simulate_keystrokes("end");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Sunday, Mar 10");
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("status", "Sunday, Mar 10")
            .next()
            .is_none()
    );
}

#[gpui::test]
fn kpi_chart_narrow_readout_is_contained_and_empty_plot_is_not_interactive(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "kpi_chart", "clean-dark");
    v.simulate_resize(size(px(1100.), px(920.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Narrow card");
    let card = bounds(&mut v, &view, "group", "Revenue");
    assert!(card.width <= 308., "{card:?}");
    for (fraction, date) in [(0.01, "Monday, Mar 4"), (0.99, "Sunday, Mar 10")] {
        hover(&mut v, &view, "Revenue chart", fraction);
        let tip = bounds(&mut v, &view, "status", date);
        assert!(
            tip.width >= 152. && tip.x >= card.x && tip.x + tip.width <= card.x + card.width + 1.,
            "{tip:?} is not a usable readout inside {card:?}"
        );
    }
    click(&mut v, &view, "button", "Drop first two days");
    hover(&mut v, &view, "Revenue chart", 0.01);
    node(&mut v, &view, "status", "Wednesday, Mar 6");
    click(&mut v, &view, "button", "Show conversion");
    hover(&mut v, &view, "Conversion chart", 0.99);
    assert!(
        node(&mut v, &view, "status", "Sunday, Mar 10")
            .description
            .contains("Direct: 7%")
    );
    click(&mut v, &view, "button", "Clear readings");
    hover(&mut v, &view, "Conversion chart", 0.99);
    let tree = snapshot(&mut v, &view);
    assert!(tree.nodes().any(|n| n.name == "No recorded readings"));
    assert!(
        tree.find_by_role_and_name("status", "Sunday, Mar 10")
            .next()
            .is_none()
    );
    click(&mut v, &view, "button", "Restore readings");
    hover(&mut v, &view, "Conversion chart", 0.99);
    node(&mut v, &view, "status", "Sunday, Mar 10");
}

#[gpui::test]
fn kpi_chart_rtl_pointer_and_visual_arrow_direction_agree(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "kpi_chart", "clean-light");
    v.simulate_resize(size(px(1240.), px(920.)));
    v.update(|_, cx| assert!(view.select_locale("ar", cx).unwrap()));
    ready(&mut v, &view);
    let plot = "Revenue مخطط";
    let check = |v: &mut VisualTestContext, stage: &str, expected: &str| {
        let rect = bounds(v, &view, "group", plot);
        let tree = snapshot(v, &view);
        let statuses: Vec<_> = tree
            .nodes()
            .filter(|node| node.role == "status")
            .map(|node| node.name.as_str())
            .collect();
        assert!(
            statuses.contains(&expected),
            "{stage} in {rect:?}: {statuses:?}"
        );
    };
    hover(&mut v, &view, plot, 0.99);
    check(&mut v, "physical right", "Monday, Mar 4");
    hover(&mut v, &view, plot, 0.01);
    check(&mut v, "physical left", "Sunday, Mar 10");
    click(&mut v, &view, "group", plot);
    node(&mut v, &view, "status", "Thursday, Mar 7");
    v.simulate_keystrokes("left");
    ready(&mut v, &view);
    check(&mut v, "physical left key", "Friday, Mar 8");
    v.simulate_keystrokes("right");
    ready(&mut v, &view);
    check(&mut v, "physical right key", "Thursday, Mar 7");
    v.simulate_keystrokes("home");
    ready(&mut v, &view);
    check(&mut v, "home key", "Monday, Mar 4");
}
