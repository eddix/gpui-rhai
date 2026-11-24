use std::time::{Duration, Instant};

use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::{GeometryBounds, ManualRuntimeClock, MotionPreference, ScriptViewHandle};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, bounds, center, filled_including_rounded_clips, node_texts, snapshot};

fn capacity_paint(v: &mut VisualTestContext, area: GeometryBounds) -> (gpui::Background, [f32; 4]) {
    crate::support::painted_quads_including_rounded_clips(v)
        .into_iter()
        .find(|quad| {
            [
                f64::from(quad.bounds.x) - area.x,
                f64::from(quad.bounds.y) - area.y,
                f64::from(quad.bounds.width) - area.width,
                f64::from(quad.bounds.height) - area.height,
            ]
            .iter()
            .all(|distance| distance.abs() <= 1.)
        })
        .map(|quad| (quad.background, quad.corner_radii))
        .unwrap_or_else(|| panic!("missing capacity paint at {area:?}"))
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn context_map_capacity_readout_matches_the_actual_preset_states(cx: &mut TestAppContext) {
    use crate::support::glyphs_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("context_map_capacity_paint"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, panel, states) in [
        (
            "modern-light",
            0xffffffff,
            [
                (0xffffffff, 0x6b6b6bff),
                (0xffecd6ff, 0xb25e00ff),
                (0xfde3dfff, 0xc4320aff),
            ],
        ),
        (
            "modern-dark",
            0x232326ff,
            [
                (0x2a2a2eff, 0xa1a1a1ff),
                (0x3a2712ff, 0xffb35cff),
                (0x3d1e16ff, 0xff8a7aff),
            ],
        ),
        (
            "clean-light",
            0xffffffff,
            [
                (0x0000000d, 0x0000009c),
                (0xff9c0024, 0xb25e00ff),
                (0xf52b0018, 0xd72400da),
            ],
        ),
        (
            "clean-dark",
            0x1c1c1fff,
            [
                (0x0000000d, 0xffffff9e),
                (0xf2a15c2e, 0xffb35cff),
                (0xff6b5f33, 0xff8a7aff),
            ],
        ),
    ] {
        let (mut v, view) = gallery(&mut real, "context_map", theme);
        v.simulate_resize(size(px(1300.), px(950.)));
        ready(&mut v, &view);
        let field = bounds(&mut v, &view, "group", "Context allocation map");
        assert!(
            filled_including_rounded_clips(&mut v, panel)
                .iter()
                .any(
                    |quad| (f64::from(quad.bounds.width) - field.width).abs() <= 1.
                        && (f64::from(quad.bounds.height) - field.height).abs() <= 1.
                ),
            "{theme}: map panel uses its own paint"
        );
        let clean = theme.starts_with("clean");
        let initial = node(&mut v, &view, "text", "74%");
        for ((label, grows), (wash, ink)) in
            [("74%", 0), ("82%", 2), ("97%", 4)].into_iter().zip(states)
        {
            for _ in 0..grows {
                click(&mut v, &view, "button", "Grow context");
            }
            let item = node(&mut v, &view, "text", label);
            assert_eq!(item.id, initial.id);
            let area = item.geometry.unwrap().visual;
            let min_width = if clean { 36. } else { 44. };
            let height = if clean { 20. } else { 24. };
            assert!(
                (area.height - height).abs() <= 0.5 && area.width + 0.5 >= min_width,
                "{theme}/{label}: distinct readout metrics, got {area:?}"
            );
            let (background, radii) = capacity_paint(&mut v, area);
            assert_eq!(background, gpui::rgba(wash).into(), "{theme}/{label}: fill");
            assert!(
                radii.iter().all(|r| if clean {
                    (*r - 5.).abs() <= 0.5
                } else {
                    *r >= 10.
                }),
                "{theme}/{label}: {radii:?}"
            );
            assert!(
                glyphs_with_ink(&mut v, area, ink) >= 2,
                "{theme}/{label}: actual ink"
            );
        }
    }
}

#[gpui::test]
fn context_map_capacity_readout_interpolates_its_author_paint(cx: &mut TestAppContext) {
    let (mut v, view, clock) = motion_gallery(cx);
    let initial = bounds(&mut v, &view, "text", "74%");
    let neutral = capacity_paint(&mut v, initial).0;
    for _ in 0..2 {
        click(&mut v, &view, "button", "Grow context");
    }
    let area = bounds(&mut v, &view, "text", "82%");
    assert_eq!(capacity_paint(&mut v, area).0, neutral);
    clock.advance(Duration::from_millis(150));
    ready(&mut v, &view);
    let intermediate = capacity_paint(&mut v, area).0;
    let target = gpui::rgba(0xffecd6ff).into();
    assert!(intermediate != neutral && intermediate != target);
    clock.advance(Duration::from_millis(180));
    ready(&mut v, &view);
    assert_eq!(capacity_paint(&mut v, area).0, target);
}

#[gpui::test]
fn context_map_capacity_uses_semantic_surfaces_in_alternate_palettes(cx: &mut TestAppContext) {
    for theme in ["nord", "tokyo-night"] {
        let (mut v, view) = gallery(cx, "context_map", theme);
        v.simulate_resize(size(px(1300.), px(950.)));
        ready(&mut v, &view);
        let variant = v.update(|_, cx| view.theme_snapshot(cx).unwrap().variant);
        let color = |token| variant.tokens.color(token).unwrap().as_rgba_hex();
        let area = bounds(&mut v, &view, "text", "74%");
        assert_eq!(
            capacity_paint(&mut v, area).0,
            gpui::rgba(color("surface_hover")).into()
        );
        for _ in 0..2 {
            click(&mut v, &view, "button", "Grow context");
        }
        let area = bounds(&mut v, &view, "text", "82%");
        assert_eq!(
            capacity_paint(&mut v, area).0,
            gpui::rgba(color("badge.warning_bg")).into()
        );
        let field = bounds(&mut v, &view, "group", "Context allocation map");
        assert!(
            filled_including_rounded_clips(&mut v, color("surface_raised"))
                .iter()
                .any(
                    |paint| (f64::from(paint.bounds.width) - field.width).abs() <= 1.
                        && (f64::from(paint.bounds.height) - field.height).abs() <= 1.
                )
        );
    }
}

#[gpui::test]
#[cfg(feature = "vendor-gpui")]
fn context_map_used_count_has_native_shine_without_changing_the_accepted_total(
    cx: &mut TestAppContext,
) {
    use super::collaboration_stream::glyph_band_center;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("context_map_used_shine"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (id, theme, preference) in [
        ("modern", "modern-light", MotionPreference::Normal),
        ("clean", "clean-dark", MotionPreference::Normal),
        ("reduced", "clean-dark", MotionPreference::Reduced),
        ("disabled", "modern-light", MotionPreference::None),
    ] {
        let clock = ManualRuntimeClock::new(Instant::now());
        let prepared = acceptance::view(&AcceptanceLaunch {
            page: "context_map".into(),
            theme: theme.into(),
            ..Default::default()
        })
        .unwrap()
        .runtime_clock(clock.clock())
        .motion_preference(preference)
        .prepare()
        .unwrap();
        let (window, view) = super::mount_prepared(
            &mut native,
            prepared,
            (id, "gallery"),
            false,
            |host, view| Host { host, view },
        );
        let mut v = VisualTestContext::from_window(*window, &native);
        v.update(|window, _| window.activate_window());
        v.simulate_resize(size(px(1300.), px(950.)));
        ready(&mut v, &view);
        let amount = node(&mut v, &view, "text", "89k");
        let sweep = |v: &mut VisualTestContext| {
            v.update(|window, _| window.visual_probe_with_painted_scene(glyph_band_center))
        };
        assert_eq!(sweep(&mut v), None, "{id}: band starts off the glyphs");
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap();

        clock.advance(Duration::from_millis(350));
        ready(&mut v, &view);
        let from = sweep(&mut v);
        clock.advance(Duration::from_millis(400));
        ready(&mut v, &view);
        let to = sweep(&mut v);
        if preference == MotionPreference::Normal {
            assert!(
                matches!((from, to), (Some(a), Some(b)) if b > a + 8.),
                "{id}: expected a moving glyph band, got {from:?} -> {to:?}"
            );
        } else {
            assert_eq!(
                (from, to),
                (None, None),
                "{id}: decorative motion is disabled"
            );
        }

        clock.advance(Duration::from_millis(350));
        ready(&mut v, &view);
        assert_eq!(sweep(&mut v), None, "{id}: the next cycle starts outside");
        let after = node(&mut v, &view, "text", "89k");
        assert_eq!(after.id, amount.id);
        assert_eq!(after.geometry, amount.geometry);
        assert_eq!(
            node(&mut v, &view, "status", "Context usage").description,
            "89,000 / 120,000 · 74%"
        );
        assert!(
            v.update(|_, cx| view.take_performance_snapshot(cx))
                .unwrap()
                .timings
                .is_empty(),
            "{id}: a frame must not invoke Rhai"
        );
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn context_map_tile_keeps_title_ink_and_fades_its_secondary_line(cx: &mut TestAppContext) {
    use crate::support::{glyph_rects_with_ink, logical_paint_rect, walk_live_paint};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("context_map_tile_label_paint"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, ink) in [("modern-light", 0x0190feff), ("clean-light", 0x0070d6ff)] {
        let (mut v, view) = gallery(&mut real, "context_map", theme);
        v.simulate_resize(size(px(1300.), px(950.)));
        ready(&mut v, &view);
        let tile = bounds(&mut v, &view, "button", "Repository scan");
        let name = glyph_rects_with_ink(&mut v, tile, ink);
        let count = glyph_rects_with_ink(&mut v, tile, (ink & 0xffffff00) | 0xb3);
        assert!(name.len() >= 9, "{theme}: actual title ink {name:?}");
        assert!(
            count.len() >= 4,
            "{theme}: expected secondary glyphs at 70% opacity, got {count:?}"
        );
        assert!(count[0].y > name[0].y + 9., "{theme}: {name:?} {count:?}");

        let mask = v.update(|window, _| {
            let scale = window.scale_factor();
            window.visual_probe_with_painted_scene(|scene| {
                let mut found = false;
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::default(),
                    &mut |scene, pose| {
                        for group in &scene.isolated_groups {
                            let r = logical_paint_rect(group.bounds, pose, scale);
                            let ramps = group.alpha_ramps();
                            if f64::from(r.x) > tile.x
                                && f64::from(r.right()) < tile.x + tile.width
                                && f64::from(r.y) > tile.y
                                && f64::from(r.bottom()) < tile.y + tile.height
                                && ramps[4] == 1.
                                && ramps[1] == 0.
                                && (1. / ramps[3] / scale - 18.).abs() < 0.2
                            {
                                found = true;
                            }
                        }
                    },
                );
                found
            })
        });
        assert!(
            mask,
            "{theme}: 18px horizontal foreground fade belongs to the tile label"
        );
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn context_map_free_tile_uses_its_own_ink_without_losing_the_caption_fade(cx: &mut TestAppContext) {
    use crate::support::glyph_rects_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("context_map_free_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    for (theme, title_ink, count_ink) in [
        ("modern-light", 0x8a8a8aff, 0x8a8a8ab3),
        ("modern-dark", 0x8a8a8aff, 0x8a8a8ab3),
        ("clean-light", 0x00000073, 0x00000050),
        ("clean-dark", 0xffffff73, 0xffffff50),
    ] {
        let (mut v, view) = gallery(&mut real, "context_map", theme);
        v.simulate_resize(size(px(1300.), px(950.)));
        ready(&mut v, &view);
        let free = node(&mut v, &view, "button", "Free");
        let area = free.geometry.unwrap().visual;
        let title = glyph_rects_with_ink(&mut v, area, title_ink);
        let count = glyph_rects_with_ink(&mut v, area, count_ink);
        assert!(title.len() >= 4, "{theme}: free title ink {title:?}");
        assert!(count.len() >= 3, "{theme}: free count ink {count:?}");
        assert!(
            count[0].y > title[0].y + 9.,
            "{theme}: the count must paint on its own line"
        );
        // The authored count opacity and both stripe phases matter. A
        // solid reading would discard the actual alternating paper.
        let findings = v.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
        let risk = findings
            .iter()
            .find(|finding| {
                finding.path.contains("/field/free:") && finding.path.ends_with("/counts/0")
            })
            .and_then(|finding| finding.contrast.as_ref())
            .expect("the translucent free-space count remains an audit finding");
        assert!(
            (risk.element_opacity - 0.7).abs() < 1e-6,
            "{theme}: {risk:?}"
        );
        assert!(
            risk.conservative && risk.background_srgb.is_none(),
            "{theme}: {risk:?}"
        );
        click(&mut v, &view, "button", "Free");
        node(&mut v, &view, "status", "Inspected unused capacity");
        assert_eq!(node(&mut v, &view, "button", "Free").id, free.id);
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn context_map_free_hatch_keeps_its_period_when_the_viewport_grows(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast::metal_renderer::MetalRenderer;
    #[cfg(feature = "gpui-pre")]
    use apple_pre::metal_renderer::MetalRenderer;

    let (mut v, view) = gallery(cx, "context_map", "modern-light");
    let free = node(&mut v, &view, "button", "Free").id;
    for width in [1300., 2020.] {
        v.simulate_resize(size(px(width), px(960.)));
        ready(&mut v, &view);
        let area = bounds(&mut v, &view, "button", "Free");
        assert_eq!(node(&mut v, &view, "button", "Free").id, free);
        let (stripe_centers, scale) = v.update(|window, _| {
            let scale = window.scale_factor();
            let extent = window.viewport_size();
            let device = size(
                gpui::DevicePixels((f32::from(extent.width) * scale).ceil() as i32),
                gpui::DevicePixels((f32::from(extent.height) * scale).ceil() as i32),
            );
            window.visual_probe_with_painted_scene(|scene| {
                let mut renderer = MetalRenderer::new_headless(Default::default()).unwrap();
                let image = renderer.render_scene_to_image(scene, device).unwrap();
                let s = f64::from(scale);
                let y = ((area.y + area.height * 0.7) * s).floor() as u32;
                let x0 = ((area.x + 16.) * s).ceil() as u32;
                let x1 = ((area.x + area.width - 16.) * s).floor() as u32;
                let mut starts = None;
                let mut centers = Vec::new();
                for x in x0..x1 {
                    if image.get_pixel(x, y).0[0] <= 251 {
                        starts.get_or_insert(x);
                    } else if let Some(start) = starts.take() {
                        centers.push((f64::from(start) + f64::from(x - 1)) / 2.);
                    }
                }
                (centers, s)
            })
        });
        assert!(stripe_centers.len() >= 7, "{width}: {stripe_centers:?}");
        let pitch = (stripe_centers.last().unwrap() - stripe_centers[0])
            / (f64::from(u32::try_from(stripe_centers.len()).unwrap() - 1) * scale);
        assert!(
            (pitch - 7. * 2_f64.sqrt()).abs() < 0.18,
            "{width}: expected the same CSS repeat period, measured {pitch:.3}px in {area:?}"
        );
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn context_map_free_ink_has_real_low_contrast_on_the_hatched_paper(_: &mut TestAppContext) {
    use crate::support::PresetRisks;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui::{AppContext as _, HeadlessAppContext};
    use gpui_rhai::{
        AutomationCommand, Rgba8, ScriptViewConfig, ScriptViewHost, ThemeTokenOverrides, UiValue,
    };

    let mut headless = HeadlessAppContext::with_platform(
        std::sync::Arc::new(apple::AppleTextSystem::new()),
        std::sync::Arc::new(()),
        || {
            Ok(Some(Box::new(
                apple::metal_renderer::MetalHeadlessRenderer::new()?,
            )))
        },
    );
    headless.update(gpui_rhai::install);
    let mut reviews = PresetRisks::new();
    let settle_frame = |headless: &mut HeadlessAppContext, window| {
        for _ in 0..12 {
            headless.advance_clock(Duration::from_millis(16));
            headless.run_until_parked();
            headless
                .update_window(window, |_, window, cx| window.simulate_next_frame(cx))
                .unwrap();
        }
    };
    for (theme, ink, paper_red, stripe_red, expected) in [
        ("modern-light", 0x8a8a8aff, 255, 246, [3.19, 2.14]),
        ("modern-dark", 0x8a8a8aff, 35, 44, [4.03, 2.72]),
        ("clean-light", 0x00000073, 255, 246, [3.32, 2.19]),
        ("clean-dark", 0xffffff73, 28, 37, [4.28, 2.82]),
    ] {
        let mut mount = |hide_ink| {
            let mut view = acceptance::view(&AcceptanceLaunch {
                page: "context_map".into(),
                theme: theme.into(),
                ..Default::default()
            })
            .unwrap()
            .motion_preference(MotionPreference::None);
            if hide_ink {
                // Unchanged RGB and metrics. Change only the alpha of the
                // named authors' ink to expose the destination under it.
                let mut overrides = ThemeTokenOverrides::default();
                overrides.colors.insert(
                    "context_map.muted".into(),
                    Rgba8::from_rgba_hex(ink & 0xffff_ff00),
                );
                view = view.theme_token_overrides(overrides);
            }
            let prepared = view.prepare().unwrap();
            let mut mounted = None;
            let id = if hide_ink {
                "hatch-paper"
            } else {
                "hatch-labels"
            };
            let window = headless
                .open_window(size(px(1300.), px(1280.)), |window, cx| {
                    let host = ScriptViewHost::new(id, cx).unwrap();
                    let view = prepared
                        .mount(ScriptViewConfig::new(id), host.clone(), window, cx)
                        .unwrap();
                    mounted = Some(view.clone());
                    cx.new(|_| Host { host, view })
                })
                .unwrap();
            let view = mounted.unwrap();
            let window = *window;
            settle_frame(&mut headless, window);
            headless
                .update_window(window, |_, window, cx| {
                    view.automate(
                        AutomationCommand::Action {
                            id: "gallery.set_scale".into(),
                            payload: Some(UiValue::String("200".into())),
                        },
                        window,
                        cx,
                    )
                })
                .unwrap()
                .unwrap();
            settle_frame(&mut headless, window);
            assert!(headless.update(|cx| view.last_error(cx).unwrap()).is_none());
            (window, view)
        };
        let (visible, visible_view) = mount(false);
        let (uninked, uninked_view) = mount(true);
        let tile = |view: &ScriptViewHandle, cx: &gpui::App| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("button", "Free")
                .next()
                .unwrap()
                .geometry
                .unwrap()
                .visual
        };
        let area = headless.update(|cx| tile(&visible_view, cx));
        assert_eq!(headless.update(|cx| tile(&uninked_view, cx)), area);
        let [r, g, b, alpha] = ink.to_be_bytes();
        let scale = headless
            .update_window(visible, |_, window, _| f64::from(window.scale_factor()))
            .unwrap();
        let other_scale = headless
            .update_window(uninked, |_, window, _| f64::from(window.scale_factor()))
            .unwrap();
        let front = headless.capture_screenshot(visible).unwrap();
        let back = headless.capture_screenshot(uninked).unwrap();
        assert!((scale - other_scale).abs() < f64::EPSILON);
        assert_eq!(front.dimensions(), back.dimensions());
        let (stock, findings, overridden) = headless.update(|cx| {
            let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
            (
                visible_view.theme_snapshot(cx).unwrap().variant,
                visible_view.composition_audit_with(&rules, cx).unwrap(),
                uninked_view.composition_audit_with(&rules, cx).unwrap(),
            )
        });
        let luminance = |rgb: [f64; 3]| {
            rgb.into_iter()
                .zip([0.2126, 0.7152, 0.0722])
                .map(|(rgb, weight)| {
                    let c = rgb / 255.;
                    weight
                        * if c <= 0.04045 {
                            c / 12.92
                        } else {
                            ((c + 0.055) / 1.055).powf(2.4)
                        }
                })
                .sum::<f64>()
        };
        // Derive the two painted text bands from the image difference.
        // In a retained compositing group, primitive alpha need not be
        // final glyph alpha. The paired images include that composition.
        let x0 = ((area.x + 4.) * scale).ceil() as u32;
        let x1 = (((area.x + area.width - 4.) * scale).floor() as u32).min(back.width());
        let y0 = ((area.y + 4.) * scale).ceil() as u32;
        let y1 = (((area.y + area.height - 4.) * scale).floor() as u32).min(back.height());
        let mut bands = Vec::new();
        let mut last = None;
        for y in y0..y1 {
            let row = (x0..x1)
                .filter_map(|x| {
                    let [pr, pg, pb, _] = back.get_pixel(x, y).0;
                    front.get_pixel(x, y).0[..3]
                        .iter()
                        .zip([pr, pg, pb])
                        .any(|(fg, bg)| fg.abs_diff(bg) >= 6)
                        .then_some([pr, pg, pb])
                })
                .collect::<Vec<_>>();
            if !row.is_empty() {
                if last.is_none_or(|last| y > last + 2) {
                    bands.push(Vec::new());
                }
                bands.last_mut().unwrap().extend(row);
                last = Some(y);
            }
        }
        assert_eq!(
            bands.len(),
            2,
            "{theme}: expected distinct title and count pixels"
        );
        for (index, band) in bands.into_iter().enumerate() {
            let mut samples = [0, 0];
            let mut worst = 21.0_f64;
            let opacity = f64::from(alpha) / 255. * [1., 0.7][index];
            for [pr, pg, pb] in band {
                if pr.abs_diff(paper_red) <= 1 {
                    samples[0] += 1;
                } else if pr.abs_diff(stripe_red) <= 1 {
                    samples[1] += 1;
                } else {
                    panic!("{theme}: unpaired backing pixel {pr},{pg},{pb} under a glyph");
                }
                let paper = [pr, pg, pb].map(f64::from);
                let nominal = std::array::from_fn(|i| {
                    paper[i] * (1.0 - opacity) + f64::from([r, g, b][i]) * opacity
                });
                let p = luminance(paper);
                let n = luminance(nominal);
                worst = worst.min((p.max(n) + 0.05) / (p.min(n) + 0.05));
            }
            assert!(
                samples.into_iter().all(|n| n >= 3),
                "{theme}/{index}: pixels {samples:?}"
            );
            eprintln!("{theme}/{index}: {samples:?} paired flat/stripe glyph pixels; {worst:.3}:1");
            assert!(
                (worst - expected[index]).abs() < 0.045,
                "{theme}/{index}: {worst}"
            );
            let tail = [
                "context:6:writer:/0/title/0",
                "context:6:writer:/0/counts/0",
            ][index];
            let raw = findings
                .iter()
                .find(|finding| {
                    finding.path.contains("/field/free:15:gallery/") && finding.path.ends_with(tail)
                })
                .unwrap_or_else(|| panic!("{theme}: the raw audit must retain {tail}"));
            let evidence = raw.contrast.as_ref().unwrap();
            // The oriented swatch may round by one channel in the image;
            // that must not turn the public lower bound into an exact number.
            assert!(evidence.ratio <= worst + 0.03 && worst < evidence.minimum);
            assert!(
                reviews.record(&stock, raw),
                "{theme}: unchanged fixture {raw:?}"
            );
            // The selected-preset snapshot is not a substitute for the
            // resolved paint. Inspect the real Host-override report.
            for altered in overridden.iter().filter(|finding| finding.path == raw.path) {
                assert!(!reviews.record(&stock, altered));
            }
            let mut nonstock = stock.clone();
            std::sync::Arc::make_mut(&mut nonstock.tokens)
                .colors
                .insert("context_map.muted".into(), Rgba8::from_rgba_hex(0));
            assert!(!reviews.record(&nonstock, raw));
            let mut altered = raw.clone();
            altered.path = raw.path.replace("context:6:writer:", "context:6:other:");
            assert!(!reviews.record(&stock, &altered));
            altered.path.clone_from(&raw.path);
            altered.contrast.as_mut().unwrap().ratio -= 0.05;
            assert!(!reviews.record(&stock, &altered));
            altered.contrast.clone_from(&raw.contrast);
            altered.contrast.as_mut().unwrap().background_srgb = Some([1.; 3]);
            assert!(!reviews.record(&stock, &altered));
        }
        for window in [visible, uninked] {
            headless
                .update_window(window, |_, window, _| window.remove_window())
                .unwrap();
        }
        headless.run_until_parked();
    }
    reviews.assert_reported();
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn context_map_clean_leaves_the_outer_paper_to_its_host(cx: &mut TestAppContext) {
    use super::acceptance::action;
    use crate::support::Mount;
    #[cfg(feature = "gpui-fast")]
    use apple_fast::metal_renderer::MetalRenderer;
    #[cfg(feature = "gpui-pre")]
    use apple_pre::metal_renderer::MetalRenderer;

    let (mut v, view) = Mount::new(r#"
        import "patterns/context_map" as map;
        fn state_schema(){#{fields:#{second:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn init(ctx){ctx.register_action("preview.paper",Fn("paper"));}
        fn paper(ctx,value){ctx.set_state("second",value);}
        fn view(ctx){
            let paper=if ctx.get_state("second"){rgb(0x703f54)}else{rgb(0x32506a)};
            column([map::ContextMap(#{key:"map",context:"hosted",revision:0,title:"Hosted context",
                limit:100000,blocks:[#{id:"policy",kind:"system",label:"Policy",tokens:22000}]})])
                .with_style(style().background(paper).padding(px(16)))
        }
    "#).registry("patterns/context_map").bundled_assets().theme(gpui_rhai_registry::CLEAN_DARK_THEME)
        .motion(MotionPreference::None).mount(cx);
    v.simulate_resize(size(px(1200.), px(900.)));
    ready(&mut v, &view);
    let free = node(&mut v, &view, "button", "Free").id;
    for (second, host) in [(false, [50, 80, 106]), (true, [112, 63, 84])] {
        action(
            &mut v,
            &view,
            "preview.paper",
            gpui_rhai::UiValue::Bool(second),
        );
        ready(&mut v, &view);
        assert_eq!(node(&mut v, &view, "button", "Free").id, free);
        let board = bounds(&mut v, &view, "group", "Hosted context");
        let tile = bounds(&mut v, &view, "button", "Free");
        v.update(|window, _| {
            let s = window.scale_factor();
            let extent = window.viewport_size();
            let device = size(
                gpui::DevicePixels((f32::from(extent.width) * s).ceil() as i32),
                gpui::DevicePixels((f32::from(extent.height) * s).ceil() as i32),
            );
            window.visual_probe_with_painted_scene(|scene| {
                let mut renderer = MetalRenderer::new_headless(Default::default()).unwrap();
                let image = renderer.render_scene_to_image(scene, device).unwrap();
                let s = f64::from(s);
                let at = |x: f64, y: f64| {
                    let p = image
                        .get_pixel((x * s).floor() as u32, (y * s).floor() as u32)
                        .0;
                    [p[0], p[1], p[2]]
                };
                assert_eq!(at(board.x + board.width / 2., board.y + 2.), host);
                let y = tile.y + tile.height * 0.7;
                let run = (0..60).map(|dx| at(tile.x + 32. + f64::from(dx), y));
                assert!(
                    run.clone().any(|p| p == [28, 28, 31]),
                    "the field owns its inner paper"
                );
                assert!(
                    run.into_iter().any(|p| p[0] >= 35),
                    "the transparent stripes remain"
                );
            });
        });
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn context_map_summary_and_footer_draw_their_own_secondary_ink(cx: &mut TestAppContext) {
    use crate::support::{PresetRisks, glyphs_with_ink};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("context_map_summary_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    for (theme, expected, empty_ink, height) in [
        ("modern-light", 0x8a8a8aff, 0xb8b8b8ff, 18.),
        ("modern-dark", 0x8a8a8aff, 0x5c5c5cff, 18.),
        ("clean-light", 0x00000073, 0x00000045, 16.),
        ("clean-dark", 0xffffff73, 0xffffff47, 16.),
    ] {
        let (mut v, view) = gallery(&mut real, "context_map", theme);
        v.simulate_resize(size(px(1300.), px(950.)));
        ready(&mut v, &view);
        let number = bounds(&mut v, &view, "text", "/ 120k");
        let note = bounds(&mut v, &view, "status", "No compaction yet");
        assert!((number.height - height).abs() <= 0.5, "{theme}: {number:?}");
        assert!(
            glyphs_with_ink(&mut v, number, expected) >= 4,
            "{theme}: {number:?}"
        );
        assert!(
            glyphs_with_ink(&mut v, note, empty_ink) >= 10,
            "{theme}: {note:?}"
        );
        let (variant, raw) = v.update(|_, cx| {
            (
                view.theme_snapshot(cx).unwrap().variant,
                view.composition_audit_with(&rules, cx).unwrap(),
            )
        });
        let hint = raw
            .iter()
            .find(|f| f.path.ends_with("/context-map/fates/0"))
            .unwrap();
        let mut policy = PresetRisks::new();
        assert!(policy.record(&variant, hint), "{theme}: {hint:#?}");
        let mut different_paper = hint.clone();
        different_paper.contrast.as_mut().unwrap().background_srgb = Some([0.5; 3]);
        assert!(!policy.record(&variant, &different_paper));
        if let Some(total) = raw
            .iter()
            .find(|f| f.path.ends_with("/context-map/header/summary/totals/0/1"))
        {
            assert!(policy.record(&variant, total), "{theme}: {total:#?}");
        }
        let striped = raw
            .iter()
            .find(|f| {
                f.path
                    .ends_with("/field/free:15:gallery/context:6:writer:/0/counts/0")
            })
            .unwrap();
        let evidence = striped.contrast.as_ref().unwrap();
        assert!(evidence.conservative, "{theme}: {striped:#?}");
        assert!(evidence.background_srgb.is_none(), "{theme}: {striped:#?}");
        assert!(
            policy.record(&variant, striped),
            "{theme}: record the independently reviewed bound, not a scalar paper"
        );
        let mut collapsed = striped.clone();
        let evidence = collapsed.contrast.as_mut().unwrap();
        evidence.conservative = false;
        evidence.background_srgb = Some([0.5; 3]);
        assert!(!policy.record(&variant, &collapsed));
        let mut changed_bound = striped.clone();
        changed_bound.contrast.as_mut().unwrap().ratio += 0.01;
        assert!(!policy.record(&variant, &changed_bound));
        policy.assert_reported();
        click(&mut v, &view, "button", "Compact context");
        v.executor().advance_clock(Duration::from_millis(800));
        ready(&mut v, &view);
        let receipt = bounds(&mut v, &view, "group", "Compaction outcomes");
        assert!(
            glyphs_with_ink(&mut v, receipt, expected) >= 12,
            "{theme}: accepted outcomes must remain distinct from the faint hint"
        );
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn context_map_model_uses_its_header_ink_in_each_preset(cx: &mut TestAppContext) {
    use crate::support::glyphs_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("context_map_model_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, ink) in [
        ("modern-light", 0x6b6b6bff),
        ("modern-dark", 0xa1a1a1ff),
        ("clean-light", 0x0000009c),
        ("clean-dark", 0xffffff9e),
    ] {
        let (mut v, view) = gallery(&mut real, "context_map", theme);
        v.simulate_resize(size(px(1300.), px(950.)));
        ready(&mut v, &view);
        let area = bounds(&mut v, &view, "text", "Atlas");
        assert!(glyphs_with_ink(&mut v, area, ink) >= 4, "{theme}: {area:?}");
    }
}

#[gpui::test]
fn context_map_gallery_compares_accepted_snapshots_and_selects_agents(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "context_map", "modern-light");
    v.simulate_resize(size(px(1300.0), px(950.0)));
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "status", "Context usage").description,
        "89,000 / 120,000 · 74%"
    );
    click(&mut v, &view, "button", "Grow context");
    node(&mut v, &view, "status", "Accepted new conversation turns");
    assert_eq!(
        node(&mut v, &view, "status", "Context usage").description,
        "93,500 / 120,000 · 78%"
    );
    click(&mut v, &view, "button", "Compact context");
    node(&mut v, &view, "status", "Compacting");
    v.executor().advance_clock(Duration::from_millis(800));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Compaction accepted");
    node(&mut v, &view, "button", "Compaction summary");
    assert_eq!(
        node(&mut v, &view, "status", "Context usage").description,
        "35,300 / 120,000 · 29%"
    );
    click(&mut v, &view, "button", "Refuse next comparison");
    click(&mut v, &view, "button", "Before");
    node(&mut v, &view, "status", "Comparison request refused");
    node(&mut v, &view, "button", "Compaction summary");
    assert_eq!(node(&mut v, &view, "button", "After").pressed, Some(true));

    click(&mut v, &view, "button", "Before");
    node(&mut v, &view, "status", "Showing before compaction");
    assert_eq!(node(&mut v, &view, "button", "Before").pressed, Some(true));
    assert!(
        node(&mut v, &view, "button", "Repository scan")
            .description
            .contains("Folded into the summary")
    );
    assert!(
        node(&mut v, &view, "button", "Tool results")
            .description
            .contains("Dropped")
    );

    click(&mut v, &view, "combobox", "Context agent");
    click(&mut v, &view, "option", "Reviewer");
    node(&mut v, &view, "status", "Reviewer context accepted");
    assert_eq!(
        node(&mut v, &view, "status", "Context usage").description,
        "96,000 / 500,000 · 19%"
    );
    node(&mut v, &view, "button", "Review notes");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Before")
            .next()
            .is_none()
    );
}

#[gpui::test]
fn context_map_gallery_native_inspection_keeps_identity_across_resize(cx: &mut TestAppContext) {
    for theme in ["modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "context_map", theme);
        v.simulate_resize(size(px(1300.0), px(940.0)));
        ready(&mut v, &view);
        let wide = bounds(&mut v, &view, "group", "Context allocation map");
        assert!(wide.width > 480.0, "{theme}: {wide:?}");
        let sys = bounds(&mut v, &view, "button", "System prompt");
        v.simulate_mouse_move(center(sys), None, Modifiers::none());
        ready(&mut v, &view);
        node(&mut v, &view, "tooltip", "System prompt");
        click(&mut v, &view, "button", "System prompt");
        node(&mut v, &view, "status", "Pinned __free");
        assert_eq!(
            node(&mut v, &view, "button", "System prompt").pressed,
            Some(true)
        );

        click(&mut v, &view, "button", "Narrow map");
        node(&mut v, &view, "button", "Wide map");
        let narrow = bounds(&mut v, &view, "group", "Context allocation map");
        assert!(
            narrow.width <= 280.5 && narrow.height < wide.height,
            "{theme}: {narrow:?}"
        );
        node(&mut v, &view, "tooltip", "System prompt");
        click(&mut v, &view, "button", "Free");
        node(&mut v, &view, "status", "Inspected unused capacity");
        assert_eq!(
            node(&mut v, &view, "button", "System prompt").pressed,
            Some(false)
        );
    }
}

fn motion_gallery(
    cx: &mut TestAppContext,
) -> (VisualTestContext, ScriptViewHandle, ManualRuntimeClock) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "context_map".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::Normal)
    .runtime_clock(clock.clock())
    .prepare()
    .unwrap();
    let (window, view) =
        super::mount_prepared(cx, prepared, ("map", "gallery"), false, |host, view| Host {
            host,
            view,
        });
    let mut v = VisualTestContext::from_window(*window, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1300.0), px(920.0)));
    clock.advance(Duration::from_millis(500));
    ready(&mut v, &view);
    (v, view, clock)
}

#[gpui::test]
fn context_map_gallery_motion_reuses_frame_for_paint_hit_and_inert_exits(cx: &mut TestAppContext) {
    let (mut v, view, clock) = motion_gallery(cx);
    let chat = |visual: &mut VisualTestContext| bounds(visual, &view, "button", "Conversation");
    let area = |bounds: GeometryBounds| bounds.width * bounds.height;
    let initial = chat(&mut v);
    click(&mut v, &view, "button", "Grow context");
    // The accepted data updates immediately; the frame does not jump.
    assert_eq!(
        node(&mut v, &view, "status", "Context usage").description,
        "93,500 / 120,000 · 78%"
    );
    assert!((area(chat(&mut v)) - area(initial)).abs() < 500.0);
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    clock.advance(Duration::from_millis(65));
    ready(&mut v, &view);
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty(),
        "projecting a pure animation frame must not evaluate Rhai"
    );
    let middle = chat(&mut v);
    let fill = filled_including_rounded_clips(&mut v, 0xf1ea_faff)
        .into_iter()
        .find(|quad| {
            (f64::from(quad.bounds.x) - middle.x).abs() < 1.5
                && (f64::from(quad.bounds.y) - middle.y).abs() < 1.5
                && (f64::from(quad.bounds.width) - middle.width).abs() < 1.5
                && (f64::from(quad.bounds.height) - middle.height).abs() < 1.5
        });
    assert!(
        fill.is_some(),
        "paint and the moving semantic target disagree: {middle:?}"
    );
    v.simulate_click(center(middle), Modifiers::none());
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Pinned chat");
    clock.advance(Duration::from_millis(500));
    ready(&mut v, &view);
    let end = chat(&mut v);
    assert!(
        area(initial) + 500.0 < area(middle) && area(middle) + 500.0 < area(end),
        "allocation should animate through a real midpoint: {initial:?} → {middle:?} → {end:?}"
    );

    click(&mut v, &view, "button", "Compact context");
    v.executor().advance_clock(Duration::from_millis(740));
    clock.advance(Duration::from_millis(740));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Compaction accepted");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Repository scan")
            .next()
            .is_none()
    );
    clock.advance(Duration::from_millis(60));
    ready(&mut v, &view);
    let ghost = filled_including_rounded_clips(&mut v, 0xe6f3_fdff)
        .into_iter()
        .find(|q| q.bounds.width > 10.0 && q.bounds.height > 10.0)
        .expect("a visible exit lane at the midpoint")
        .bounds;
    v.simulate_click(
        point(
            px(ghost.x + ghost.width * 0.5),
            px(ghost.y + ghost.height * 0.5),
        ),
        Modifiers::none(),
    );
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Compaction accepted");
    click(&mut v, &view, "button", "Before");
    assert_eq!(node(&mut v, &view, "button", "Before").pressed, Some(true));
    clock.advance(Duration::from_millis(42));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Narrow map");
    let narrow = bounds(&mut v, &view, "group", "Context allocation map");
    assert!(narrow.width <= 280.5 && narrow.height < 280.0, "{narrow:?}");
    clock.advance(Duration::from_millis(500));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Repository scan");
    node(&mut v, &view, "status", "Pinned files/资料");
}

#[gpui::test]
fn context_map_gallery_capacity_and_ordinary_removal_retarget_without_stale_hits(
    cx: &mut TestAppContext,
) {
    let (mut v, view, clock) = motion_gallery(cx);
    let tick = |v: &mut VisualTestContext, ms| {
        clock.advance(Duration::from_millis(ms));
        ready(v, &view);
    };
    let area = |visual: &mut VisualTestContext| {
        let b = bounds(visual, &view, "button", "Free");
        b.width * b.height
    };

    let initial = area(&mut v);
    click(&mut v, &view, "button", "Raise limit");
    assert_eq!(
        node(&mut v, &view, "status", "Context usage").description,
        "89,000 / 180,000 · 49%"
    );
    assert!((area(&mut v) - initial).abs() < 500.0);
    tick(&mut v, 70);
    let middle = area(&mut v);
    assert!(middle > initial + 4_000.0);
    click(&mut v, &view, "button", "Restore limit");
    assert!(
        (area(&mut v) - middle).abs() < 500.0,
        "retarget must reuse the sampled extent"
    );
    tick(&mut v, 600);
    assert!((area(&mut v) - initial).abs() < 500.0);

    let result_fill = |visual: &mut VisualTestContext| {
        filled_including_rounded_clips(visual, 0xffec_d6ff)
            .into_iter()
            .find(|q| q.bounds.width > 3.0 && q.bounds.height > 3.0)
            .map(|q| q.bounds)
    };
    let has_visual_record = |visual: &mut VisualTestContext| {
        let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
        let mut labels = Vec::new();
        node_texts(&root, &mut labels);
        labels.iter().any(|label| label == "Tool results")
    };
    let old = result_fill(&mut v).expect("accepted tile paints");
    click(&mut v, &view, "button", "Remove tool results");
    tick(&mut v, 65);
    let ghost = result_fill(&mut v).expect("ordinary removal must keep a painted exit");
    assert!(has_visual_record(&mut v));
    assert!(ghost.width * ghost.height < old.width * old.height);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Tool results")
            .next()
            .is_none()
    );
    v.simulate_click(
        point(
            px(ghost.x + ghost.width * 0.5),
            px(ghost.y + ghost.height * 0.5),
        ),
        Modifiers::none(),
    );
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Tool results removed");
    click(&mut v, &view, "button", "Restore tool results");
    let restored = result_fill(&mut v).expect("reinstated lane continues painting");
    assert!((restored.width * restored.height - ghost.width * ghost.height).abs() < 400.0);
    tick(&mut v, 550);
    click(&mut v, &view, "button", "Tool results");
    node(&mut v, &view, "status", "Pinned results");
    click(&mut v, &view, "button", "Remove tool results");
    v.executor().advance_clock(Duration::from_millis(600));
    tick(&mut v, 600);
    assert!(
        result_fill(&mut v).is_none(),
        "retired lanes may not keep painting"
    );
    assert!(
        !has_visual_record(&mut v),
        "settled exits release their retained labels"
    );
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
