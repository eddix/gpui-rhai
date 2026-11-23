//! Part cascade reaches native paint; temporal wrappers reuse the real editor.

use crate::support::{
    Mount, Step, bounds, by_test_id, center, click_center, dispatch, last_error, painted_quads,
    settle, text_of,
};
use gpui::{Modifiers, TestAppContext, point, px, rgba};
use gpui_rhai::{MotionPreference, ScriptViewHandle, UiValue};

#[gpui::test]
fn input_intrinsic_width_tracks_text_but_obeys_its_flex_allocation(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = Mount::components(r#"
            import "components/input" as input;
            fn state_schema(){#{fields:#{value:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}}
            fn changed(ctx,value){ctx.set_state("value",value);}
            fn view(ctx){column([row([
                input::Input(#{key:"fit",label:"Fit",value:ctx.get_state("value"),
                    placeholder:"Reference",intrinsic_width:true,on_change:Fn("changed")})
                    .with_style(style().min_width(px(0)).max_width(px(160))),
                text("Sibling").with_style(style().width(px(70)).flex_shrink(false))
            ]).with_style(style().width(px(200)).gap(px(8))),
            input::Input(#{key:"fixed",label:"Fixed fit",value:"X",intrinsic_width:true})
                .with_style(style().width(px(180)))]).with_style(style().items_start())}
        "#).theme(theme).motion(MotionPreference::None).mount(cx);
        visual.update(|window, _| window.activate_window());
        settle(&mut visual, Step::Refresh, 3);
        let placeholder = bounds(&mut visual, &view, "text_field", "Fit");
        click_center(&mut visual, placeholder);
        let focus = visual.update(|window, cx| window.focused(cx)).unwrap();
        visual.simulate_input("Q");
        settle(&mut visual, Step::Refresh, 3);
        let short = bounds(&mut visual, &view, "text_field", "Fit");
        assert!(
            short.width + 20. < placeholder.width,
            "{name}: {short:?} {placeholder:?}"
        );
        visual.simulate_input("extra wide field text to reach the maximum");
        settle(&mut visual, Step::Refresh, 3);
        let grown = bounds(&mut visual, &view, "text_field", "Fit");
        let sibling = bounds(&mut visual, &view, "text", "Sibling");
        assert!(
            grown.width > short.width + 60. && grown.width <= 122.5,
            "{name}: {grown:?} must fit beside {sibling:?}"
        );
        assert!((grown.x + grown.width + 8. - sibling.x).abs() <= 1.);
        assert_eq!(
            visual.update(|window, cx| window.focused(cx)),
            Some(focus.clone())
        );
        visual.simulate_keystrokes("cmd-a");
        visual.simulate_input("Z");
        settle(&mut visual, Step::Refresh, 3);
        assert!((bounds(&mut visual, &view, "text_field", "Fit").width - short.width).abs() <= 1.);
        assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(focus));
        let tree = crate::support::snapshot(&mut visual, &view);
        assert_eq!(
            tree.find_by_role_and_name("text_field", "Fit")
                .next()
                .unwrap()
                .value,
            Some(UiValue::String("Z".into()))
        );
        let fixed = bounds(&mut visual, &view, "text_field", "Fixed fit");
        assert!((fixed.width - 180.).abs() <= 0.5, "{name}: {fixed:?}");
        visual.simulate_click(
            point(
                px((fixed.x + fixed.width - 5.) as f32),
                px((fixed.y + fixed.height / 2.) as f32),
            ),
            Modifiers::none(),
        );
        settle(&mut visual, Step::Refresh, 3);
        let expected = crate::support::snapshot(&mut visual, &view)
            .find_by_role_and_name("text_field", "Fixed fit")
            .next()
            .unwrap()
            .id;
        let focus =
            visual.update(|window, cx| view.debug_frame_digest(window, cx).unwrap().focus_path);
        assert_eq!(
            focus.first(),
            Some(&expected),
            "{name}: the entire fixed field accepts clicks"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn input_and_time_input_have_stable_intrinsic_width_and_still_fill_a_column(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual,view)=Mount::components(r#"
            import "components/input" as input;
            import "components/time_input" as time_input;
            fn state_schema(){#{fields:#{long:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
            fn longer(ctx,p){ctx.set_state("long",true);}
            fn view(ctx){column([
                row([input::Input(#{key:"plain",label:"Intrinsic",value:if ctx.get_state("long"){ "A much longer draft than twenty characters" }else{""},placeholder:"Draft"}),
                    time_input::TimeInput(#{key:"clock",label:"Clock",value:"11:59 PM"})]),
                column([input::Input(#{key:"fill",label:"Fill",value:"Draft"})]).with_style(style().width(px(600))),
                row([input::Input(#{key:"narrow",label:"Narrow",value:"Draft"})]).with_style(style().width(px(100))),
                text("Longer").test_id("longer").on_click(Fn("longer"))
            ]).with_style(style().width(px(700)).items_start())}
        "#).theme(theme).motion(MotionPreference::None).mount(cx);
        settle(&mut visual, Step::Frame, 3);
        let intrinsic = bounds(&mut visual, &view, "text_field", "Intrinsic");
        let clock = bounds(&mut visual, &view, "text_field", "Clock");
        assert!(
            intrinsic.width > 120.0 && clock.width > 120.0,
            "{name}: {intrinsic:?} {clock:?}"
        );
        assert!((intrinsic.width - clock.width).abs() < 0.5);
        assert!((bounds(&mut visual, &view, "text_field", "Fill").width - 600.0).abs() < 0.5);
        assert!(bounds(&mut visual, &view, "text_field", "Narrow").width <= 100.5);
        dispatch(&mut visual, &view, by_test_id("longer"), "click").unwrap();
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(
            bounds(&mut visual, &view, "text_field", "Intrinsic"),
            intrinsic,
            "typing must not resize the field"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

fn has_color(visual: &mut gpui::VisualTestContext, color: u32) {
    let color: gpui::Background = rgba(color).into();
    assert!(
        painted_quads(visual)
            .iter()
            .any(|quad| quad.background == color),
        "missing native fill {color:?}"
    );
}

#[gpui::test]
fn input_part_overrides_reach_hover_focus_and_incremental_controlled_updates(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual,view)=Mount::components(r#"
            import "components/input" as input;
            fn state_schema(){#{fields:#{
                value:#{schema:#{type:"string"},"default":#{type:"string",value:"Draft"}},
                invalid:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
            }}}
            fn changed(ctx,value){ctx.set_state("value",value);}
            fn invalid(ctx,payload){ctx.set_state("invalid",true);}
            fn view(ctx){column([
                input::Input(#{key:"field",label:"Field",value:ctx.get_state("value"),
                    error:ctx.get_state("invalid"),on_change:Fn("changed"),style:style().width(px(220))}),
                text("Invalid").test_id("invalid").on_click(Fn("invalid")),
                text(ctx.get_state("value")).test_id("value")
            ]).with_style(style().padding(px(20)).gap(px(16)))}
        "#).theme(theme).motion(MotionPreference::None)
            .with(|view|view.component_styles(r#"fn component_styles(){#{"components/input":#{
                input:style().background(rgb(0x00ff00)).radius(px(7))
                    .hover(style().background(rgb(0x0000ff)))
                    .focus(style().background(rgb(0xff00ff)))
            }}}"#)).mount(cx);
        visual.update(|window, _| window.activate_window());
        settle(&mut visual, Step::Frame, 3);
        has_color(&mut visual, 0x00ff00ff);
        let field = bounds(&mut visual, &view, "text_field", "Field");
        visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        visual.simulate_mouse_move(center(field), None, Modifiers::none());
        settle(&mut visual, Step::Frame, 3);
        has_color(&mut visual, 0x0000ffff);
        assert!(
            visual
                .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
                .timings
                .is_empty(),
            "{name}: hover must be native"
        );
        click_center(&mut visual, field);
        settle(&mut visual, Step::Frame, 3);
        has_color(&mut visual, 0xff00ffff);
        let owner = visual.update(|window, cx| window.focused(cx)).unwrap();
        dispatch(&mut visual, &view, by_test_id("invalid"), "click").unwrap();
        settle(&mut visual, Step::Frame, 3);
        has_color(&mut visual, 0xff00ffff);
        assert_eq!(
            visual.update(|window, cx| window.focused(cx)),
            Some(owner.clone())
        );
        visual.simulate_keystrokes("cmd-a");
        visual.simulate_input("Edited");
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(text_of(&mut visual, &view, "value"), "Edited");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

fn compare_field_surfaces(visual: &mut gpui::VisualTestContext, view: &ScriptViewHandle) {
    let plain = bounds(visual, view, "text_field", "Plain");
    let clock = bounds(visual, view, "text_field", "Clock");
    assert!((plain.width - clock.width).abs() < 0.5 && (plain.height - clock.height).abs() < 0.5);
    let quads = painted_quads(visual);
    let surface = |area: gpui_rhai::GeometryBounds| {
        quads
            .iter()
            .find(|quad| {
                (f64::from(quad.bounds.x) - area.x).abs() < 0.5
                    && (f64::from(quad.bounds.y) - area.y).abs() < 0.5
                    && (f64::from(quad.bounds.width) - area.width).abs() < 0.5
                    && (f64::from(quad.bounds.height) - area.height).abs() < 0.5
                    && quad
                        .background
                        .as_solid()
                        .is_some_and(|color| color.a > 0.0)
            })
            .unwrap()
    };
    let a = surface(plain);
    let b = surface(clock);
    assert_eq!(a.background, b.background);
    assert_eq!(
        a.border_widths, [0.0; 4],
        "native field edges are shadows, not layout borders"
    );
    assert_eq!(a.border_widths, b.border_widths);
    assert_eq!(a.corner_radii, b.corner_radii);
}

#[gpui::test]
fn time_input_keeps_native_focus_geometry_and_callbacks_across_all_designs(
    cx: &mut TestAppContext,
) {
    let (mut visual,view)=Mount::components(r#"
        import "components/input" as input;
        import "components/time_input" as time_input;
        fn state_schema(){#{fields:#{value:#{schema:#{type:"string"},"default":#{type:"string",value:"11:59 PM"}}}}}
        fn changed(ctx,value){ctx.set_state("value",value);}
        fn view(ctx){column([
            input::Input(#{key:"plain",label:"Plain",value:"11:59 PM",style:style().width(px(220))}),
            time_input::TimeInput(#{key:"clock",label:"Clock",value:ctx.get_state("value"),on_change:Fn("changed"),style:style().width(px(220))}),
            text(ctx.get_state("value")).test_id("value")
        ]).with_style(style().padding(px(20)).gap(px(16)))}
    "#).theme(gpui_rhai_registry::MODERN_LIGHT_THEME).locale("en.rhai",gpui_rhai_registry::EN_LOCALE)
        .motion(MotionPreference::None).with(|v|v.theme_sources(
            gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES.iter().filter(|(name,_)|*name!="modern_light.rhai")
                .map(|&(name,source)|(name.to_owned(),source.to_owned())))).mount(cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 3);
    let clock = bounds(&mut visual, &view, "text_field", "Clock");
    click_center(&mut visual, clock);
    settle(&mut visual, Step::Frame, 3);
    let owner = visual.update(|window, cx| window.focused(cx)).unwrap();
    for (family, appearance) in [
        ("Modern", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Clean", "Dark"),
        ("Modern", "Light"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Frame, 3);
        compare_field_surfaces(&mut visual, &view);
        assert_eq!(
            visual.update(|window, cx| window.focused(cx)),
            Some(owner.clone())
        );
        visual.simulate_keystrokes("up");
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(text_of(&mut visual, &view, "value"), "12:00 AM");
        visual.simulate_keystrokes("down");
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(text_of(&mut visual, &view, "value"), "11:59 PM");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}
