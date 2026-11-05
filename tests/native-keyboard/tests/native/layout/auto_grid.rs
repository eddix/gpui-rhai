//! Responsive geometry, intrinsic content, idle convergence and no Rhai resize loop.
use crate::support::{Mount, Step, bounds, last_error, settle};
use gpui::{TestAppContext, px, size};

#[gpui::test]
fn responsive_tracks_are_equal_bounded_and_square_through_resize(cx: &mut TestAppContext) {
    let source = r#"
        fn view(ctx){let tiles=[];
            for index in 0..20 {
                let label = `Tile ${index}`;
                tiles.push(box([text(label)]).with_key(label)
                    .accessibility_role("group").accessibility_label(label)
                    .with_style(style().width(relative(1.0)).aspect_ratio(1).background(theme_color("surface_raised"))));
            }
            box(tiles).accessibility_role("group").accessibility_label("Grid")
                .with_style(style().width(relative(1.0)).auto_grid_columns(128,160,224)
                    .gap(px(16)).padding(px(12)).border(px(2)))
        }
    "#;
    let (mut visual, view) = Mount::components(source).mount(cx);
    settle(&mut visual, Step::Frame, 5);
    visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
    for width in [720.0, 600.0, 1000.0, 480.0, 720.0] {
        visual.simulate_resize(size(px(width), px(1000.0)));
        settle(&mut visual, Step::Frame, 4);
        let grid = bounds(&mut visual, &view, "group", "Grid");
        let tiles: Vec<_> = (0..20)
            .map(|index| bounds(&mut visual, &view, "group", &format!("Tile {index}")))
            .collect();
        let first = tiles[0];
        assert!(
            first.width >= 128.0 && first.width <= 224.0,
            "{width}: {first:?}"
        );
        assert!(
            (first.x - grid.x - 14.0).abs() < 1.0,
            "padding/border: {grid:?} {first:?}"
        );
        for tile in &tiles {
            assert!(
                (tile.width - first.width).abs() <= 1.0,
                "last row must not stretch: {tiles:?}"
            );
            assert!(
                (tile.height - tile.width).abs() <= 1.0,
                "aspect ratio: {tile:?}"
            );
            assert!(
                tile.x + tile.width <= grid.x + grid.width - 13.0,
                "outside content: {tile:?}"
            );
        }
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(bounds(&mut visual, &view, "group", "Tile 0"), first);
        let next = visual.update(|window, cx| window.simulate_next_frame(cx));
        assert_eq!(next, 0, "stable native layout must become idle");
        assert_eq!(last_error(&mut visual, &view), None);
    }
    let performance = visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
    assert!(
        performance.timings.is_empty(),
        "resize refinement must not run Rhai: {:?}",
        performance.timings
    );
}

#[gpui::test]
fn automatic_rows_keep_intrinsic_content_heights(cx: &mut TestAppContext) {
    let source = r#"
        fn view(ctx){box([
            text("A").with_style(style().height(px(40))).accessibility_role("group").accessibility_label("A"),
            text("B").with_style(style().height(px(90))).accessibility_role("group").accessibility_label("B"),
            text("C").with_style(style().height(px(50))).accessibility_role("group").accessibility_label("C")
        ]).with_style(style().width(px(400)).auto_grid_columns(128,160,224).gap(px(16)))}
    "#;
    let (mut visual, view) = Mount::components(source).mount(cx);
    settle(&mut visual, Step::Frame, 5);
    let a = bounds(&mut visual, &view, "group", "A");
    let b = bounds(&mut visual, &view, "group", "B");
    let c = bounds(&mut visual, &view, "group", "C");
    assert_eq!(a.y, b.y);
    assert!((c.y - b.y - 106.0).abs() < 1.0, "{a:?} {b:?} {c:?}");
    assert_eq!(last_error(&mut visual, &view), None);
}

/// Tiles in a 600px grid whose minimum track is a density token (24px or
/// 16px times six), so comfortable lays out four columns and compact six.
const TOKEN_GRID: &str = r#"
    fn state_schema(){#{fields:#{compact:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
    fn toggle(ctx,payload){ctx.set_state("compact",!ctx.get_state("compact"));}
    fn view(ctx){let tiles=[];
        for index in 0..12 {
            let label = `Tile ${index}`;
            tiles.push(box([text(label)]).with_key(label)
                .accessibility_role("group").accessibility_label(label)
                .with_style(style().width(relative(1.0)).height(px(40))));
        }
        column([
            text("Toggle").accessibility_role("button").accessibility_label("Toggle").on_click(Fn("toggle")),
            box(tiles).accessibility_role("group").accessibility_label("Grid")
                .with_style(style().width(px(600)).auto_grid_columns(theme_spacing("xl") * 6.0,
                    theme_spacing("xl") * 8.0, px(400))),
        ]).env(#{density: if ctx.get_state("compact") {"compact"} else {"comfortable"}})
    }
"#;

#[gpui::test]
fn token_tracks_follow_the_density_they_resolve_in(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(TOKEN_GRID).mount(cx);
    visual.simulate_resize(size(px(800.0), px(600.0)));
    settle(&mut visual, Step::Frame, 5);
    let tile = bounds(&mut visual, &view, "group", "Tile 0");
    assert!((tile.width - 150.0).abs() < 1.0, "comfortable: {tile:?}");
    let toggle = bounds(&mut visual, &view, "button", "Toggle");
    crate::support::click_center(&mut visual, toggle);
    settle(&mut visual, Step::Frame, 5);
    let tile = bounds(&mut visual, &view, "group", "Tile 0");
    assert!((tile.width - 100.0).abs() < 1.0, "compact: {tile:?}");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn token_tracks_out_of_order_lay_out_adjusted_and_report_it(cx: &mut TestAppContext) {
    use std::cell::RefCell;
    use std::rc::Rc;
    let reports = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&reports);
    // 24px times twenty is a 480px minimum above the 400px maximum: the
    // minimum holds, so a card never gets less than its minimum width.
    let source = r#"
        fn view(ctx){let tiles=[];
            for index in 0..3 {
                let label = `Tile ${index}`;
                tiles.push(box([text(label)]).with_key(label)
                    .accessibility_role("group").accessibility_label(label)
                    .with_style(style().width(relative(1.0)).height(px(40))));
            }
            box(tiles).accessibility_role("group").accessibility_label("Grid")
                .with_style(style().width(px(900)).auto_grid_columns(theme_spacing("xl") * 20.0,
                    px(420), px(400)))
        }
    "#;
    let (mut visual, view) = Mount::components(source)
        .config(move |config| {
            config.on_error(move |report, _| sink.borrow_mut().push(report.clone()))
        })
        .mount(cx);
    visual.simulate_resize(size(px(1000.0), px(600.0)));
    settle(&mut visual, Step::Frame, 5);
    let tile = bounds(&mut visual, &view, "group", "Tile 0");
    assert!((tile.width - 480.0).abs() < 1.0, "the maximum rises to the minimum: {tile:?}");
    let reports = reports.borrow();
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert_eq!(reports[0].domain, gpui_rhai::ScriptErrorDomain::Presentation);
    assert!(
        reports[0].message.contains("auto_grid_columns resolved to minimum 480"),
        "{}",
        reports[0].message
    );
    assert_eq!(last_error(&mut visual, &view), None);
}
