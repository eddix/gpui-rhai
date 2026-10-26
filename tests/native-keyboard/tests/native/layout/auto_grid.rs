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
