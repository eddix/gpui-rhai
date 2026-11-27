//! Reading scale affects painted parts as well as their surrounding text.

use gpui::{Modifiers, TestAppContext, point, px, size};
use gpui_rhai::{MotionPreference, ThemeTokenOverrides};

use crate::support::{
    Mount, Step, bounds, center, click_center, filled, last_error, painted_quads, settle, snapshot,
};

#[gpui::test]
fn derived_radio_group_keeps_one_mark_focus_owner_and_skips_disabled_options(
    cx: &mut TestAppContext,
) {
    const RING: u32 = 0x00ff_00ff;
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 200] {
            let factor = f64::from(scale) / 100.0;
            let overrides = ThemeTokenOverrides {
                colors: [(
                    "focus_ring".to_owned(),
                    gpui_rhai::Rgba8::from_rgba_hex(RING),
                )]
                .into(),
                environment_defaults: [("scale".to_owned(), scale.to_string())].into(),
                ..Default::default()
            };
            let (mut visual, view) = Mount::components(r#"
                import "components/radio_group" as radio_group;
                fn state_schema(){#{fields:#{
                    value:#{schema:#{type:"string"},"default":#{type:"string",value:"c"}},
                    changes:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
                }}}
                fn changed(ctx,value){ctx.set_state("value",value);ctx.set_state("changes",ctx.get_state("changes")+1);}
                fn view(ctx){column([
                    radio_group::RadioGroup(#{label:"Plan",value:ctx.get_state("value"),on_change:Fn("changed"),options:[
                        #{value:"a",label:"First option with a long wrapping explanation"},
                        #{value:"b",label:"Disabled option",disabled:true},
                        #{value:"c",label:"Last option"}
                    ]}).with_style(style().width(px(180))),
                    text(`${ctx.get_state("value")}/${ctx.get_state("changes")}`).accessibility_role("status")
                ]).with_style(style().padding(px(20)).gap(px(12)))}
            "#).theme(theme).motion(MotionPreference::None)
                .with(move |view|view.theme_token_overrides(overrides))
                .config(|config|config.focus_indicators(gpui_rhai::FocusIndicators::Always))
                .id("radio-owner").mount(cx);
            settle(&mut visual, Step::Frame, 2);
            visual.update(|window, cx| view.focus(window, cx)).unwrap();
            visual.simulate_keystrokes("tab");
            settle(&mut visual, Step::Frame, 2);
            let selected = bounds(&mut visual, &view, "radio", "Last option");
            let ring: gpui::Hsla = gpui::rgba(RING).into();
            let quads = painted_quads(&mut visual)
                .into_iter()
                .filter(|quad| quad.border_color == ring && quad.has_border())
                .collect::<Vec<_>>();
            assert!(
                !quads.is_empty(),
                "{name}/{scale}: selected mark must show group focus"
            );
            assert!(
                quads.iter().all(
                    |quad| (f64::from(quad.bounds.x) - selected.x + 4.0).abs() < 0.5
                        && (f64::from(quad.bounds.y) - selected.y - 2.0 * factor + 4.0).abs() < 0.5
                        && (f64::from(quad.bounds.width) - 16.0 * factor - 8.0).abs() < 0.5
                ),
                "{name}/{scale}: exactly the active mark owns focus, {quads:?}, {selected:?}"
            );
            visual.simulate_keystrokes("right");
            settle(&mut visual, Step::Frame, 2);
            bounds(&mut visual, &view, "status", "a/1");
            visual.simulate_keystrokes("right");
            settle(&mut visual, Step::Frame, 2);
            bounds(&mut visual, &view, "status", "c/2");
            assert!(last_error(&mut visual, &view).is_none());
        }
    }
}

#[gpui::test]
fn chip_radio_group_keeps_native_focus_and_skips_disabled_choices(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(r#"
        import "components/radio_group" as radios;
        fn state_schema(){#{fields:#{
            value:#{schema:#{type:"string"},"default":#{type:"string",value:"once"}}
        }}}
        fn choose(ctx,value){ctx.set_state("value",value);}
        fn view(ctx){
            radios::RadioGroup(#{label:"Duration",value:ctx.get_state("value"),appearance:"chip",orientation:"horizontal",
                on_change:Fn("choose"),options:[
                    #{value:"once",label:"Once"},#{value:"hour",label:"One hour",disabled:true},
                    #{value:"day",label:"One day"}
                ]})
        }
    "#)
    .theme(gpui_rhai_registry::CLEAN_LIGHT_THEME)
    .motion(MotionPreference::None)
    .mount(cx);
    settle(&mut visual, Step::Frame, 3);
    visual.update(|window, cx| view.focus(window, cx)).unwrap();
    visual.simulate_keystrokes("tab right");
    settle(&mut visual, Step::Frame, 3);
    let day = snapshot(&mut visual, &view)
        .find_by_role_and_name("radio", "One day")
        .next()
        .unwrap()
        .clone();
    assert_eq!(day.checked, Some(gpui_rhai::UiValue::Bool(true)));
    visual.simulate_keystrokes("right");
    settle(&mut visual, Step::Frame, 3);
    let once = snapshot(&mut visual, &view)
        .find_by_role_and_name("radio", "Once")
        .next()
        .unwrap()
        .clone();
    assert_eq!(once.checked, Some(gpui_rhai::UiValue::Bool(true)));
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn choice_marks_stay_on_the_first_line_of_wrapped_labels(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 200] {
            let factor = f64::from(scale) / 100.0;
            for choice in ["checkbox", "switch", "radio"] {
                let overrides = ThemeTokenOverrides {
                    environment_defaults: [("scale".to_owned(), scale.to_string())].into(),
                    ..Default::default()
                };
                let (widget, props) = match choice {
                    "checkbox" => (
                        "checkbox::Checkbox",
                        "checked:true,on_change:Fn(\"changed\")",
                    ),
                    "radio" => (
                        "radio::Radio",
                        "value:\"option\",selected:true,on_select:Fn(\"changed\")",
                    ),
                    _ => ("sw::Switch", "checked:true,on_change:Fn(\"changed\")"),
                };
                let source = format!(
                    r#"
                    import "components/checkbox" as checkbox;
                    import "components/switch" as sw;
                    import "components/radio" as radio;
                    fn state_schema(){{#{{fields:#{{changes:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}}}}}}}}
                    fn changed(ctx,value){{ctx.set_state("changes",ctx.get_state("changes")+1);}}
                    fn view(ctx){{column([
                        {widget}(#{{{props},label:"Enable this option for every region in the deployment."}})
                            .with_style(style().width(px(180))),
                        text(`${{ctx.get_state("changes")}}`).accessibility_role("status")
                    ]).with_style(style().padding(px(20)).gap(px(12)))}}
                "#
                );
                let (mut visual, view) = Mount::components(source)
                    .theme(theme)
                    .motion(MotionPreference::None)
                    .with(move |view| view.theme_token_overrides(overrides))
                    .id("choice-first-line")
                    .mount(cx);
                settle(&mut visual, Step::Frame, 2);
                let label = bounds(
                    &mut visual,
                    &view,
                    "text",
                    "Enable this option for every region in the deployment.",
                );
                let root = bounds(
                    &mut visual,
                    &view,
                    choice,
                    "Enable this option for every region in the deployment.",
                );
                assert!(
                    label.height > 20.0 * factor + 1.0,
                    "{name}/{scale}: label must actually wrap: {label:?}"
                );
                let color = visual.update(|_, cx| {
                    view.theme_snapshot(cx)
                        .unwrap()
                        .variant
                        .tokens
                        .color("choice.fill")
                        .unwrap()
                });
                let mark = choice != "switch";
                let width = if mark { 16.0 } else { 32.0 } * factor;
                let height = if mark { 16.0 } else { 20.0 } * factor;
                let quads = filled(&mut visual, color.as_rgba_hex());
                let painted_mark = quads
                    .iter()
                    .find(|q| {
                        (f64::from(q.bounds.width) - width).abs() < 0.5
                            && (f64::from(q.bounds.height) - height).abs() < 0.5
                    })
                    .expect("native mark/track");
                let expected_y = root.y + if mark { 2.0 * factor } else { 0.0 };
                assert!(
                    (f64::from(painted_mark.bounds.y) - expected_y).abs() < 0.5,
                    "{name}/{scale}/{choice}: mark follows the first line, not the full paragraph: {painted_mark:?}, {root:?}"
                );
                click_center(&mut visual, label);
                settle(&mut visual, Step::Frame, 2);
                bounds(&mut visual, &view, "status", "1");
                assert!(last_error(&mut visual, &view).is_none());
            }
        }
    }
}

#[gpui::test]
fn pending_switch_blocks_changes_without_receiving_disabled_opacity(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual,view)=Mount::components(r#"import "components/switch" as sw;
            fn state_schema(){#{fields:#{changes:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
            fn changed(ctx,value){ctx.set_state("changes",ctx.get_state("changes")+1);}
            fn view(ctx){column([
                sw::Switch(#{label:"Idle",checked:true,on_change:Fn("changed")}),
                sw::Switch(#{label:"Pending",checked:true,loading:true,on_change:Fn("changed")}),
                text(`${ctx.get_state("changes")}`).accessibility_role("status")
            ]).with_style(style().padding(px(20)).gap(px(12)))}"#)
            .theme(theme).motion(MotionPreference::None).id("pending-switch").mount(cx);
        settle(&mut visual, Step::Frame, 2);
        let color = visual.update(|_, cx| {
            view.theme_snapshot(cx)
                .unwrap()
                .variant
                .tokens
                .color("choice.fill")
                .unwrap()
        });
        let tracks = filled(&mut visual, color.as_rgba_hex())
            .iter()
            .filter(|q| (q.bounds.width - 32.0).abs() < 0.5 && (q.bounds.height - 20.0).abs() < 0.5)
            .count();
        assert!(
            tracks >= 2,
            "{name}: pending track must keep its normal opacity"
        );
        let pending = bounds(&mut visual, &view, "switch", "Pending");
        click_center(&mut visual, pending);
        settle(&mut visual, Step::Frame, 2);
        bounds(&mut visual, &view, "status", "0");
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn switch_track_hover_follows_the_whole_row_and_its_controlled_state(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for checked in [false, true] {
            let source=r#"import "components/switch" as sw;
                fn noop(ctx,value){}
                fn view(ctx){sw::Switch(#{label:"Service enabled",checked:CHECKED,on_change:Fn("noop")})
                    .with_style(style().margin(px(20)))}"#
                .replace("CHECKED",if checked{"true"}else{"false"});
            let (mut visual, view) = Mount::components(source)
                .theme(theme)
                .motion(MotionPreference::None)
                .id("switch-state-paint")
                .mount(cx);
            settle(&mut visual, Step::Frame, 2);
            let tokens = visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant.tokens);
            let idle = tokens
                .color(if checked { "choice.fill" } else { "switch.off" })
                .unwrap();
            let hover = tokens
                .color(if checked {
                    "switch.on_hover"
                } else {
                    "switch.off_hover"
                })
                .unwrap();
            assert_ne!(
                idle, hover,
                "{name}: observable checked/unchecked hover state"
            );
            let track = |visual: &mut gpui::VisualTestContext, color: gpui_rhai::Rgba8| {
                filled(visual, color.as_rgba_hex()).iter().any(|quad| {
                    (quad.bounds.width - 32.0).abs() < 0.5
                        && (quad.bounds.height - 20.0).abs() < 0.5
                })
            };
            assert!(track(&mut visual, idle), "{name}/{checked}: idle track");
            let root = bounds(&mut visual, &view, "switch", "Service enabled");
            let over_label = center(gpui_rhai::GeometryBounds {
                x: root.x + root.width - 4.0,
                width: 4.0,
                ..root
            });
            visual.simulate_mouse_move(over_label, None, Modifiers::none());
            settle(&mut visual, Step::Frame, 3);
            assert!(
                track(&mut visual, hover),
                "{name}/{checked}: label hover must reach the track"
            );
            visual.simulate_mouse_move(point(px(-10.0), px(-10.0)), None, Modifiers::none());
            settle(&mut visual, Step::Frame, 2);
            assert!(
                track(&mut visual, idle),
                "{name}/{checked}: hover must restore without changing value"
            );
            assert!(last_error(&mut visual, &view).is_none());
        }
    }
}

#[gpui::test]
fn live_badges_use_native_vector_transforms_in_every_design_and_motion_policy(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for preference in [
            MotionPreference::Normal,
            MotionPreference::Reduced,
            MotionPreference::None,
        ] {
            let (mut visual, view) = Mount::components(
                r#"
                import "components/badge" as badge;
                fn view(ctx){badge::Badge(#{text:"Running",variant:"success",live:true})}
            "#,
            )
            .theme(theme)
            .motion(preference)
            .id("live-preset-badge")
            .mount(cx);
            settle(&mut visual, Step::Frame, 4);
            let badge = bounds(&mut visual, &view, "status", "Running");
            assert!(badge.width > 0.0 && badge.height > 0.0);
            assert!(
                last_error(&mut visual, &view).is_none(),
                "{name}/{preference:?}"
            );
        }
    }
}

#[gpui::test]
fn preset_choices_calendar_and_slider_parts_follow_reading_scale(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 125, 200] {
            let factor = f64::from(scale) / 100.0;
            let overrides = ThemeTokenOverrides {
                environment_defaults: [("scale".to_owned(), scale.to_string())].into(),
                ..Default::default()
            };
            let (mut visual, view) = Mount::components(r#"
                import "components/checkbox" as checkbox;
                import "components/calendar" as calendar;
                import "components/slider" as slider;
                fn view(ctx){row([
                    column([
                        checkbox::Checkbox(#{checked:true,label:"Remember"}),
                        slider::Slider(#{key:"horizontal",label:"Horizontal",value:50}),
                        slider::Slider(#{key:"vertical",label:"Vertical",value:50,orientation:"vertical"})
                    ]).with_style(style().gap(theme_spacing("sm"))),
                    calendar::Calendar(#{key:"date",label:"Date",value:"2026-11-16",today:"2026-11-16"})
                ]).with_style(style().gap(theme_spacing("lg")).items_start())}
            "#)
                .theme(theme)
                .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
                .motion(MotionPreference::None)
                .with(move |view| view.theme_token_overrides(overrides))
                .id("preset-reading-geometry")
                .mount(cx);
            visual.simulate_resize(size(px(1800.0), px(1200.0)));
            settle(&mut visual, Step::Frame, 3);
            let quads = painted_quads(&mut visual);
            let checkbox = bounds(&mut visual, &view, "checkbox", "Remember");
            let label = bounds(&mut visual, &view, "text", "Remember");
            assert!(
                (checkbox.height - 20.0 * factor).abs() < 0.5
                    && (label.x - checkbox.x - 26.0 * factor).abs() < 0.5,
                "{name}/{scale}: checkbox first-line geometry: {checkbox:?}, {label:?}"
            );
            assert!(
                quads.iter().any(|quad| {
                    let b = quad.bounds;
                    (f64::from(b.width) - 16.0 * factor).abs() < 0.5
                        && (f64::from(b.height) - 16.0 * factor).abs() < 0.5
                        && f64::from(b.x) >= checkbox.x
                        && f64::from(b.y) >= checkbox.y
                        && f64::from(b.x + b.width) <= checkbox.x + checkbox.width + 0.5
                        && f64::from(b.y + b.height) <= checkbox.y + checkbox.height + 0.5
                }),
                "{name}/{scale}: checkbox paint stayed at a literal size"
            );
            let tree = snapshot(&mut visual, &view);
            let day = tree
                .nodes()
                .find(|node| {
                    node.role == "gridcell"
                        && node.value == Some(gpui_rhai::UiValue::String("2026-11-16".to_owned()))
                })
                .expect("selected calendar day")
                .geometry
                .as_ref()
                .unwrap()
                .layout;
            let control = if name.starts_with("clean") {
                32.0
            } else {
                36.0
            } * factor;
            assert!(
                (day.width - control).abs() < 0.5 && (day.height - control).abs() < 0.5,
                "{name}/{scale}: calendar day {day:?}, expected {control}"
            );
            for orientation in ["Horizontal", "Vertical"] {
                let slider = bounds(&mut visual, &view, "slider", orientation);
                let horizontal = orientation == "Horizontal";
                assert!(
                    quads.iter().any(|quad| {
                        let b = quad.bounds;
                        let inside = f64::from(b.x) >= slider.x - 0.5
                            && f64::from(b.y) >= slider.y - 0.5
                            && f64::from(b.x + b.width) <= slider.x + slider.width + 0.5
                            && f64::from(b.y + b.height) <= slider.y + slider.height + 0.5;
                        inside
                            && if horizontal {
                                (f64::from(b.height) - 4.0 * factor).abs() < 0.5
                                    && b.width > b.height * 10.0
                            } else {
                                (f64::from(b.width) - 4.0 * factor).abs() < 0.5
                                    && b.height > b.width * 10.0
                            }
                    }),
                    "{name}/{scale}: {orientation} track is not a scaled cross-axis strip"
                );
            }
            assert!(last_error(&mut visual, &view).is_none());
        }
    }
}
