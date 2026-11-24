//! Retained editors measure, hit-test and draw the same live public typography.

use crate::support::{
    Mount, Step, bounds, by_test_id, click_center, dispatch, last_error, painted_quads, settle,
    text_of,
};
use gpui::{ClipboardItem, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{MotionPreference, ScriptViewHandle};

fn caret(visual: &mut VisualTestContext) -> crate::support::Rect {
    painted_quads(visual)
        .into_iter()
        .find(|quad| (quad.bounds.width - 1.5).abs() < 0.01 && quad.background.as_solid().is_some())
        .expect("the active editor must draw its native caret")
        .bounds
}

fn change_type(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    dispatch(visual, view, by_test_id("type"), "click").unwrap();
    settle(visual, Step::Frame, 4);
}

#[gpui::test]
fn textarea_paste_reveals_the_caret_using_the_new_scroll_extent(cx: &mut TestAppContext) {
    use crate::support::{center, live_fills};
    for (limits, floor, cap) in [
        ("min_rows:1,max_rows:2", 20., 40.),
        (
            "min_rows:1,max_rows:8,min_viewport_height:px(31),max_viewport_height:px(46)",
            31.,
            46.,
        ),
    ] {
        let script = r#"
        import "components/textarea" as textarea;
        fn state_schema(){#{fields:#{text:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}}
        fn edit(ctx,value){ctx.set_state("text",value);}
        fn view(ctx){column([textarea::Textarea(#{key:"draft",label:"Draft",value:ctx.get_state("text"),
            __LIMITS__,on_change:Fn("edit"),part_styles:#{
                editor:style().padding(px(0)).line_height(px(20)),
                caret:style().background(rgb(0xc31725))
            }})]).with_style(style().padding(px(20)).width(px(300)))}
    "#.replace("__LIMITS__", limits);
        let (mut v, view) = Mount::components(&script)
            .motion(MotionPreference::None)
            .mount(cx);
        v.update(|window, _| window.activate_window());
        settle(&mut v, Step::Frame, 5);
        let rect = bounds(&mut v, &view, "text_field", "Draft");
        assert!((rect.height - floor).abs() <= 0.5, "{limits}: {rect:?}");
        click_center(&mut v, rect);
        let focus = v.update(|window, cx| window.focused(cx)).unwrap();
        let measured_carets = |v: &mut VisualTestContext| live_fills(v, 0xc31725ff);
        for lines in [5, 15] {
            let text = (1..=lines)
                .map(|n| format!("line {n}"))
                .collect::<Vec<_>>()
                .join("\n");
            v.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string(text)));
            v.simulate_keystrokes("cmd-a cmd-v");
            settle(&mut v, Step::Frame, 6);
            let rect = bounds(&mut v, &view, "text_field", "Draft");
            assert!((rect.height - cap).abs() <= 0.5, "{limits}: {rect:?}");
            let carets = measured_carets(&mut v);
            assert!(
                carets.iter().any(|cursor| {
                    (f64::from(cursor.y + cursor.height) - rect.y - rect.height).abs() <= 1.
                        && cursor.height >= 19.
                }),
                "{lines} lines: tail caret must be fully visible: {rect:?} {carets:?}"
            );
        }
        let rect = bounds(&mut v, &view, "text_field", "Draft");
        v.simulate_event(ScrollEvent {
            position: center(rect),
            delta: ScrollDelta::Pixels(point(px(0.), px(1000.))),
            ..Default::default()
        });
        settle(&mut v, Step::Frame, 6);
        assert!(
            measured_carets(&mut v).is_empty(),
            "manual scroll must not snap back"
        );
        v.simulate_keystrokes("right");
        settle(&mut v, Step::Frame, 5);
        assert!(
            !measured_carets(&mut v).is_empty(),
            "the next caret action reveals it"
        );
        assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
        assert_eq!(last_error(&mut v, &view), None);
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn isolated_tracking_keeps_retained_editor_caret_and_pointer_geometry(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut actual_fonts = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("isolated_tracking"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let cx = &mut actual_fonts;
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for area in [false, true] {
            let script = r#"
                import "components/input" as input;
                import "components/textarea" as textarea;
                fn state_schema(){#{fields:#{
                    value:#{schema:#{type:"string"},"default":#{type:"string",value:"MMMM"}},
                    spaced:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
                }}}
                fn changed(ctx,value){ctx.set_state("value",value);}
                fn toggle(ctx,p){ctx.set_state("spaced",!ctx.get_state("spaced"));}
                fn view(ctx){
                    let editor=style().padding(px(0)).height(px(100)).min_height(px(0))
                        .font_family("Inter").font_weight(400).font_size(px(14)).line_height(px(20))
                        .letter_spacing_em(if ctx.get_state("spaced"){0.5}else{0.0});
                    let field=if __AREA__ {
                        textarea::Textarea(#{key:"field",label:"Field",value:ctx.get_state("value"),
                            rows:2,min_rows:2,max_rows:2,on_change:Fn("changed"),
                            style:style().width(px(300)),part_styles:#{editor:editor}})
                    } else {
                        input::Input(#{key:"field",label:"Field",value:ctx.get_state("value"),
                            on_change:Fn("changed"),style:editor.width(px(300))})
                    };
                    column([field,text("Track").test_id("type").on_click(Fn("toggle")),
                        text(ctx.get_state("value")).test_id("value")])
                        .with_style(style().padding(px(20)).gap(px(12)))
                }
            "#
            .replace("__AREA__", if area { "true" } else { "false" });
            let (mut visual, view) = Mount::components(script)
                .theme(theme)
                .motion(MotionPreference::None)
                .mount(cx);
            visual.update(|window, _| window.activate_window());
            settle(&mut visual, Step::Frame, 4);
            let field = bounds(&mut visual, &view, "text_field", "Field");
            click_center(&mut visual, field);
            visual.simulate_keystrokes("home");
            settle(&mut visual, Step::Frame, 3);
            let start = caret(&mut visual);
            visual.simulate_keystrokes("end");
            settle(&mut visual, Step::Frame, 3);
            let before = caret(&mut visual);
            let owner = visual.update(|window, cx| window.focused(cx)).unwrap();
            change_type(&mut visual, &view);
            let after = caret(&mut visual);
            let expected_advance = if gpui_rhai::EXTENDED_RENDERING {
                28.
            } else {
                0.
            };
            assert!(
                (after.x - before.x - expected_advance).abs() < 0.02,
                "{name}/area={area}: expected tracking advance {expected_advance}; start={start:?}, before={before:?}, after={after:?}"
            );
            assert_eq!(
                visual.update(|window, cx| window.focused(cx)),
                Some(owner.clone())
            );
            let hit = point(px(start.x + (after.x - start.x) / 2.), px(after.y + 10.));
            visual.simulate_mouse_down(hit, gpui::MouseButton::Left, gpui::Modifiers::none());
            visual.simulate_mouse_up(hit, gpui::MouseButton::Left, gpui::Modifiers::none());
            visual.simulate_input("Z");
            settle(&mut visual, Step::Frame, 4);
            assert_eq!(
                text_of(&mut visual, &view, "value"),
                "MMZMM",
                "{name}/area={area}"
            );
            change_type(&mut visual, &view);
            assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(owner));
            assert_eq!(last_error(&mut visual, &view), None);
        }
    }
}

#[gpui::test]
fn native_visual_defaults_do_not_enlarge_every_recursive_render_frame(cx: &mut TestAppContext) {
    for depth in [12, 24] {
        let script = r#"
            import "components/input" as input;
            fn state_schema(){#{fields:#{value:#{schema:#{type:"string"},
                "default":#{type:"string",value:"Retained"}}}}}
            fn changed(ctx,value){ctx.set_state("value",value);}
            fn view(ctx){
                let child=input::Input(#{key:"field",label:"Nested field",value:ctx.get_state("value"),
                    on_change:Fn("changed"),style:style().width(px(240)).font_size(px(18))});
                for level in 0..__DEPTH__ {child=column([child]);}
                column([child,text(ctx.get_state("value")).test_id("value")])
            }
        "#.replace("__DEPTH__", &depth.to_string());
        let (mut visual, view) = Mount::components(script)
            .motion(MotionPreference::None)
            .mount(cx);
        visual.update(|window, _| window.activate_window());
        settle(&mut visual, Step::Frame, 4);
        let field = bounds(&mut visual, &view, "text_field", "Nested field");
        assert!((field.width - 240.0).abs() < 0.5);
        click_center(&mut visual, field);
        visual.simulate_keystrokes("cmd-a");
        visual.simulate_input("Edited");
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(text_of(&mut visual, &view, "value"), "Edited");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn retained_editor_caret_and_pointer_hit_follow_live_type_without_remount(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for area in [false, true] {
            let script = r#"
                import "components/input" as input;
                import "components/textarea" as textarea;
                fn state_schema(){#{fields:#{
                    value:#{schema:#{type:"string"},"default":#{type:"string",value:"MMMM"}},
                    large:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
                }}}
                fn changed(ctx,value){ctx.set_state("value",value);}
                fn type_changed(ctx,p){ctx.set_state("large",!ctx.get_state("large"));}
                fn view(ctx){
                    let large=ctx.get_state("large");
                    let editor=style().padding(px(0)).min_height(px(0)).height(px(100))
                        .font_family("Inter").font_weight(400)
                        .font_size(px(if large{20}else{13}))
                        .line_height(px(if large{32}else{20}));
                    let field=if __AREA__ {
                        textarea::Textarea(#{key:"field",label:"Field",value:ctx.get_state("value"),
                            rows:2,min_rows:2,max_rows:2,on_change:Fn("changed"),
                            style:style().width(px(300)),part_styles:#{editor:editor}})
                    } else {
                        input::Input(#{key:"field",label:"Field",value:ctx.get_state("value"),
                            on_change:Fn("changed"),style:editor.width(px(300))})
                    };
                    column([field,text("Type").test_id("type").on_click(Fn("type_changed")),
                        text(ctx.get_state("value")).test_id("value")])
                        .with_style(style().padding(px(20)).gap(px(12)))
                }
            "#
            .replace("__AREA__", if area { "true" } else { "false" });
            let (mut visual, view) = Mount::components(script)
                .theme(theme)
                .motion(MotionPreference::None)
                .mount(cx);
            visual.update(|window, _| window.activate_window());
            settle(&mut visual, Step::Frame, 4);
            let field = bounds(&mut visual, &view, "text_field", "Field");
            click_center(&mut visual, field);
            visual.simulate_keystrokes("home");
            settle(&mut visual, Step::Frame, 3);
            let start = caret(&mut visual);
            visual.simulate_keystrokes("end");
            settle(&mut visual, Step::Frame, 3);
            let regular = caret(&mut visual);
            assert!((regular.height - 20.0).abs() < 0.01, "{name}/area={area}");
            let regular_advance = regular.x - start.x;
            assert!(regular_advance > 30.0);
            let owner = visual.update(|window, cx| window.focused(cx)).unwrap();
            change_type(&mut visual, &view);
            assert_eq!(
                visual.update(|window, cx| window.focused(cx)),
                Some(owner.clone())
            );
            let large = caret(&mut visual);
            assert!(
                (large.height - 32.0).abs() < 0.01,
                "{name}/area={area}: {large:?}"
            );
            assert!(large.x - start.x > regular_advance * 1.4);
            // Hit the middle of the *presented* four-M advance. Stale shaping
            // would insert after the third glyph rather than the second.
            let hit = point(px(start.x + (large.x - start.x) / 2.0), px(large.y + 16.0));
            visual.simulate_mouse_down(hit, gpui::MouseButton::Left, gpui::Modifiers::none());
            visual.simulate_mouse_up(hit, gpui::MouseButton::Left, gpui::Modifiers::none());
            visual.simulate_input("Z");
            settle(&mut visual, Step::Frame, 4);
            assert_eq!(
                text_of(&mut visual, &view, "value"),
                "MMZMM",
                "{name}/area={area}"
            );
            change_type(&mut visual, &view);
            assert!((caret(&mut visual).height - 20.0).abs() < 0.01);
            assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(owner));
            assert_eq!(last_error(&mut visual, &view), None);
            assert!((field.width - 300.0).abs() < 0.5);
        }
    }
}
