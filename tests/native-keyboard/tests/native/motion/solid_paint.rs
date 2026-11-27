use std::time::{Duration, Instant};

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{ManualRuntimeClock, MotionPreference};

use crate::support::{Mount, Step, bounds, center, painted_quads, settle};

#[gpui::test]
fn solid_paint_retargets_native_hover_and_press_without_layout_changes(cx: &mut TestAppContext) {
    for policy in [
        MotionPreference::Normal,
        MotionPreference::Reduced,
        MotionPreference::None,
    ] {
        let clock = ManualRuntimeClock::new(Instant::now());
        let (mut visual, view) = Mount::new(
            r#"
            fn noop(ctx,payload) { () }
            fn view(ctx) {
                column([box([]).with_key("target")
                    .with_style(style().width(px(120)).height(px(48))
                        .background(rgb(0x200000)).border(px(1)).border_color(rgb(0x000020))
                        .hover(style().background(rgb(0x600000)).border_color(rgb(0x000060)))
                        .active(style().background(rgb(0xa00000))))
                    .paint_transition(160,"linear").on_click(Fn("noop"))
                    .accessibility_role("button").accessibility_label("Target")])
                    .with_style(style().padding(px(20)))
            }
        "#,
        )
        .with({
            let clock = clock.clock();
            move |view| view.runtime_clock(clock)
        })
        .motion(policy)
        .mount(cx);
        settle(&mut visual, Step::TickRefresh, 3);
        let region = bounds(&mut visual, &view, "button", "Target");
        let p = center(region);
        let expect = |v: &mut VisualTestContext, red: u8| {
            let painted = painted_quads(v)
                .into_iter()
                .find_map(|quad| {
                    let r = quad.visible();
                    let color = quad.background.as_solid()?.to_rgb();
                    (px(r.x) < p.x
                        && p.x < px(r.right())
                        && px(r.y) < p.y
                        && p.y < px(r.bottom())
                        && color.g < 0.01
                        && color.b < 0.01
                        && color.r > 0.1)
                        .then_some(color.r * 255.)
                })
                .expect("painted target");
            assert!(
                (painted - f32::from(red)).abs() < 1.1,
                "{policy:?}: {painted} != {red}"
            );
        };
        let run_frames = |visual: &mut VisualTestContext, millis| {
            clock.advance(Duration::from_millis(millis));
            settle(visual, Step::TickRefresh, 3);
        };
        expect(&mut visual, 0x20);
        visual
            .update(|_, cx| view.take_render_statistics(cx))
            .unwrap();
        visual.simulate_mouse_move(p, None, Modifiers::none());
        settle(&mut visual, Step::TickRefresh, 3);
        let animated = policy == MotionPreference::Normal;
        expect(&mut visual, if animated { 0x20 } else { 0x60 });
        run_frames(&mut visual, 80);
        expect(&mut visual, if animated { 0x40 } else { 0x60 });
        let blue = if animated { 64. } else { 96. };
        assert!(
            painted_quads(&mut visual)
                .iter()
                .filter(|q| q.has_border())
                .any(|q| {
                    let color = q.border_color.to_rgb();
                    color.r < 0.01 && color.g < 0.01 && (color.b * 255. - blue).abs() < 1.
                }),
            "{policy:?}: the native border samples the same transition"
        );
        let stats = visual
            .update(|_, cx| view.take_render_statistics(cx))
            .unwrap();
        assert_eq!(
            (stats.polls, stats.commits),
            (0, 0),
            "paint motion must not rerun Rhai"
        );
        visual.simulate_mouse_down(p, MouseButton::Left, Modifiers::none());
        settle(&mut visual, Step::TickRefresh, 3);
        expect(&mut visual, if animated { 0x40 } else { 0xa0 });
        run_frames(&mut visual, 80);
        expect(&mut visual, if animated { 0x70 } else { 0xa0 });
        visual.simulate_mouse_up(p, MouseButton::Left, Modifiers::none());
        settle(&mut visual, Step::TickRefresh, 3);
        expect(&mut visual, if animated { 0x70 } else { 0x60 });
        run_frames(&mut visual, 160);
        expect(&mut visual, 0x60);
        visual.simulate_mouse_move(point(px(1.), px(1.)), None, Modifiers::none());
        settle(&mut visual, Step::TickRefresh, 3);
        run_frames(&mut visual, 160);
        expect(&mut visual, 0x20);
        assert_eq!(bounds(&mut visual, &view, "button", "Target"), region);
        assert_eq!(crate::support::last_error(&mut visual, &view), None);
    }
}

#[cfg(all(target_os = "macos", not(feature = "vendor-gpui")))]
#[gpui::test]
fn loading_label_sweep_falls_back_to_a_continuous_alpha_pulse(cx: &mut TestAppContext) {
    use crate::support::glyphs_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("stock_loading_pulse"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for policy in [
        MotionPreference::Normal,
        MotionPreference::Reduced,
        MotionPreference::None,
    ] {
        let clock = ManualRuntimeClock::new(Instant::now());
        let (mut visual, view) = Mount::new(r#"
            fn view(ctx) {
                column([text("Working").with_key("label")
                    .accessibility_role("text").accessibility_label("Working")
                    .with_style(style().text_color(rgb(0x304050)).opacity(0.8)
                        .glyph_sweep(rgb(0x808080),0.36,0))
                    .motion(motion_transition("glyph_sweep",0.0,1.0,
                        #{duration_ms:1000,iterations:"infinite",easing:"linear",intent:"decorative"}))])
                    .with_style(style().padding(px(20)))
            }
        "#)
        .with({ let clock = clock.clock(); move |view| view.runtime_clock(clock) })
        .motion(policy)
        .mount(&mut native);
        settle(&mut visual, Step::TickRefresh, 4);
        let region = bounds(&mut visual, &view, "text", "Working");
        let initial = glyphs_with_ink(&mut visual, region, 0x304050cc);
        assert!(initial >= 5, "{policy:?}: visible label");
        clock.advance(Duration::from_millis(500));
        settle(&mut visual, Step::TickRefresh, 4);
        let halfway = if policy == MotionPreference::Normal {
            0x304050b8
        } else {
            0x304050cc
        };
        assert_eq!(
            glyphs_with_ink(&mut visual, region, halfway),
            initial,
            "{policy:?}"
        );
        clock.advance(Duration::from_millis(500));
        settle(&mut visual, Step::TickRefresh, 4);
        assert_eq!(glyphs_with_ink(&mut visual, region, 0x304050cc), initial);
        assert_eq!(bounds(&mut visual, &view, "text", "Working"), region);
    }
}
