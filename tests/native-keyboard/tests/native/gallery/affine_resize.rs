//! The real Gallery mounts both specimens and accepts independent proposals.

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::{AuditRule, AutomationCommand, ElementGeometry, UiValue};

use super::conversation::{click, gallery, node, ready};
use crate::support::center;

fn drag(visual: &mut VisualTestContext, target: ElementGeometry, delta: (f32, f32)) {
    let start = center(target.visual);
    let (sin, cos) = 12.0_f32.to_radians().sin_cos();
    let end = point(
        start.x + px(delta.0 * cos - delta.1 * sin),
        start.y + px(delta.0 * sin + delta.1 * cos),
    );
    visual.simulate_mouse_move(start, None, Modifiers::none());
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
}

fn near(value: f64, expected: f64) {
    assert!((value - expected).abs() <= 0.6, "{value} != {expected}");
}

#[gpui::test]
fn gallery_affine_resize_controls_keep_distinct_accepted_state_after_a_theme_switch(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "foundations.affine", "modern-dark");
    visual.simulate_resize(size(px(1360.), px(1000.)));
    ready(&mut visual, &view);
    let start_pane = |v: &mut VisualTestContext| {
        node(v, &view, "region", "Transformed start pane")
            .geometry
            .unwrap()
    };
    let card = |v: &mut VisualTestContext| {
        node(v, &view, "region", "Transformed card surface")
            .geometry
            .unwrap()
    };
    let split_grip = |v: &mut VisualTestContext| {
        node(v, &view, "separator", "Transformed split")
            .geometry
            .unwrap()
    };
    let corner_grip = |v: &mut VisualTestContext| {
        node(v, &view, "separator", "Transformed card: se resize handle")
            .geometry
            .unwrap()
    };
    let start = start_pane(&mut visual);
    let old_card = card(&mut visual);
    let split = split_grip(&mut visual);
    assert!(
        split.visual.width > 20. && split.layout.width < 2.,
        "{split:?}"
    );
    drag(&mut visual, split, (24., 0.));
    ready(&mut visual, &view);
    near(
        start_pane(&mut visual).layout.width,
        start.layout.width + 24.,
    );
    near(card(&mut visual).layout.width, old_card.layout.width);
    let corner = corner_grip(&mut visual);
    drag(&mut visual, corner, (20., 12.));
    ready(&mut visual, &view);
    near(card(&mut visual).layout.width, old_card.layout.width + 20.);
    near(
        card(&mut visual).layout.height,
        old_card.layout.height + 12.,
    );
    click(&mut visual, &view, "button", "Straighten specimens");
    let straight = split_grip(&mut visual);
    near(straight.visual.width, straight.layout.width);
    visual
        .update(|_, cx| view.select_theme("Clean", "Light", cx))
        .unwrap();
    ready(&mut visual, &view);
    near(
        start_pane(&mut visual).layout.width,
        start.layout.width + 24.,
    );
    near(card(&mut visual).layout.width, old_card.layout.width + 20.);
    near(
        card(&mut visual).layout.height,
        old_card.layout.height + 12.,
    );
}

#[gpui::test]
fn gallery_affine_table_commits_a_column_width_and_preserves_it_when_straightened(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "foundations.affine", "clean-dark");
    visual.simulate_resize(size(px(1360.), px(1420.)));
    ready(&mut visual, &view);
    let geom = |v: &mut VisualTestContext, role, name| node(v, &view, role, name).geometry.unwrap();
    let width = |v: &mut VisualTestContext| geom(v, "columnheader", "Task").layout.width;
    let start = width(&mut visual);
    let width_of_neighbor = geom(&mut visual, "columnheader", "State").layout.width;
    let grip = geom(&mut visual, "separator", "Resize Task column");
    assert!(grip.visual.width > grip.layout.width + 4., "{grip:?}");
    drag(&mut visual, grip, (24., 0.));
    ready(&mut visual, &view);
    near(width(&mut visual), start + 24.);
    near(
        geom(&mut visual, "gridcell", "Prepare").layout.width,
        start + 24.,
    );
    near(
        geom(&mut visual, "columnheader", "State").layout.width,
        width_of_neighbor,
    );
    click(&mut visual, &view, "button", "Straighten specimens");
    visual
        .update(|_, cx| view.select_theme("Modern", "Light", cx))
        .unwrap();
    ready(&mut visual, &view);
    near(width(&mut visual), start + 24.);
}

#[gpui::test]
fn gallery_affine_sliders_commit_on_the_short_rail_and_keep_values_across_presets(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "foundations.affine", "modern-dark");
    visual.simulate_resize(size(px(1360.), px(1420.)));
    ready(&mut visual, &view);
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Action {
                    id: "gallery.set_scale".into(),
                    payload: Some(UiValue::String("150".into())),
                },
                window,
                cx,
            )
        })
        .unwrap();
    ready(&mut visual, &view);
    let findings = visual.update(|_, cx| view.composition_audit(cx)).unwrap();
    assert!(
        !findings
            .iter()
            .any(|finding| finding.rule == AuditRule::SpacingNotNested
                && finding.path.contains("affine-slider-stage")),
        "{findings:?}"
    );
    let level = |v: &mut VisualTestContext| node(v, &view, "slider", "Transformed level");
    let span = |v: &mut VisualTestContext| node(v, &view, "group", "Transformed span");
    let check = |v: &mut VisualTestContext| {
        assert_eq!(level(v).value, Some(UiValue::Float(85.)));
        let Some(UiValue::String(value)) = span(v).value else {
            panic!("The span should expose its accepted endpoints");
        };
        let (low, high) = value.split_once(" – ").expect("range text");
        assert_eq!(low.parse::<f64>().unwrap().to_bits(), 25.0_f64.to_bits());
        assert_eq!(high.parse::<f64>().unwrap().to_bits(), 60.0_f64.to_bits());
    };
    let click_value = |v: &mut VisualTestContext, target: ElementGeometry, value: f32| {
        let origin = center(target.visual);
        let (sin, cos) = 12.0_f32.to_radians().sin_cos();
        let dx = 156. * (value / 100. - 0.5);
        v.simulate_click(
            point(origin.x + px(dx * cos), origin.y + px(dx * sin)),
            Modifiers::none(),
        );
    };
    let target = level(&mut visual).geometry.unwrap();
    assert!(target.visual.height > target.layout.height + 20.);
    click_value(&mut visual, target, 85.);
    ready(&mut visual, &view);
    let target = span(&mut visual).geometry.unwrap();
    click_value(&mut visual, target, 60.);
    ready(&mut visual, &view);
    check(&mut visual);
    click(&mut visual, &view, "button", "Straighten specimens");
    visual
        .update(|_, cx| view.select_theme("Clean", "Light", cx))
        .unwrap();
    ready(&mut visual, &view);
    check(&mut visual);
}

#[gpui::test]
fn gallery_affine_scrollbar_moves_real_content_and_preserves_offset_on_theme_switch(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "foundations.affine", "clean-dark");
    visual.simulate_resize(size(px(1360.), px(1240.)));
    ready(&mut visual, &view);
    let region = |v: &mut VisualTestContext| node(v, &view, "region", "Transformed scroll area");
    let before = region(&mut visual);
    let geom = before.geometry.unwrap();
    let offset = |v: &mut VisualTestContext| {
        v.update(|window, cx| view.debug_frame_digest(window, cx).unwrap())
            .scroll_offsets
            .into_iter()
            .find(|(id, _)| *id == before.id)
            .unwrap()
            .1
    };
    assert_eq!(offset(&mut visual), [0., 0.]);
    let origin = center(geom.visual);
    let (sin, cos) = 12.0_f32.to_radians().sin_cos();
    #[allow(clippy::cast_possible_truncation)]
    let half = (
        geom.layout.width as f32 * 0.5,
        geom.layout.height as f32 * 0.5,
    );
    for (x, y, delta) in [
        (half.0 - 6., 14. - half.1, (0., 40.)),
        (14. - half.0, half.1 - 6., (40., 0.)),
    ] {
        let map = |x: f32, y: f32| {
            point(
                origin.x + px(x * cos - y * sin),
                origin.y + px(x * sin + y * cos),
            )
        };
        let (start, end) = (map(x, y), map(x + delta.0, y + delta.1));
        let original = offset(&mut visual);
        visual.simulate_mouse_move(start, None, Modifiers::none());
        visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
        assert_eq!(
            offset(&mut visual),
            original,
            "drag grab must not jump the content"
        );
        visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
        ready(&mut visual, &view);
    }
    let scrolled = offset(&mut visual);
    assert!(scrolled[0] < -40. && scrolled[1] < -100., "{scrolled:?}");
    click(&mut visual, &view, "button", "Straighten specimens");
    visual
        .update(|_, cx| view.select_theme("Modern", "Dark", cx))
        .unwrap();
    ready(&mut visual, &view);
    assert_eq!(region(&mut visual).id, before.id);
    assert_eq!(offset(&mut visual), scrolled);
}
