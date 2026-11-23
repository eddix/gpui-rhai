use gpui::{
    Modifiers, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px, size,
};
use gpui_rhai::{ScriptViewHandle, UiValue};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Mount, bounds, center, snapshot};

fn pick(v: &mut VisualTestContext, view: &ScriptViewHandle, label: &str, value: &str) {
    click(v, view, "combobox", label);
    click(v, view, "option", value);
}

fn status(v: &mut VisualTestContext, view: &ScriptViewHandle, name: &str, value: &str) {
    let item = node(v, view, "status", name);
    assert_eq!(item.description, value);
}

#[gpui::test]
fn breakdown_chart_header_uses_its_design_insets_and_wraps_the_total(cx: &mut TestAppContext) {
    for (theme, wide, narrow) in [("modern-light", 24., 24.), ("clean-dark", 16., 12.)] {
        let (mut v, view) = gallery(cx, "breakdown_chart", theme);
        v.simulate_resize(size(px(1260.), px(1000.)));
        ready(&mut v, &view);
        let check = |v: &mut VisualTestContext, inset: f64| {
            let card = bounds(v, &view, "group", "Weekly workload");
            let heading = bounds(v, &view, "heading", "Weekly workload");
            assert!(
                (heading.x - card.x - inset).abs() <= 0.5,
                "{theme}: {card:?} {heading:?}"
            );
        };
        check(&mut v, wide);
        click(&mut v, &view, "button", "Narrow card");
        check(&mut v, narrow);
    }

    let script = r#"
        import "patterns/breakdown_chart" as chart;
        fn view(ctx){
            chart::BreakdownChart(#{key:"work",dataset:"delivery",title:"Long workload",
                records:[#{team:"a", count:987654321}], dimensions:[#{id:"team",label:"Team",
                    levels:[#{id:"a",label:"A"}]}], measures:[#{id:"units",label:"Attempts",
                    field:"count",unit:"validation attempts"}]})
                .with_style(style().width(px(230)))
        }
    "#;
    let (mut v, view) = Mount::new(script)
        .registry("patterns/breakdown_chart")
        .bundled_assets()
        .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
        .motion(gpui_rhai::MotionPreference::None)
        .mount(cx);
    ready(&mut v, &view);
    let card = bounds(&mut v, &view, "group", "Long workload");
    let title = bounds(&mut v, &view, "heading", "Long workload");
    let tree = snapshot(&mut v, &view);
    let total = tree
        .nodes()
        .find(|node| node.name.starts_with("Visible total:"))
        .unwrap();
    let total = total.geometry.unwrap().visual;
    assert!(
        total.y >= title.y + title.height - 1.,
        "{title:?} {total:?}"
    );
    assert!((card.x + 24. ..=card.x + card.width - 24.).contains(&total.x));
    assert!(
        total.x + total.width <= card.x + card.width - 24. + 1.,
        "{card:?} {total:?}"
    );
}

#[gpui::test]
fn breakdown_chart_legend_wraps_in_modern_and_scrolls_in_clean(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "breakdown_chart", "clean-light");
    v.simulate_resize(size(px(1260.), px(1100.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Narrow card");
    click(&mut v, &view, "button", "Dense legend");
    click(&mut v, &view, "button", "Automated");
    let group = bounds(&mut v, &view, "group", "Segments");
    let first = bounds(&mut v, &view, "button", "Automated");
    let edge = bounds(&mut v, &view, "button", "Independent verification");
    let reset = bounds(&mut v, &view, "button", "Show all");
    assert!(
        (first.y - edge.y).abs() <= 1. && edge.x > group.x + group.width,
        "Clean keeps a single scroll lane: {first:?}, {edge:?}, {group:?}"
    );
    assert!(
        reset.x >= group.x + group.width - 1.,
        "reset is outside the scroll port: {reset:?}, {group:?}"
    );
    v.simulate_event(ScrollEvent {
        position: center(group),
        delta: ScrollDelta::Pixels(point(px(-900.), px(0.))),
        ..Default::default()
    });
    ready(&mut v, &view);
    let visible = bounds(&mut v, &view, "button", "Independent verification");
    assert!(visible.x + visible.width <= group.x + group.width + 1.);
    assert!(visible.x < edge.x - 50., "{edge:?} -> {visible:?}");
    assert_eq!(bounds(&mut v, &view, "button", "Show all"), reset);
    v.simulate_event(ScrollEvent {
        position: center(group),
        delta: ScrollDelta::Pixels(point(px(900.), px(0.))),
        ..Default::default()
    });
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Automated");
    let group = bounds(&mut v, &view, "group", "Segments");
    for label in [
        "Manual",
        "Queued",
        "Remote synchronization",
        "Historical backfill",
        "Independent verification",
    ] {
        v.simulate_keystrokes("tab");
        ready(&mut v, &view);
        let expected = node(&mut v, &view, "button", label).id;
        let focus = v.update(|window, cx| view.debug_frame_digest(window, cx).unwrap().focus_path);
        assert_eq!(focus.first(), Some(&expected), "Tab reaches {label}");
    }
    let shown = bounds(&mut v, &view, "button", "Independent verification");
    assert!(
        shown.x + shown.width <= group.x + group.width + 1.,
        "focused chip is revealed: {shown:?} inside {group:?}"
    );
    assert!(shown.x >= group.x - 1., "{shown:?} inside {group:?}");
    v.simulate_keystrokes("space");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "button", "Independent verification").pressed,
        Some(false),
        "the scrolled-to chip still responds to keyboard activation"
    );
    click(&mut v, &view, "button", "Show all");
    assert_eq!(
        node(&mut v, &view, "button", "Independent verification").pressed,
        Some(true)
    );

    v.update(|_, cx| assert!(view.select_theme("Modern", "Light", cx).unwrap()));
    ready(&mut v, &view);
    let group = bounds(&mut v, &view, "group", "Segments");
    let first = bounds(&mut v, &view, "button", "Automated");
    let edge = bounds(&mut v, &view, "button", "Independent verification");
    assert!(edge.y > first.y + 10., "Modern wraps: {first:?}, {edge:?}");
    assert!(edge.x + edge.width <= group.x + group.width + 1.);
}

#[gpui::test]
fn breakdown_chart_measure_and_visibility_share_totals_and_retain_raw_cells(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "breakdown_chart", "modern-light");
    v.simulate_resize(size(px(1260.), px(1000.)));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Visible total: 9 tasks");
    pick(&mut v, &view, "Measure", "Effort");
    node(&mut v, &view, "status", "Visible total: 31 points");
    let bar = node(&mut v, &view, "button", "Operations bar");
    assert_eq!(
        bar.description,
        "Operations: -2 points; Automated: 2; Manual: -7; Queued: 3"
    );
    assert_eq!(
        node(&mut v, &view, "row", "Operations").description,
        bar.description
    );

    click(&mut v, &view, "button", "Automated");
    node(&mut v, &view, "status", "Visible total: 10 points");
    assert_eq!(
        node(&mut v, &view, "button", "Automated").pressed,
        Some(false)
    );
    assert_eq!(
        node(&mut v, &view, "row", "API").description,
        "API: 3 points; Manual: 6; Queued: -3"
    );
    assert_eq!(node(&mut v, &view, "gridcell", "12").name, "12");
    click(&mut v, &view, "button", "Manual");
    click(&mut v, &view, "button", "Queued");
    assert_eq!(node(&mut v, &view, "button", "Queued").pressed, Some(true));
    node(&mut v, &view, "status", "Visible total: 0 points");
    click(&mut v, &view, "button", "Show all");
    node(&mut v, &view, "status", "Visible total: 31 points");
    click(&mut v, &view, "button", "Post a correction");
    node(&mut v, &view, "status", "Visible total: 19 points");
    status_after_api_click(
        &mut v,
        &view,
        "API: 3 points; Automated: 0; Manual: 6; Queued: -3",
    );
}

fn status_after_api_click(v: &mut VisualTestContext, view: &ScriptViewHandle, expected: &str) {
    click(v, view, "button", "API bar");
    status(v, view, "API breakdown", expected);
}

#[gpui::test]
fn breakdown_chart_narrow_plot_has_contained_tip_and_keyboard_attention(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "breakdown_chart", "clean-dark");
    v.simulate_resize(size(px(1260.), px(1000.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Narrow card");
    let card = bounds(&mut v, &view, "group", "Weekly workload");
    assert!(card.width <= 320., "{card:?}");
    status_after_api_click(
        &mut v,
        &view,
        "API: 4 tasks; Automated: 2; Manual: 1; Queued: 1",
    );
    v.simulate_mouse_move(point(px(8.), px(8.)), None, Modifiers::none());
    ready(&mut v, &view);
    v.simulate_keystrokes("right");
    ready(&mut v, &view);
    status(
        &mut v,
        &view,
        "Operations breakdown",
        "Operations: 3 tasks; Automated: 1; Manual: 1; Queued: 1",
    );
    let tip = bounds(&mut v, &view, "status", "Operations breakdown");
    assert!(
        tip.x >= card.x && tip.x + tip.width <= card.x + card.width + 1.,
        "{tip:?} inside {card:?}"
    );
    let web = bounds(&mut v, &view, "row", "Web");
    v.simulate_mouse_move(center(web), None, Modifiers::none());
    ready(&mut v, &view);
    status(
        &mut v,
        &view,
        "Web breakdown",
        "Web: 2 tasks; Automated: 1; Manual: 1; Queued: 0",
    );
    v.simulate_mouse_move(point(px(8.), px(8.)), None, Modifiers::none());
    ready(&mut v, &view);
    let focus = v.update(|window, cx| window.focused(cx)).unwrap();
    v.update(|_, cx| assert!(view.select_theme("Modern", "Light", cx).unwrap()));
    ready(&mut v, &view);
    assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
    v.simulate_keystrokes("end");
    ready(&mut v, &view);
    assert!(
        node(&mut v, &view, "status", "Support breakdown")
            .description
            .starts_with("Support: 0 tasks")
    );

    let before = bounds(&mut v, &view, "columnheader", "Queued");
    let row = bounds(&mut v, &view, "row", "API");
    let sized_card = bounds(&mut v, &view, "group", "Weekly workload");
    assert!(before.x + before.width > sized_card.x + sized_card.width);
    v.simulate_event(ScrollEvent {
        position: point(
            px((sized_card.x + sized_card.width / 2.) as f32),
            px((row.y + row.height / 2.) as f32),
        ),
        delta: ScrollDelta::Pixels(point(px(-240.), px(0.))),
        ..Default::default()
    });
    ready(&mut v, &view);
    let after = bounds(&mut v, &view, "columnheader", "Queued");
    assert!(after.x < before.x - 30., "{before:?} -> {after:?}");
    assert!((after.y - before.y).abs() <= 1., "{before:?} -> {after:?}");
}

#[gpui::test]
fn breakdown_chart_switches_domains_and_offers_unsegmented_and_empty_cases(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "breakdown_chart", "modern-dark");
    v.simulate_resize(size(px(1260.), px(1000.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Automated");
    pick(&mut v, &view, "Slice", "Source");
    node(&mut v, &view, "button", "Automated bar");
    assert_eq!(
        node(&mut v, &view, "combobox", "Segment").value,
        Some(UiValue::String("None".into()))
    );
    node(&mut v, &view, "status", "Visible total: 9 tasks");
    pick(&mut v, &view, "Segment", "Stage");
    assert_eq!(node(&mut v, &view, "button", "Done").pressed, Some(true));
    click(&mut v, &view, "button", "Empty workload");
    node(&mut v, &view, "status", "Visible total: 0 tasks");
    assert!(
        node(&mut v, &view, "row", "Automated")
            .description
            .starts_with("Automated: 0 tasks")
    );
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("row", "API")
            .next()
            .is_none()
    );
}
