//! Horizontal and vertical spacing is one shared layout contract, not a pager painter.

use crate::support::{Mount, Step, bounds, last_error, settle};
use gpui::{TestAppContext, px, size};
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE};

#[gpui::test]
fn weighted_flex_shrink_distributes_deficit_but_does_not_grow_on_resize(cx: &mut TestAppContext) {
    let source = r#"
        fn view(ctx) {
            let basis=style().min_width(px(0)).width(px(100)).height(px(24));
            row([
                box([]).accessibility_role("group").accessibility_label("Directory")
                    .with_style(basis.flex_shrink_weight(3)),
                box([]).accessibility_role("group").accessibility_label("Filename")
                    .with_style(basis.flex_shrink(true))
            ]).with_style(style().width(relative(1.0)))
        }
    "#;
    let (mut visual, view) = Mount::new(source).mount(cx);
    for (width, expected) in [
        (240., [100., 100.]),
        (140., [55., 85.]),
        (240., [100., 100.]),
    ] {
        visual.simulate_resize(size(px(width), px(200.)));
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(last_error(&mut visual, &view), None);
        let measured =
            ["Directory", "Filename"].map(|name| bounds(&mut visual, &view, "group", name).width);
        assert!(
            measured
                .into_iter()
                .zip(expected)
                .all(|(real, expect)| (real - expect).abs() < 0.5),
            "{width}: {measured:?}"
        );
    }
}

#[gpui::test]
fn axis_lengths_resolve_in_their_native_dimension_and_shorthand_can_clear_them(
    cx: &mut TestAppContext,
) {
    for scale in [100, 200] {
        for (declaration, expected) in [
            (".gap_x(rem(0.5)).gap_y(rem(0.25))", None),
            (
                ".gap_x(relative(0.1)).gap_y(relative(0.2))",
                Some((20.0, 20.0)),
            ),
            (
                ".gap_x(theme_length(\"space.unit\").times(3)).gap_y(theme_length(\"space.unit\").times(2))",
                Some((
                    12.0 * f64::from(scale) / 100.0,
                    8.0 * f64::from(scale) / 100.0,
                )),
            ),
            (".gap_x(px(99)).gap_y(px(99)).gap(px(0))", Some((0.0, 0.0))),
        ] {
            let source = r#"
                fn view(ctx){let children=[];
                    for name in ["A","B","C","D"] {
                        children.push(box([]).accessibility_role("group").accessibility_label(name)
                            .with_style(style().width(relative(1.0)).height(relative(1.0))));
                    }
                    box(children).with_style(style().grid_cols(2).grid_rows(2).width(px(200)).height(px(100))__GAPS__)
                        .env(#{scale:"__SCALE__"})
                }
            "#.replace("__GAPS__", declaration).replace("__SCALE__", &scale.to_string());
            let (mut visual, view) = Mount::components(source).mount(cx);
            settle(&mut visual, Step::Frame, 4);
            let expected = expected.unwrap_or_else(|| {
                let rem = visual.update(|window, _| f64::from(window.rem_size()));
                (rem * 0.5, rem * 0.25)
            });
            let a = bounds(&mut visual, &view, "group", "A");
            let b = bounds(&mut visual, &view, "group", "B");
            let c = bounds(&mut visual, &view, "group", "C");
            assert!(
                (b.x - a.x - a.width - expected.0).abs() < 0.01,
                "{declaration}/{scale}: {a:?} {b:?}"
            );
            assert!(
                (c.y - a.y - a.height - expected.1).abs() < 0.01,
                "{declaration}/{scale}: {a:?} {c:?}"
            );
            assert_eq!(last_error(&mut visual, &view), None);
        }
    }
}

#[gpui::test]
fn wrapped_flex_and_explicit_grid_keep_independent_gaps_in_rtl(cx: &mut TestAppContext) {
    for rtl in [false, true] {
        for layout in [".flex_row().flex_wrap()", ".grid_cols(2)"] {
            let source = r#"
                fn init(ctx){ctx.set_locale("__LOCALE__");}
                fn view(ctx){let children=[];
                    for name in ["A","B","C","D"] {
                        children.push(box([]).accessibility_role("group").accessibility_label(name)
                            .with_style(style().width(px(20)).height(px(20)).flex_shrink(false)));
                    }
                    box(children).with_style(style()__LAYOUT__.width(px(53))
                        .gap(px(99)).gap_x(px(13)).gap_y(px(7)))
                }
            "#
            .replace("__LOCALE__", if rtl { "ar" } else { "en" })
            .replace("__LAYOUT__", layout);
            let (mut visual, view) = Mount::components(source)
                .locale("en.rhai", EN_LOCALE)
                .locale("ar.rhai", AR_LOCALE)
                .mount(cx);
            settle(&mut visual, Step::Frame, 4);
            let a = bounds(&mut visual, &view, "group", "A");
            let b = bounds(&mut visual, &view, "group", "B");
            let c = bounds(&mut visual, &view, "group", "C");
            assert!(
                ((a.x - b.x).abs() - 33.0).abs() < 0.01,
                "{rtl}/{layout}: {a:?} {b:?}"
            );
            assert!((a.y - b.y).abs() < 0.01);
            assert!((c.y - a.y - 27.0).abs() < 0.01, "{a:?} {c:?}");
            assert_eq!(last_error(&mut visual, &view), None);
        }
    }
}

#[gpui::test]
fn automatic_grid_uses_horizontal_gap_for_breakpoints_and_vertical_gap_for_rows(
    cx: &mut TestAppContext,
) {
    let source = r#"
        fn view(ctx){let children=[];
            for name in ["A","B","C","D"] {
                children.push(box([]).accessibility_role("group").accessibility_label(name)
                    .with_style(style().width(relative(1.0)).height(px(20))));
            }
            box(children).with_style(style().width(relative(1.0)).auto_grid_columns(40,40,40)
                .gap(px(99)).gap_x(px(16)).gap_y(px(7)))
        }
    "#;
    let (mut visual, view) = Mount::components(source).mount(cx);
    settle(&mut visual, Step::Frame, 4);
    visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
    for width in [96.0, 95.0, 96.0] {
        visual.simulate_resize(size(px(width), px(300.0)));
        settle(&mut visual, Step::Frame, 5);
        let a = bounds(&mut visual, &view, "group", "A");
        let b = bounds(&mut visual, &view, "group", "B");
        let c = bounds(&mut visual, &view, "group", "C");
        if width > 95.5 {
            assert!((b.x - a.x - 56.0).abs() < 0.01, "{a:?} {b:?}");
            assert!((b.y - a.y).abs() < 0.01);
            assert!((c.y - a.y - 27.0).abs() < 0.01);
        } else {
            assert!((b.x - a.x).abs() < 0.01);
            assert!((b.y - a.y - 27.0).abs() < 0.01);
        }
        assert_eq!(last_error(&mut visual, &view), None);
    }
    assert!(
        visual
            .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
            .timings
            .is_empty()
    );
}
