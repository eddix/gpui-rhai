//! Exterior focus rings preserve border-box geometry, real gaps and input policy.
use crate::support::{Mount, Step, bounds, click_center, last_error, painted_quads, settle};
use gpui::{TestAppContext, rgba};
use gpui_rhai::{FocusIndicators, MotionPreference, Rgba8, ThemeTokenOverrides};

const COLOR: u32 = 0xff56aa80;

#[gpui::test]
fn signed_focus_offsets_paint_insets_without_reflow_or_invalid_geometry(cx: &mut TestAppContext) {
    for (offset, width, height, radius) in [
        (-2, 80.0, 40.0, 8.0),
        (-6, 72.0, 32.0, 4.0),
        (2, 88.0, 48.0, 12.0),
        (-100, 0.0, 0.0, 0.0),
    ] {
        let script = format!(
            r#"fn noop(ctx,value){{}}
            fn view(ctx){{row([text("Action")]).with_style(style().width(px(80)).height(px(40)).radius(px(8)).margin(px(20))
                .focus_ring(px(2),offset_px({offset}),rgba(0xff56aa80)))
                .accessibility_role("button").accessibility_label("Action").on_click(Fn("noop"))}}"#
        );
        let (mut visual, view) = Mount::new(script)
            .motion(MotionPreference::None)
            .id("signed-focus-offset")
            .mount(cx);
        visual.update(|window, cx| view.focus(window, cx).unwrap());
        visual.simulate_keystrokes("tab");
        settle(&mut visual, Step::Frame, 2);
        let layout = bounds(&mut visual, &view, "button", "Action");
        assert_eq!((layout.width, layout.height), (80.0, 40.0));
        let quads = painted_quads(&mut visual);
        let ring = quads.iter().find(|q| q.border_widths == [2.0; 4]);
        if width == 0.0 {
            assert!(ring.is_none(), "collapsed inset");
        } else {
            let ring = ring.expect("ring paint");
            assert!(
                (ring.bounds.width - width).abs() < 0.5
                    && (ring.bounds.height - height).abs() < 0.5
            );
            assert!((ring.corner_radii[0] - radius).abs() < 0.5);
            assert!((f64::from(ring.bounds.x) - layout.x + f64::from(offset + 2)).abs() < 0.5);
        }
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn an_explicit_focus_refinement_can_supply_the_exterior_ring(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(
        r#"
        fn noop(ctx,value){}
        fn view(ctx){row([text("Action")]).with_style(style().width(px(80)).height(px(40))
            .margin(px(20)).focus(style().focus_ring(px(2),px(2),rgba(0xff56aa80)).opacity(0.5)))
            .accessibility_role("button").accessibility_label("Action").on_click(Fn("noop"))}
    "#,
    )
    .motion(MotionPreference::None)
    .id("focused-ring-property")
    .mount(cx);
    visual.update(|window, cx| view.focus(window, cx).unwrap());
    visual.simulate_keystrokes("tab");
    settle(&mut visual, Step::Frame, 2);
    assert!(
        painted_quads(&mut visual)
            .iter()
            .any(|q| q.border_widths == [2.0; 4]
                && (q.bounds.width - 88.0).abs() < 0.5
                && (q.bounds.height - 48.0).abs() < 0.5
                && (q.border_color.a - (128.0 / 255.0) * 0.5).abs() < 0.002),
        "a focus-only ring must not be silently ignored"
    );
}

#[gpui::test]
fn native_slider_parts_follow_the_active_thumb_and_keep_keyboard_commits(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let overrides = ThemeTokenOverrides {
            colors: [("focus_ring".to_owned(), Rgba8::from_rgba_hex(COLOR))].into(),
            ..Default::default()
        };
        let (mut visual,view)=Mount::components(r#"
            import "components/slider" as slider;
            import "components/range_slider" as range;
            fn state_schema(){#{fields:#{
                volume:#{schema:#{type:"number"},"default":#{type:"integer",value:50}},
                low:#{schema:#{type:"number"},"default":#{type:"integer",value:20}},
                high:#{schema:#{type:"number"},"default":#{type:"integer",value:80}},
                changes:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
            }}}
            fn volume(ctx,value){ctx.set_state("volume",value);ctx.set_state("changes",ctx.get_state("changes")+1);}
            fn interval(ctx,value){ctx.set_state("low",value.low);ctx.set_state("high",value.high);ctx.set_state("changes",ctx.get_state("changes")+1);}
            fn view(ctx){column([
                slider::Slider(#{key:"volume",label:"Volume",value:ctx.get_state("volume"),on_change:Fn("volume")}),
                range::RangeSlider(#{key:"interval",label:"Interval",low_label:"Low",high_label:"High",
                    values:#{low:ctx.get_state("low"),high:ctx.get_state("high")},on_change:Fn("interval")}),
                text(`${ctx.get_state("changes")}`).accessibility_role("status")
            ]).with_style(style().padding(px(20)).gap(px(12)))}
        "#).theme(theme).motion(MotionPreference::None)
            .with(move|view|view.theme_token_overrides(overrides)).id("native-part-focus").mount(cx);
        visual.update(|window, cx| view.focus(window, cx).unwrap());
        let mut centers = Vec::new();
        for index in 0..3 {
            visual.simulate_keystrokes("tab");
            settle(&mut visual, Step::Frame, 2);
            let color: gpui::Hsla = rgba(COLOR).into();
            let rings = painted_quads(&mut visual)
                .into_iter()
                .filter(|q| {
                    q.border_widths == [2.0; 4] && (q.border_color.h - color.h).abs() < 0.001
                })
                .collect::<Vec<_>>();
            let expected = if name.starts_with("clean") {
                20.0
            } else {
                22.0
            };
            assert!(
                !rings.is_empty()
                    && rings.iter().all(|q| (q.bounds.width - expected).abs() < 0.5
                        && (q.bounds.height - expected).abs() < 0.5),
                "{name}/{index}: one focused native thumb, not the whole control: {rings:?}"
            );
            centers.push(rings[0].bounds.x + rings[0].bounds.width / 2.0);
            visual.simulate_keystrokes("right");
            settle(&mut visual, Step::Frame, 2);
            bounds(&mut visual, &view, "status", &(index + 1).to_string());
            assert!(last_error(&mut visual, &view).is_none());
        }
        assert!(
            centers[1] < centers[0] && centers[0] < centers[2],
            "{name}: focus follows low / high native owners: {centers:?}"
        );
    }
}

#[gpui::test]
fn compound_controls_outline_the_visual_mark_not_their_label_row(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let overrides = ThemeTokenOverrides {
            colors: [("focus_ring".to_owned(), Rgba8::from_rgba_hex(COLOR))].into(),
            ..Default::default()
        };
        let (mut visual,view)=Mount::components(r#"
            import "components/checkbox" as checkbox;
            import "components/switch" as sw;
            fn state_schema(){#{fields:#{changes:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
            fn changed(ctx,value){ctx.set_state("changes",ctx.get_state("changes")+1);}
            fn view(ctx){column([
                checkbox::Checkbox(#{checked:true,label:"Remember",on_change:Fn("changed")}),
                sw::Switch(#{checked:true,label:"Enabled",on_change:Fn("changed")}),
                text(`${ctx.get_state("changes")}`).accessibility_role("status")
            ]).with_style(style().padding(px(20)).gap(px(12)))}
        "#).theme(theme).motion(MotionPreference::None)
            .with(move|view|view.theme_token_overrides(overrides)).id("compound-focus-ring").mount(cx);
        settle(&mut visual, Step::Frame, 2);
        visual.update(|window, cx| view.focus(window, cx).unwrap());
        let rings = |visual: &mut gpui::VisualTestContext| {
            let color: gpui::Hsla = rgba(COLOR).into();
            painted_quads(visual)
                .into_iter()
                .filter(|q| {
                    q.border_widths == [2.0; 4] && (q.border_color.h - color.h).abs() < 0.001
                })
                .collect::<Vec<_>>()
        };
        visual.simulate_keystrokes("tab");
        settle(&mut visual, Step::Frame, 2);
        let checkbox = bounds(&mut visual, &view, "checkbox", "Remember");
        assert!(
            (checkbox.height - 20.0).abs() < 0.5,
            "{name}: checkbox row follows its first line, not field height"
        );
        let marks = rings(&mut visual);
        assert!(
            marks.iter().any(
                |q| (q.bounds.width - 24.0).abs() < 0.5 && (q.bounds.height - 24.0).abs() < 0.5
            ),
            "{name}: box-only focus ring: {marks:?}"
        );
        assert!(
            marks
                .iter()
                .all(|q| f64::from(q.bounds.width) < checkbox.width),
            "not a label-row frame"
        );
        visual.simulate_keystrokes("space");
        settle(&mut visual, Step::Frame, 2);
        bounds(&mut visual, &view, "status", "1");
        visual.simulate_keystrokes("tab");
        settle(&mut visual, Step::Frame, 2);
        let marks = rings(&mut visual);
        assert!(
            marks.iter().any(
                |q| (q.bounds.width - 40.0).abs() < 0.5 && (q.bounds.height - 28.0).abs() < 0.5
            ),
            "{name}: track-only focus ring: {marks:?}"
        );
        assert!(
            marks.iter().all(|q| (q.bounds.width - 24.0).abs() > 0.5),
            "old owner must lose its ring"
        );
        let switch = bounds(&mut visual, &view, "switch", "Enabled");
        assert!(
            (switch.height - 20.0).abs() < 0.5,
            "{name}: switch first-line geometry"
        );
        click_center(&mut visual, switch);
        settle(&mut visual, Step::Frame, 2);
        assert!(
            rings(&mut visual).is_empty(),
            "pointer input hides navigation-only indicators"
        );
        bounds(&mut visual, &view, "status", "2");
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn preset_buttons_draw_exterior_focus_rings_without_a_layout_or_hitbox_border(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for policy in [
            FocusIndicators::Navigation,
            FocusIndicators::Always,
            FocusIndicators::Never,
        ] {
            let overrides = ThemeTokenOverrides {
                colors: [("focus_ring".to_owned(), Rgba8::from_rgba_hex(COLOR))].into(),
                ..Default::default()
            };
            let (mut visual, view) = Mount::components(
                r#"
                import "components/button" as button;
                fn noop(ctx,payload){}
                fn view(ctx){column([button::Button(#{text:"Action",on_click:Fn("noop")})
                    .with_style(style().opacity(0.5).translate_x(11).translate_y(7))])
                    .with_style(style().padding(px(20)).opacity(0.5))}
            "#,
            )
            .theme(theme)
            .motion(MotionPreference::None)
            .with(move |view| view.theme_token_overrides(overrides))
            .config(move |config| config.focus_indicators(policy))
            .id("preset-focus-ring")
            .mount(cx);
            settle(&mut visual, Step::Frame, 2);
            visual.update(|window, cx| view.focus(window, cx).unwrap());
            visual.simulate_keystrokes("tab");
            settle(&mut visual, Step::Frame, 3);
            let before = bounds(&mut visual, &view, "button", "Action");
            let check = |visual: &mut gpui::VisualTestContext, expected: bool| {
                let reference: gpui::Hsla = rgba(COLOR).into();
                let quads = painted_quads(visual);
                assert!(
                    !quads.iter().any(|quad| quad.border_widths == [1.0; 4]
                        && (quad.border_color.h - reference.h).abs() < 0.001
                        && (f64::from(quad.bounds.width) - before.width).abs() < 0.5),
                    "an exterior ring must not also recolor an unspecified inner border"
                );
                let ring = quads.into_iter().find(|quad| {
                    (quad.border_color.h - reference.h).abs() < 0.001
                        && quad.border_widths == [2.0; 4]
                        && (f64::from(quad.bounds.width) - before.width - 8.0).abs() < 0.5
                });
                assert_eq!(ring.is_some(), expected, "{name}/{policy:?}");
                if let Some(ring) = ring {
                    assert!(
                        (f64::from(ring.bounds.x) - before.x + 4.0).abs() < 0.5
                            && (f64::from(ring.bounds.y) - before.y + 4.0).abs() < 0.5,
                        "ring follows translated border box: {ring:?}, {before:?}"
                    );
                    assert!(
                        ring.background == gpui::transparent_black().into(),
                        "offset gap must not be painted"
                    );
                    assert!(
                        (ring.border_color.a - reference.a * 0.25).abs() < 0.002,
                        "parent and own opacity each apply once: {ring:?}"
                    );
                    assert!(
                        ring.corner_radii.iter().all(|&radius| radius > 4.0),
                        "ring follows rounded corners"
                    );
                }
            };
            check(&mut visual, policy != FocusIndicators::Never);
            click_center(&mut visual, before);
            settle(&mut visual, Step::Frame, 3);
            let after = bounds(&mut visual, &view, "button", "Action");
            assert_eq!(
                before, after,
                "focus decoration must never reflow or enlarge the hit bounds"
            );
            check(&mut visual, policy == FocusIndicators::Always);
            assert!(last_error(&mut visual, &view).is_none());
        }
    }
}
