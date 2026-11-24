use std::time::{Duration, Instant};

use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, UiValue};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, Mount, PresetRisks, bounds, snapshot};

#[gpui::test]
fn fan_out_grid_stock_ink_review_keeps_header_and_footer_backings_distinct(
    cx: &mut TestAppContext,
) {
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for (preset, expected) in [
        ("modern-light", 6),
        ("modern-dark", 0),
        ("clean-light", 7),
        ("clean-dark", 6),
    ] {
        let (mut visual, view) = gallery(cx, "fan_out_grid", preset);
        visual.simulate_resize(size(px(1380.), px(980.)));
        for density in ["comfortable", "compact"] {
            super::acceptance::action(
                &mut visual,
                &view,
                "gallery.set_density",
                UiValue::String(density.into()),
            );
            ready(&mut visual, &view);
            let (stock, findings) = visual.update(|_, cx| {
                (
                    view.theme_snapshot(cx).unwrap().variant,
                    view.composition_audit_with(&rules, cx).unwrap(),
                )
            });
            let board = findings
                .iter()
                .filter(|f| {
                    f.path.contains("/page-fan_out_grid/") && f.path.contains("/size/fan-out-grid/")
                })
                .collect::<Vec<_>>();
            assert_eq!(board.len(), expected, "{preset}/{density}: {board:?}");
            for finding in board {
                assert!(
                    risks.record(&stock, finding),
                    "{preset}/{density}: {finding:?}"
                );
                let mut swapped = finding.clone();
                let evidence = swapped.contrast.as_mut().unwrap();
                let other = if preset == "clean-dark" {
                    // A dark Clean header is on the Gallery paper, not
                    // the material of the card with the legend inside it.
                    if finding.path.contains("/body/footer/") {
                        [27, 27, 30]
                    } else {
                        [28, 28, 31]
                    }
                } else {
                    [166, 166, 166]
                };
                evidence.background_srgb = Some(other.map(|v| f64::from(v) / 255.));
                assert!(!risks.record(&stock, &swapped));
                swapped.contrast.clone_from(&finding.contrast);
                swapped.path.push_str("/unreviewed-child");
                assert!(!risks.record(&stock, &swapped));
            }
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn fan_out_grid_keyboard_uses_real_rows_and_focus_survives_reflow(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "fan_out_grid", "modern-light");
    v.simulate_resize(size(px(1250.), px(920.)));
    ready(&mut v, &view);
    let first = bounds(&mut v, &view, "gridcell", "Task 0");
    let next_row = (1..144)
        .find(|i| bounds(&mut v, &view, "gridcell", &format!("Task {i}")).y > first.y + 5.0)
        .unwrap();
    click(&mut v, &view, "gridcell", "Task 0");
    node(&mut v, &view, "tooltip", "Task 0");
    assert_eq!(
        node(&mut v, &view, "gridcell", "Task 0").selected,
        Some(true)
    );
    v.simulate_keystrokes("down");
    ready(&mut v, &view);
    let name = format!("Task {next_row}");
    node(&mut v, &view, "tooltip", &name);
    let stable = node(&mut v, &view, "gridcell", &name).id;
    let focus = v.update(|window, cx| window.focused(cx)).unwrap();
    v.simulate_resize(size(px(700.), px(920.)));
    ready(&mut v, &view);
    assert!(bounds(&mut v, &view, "gridcell", &name).y > first.y + 20.0);
    assert_eq!(node(&mut v, &view, "gridcell", &name).id, stable);
    assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        &format!("Pinned task:{next_row} at revision 0"),
    );
    assert_eq!(node(&mut v, &view, "gridcell", &name).selected, Some(true));
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert!(
        !snapshot(&mut v, &view)
            .nodes()
            .any(|item| item.role == "tooltip")
    );
}

#[gpui::test]
fn fan_out_grid_filters_without_moving_and_retries_only_accepted_failures(cx: &mut TestAppContext) {
    for theme in ["modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "fan_out_grid", theme);
        v.simulate_resize(size(px(1240.), px(920.)));
        ready(&mut v, &view);
        let before = bounds(&mut v, &view, "gridcell", "Task 2");
        click(&mut v, &view, "button", "2 failed");
        assert_eq!(bounds(&mut v, &view, "gridcell", "Task 2"), before);
        assert_eq!(
            node(&mut v, &view, "button", "2 failed").pressed,
            Some(true)
        );
        click(&mut v, &view, "button", "Host-controlled inspection");
        click(&mut v, &view, "button", "Refuse next pin");
        click(&mut v, &view, "gridcell", "Task 2");
        node(&mut v, &view, "status", "The Host kept the pinned task");
        assert_eq!(
            node(&mut v, &view, "gridcell", "Task 2").selected,
            Some(false)
        );
        let g = node(&mut v, &view, "gridcell", "Task 2").geometry.unwrap();
        assert!(
            g.visual.x < g.layout.x - 1.0,
            "attention expands native geometry"
        );
        let exposed = (g.visual.x + g.layout.x) * 0.5;
        v.simulate_click(
            point(
                px(exposed as f32),
                px((g.visual.y + g.visual.height * 0.5) as f32),
            ),
            Modifiers::none(),
        );
        ready(&mut v, &view);
        node(&mut v, &view, "status", "Pinned docs/索引 at revision 0");
        node(&mut v, &view, "tooltip", "Task 2");
        click(&mut v, &view, "button", "Retry failed");
        node(
            &mut v,
            &view,
            "status",
            "Retry accepted: docs/索引, notes:2",
        );
        assert!(
            !snapshot(&mut v, &view)
                .nodes()
                .any(|n| n.name == "Retry failed")
        );
        assert_eq!(
            node(&mut v, &view, "gridcell", "Task 2").selected,
            Some(true)
        );
        assert!(
            node(&mut v, &view, "tooltip", "Task 2")
                .description
                .contains("Retrying")
        );
        assert_eq!(
            node(&mut v, &view, "button", "0 failed").pressed,
            Some(true)
        );
        click(&mut v, &view, "button", "0 failed");
        assert!(
            !snapshot(&mut v, &view)
                .nodes()
                .any(|n| n.name == "0 failed")
        );
        click(&mut v, &view, "button", "Empty field");
        assert!(
            !snapshot(&mut v, &view)
                .nodes()
                .any(|n| n.role == "gridcell")
        );
    }
}

#[gpui::test]
fn fan_out_grid_switches_footer_paper_without_replacing_its_controls(cx: &mut TestAppContext) {
    use crate::support::{glyphs_with_ink, live_fills};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("fan_out_grid_footer_paper"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "fan_out_grid", "modern-dark");
    v.simulate_resize(size(px(1240.), px(920.)));
    ready(&mut v, &view);
    let cell = node(&mut v, &view, "gridcell", "Task 2").id;
    let retry = node(&mut v, &view, "button", "Retry failed").id;
    let filter = node(&mut v, &view, "button", "2 failed");
    let checked = |v: &mut VisualTestContext, ink, enclosed| {
        let grid = bounds(v, &view, "grid", "Tasks");
        let retry = bounds(v, &view, "button", "Retry failed");
        let grid_center = (grid.x + grid.width * 0.5, grid.y + grid.height * 0.5);
        let retry_center = (retry.x + retry.width * 0.5, retry.y + retry.height * 0.5);
        let rectangles = live_fills(v, ink);
        let inside = |r: &crate::support::Rect, (x, y)| {
            f64::from(r.x) <= x
                && x <= f64::from(r.right())
                && f64::from(r.y) <= y
                && y <= f64::from(r.bottom())
        };
        assert!(rectangles.iter().any(|r| inside(r, grid_center)));
        assert_eq!(
            rectangles
                .iter()
                .any(|r| inside(r, grid_center) && inside(r, retry_center)),
            enclosed,
            "grid: {grid:?}, retry: {retry:?}, paper: {rectangles:?}"
        );
    };
    checked(&mut v, 0x232326ff, false);
    let painted = filter.geometry.unwrap().visual;
    assert!(glyphs_with_ink(&mut v, painted, 0xf0f0f0ff) > 0);
    assert!(glyphs_with_ink(&mut v, painted, 0x8a8a8aff) > 0);
    click(&mut v, &view, "button", "2 failed");
    let focus = v.update(|window, cx| window.focused(cx));
    assert!(focus.is_some());

    v.update(|_, cx| view.select_theme("Clean", "Dark", cx))
        .unwrap();
    ready(&mut v, &view);
    checked(&mut v, 0x1c1c1fff, true);
    assert_eq!(node(&mut v, &view, "gridcell", "Task 2").id, cell);
    assert_eq!(node(&mut v, &view, "button", "Retry failed").id, retry);
    assert_eq!(node(&mut v, &view, "button", "2 failed").id, filter.id);
    assert_eq!(
        node(&mut v, &view, "button", "2 failed").pressed,
        Some(true)
    );
    assert_eq!(v.update(|window, cx| window.focused(cx)), focus);

    v.update(|_, cx| view.select_theme("Modern", "Dark", cx))
        .unwrap();
    ready(&mut v, &view);
    checked(&mut v, 0x232326ff, false);
    click(&mut v, &view, "button", "Retry failed");
    node(
        &mut v,
        &view,
        "status",
        "Retry accepted: docs/索引, notes:2",
    );
}

#[gpui::test]
fn fan_out_grid_1024_cells_stay_mounted_and_use_business_identity(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(
        r#"
        import "patterns/fan_out_grid" as fan;
        fn view(ctx){
            let tasks=[];
            for i in 0..1024{tasks.push(#{id:"item/"+i.to_string(),label:"Item "+i.to_string(),
                status:if i==20{"failed"}else if i%9==1{"done"}else{"queued"},
                cost_micros:if i==20{-3000}else{500}});}
            column([fan::FanOutGrid(#{key:"big",run:"large",tasks:tasks})]).with_style(style().width(px(724)))
        }
        "#,
    )
    .registry("patterns/fan_out_grid")
    .motion(MotionPreference::None)
    .mount(cx);
    v.update(|window, _| window.activate_window());
    ready(&mut v, &view);
    assert_eq!(
        snapshot(&mut v, &view)
            .nodes()
            .filter(|n| n.role == "gridcell")
            .count(),
        1024
    );
    node(
        &mut v,
        &view,
        "status",
        "114 Done · 1 Failed · $0.51 · 0ms so far",
    );
    let a = bounds(&mut v, &view, "gridcell", "Item 0");
    let b = bounds(&mut v, &view, "gridcell", "Item 1023");
    assert!(b.y > a.y && (b.width - a.width).abs() <= 0.5);
    click(&mut v, &view, "gridcell", "Item 0");
    v.simulate_keystrokes("end");
    ready(&mut v, &view);
    node(&mut v, &view, "tooltip", "Item 1023");
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn fan_out_grid_shine_frames_do_not_evaluate_rhai(cx: &mut TestAppContext) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "fan_out_grid".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .runtime_clock(manual.clock())
    .motion_preference(MotionPreference::Normal)
    .prepare()
    .unwrap();
    let (win, view) = super::mount_prepared(
        cx,
        prepared,
        ("fan-native", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*win, cx);
    v.simulate_resize(size(px(1220.), px(920.)));
    ready(&mut v, &view);
    let graph = bounds(&mut v, &view, "grid", "Tasks");
    let points = |v: &mut VisualTestContext| {
        v.update(|window, _| {
            window.visual_probe_with_painted_scene(|scene| {
                fn count(s: &gpui::Scene) -> usize {
                    let total = s.layered_fills.len()
                        + s.isolated_groups
                            .iter()
                            .map(|g| count(g.source()))
                            .sum::<usize>();
                    #[cfg(feature = "gpui-fast")]
                    let total = total
                        + s.layers
                            .frames
                            .iter()
                            .filter(|f| f.tile_size == 0)
                            .flat_map(|f| f.content.scenes())
                            .map(count)
                            .sum::<usize>();
                    total
                }
                count(scene)
            })
        })
    };
    manual.advance(Duration::from_millis(720));
    ready(&mut v, &view);
    let fills = points(&mut v);
    assert!(fills >= 8, "native gradient cohort: {fills}");
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    for _ in 0..5 {
        manual.advance(Duration::from_millis(160));
        ready(&mut v, &view);
    }
    assert_eq!(bounds(&mut v, &view, "grid", "Tasks"), graph);
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
