use std::time::{Duration, Instant};

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, center, snapshot};

#[gpui::test]
fn rate_limit_planner_hover_reads_exact_minutes_and_insets_at_the_trailing_edge(
    cx: &mut TestAppContext,
) {
    use crate::support::live_fills;
    let (mut v, view) = gallery(cx, "rate_limit_planner", "modern-light");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let plot = bounds(&mut v, &view, "figure", "Planned load");
    let move_to = |v: &mut VisualTestContext, minute: f64| {
        v.simulate_mouse_move(
            point(
                px((plot.x + plot.width * (minute + 0.25) / 1440.) as f32),
                px((plot.y + plot.height / 2.) as f32),
            ),
            None,
            Modifiers::none(),
        );
        ready(v, &view);
    };
    let tip = |v: &mut VisualTestContext| {
        live_fills(v, 0x171717ff).into_iter().find(|r| {
            f64::from(r.x) >= plot.x
                && f64::from(r.y) >= plot.y - 31.
                && f64::from(r.bottom()) <= plot.y + plot.height
                && r.height > 10.
                && r.width > 20.
        })
    };

    // The measured sample for minute 71 is 16,000, unlike the preceding
    // drawn vertex at minute 66. The tooltip reads the original minute.
    move_to(&mut v, 71.);
    let measured = tip(&mut v).expect("measured tooltip");
    let x = plot.x + plot.width * 71. / 1440.;
    let y = plot.y + plot.height * (1. - 16_000. / 52_000.) - 30.;
    assert!(
        (f64::from(measured.x) - (x + 8.)).abs() < 1.5 && (f64::from(measured.y) - y).abs() < 1.5,
        "tip {measured:?} must anchor to minute 71 at ({x}, {y})"
    );
    assert!((measured.height - 20.).abs() < 1.5, "{measured:?}");

    move_to(&mut v, 1189.);
    let forecast = tip(&mut v).expect("forecast tooltip");
    let x = plot.x + plot.width * 1189. / 1440.;
    assert!(
        (f64::from(forecast.right()) - (x - 8.)).abs() < 1.5,
        "trailing tooltip {forecast:?} must end before the guide at {x}"
    );
    assert!(
        measured.width > forecast.width + 28.,
        "the real tooltip labels history as measured"
    );

    v.simulate_mouse_move(point(px(6.), px(6.)), None, Modifiers::none());
    ready(&mut v, &view);
    assert!(tip(&mut v).is_none(), "leaving the plot clears the readout");
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn rate_limit_planner_distinguishes_history_forecast_and_over_limit_stroke(
    cx: &mut TestAppContext,
) {
    use crate::support::{Rect, logical_paint_rect, walk_live_paint};

    let (mut v, view) = gallery(cx, "rate_limit_planner", "modern-light");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let figure = node(&mut v, &view, "figure", "Planned load");
    let plot = figure.geometry.unwrap().visual;
    let seam = plot.x + plot.width * 75. / 1440.;
    let limit = plot.y + plot.height * (1. - 1. / 1.3);
    let paths = |v: &mut VisualTestContext, ink: u32| {
        let target = gpui::Background::from(gpui::rgba(ink));
        v.update(|window, _| {
            let scale = window.scale_factor();
            window.visual_probe_with_painted_scene(|scene| {
                let mut shapes = Vec::<(Rect, Rect, Vec<(f32, f32)>)>::new();
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::unit(),
                    &mut |scene, pose| {
                        for path in scene.paths.iter().filter(|path| path.color == target) {
                            let rect = logical_paint_rect(path.bounds, pose, scale);
                            if f64::from(rect.y) < plot.y - 2.
                                || f64::from(rect.bottom()) > plot.y + plot.height + 2.
                            {
                                continue;
                            }
                            let mask = logical_paint_rect(path.content_mask.bounds, pose, scale);
                            let x = |p: gpui::Point<gpui::ScaledPixels>| {
                                let [[a, b], _] = pose.rotation_scale;
                                (a * p.x.0 + b * p.y.0 + pose.translation[0]) / scale
                            };
                            let intervals = path
                                .vertices
                                .as_chunks::<3>()
                                .0
                                .iter()
                                .map(|vs| {
                                    let [a, b, c] = vs.each_ref().map(|v| x(v.xy_position));
                                    (a.min(b).min(c), a.max(b).max(c))
                                })
                                .collect();
                            shapes.push((rect, mask, intervals));
                        }
                    },
                );
                shapes
            })
        })
    };
    let right_coverage = |spans: &[(f32, f32)]| {
        (0..100)
            .filter(|&n| {
                let at = plot.x + plot.width - 110. + f64::from(n);
                spans
                    .iter()
                    .any(|&(left, right)| f64::from(left) <= at && at <= f64::from(right))
            })
            .count()
    };
    let gray = paths(&mut v, 0xd4d4d8ff);
    let dashed = gray
        .iter()
        .find(|(r, _, _)| f64::from(r.width) > plot.width * 0.7 && r.height > 5.)
        .expect("future forecast must be painted");
    assert!((f64::from(dashed.1.x) - seam).abs() < 1.);
    assert!(
        (20..=80).contains(&right_coverage(&dashed.2)),
        "the forecast must have real stroke gaps"
    );
    assert!(
        paths(&mut v, 0x8f8f8fff)
            .iter()
            .any(|(r, _, _)| { r.width > 8. && (f64::from(r.right()) - seam).abs() < 2. }),
        "the solid measured line joins the forecast at the sample cutoff"
    );
    let warnings = paths(&mut v, 0xf04438ff);
    assert!(
        warnings.iter().any(|(r, mask, _)| {
            f64::from(r.width) > plot.width * 0.7
                && (f64::from(mask.bottom()) - limit).abs() < 1.
                && (f64::from(mask.x) - seam).abs() < 1.
        }),
        "the warning stroke must be clipped above the limit and after the cutoff"
    );

    click(&mut v, &view, "button", "Hide measured load");
    assert_eq!(node(&mut v, &view, "figure", "Planned load").id, figure.id);
    let gray = paths(&mut v, 0xd4d4d8ff);
    let solid = gray
        .iter()
        .find(|(r, _, _)| f64::from(r.width) > plot.width * 0.9 && r.height > 5.)
        .expect("full forecast must remain visible with no measured history");
    assert!(right_coverage(&solid.2) > 95);
    assert!(
        !paths(&mut v, 0x8f8f8fff)
            .iter()
            .any(|(r, _, _)| r.width > 8.)
    );
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn rate_limit_planner_warning_enters_with_the_forecast_not_at_its_final_value(
    cx: &mut TestAppContext,
) {
    use crate::support::{Host, logical_paint_rect, walk_live_paint};
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "rate_limit_planner".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::Normal)
    .runtime_clock(clock.clock())
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("planner-warning-motion", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*window, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1380.), px(1040.)));
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    let bands = |v: &mut VisualTestContext| {
        let plot = bounds(v, &view, "figure", "Planned load");
        v.update(|window, _| {
            let scale = window.scale_factor();
            window.visual_probe_with_painted_scene(|scene| {
                let mut spans = Vec::new();
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::unit(),
                    &mut |scene, pose| {
                        for fill in &scene.layered_fills {
                            let area = logical_paint_rect(fill.bounds, pose, scale);
                            let clip = logical_paint_rect(fill.content_mask.bounds, pose, scale);
                            if (f64::from(area.x) - plot.x).abs() < 1.
                                && (f64::from(area.y) - plot.y).abs() < 1.
                                && (f64::from(area.width) - plot.width).abs() < 1.
                                && fill
                                    .layers
                                    .iter()
                                    .any(|layer| layer.repeat_period.is_some())
                            {
                                spans.push((clip.x, clip.right()));
                            }
                        }
                    },
                );
                spans
            })
        })
    };
    let paint = |v: &mut VisualTestContext| {
        let frame = bounds(v, &view, "figure", "Planned load");
        v.update(|window, _| {
            let scale = window.scale_factor();
            window.visual_probe_with_painted_scene(|scene| {
                let red = gpui::Background::from(gpui::rgba(0xf04438ff));
                let gray = gpui::Background::from(gpui::rgba(0xd4d4d8ff));
                let danger = gpui::Background::from(gpui::rgba(0xc4320aff));
                let tag = gpui::Background::from(gpui::rgba(0xf0f0f0ff));
                let alert_tag = gpui::Background::from(gpui::rgba(0xfdeae9ff));
                let mut visible = false;
                let mut area_y = f32::INFINITY;
                let mut line_y = f32::INFINITY;
                let mut marker_y = f32::INFINITY;
                let mut tag_y = f32::INFINITY;
                let mut marker_warned = false;
                let mut tag_warned = false;
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::unit(),
                    &mut |scene, pose| {
                        for quad in scene
                            .quads
                            .iter()
                            .filter(|q| q.background == tag || q.background == alert_tag)
                        {
                            let r = logical_paint_rect(quad.bounds, pose, scale);
                            if r.width > 45.
                                && r.width < 165.
                                && (17.0..=19.0).contains(&r.height)
                                && f64::from(r.x) >= frame.x - 1.
                                && f64::from(r.y) >= frame.y - 1.
                                && f64::from(r.bottom()) <= frame.y + frame.height + 1.
                            {
                                tag_y = r.y;
                                tag_warned = quad.background == alert_tag;
                            }
                        }
                        for path in &scene.paths {
                            let r = logical_paint_rect(path.bounds, pose, scale);
                            if (6.5..=7.5).contains(&r.width)
                                && (6.5..=7.5).contains(&r.height)
                                && (path.color == gray || path.color == danger || path.color == red)
                                && f64::from(r.x) >= frame.x - 4.
                                && f64::from(r.bottom()) <= frame.y + frame.height + 4.
                            {
                                marker_y = r.y + r.height / 2.;
                                marker_warned = path.color != gray;
                            }
                            let mask = logical_paint_rect(path.content_mask.bounds, pose, scale);
                            // Spline control points may extend beyond a flat
                            // baseline. Compare what the plot actually clips.
                            let top = r.y.max(mask.y);
                            let bottom = r.bottom().min(mask.bottom());
                            if f64::from(r.width) <= frame.width * 0.7
                                || top >= bottom
                                || f64::from(top) < frame.y - 2.
                                || f64::from(bottom) > frame.y + frame.height + 2.
                            {
                                continue;
                            }
                            if path.color.as_solid().is_none()
                                && (f64::from(bottom) - frame.y - frame.height).abs() < 1.
                            {
                                area_y = area_y.min(top);
                            }
                            if path.color == gray && bottom - top > 5. {
                                line_y = line_y.min(top);
                            }
                            if path.color != red {
                                continue;
                            }
                            visible |= f64::from(mask.x) >= frame.x - 1.
                                && (f64::from(mask.y) - frame.y).abs() < 1.
                                && r.y < mask.bottom() - 1.;
                        }
                    },
                );
                (
                    visible,
                    area_y,
                    line_y,
                    marker_y,
                    tag_y,
                    frame.y as f32,
                    marker_warned,
                    tag_warned,
                    (frame.y + frame.height * (1. - 1. / 1.3)) as f32,
                )
            })
        })
    };
    assert!(paint(&mut v).0);
    click(&mut v, &view, "button", "Show weekend");
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    // Clear any previous high forecast before the under-to-over update.
    click(&mut v, &view, "button", "Hide measured load");
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    let (red, before, _, _, _, _, _, _, _) = paint(&mut v);
    assert!(!red && before.is_finite());
    assert!(bands(&mut v).is_empty());
    click(&mut v, &view, "button", "Show weekdays");
    let (red, first, line, marker, tag, top, warned, tag_warned, _) = paint(&mut v);
    assert!(
        bands(&mut v).is_empty(),
        "hatches must not jump to the final intervals"
    );
    assert!(
        !red,
        "the warning may not jump ahead of the interpolated forecast"
    );
    assert!(
        (before - first).abs() < 1. && (first - line).abs() < 1.5,
        "area may not jump to the accepted final shape: before {before}, area {first}, line {line}"
    );
    assert!(
        (first - marker).abs() < 1.5,
        "peak marker must be on the old curve at onset, curve {first}, marker {marker}"
    );
    assert!(
        !warned && !tag_warned,
        "marker and tag must stay neutral while the visible curve is below the limit"
    );
    assert!(
        (tag - (marker - 21.).max(top)).abs() < 1.5,
        "tag {tag} follows marker {marker}"
    );
    clock.advance(Duration::from_millis(120));
    ready(&mut v, &view);
    let (_, middle, line, marker, tag, top, warned, tag_warned, limit) = paint(&mut v);
    let intermediate_bands = bands(&mut v);
    assert_eq!(intermediate_bands.is_empty(), marker >= limit);
    assert!(
        first - middle > 4. && (middle - line).abs() < 1.5,
        "area and line must move together on an intermediate frame: {first}, {middle}, {line}"
    );
    assert!(
        (middle - marker).abs() < 1.5,
        "peak marker must follow the sampled curve at mid-frame"
    );
    assert!(
        (tag - (marker - 21.).max(top)).abs() < 1.5,
        "mid-frame tag {tag} follows marker {marker}"
    );
    assert_eq!(
        (warned, tag_warned),
        (marker < limit, marker < limit),
        "paint uses the same sampled marker as placement"
    );
    clock.advance(Duration::from_millis(100));
    ready(&mut v, &view);
    let growing_bands = bands(&mut v);
    assert!(
        !growing_bands.is_empty(),
        "hatches appear on an in-flight crossing"
    );
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    let (red, last, line, marker, tag, top, warned, tag_warned, _) = paint(&mut v);
    let settled_bands = bands(&mut v);
    assert!(!settled_bands.is_empty() && settled_bands != intermediate_bands);
    let width = |spans: &[(f32, f32)]| spans.iter().map(|(left, right)| right - left).sum::<f32>();
    assert!(
        width(&growing_bands) < width(&settled_bands),
        "an in-flight crossing should expose less hatch than the terminal plan: {growing_bands:?}, {settled_bands:?}"
    );
    assert!(
        red && middle - last > 1. && (last - line).abs() < 1.5,
        "finished area and warning should match the accepted forecast: {middle}, {last}, {line}"
    );
    assert!(
        (last - marker).abs() < 1.5,
        "peak marker must settle on the accepted curve"
    );
    assert!(
        warned && tag_warned,
        "both peak paints must warn when the visible curve is above the limit"
    );
    assert!(
        (tag - (marker - 21.).max(top)).abs() < 1.5,
        "final tag {tag} follows marker {marker}"
    );
    click(&mut v, &view, "button", "Show weekend");
    let (_, _, _, _, _, _, warned, tag_warned, _) = paint(&mut v);
    assert_eq!(bands(&mut v), settled_bands);
    assert!(
        warned && tag_warned,
        "descending curve starts with the old warning paints"
    );
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    let (_, _, _, _, tag, _, warned, tag_warned, _) = paint(&mut v);
    assert!(bands(&mut v).is_empty());
    assert!(
        tag.is_finite() && !warned && !tag_warned,
        "both paints must recover below the limit"
    );

    let wide = bounds(&mut v, &view, "figure", "Planned load");
    let figure = node(&mut v, &view, "figure", "Planned load").id;
    click(&mut v, &view, "button", "Show weekdays");
    assert!(!paint(&mut v).0);
    clock.advance(Duration::from_millis(120));
    ready(&mut v, &view);
    v.simulate_resize(size(px(840.), px(1040.)));
    ready(&mut v, &view);
    let narrow = bounds(&mut v, &view, "figure", "Planned load");
    assert!(
        wide.width - narrow.width > 200.,
        "the sampling width must change"
    );
    assert_eq!(node(&mut v, &view, "figure", "Planned load").id, figure);
    let (over, area, line, _, _, _, _, _, _) = paint(&mut v);
    assert!(
        over && (area - line).abs() < 1.5,
        "new grid must present a complete matching fill and stroke: {area}, {line}"
    );
    assert!(!bands(&mut v).is_empty());
    click(&mut v, &view, "button", "Show weekend");
    assert!(
        paint(&mut v).0,
        "the next change should animate on the new grid"
    );
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    assert!(!paint(&mut v).0);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn rate_limit_planner_running_sheen_tracks_actual_bar_width_without_script_frames(
    cx: &mut TestAppContext,
) {
    use crate::support::{Host, logical_paint_rect, walk_live_paint};

    for preference in [
        MotionPreference::Normal,
        MotionPreference::Reduced,
        MotionPreference::None,
    ] {
        let clock = ManualRuntimeClock::new(Instant::now());
        let prepared = acceptance::view(&AcceptanceLaunch {
            page: "rate_limit_planner".into(),
            theme: "modern-light".into(),
            ..Default::default()
        })
        .unwrap()
        .motion_preference(preference)
        .runtime_clock(clock.clock())
        .prepare()
        .unwrap();
        let (window, view) = super::mount_prepared(
            cx,
            prepared,
            ("planner-sheen", "gallery"),
            false,
            |host, view| Host { host, view },
        );
        let mut v = VisualTestContext::from_window(*window, cx);
        v.update(|window, _| window.activate_window());
        v.simulate_resize(size(px(1380.), px(1040.)));
        ready(&mut v, &view);
        let sheen = |v: &mut VisualTestContext| {
            v.update(|window, _| {
                let scale = window.scale_factor();
                window.visual_probe_with_painted_scene(|scene| {
                    let mut patches = Vec::new();
                    walk_live_paint(
                        scene,
                        gpui::TransformationMatrix::unit(),
                        &mut |scene, pose| {
                            for fill in &scene.layered_fills {
                                let rect = logical_paint_rect(fill.bounds, pose, scale);
                                if (rect.width - 60.).abs() < 0.6
                                    && fill.layers.iter().any(|layer| {
                                        layer.stops.len() == 3
                                            && layer.repeat_period.is_none()
                                            && layer.direction.x == 1.
                                            && layer.direction.y == 0.
                                    })
                                {
                                    let clip =
                                        logical_paint_rect(fill.content_mask.bounds, pose, scale);
                                    patches.push((rect, clip));
                                }
                            }
                        },
                    );
                    patches
                })
            })
        };
        let bar = node(&mut v, &view, "slider", "Index notes");
        assert!(bar.disabled);
        assert!(!node(&mut v, &view, "slider", "Review guides").disabled);
        let area = bar.geometry.unwrap().visual;
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap();
        clock.advance(Duration::from_millis(750));
        ready(&mut v, &view);
        let patches = sheen(&mut v);
        assert_eq!(
            patches.len(),
            1,
            "{preference:?}: only the running task sweeps"
        );
        let (first, clip) = patches[0];
        let rest = preference != MotionPreference::Normal;
        let expected = if rest {
            area.x
        } else {
            area.x + area.width / 2. - 30.
        };
        assert!(
            (f64::from(first.x) - expected).abs() < 1.5,
            "{preference:?}: {first:?}, {area:?}"
        );
        assert!(
            f64::from(clip.x) >= area.x - 1. && f64::from(clip.right()) <= area.x + area.width + 1.
        );
        assert!(
            v.update(|_, cx| view.take_performance_snapshot(cx))
                .unwrap()
                .timings
                .is_empty()
        );
        click(&mut v, &view, "button", "Narrow planner");
        let narrow = bounds(&mut v, &view, "slider", "Index notes");
        assert!(narrow.width < area.width - 15.);
        let expected_narrow = if rest {
            narrow.x
        } else {
            narrow.x + narrow.width / 2. - 30.
        };
        let resized_sheen = sheen(&mut v);
        assert_eq!(resized_sheen.len(), 1);
        assert!(
            (f64::from(resized_sheen[0].0.x) - expected_narrow).abs() < 1.5,
            "{preference:?}: phase must use the resized bar, not restart: {resized_sheen:?}, {narrow:?}, wanted {expected_narrow}"
        );
        clock.advance(Duration::from_millis(120));
        ready(&mut v, &view);
        let moved = f64::from(sheen(&mut v)[0].0.x);
        assert_eq!(moved > expected_narrow + 2.0, !rest, "{preference:?}");

        super::acceptance::action(
            &mut v,
            &view,
            "gallery.set_scale",
            gpui_rhai::UiValue::String("150".into()),
        );
        ready(&mut v, &view);
        assert_eq!(
            sheen(&mut v).len(),
            1,
            "reading scale must retain the 60px measured sweep image"
        );

        click(&mut v, &view, "button", "Show completed runs");
        node(&mut v, &view, "status", "Planner clock advanced");
        let completed = node(&mut v, &view, "slider", "Index notes");
        assert_eq!(
            completed.id, bar.id,
            "the running bar is not replaced by a new control"
        );
        assert!(completed.disabled && sheen(&mut v).is_empty());
    }
}

#[gpui::test]
fn rate_limit_planner_keeps_the_focused_slider_through_a_responsive_resize(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "rate_limit_planner", "clean-dark");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let wide = bounds(&mut v, &view, "figure", "Planned load").width;
    click(&mut v, &view, "slider", "Review guides");
    let id = node(&mut v, &view, "slider", "Review guides").id;
    let focused = v.update(|window, cx| window.focused(cx)).unwrap();

    v.simulate_resize(size(px(760.), px(1040.)));
    ready(&mut v, &view);
    let narrow = bounds(&mut v, &view, "figure", "Planned load").width;
    assert!(wide - narrow > 150., "{wide} -> {narrow}");
    assert_eq!(node(&mut v, &view, "slider", "Review guides").id, id);
    assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focused));

    v.simulate_keystrokes("home");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Schedule accepted: move");
    let moved = node(&mut v, &view, "slider", "Review guides");
    assert_eq!(moved.id, id);
    assert!(moved.description.contains("07:15"));
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn rate_limit_planner_places_time_flags_and_quiets_only_the_label(cx: &mut TestAppContext) {
    use crate::support::{Rect, glyph_rects_with_ink, logical_paint_rect, walk_live_paint};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui_rhai::GeometryBounds;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("planner_time_flags"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "rate_limit_planner", "modern-light");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let plot = |v: &mut VisualTestContext| bounds(v, &view, "figure", "Planned load");
    let x_at = |plot: GeometryBounds, minute: f64| plot.x + plot.width * minute / 1440.;
    let ink = |v: &mut VisualTestContext, plot: GeometryBounds, x: f64, half: f64| {
        glyph_rects_with_ink(
            v,
            GeometryBounds::new(x - half, plot.y - 20., half * 2., 18.).unwrap(),
            0x8f8f8fff,
        )
    };
    let edges = |glyphs: &[Rect]| {
        (
            glyphs.iter().map(|r| r.x).fold(f32::INFINITY, f32::min),
            glyphs
                .iter()
                .map(Rect::right)
                .fold(f32::NEG_INFINITY, f32::max),
        )
    };
    let now_line = |v: &mut VisualTestContext, plot: GeometryBounds, x: f64| {
        v.update(|window, _| {
            let scale = window.scale_factor();
            window.visual_probe_with_painted_scene(|scene| {
                let mut painted = false;
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::unit(),
                    &mut |scene, pose| {
                        for path in &scene.paths {
                            let r = logical_paint_rect(path.bounds, pose, scale);
                            painted |= r.width < 3.
                                && f64::from(r.height) > plot.height - 3.
                                && (f64::from(r.x + r.width / 2.) - x).abs() < 1.;
                        }
                    },
                );
                painted
            })
        })
    };

    let wide = plot(&mut v);
    let check_x = x_at(wide, 1020.);
    let check = ink(&mut v, wide, check_x, 160.);
    assert!(
        check.len() >= 14,
        "wide plot paints the checkpoint's label and time"
    );
    let (left, _) = edges(&check);
    assert!(
        (f64::from(left) - check_x - 6.).abs() < 3.,
        "{left} vs {check_x}"
    );
    let now_x = x_at(wide, 75.);
    assert!(
        ink(&mut v, wide, now_x, 27.).is_empty(),
        "peak suppresses nearby Now text"
    );
    assert!(
        now_line(&mut v, wide, now_x),
        "the Now line remains visible"
    );

    click(&mut v, &view, "button", "Show weekend");
    let wide = plot(&mut v);
    let now_x = x_at(wide, 75.);
    let now = ink(&mut v, wide, now_x, 30.);
    assert!(now.len() >= 7, "Now is painted when the peak moves away");
    let (left, right) = edges(&now);
    assert!(
        (f64::from(left + right) / 2. - now_x).abs() < 2.,
        "Now is centered on its line"
    );

    click(&mut v, &view, "button", "Narrow planner");
    let narrow = plot(&mut v);
    assert!(wide.width - narrow.width > 150.);
    let check_x = x_at(narrow, 1020.);
    let check = ink(&mut v, narrow, check_x, 155.);
    assert!(
        check.len() >= 14,
        "the checkpoint remains readable in the narrow plot"
    );
    let (_, right) = edges(&check);
    assert!(
        (f64::from(right) - check_x + 6.).abs() < 4.,
        "{right} vs {check_x}"
    );
    let clear_of_header = |v: &mut VisualTestContext, glyphs: &[Rect]| {
        let button = bounds(v, &view, "button", "Spread out");
        let top = glyphs.iter().map(|r| r.y).fold(f32::INFINITY, f32::min);
        assert!(
            f64::from(top) > button.y + button.height + 2.,
            "flag must stay below the wrapped header's Spread out button: {top} vs {button:?}"
        );
    };
    clear_of_header(&mut v, &check);

    super::acceptance::action(
        &mut v,
        &view,
        "gallery.set_scale",
        gpui_rhai::UiValue::String("150".into()),
    );
    ready(&mut v, &view);
    let enlarged = plot(&mut v);
    assert!(
        (enlarged.height - 197.).abs() < 0.1,
        "132px at 150%, less the fixed 1px rule: {enlarged:?}"
    );
    let check_x = x_at(enlarged, 1020.);
    let enlarged_ink = glyph_rects_with_ink(
        &mut v,
        GeometryBounds::new(check_x - 180., enlarged.y - 40., 180., 50.).unwrap(),
        0x8f8f8fff,
    );
    assert!(enlarged_ink.len() >= 14);
    let (_, right) = edges(&enlarged_ink);
    assert!((f64::from(right) - check_x + 6.).abs() < 4.);
    let lowest = enlarged_ink
        .iter()
        .map(Rect::bottom)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        f64::from(lowest) < enlarged.y - 1.,
        "enlarged text stays above the plot"
    );
    clear_of_header(&mut v, &enlarged_ink);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn rate_limit_planner_late_clock_keeps_flags_separate_and_marks_the_planned_peak(
    cx: &mut TestAppContext,
) {
    use crate::support::{glyph_rects_with_ink, logical_paint_rect, walk_live_paint};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui_rhai::GeometryBounds;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("planner_late_clock"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "rate_limit_planner", "modern-light");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    // The platform may emit a shaped sprite for whitespace. Use an
    // isolated label with the same time template as a positive control.
    click(&mut v, &view, "button", "Show weekend");
    let early = bounds(&mut v, &view, "figure", "Planned load");
    let early_now = early.x + early.width * 75. / 1440.;
    let isolated_now = glyph_rects_with_ink(
        &mut v,
        GeometryBounds::new(early_now - 30., early.y - 23., 60., 22.).unwrap(),
        0x8f8f8fff,
    )
    .len();
    assert!(isolated_now >= 8);
    click(&mut v, &view, "button", "Show weekdays");
    click(&mut v, &view, "button", "Show completed runs");
    let plot = bounds(&mut v, &view, "figure", "Planned load");
    let x_at = |minute| plot.x + plot.width * minute / 1440.;
    let cp = x_at(1020.);
    let now = x_at(1065.);
    let ink = |v: &mut VisualTestContext, left, width| {
        glyph_rects_with_ink(
            v,
            GeometryBounds::new(left, plot.y - 23., width, 22.).unwrap(),
            0x8f8f8fff,
        )
    };
    let before_checkpoint = ink(&mut v, cp - 165., 165.);
    let around_now = ink(&mut v, now - 30., 60.);
    let markers = v.update(|window, _| {
        let scale = window.scale_factor();
        window.visual_probe_with_painted_scene(|scene| {
            let mut markers = Vec::new();
            walk_live_paint(
                scene,
                gpui::TransformationMatrix::unit(),
                &mut |scene, pose| {
                    let warning = gpui::Background::from(gpui::rgba(0xf04438ff));
                    let neutral = gpui::Background::from(gpui::rgba(0xd4d4d8ff));
                    for path in &scene.paths {
                        let rect = logical_paint_rect(path.bounds, pose, scale);
                        if (6.0..=8.0).contains(&rect.width)
                            && (6.0..=8.0).contains(&rect.height)
                            && (path.color == warning || path.color == neutral)
                            && f64::from(rect.y) >= plot.y - 4.
                            && f64::from(rect.bottom()) <= plot.y + plot.height + 4.
                        {
                            markers
                                .push((f64::from(rect.x + rect.width / 2.), path.color == warning));
                        }
                    }
                },
            );
            markers
        })
    });
    assert!(
        before_checkpoint.len() >= 14
            && around_now.len() == isolated_now
            && markers
                .iter()
                .any(|&(x, warn)| warn && (x - x_at(199.)).abs() <= plot.width / 240. + 1.),
        "late guide glyphs must stay separate and the warning dot must mark the 09:19 plan, not the 23:45 splice: checkpoint={}, Now={}, markers={markers:?}",
        before_checkpoint.len(),
        around_now.len()
    );
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn rate_limit_planner_overflow_uses_native_hatches_with_independent_periods(
    cx: &mut TestAppContext,
) {
    use gpui::VisualTestContext;

    use crate::support::{logical_paint_rect, walk_live_paint};

    let (mut v, view) = gallery(cx, "rate_limit_planner", "modern-light");
    let repeating = |v: &mut VisualTestContext| {
        v.update(|window, _| {
            let scale = window.scale_factor();
            window.visual_probe_with_painted_scene(|scene| {
                let mut paints = Vec::new();
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::unit(),
                    &mut |scene, pose| {
                        for fill in &scene.layered_fills {
                            let area = logical_paint_rect(fill.bounds, pose, scale);
                            let clip = logical_paint_rect(fill.content_mask.bounds, pose, scale);
                            for layer in &*fill.layers {
                                if let Some(period) = layer.repeat_period {
                                    let d = layer.direction;
                                    paints.push((
                                        area,
                                        clip,
                                        period.0 / scale / d.x.hypot(d.y),
                                        (d.x, d.y),
                                    ));
                                }
                            }
                        }
                    },
                );
                paints
            })
        })
    };
    let mut wide_plot = 0.0;
    for (width, family, variant, narrow) in [
        (1320., "Modern", "Light", false),
        (1720., "Clean", "Dark", false),
        (1720., "Clean", "Dark", true),
    ] {
        v.update(|_, cx| view.select_theme(family, variant, cx))
            .unwrap();
        v.simulate_resize(size(px(width), px(1040.)));
        ready(&mut v, &view);
        if narrow {
            click(&mut v, &view, "button", "Narrow planner");
        }
        let plot = bounds(&mut v, &view, "figure", "Planned load");
        let rule = if family == "Clean" { 0.5 } else { 1. };
        assert!(
            (plot.height + rule - 132.).abs() < 0.1,
            "{family} chart box including rule: {plot:?}"
        );
        let has_rule = v.update(|window, _| {
            let scale = window.scale_factor();
            window.visual_probe_with_painted_scene(|scene| {
                let mut found = false;
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::unit(),
                    &mut |scene, pose| {
                        found |= scene.quads.iter().any(|quad| {
                            let r = logical_paint_rect(quad.bounds, pose, scale);
                            (f64::from(quad.border_widths.bottom.0 / scale) - rule).abs() < 0.1
                                && (f64::from(r.width) - plot.width).abs() < 0.75
                                && (f64::from(r.bottom()) - plot.y - 132.).abs() < 0.75
                        });
                    },
                );
                found
            })
        });
        assert!(
            has_rule,
            "{family} must paint a distinct rule outside the drawable"
        );
        if narrow {
            assert!(wide_plot - plot.width > 150., "plot must actually resize");
        } else {
            wide_plot = plot.width;
        }
        let bar = bounds(&mut v, &view, "slider", "Review guides");
        let paints = repeating(&mut v);
        assert!(
            paints.iter().any(|(area, clip, period, direction)| {
                (period - 6.).abs() < 0.01
                    && *direction == (1., 1.)
                    && (f64::from(area.x) - plot.x).abs() < 1.
                    && (f64::from(area.y) - plot.y).abs() < 1.
                    && (f64::from(area.width) - plot.width).abs() < 1.
                    && f64::from(clip.width) < plot.width * 0.9
            }),
            "{family} forecast band must clip a single plot-anchored rhythm: {paints:?}"
        );
        assert!(
            paints.iter().any(|(area, _, period, direction)| {
                (period - 4.5).abs() < 0.01
                    && *direction == (-1., -1.)
                    && f64::from(area.x) >= bar.x - 1.
                    && f64::from(area.right()) <= bar.x + bar.width + 1.
                    && (f64::from(area.y) - bar.y).abs() < 2.
            }),
            "{family} lane hatch belongs to the actual bar interval: {paints:?}"
        );
    }
    click(&mut v, &view, "button", "Show weekend");
    assert!(
        repeating(&mut v)
            .iter()
            .all(|(_, _, p, _)| (*p - 6.).abs() > 0.01 && (*p - 4.5).abs() > 0.01)
    );
}

#[gpui::test]
fn rate_limit_planner_bar_text_uses_the_selected_srgb_mix(cx: &mut TestAppContext) {
    use crate::support::{PresetRisks, live_fills};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("planner_bar_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "rate_limit_planner", "modern-light");
    v.simulate_resize(size(px(1450.), px(1040.)));
    let mut risks = PresetRisks::new();
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    for (family, appearance, selected, regular, lane_wash) in [
        ("Modern", "Light", 0x9148cfff, 0x148477ff, 0xa651f00d),
        ("Modern", "Dark", 0xc199f0ff, 0x7ed0c5ff, 0xb98af00d),
        ("Clean", "Light", 0x9046d0fa, 0x0e8274f6, 0xa651f00d),
        ("Clean", "Dark", 0xc39af2fb, 0x80d4c9f8, 0xb98af00d),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut v, &view);
        let selected_bar = bounds(&mut v, &view, "slider", "Review guides");
        let regular_bar = bounds(&mut v, &view, "slider", "Draft digest");
        assert!(
            crate::support::glyphs_with_ink(&mut v, selected_bar, selected) >= 2,
            "{family}/{appearance}: selected amount should use 85% lane ink"
        );
        assert!(
            crate::support::glyphs_with_ink(&mut v, regular_bar, regular) >= 2,
            "{family}/{appearance}: ordinary amount should use 70% lane ink"
        );
        let y = selected_bar.y + selected_bar.height / 2.;
        assert!(
            live_fills(&mut v, lane_wash).iter().any(|r| {
                f64::from(r.y) <= y
                    && y < f64::from(r.bottom())
                    && f64::from(r.width) > selected_bar.width * 2.
            }),
            "{family}/{appearance}: own selected lane must paint the author's 5% purple tint"
        );
        if appearance == "Light" {
            let (stock, findings) = v.update(|_, cx| {
                (
                    view.theme_snapshot(cx).unwrap().variant,
                    view.composition_audit_with(&rules, cx).unwrap(),
                )
            });
            for path in [
                "/run:review/who/start",
                "/run:review/lane-track/track/drag-window/drag-item/bar-display/amount",
                "/run:digest/lane-track/track/drag-window/drag-item/bar-display/amount",
                "/plot-row/plot-frame/load-plot/checkpoint-flag",
            ] {
                let finding = findings
                    .iter()
                    .find(|f| f.path.ends_with(path))
                    .unwrap_or_else(|| panic!("{family}: raw audit missing {path}"));
                assert!(risks.record(&stock, finding), "{family}: {finding:?}");
                let mut wrong = finding.clone();
                wrong.contrast.as_mut().unwrap().background_srgb = Some([0.9; 3]);
                assert!(!risks.record(&stock, &wrong));
            }
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn rate_limit_planner_marks_keep_a_three_by_three_grid_at_reading_scale(cx: &mut TestAppContext) {
    use super::acceptance::action;
    use crate::support::filled_including_rounded_clips;
    use gpui_rhai::{AuditRules, UiValue};

    let (mut v, view) = gallery(cx, "rate_limit_planner", "modern-dark");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let rules = AuditRules::from_ids(["text-edge-misaligned", "mixed-type-in-row"]).unwrap();
    for (scale, factor) in [("100", 1.), ("150", 1.5)] {
        action(
            &mut v,
            &view,
            "gallery.set_scale",
            UiValue::String(scale.into()),
        );
        ready(&mut v, &view);
        let plot = bounds(&mut v, &view, "figure", "Planned load");
        for label in ["Update library", "14:30"] {
            let text = bounds(&mut v, &view, "text", label);
            assert!(
                (text.height - f64::from(20. * factor)).abs() <= 1.,
                "{scale}%: {label} does not inherit a 20-unit line: {text:?}"
            );
        }
        let selected = bounds(&mut v, &view, "slider", "Review guides");
        let quads = filled_including_rounded_clips(&mut v, 0xb98af0ff);
        let center_y = selected.y + selected.height / 2.;
        let dots: Vec<_> = quads
            .iter()
            .map(|q| q.visible())
            .filter(|q| {
                f64::from(q.right()) < plot.x
                    && (f64::from(q.y + q.height / 2.) - center_y).abs() <= f64::from(8. * factor)
                    && (q.width - 3. * factor).abs() <= 0.6
                    && (q.height - 3. * factor).abs() <= 0.6
            })
            .collect();
        assert_eq!(dots.len(), 6, "{scale}: {selected:?} {dots:#?}");
        for sites in [
            dots.iter().map(|q| q.x).collect::<Vec<_>>(),
            dots.iter().map(|q| q.y).collect::<Vec<_>>(),
        ] {
            let mut sites = sites;
            sites.sort_by(f32::total_cmp);
            sites.dedup_by(|a, b| (*a - *b).abs() < 0.5);
            assert_eq!(sites.len(), 3, "{scale}: {sites:?}");
            assert!(
                sites
                    .windows(2)
                    .all(|pair| (pair[1] - pair[0] - 4.5 * factor).abs() <= 0.7),
                "{scale}: {sites:?}"
            );
        }
        let findings = v.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
        assert!(
            !findings
                .iter()
                .any(|f| f.path.ends_with("/mark") || f.path.ends_with("/who")),
            "{findings:#?}"
        );
        let heading = bounds(&mut v, &view, "heading", "Rate limit");
        let semantics = snapshot(&mut v, &view);
        let verdict = semantics
            .nodes()
            .find(|n| {
                n.role == "status"
                    && [
                        "Stays under",
                        "The plan exceeds",
                        "Over the limit even",
                        "Measured pace",
                    ]
                    .iter()
                    .any(|prefix| n.name.starts_with(prefix))
            })
            .expect("planner verdict is presented")
            .geometry
            .unwrap()
            .visual;
        let add = bounds(&mut v, &view, "button", "Add run");
        assert!(
            (heading.x - verdict.x).abs() <= 0.5,
            "{scale}: {heading:?}, {verdict:?}"
        );
        assert!(
            (heading.x - add.x - f64::from(6. * factor)).abs() <= 0.5,
            "{scale}: {heading:?}, {add:?}"
        );
    }
}

#[gpui::test]
fn rate_limit_planner_spread_never_moves_started_or_filtered_rows(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "rate_limit_planner", "modern-light");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let initial = node(&mut v, &view, "figure", "Planned load").description;
    let locked = node(&mut v, &view, "slider", "Index notes");
    assert!(locked.disabled && locked.description.contains("06:45"));
    click(&mut v, &view, "button", "Refuse next schedule proposal");
    click(&mut v, &view, "button", "Spread out");
    node(&mut v, &view, "status", "Schedule proposal refused");
    assert_eq!(
        node(&mut v, &view, "figure", "Planned load").description,
        initial
    );
    click(&mut v, &view, "button", "Spread out");
    node(&mut v, &view, "status", "Schedule accepted: spread");
    assert_ne!(
        node(&mut v, &view, "figure", "Planned load").description,
        initial
    );
    assert_eq!(
        node(&mut v, &view, "slider", "Index notes").description,
        locked.description
    );
    click(&mut v, &view, "button", "Show weekend");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("slider", "Review guides")
            .next()
            .is_none()
    );
    assert!(
        node(&mut v, &view, "slider", "Archive history")
            .description
            .contains("11:10")
    );
}

#[gpui::test]
fn rate_limit_planner_native_drag_previews_then_submits_one_release(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "rate_limit_planner", "clean-dark");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let original = node(&mut v, &view, "slider", "Review guides").description;
    let bar = bounds(&mut v, &view, "slider", "Review guides");
    let plot = bounds(&mut v, &view, "figure", "Planned load");
    let start = center(bar);
    let dx = (plot.width / 24.) as f32;
    let mid = point(start.x + px(dx / 2.), start.y);
    let end = point(start.x + px(dx), start.y);
    v.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    v.simulate_mouse_move(mid, Some(MouseButton::Left), Modifiers::none());
    v.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
    assert_eq!(
        node(&mut v, &view, "slider", "Review guides").description,
        original
    );
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
    v.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Schedule accepted: move");
    assert!(
        node(&mut v, &view, "slider", "Review guides")
            .description
            .contains("09:30")
    );
}

#[gpui::test]
fn rate_limit_planner_narrow_edit_and_add_use_real_controls(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "rate_limit_planner", "modern-dark");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let wide = bounds(&mut v, &view, "figure", "Planned load");
    click(&mut v, &view, "button", "Narrow planner");
    let narrow = bounds(&mut v, &view, "figure", "Planned load");
    assert!(narrow.width + 100. < wide.width && narrow.width > 250.);
    click(&mut v, &view, "button", "Edit Review guides");
    node(&mut v, &view, "dialog", "Edit Review guides");
    assert!(
        !node(&mut v, &view, "button", "Save run").disabled,
        "the initial time is editable in this locale"
    );
    click(&mut v, &view, "text_field", "Name");
    v.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-a"
    } else {
        "ctrl-a"
    });
    v.simulate_input("Review new guides");
    ready(&mut v, &view);
    let peak = bounds(&mut v, &view, "spinbutton", "Peak");
    let climb = bounds(&mut v, &view, "spinbutton", "Climb");
    // A spinbutton's semantic bounds follow the native text content, not
    // the surrounding field (which includes a unit and padding).
    assert!(
        peak.width >= 45. && peak.x > climb.x + climb.width && climb.y > peak.y,
        "{peak:?}, {climb:?}"
    );
    click(&mut v, &view, "spinbutton", "Peak");
    v.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-a"
    } else {
        "ctrl-a"
    });
    v.simulate_input("32000");
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Save run");
    node(&mut v, &view, "status", "Schedule accepted: edit");
    assert!(
        node(&mut v, &view, "slider", "Review new guides")
            .description
            .contains("32k")
    );
    click(&mut v, &view, "button", "Add run");
    node(&mut v, &view, "status", "Schedule accepted: add");
    node(&mut v, &view, "slider", "New run");
    node(&mut v, &view, "dialog", "Edit New run");
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
