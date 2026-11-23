//! Compound chrome has one edge owner, one live editor and a separate footer key scope.

use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{GeometryBounds, MotionPreference, ScriptViewHandle, ThemeTokenOverrides};

use crate::support::{
    Mount, Step, bounds, click_center, filled, last_error, logical_paint_rect, node_by_test_id,
    painted_quads, settle, snapshot, text_of, walk_live_paint,
};

const MAIN: &str = r#"
    import "components/command" as command;
    import "components/button" as button;
    fn state_schema(){#{fields:#{
        query:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        active:#{schema:#{type:"string"},"default":#{type:"string",value:"a"}},
        actions:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
        moves:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
        chrome:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
        footer:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
    }}}
    fn init(ctx){ctx.set_locale("__LOCALE__");ctx.register_action("test.chrome",Fn("chrome"));}
    fn chrome(ctx,value){ctx.set_state("chrome",(ctx.get_state("chrome")+1)%3);}
    fn queried(ctx,value){ctx.set_state("query",value);}
    fn moved(ctx,value){ctx.set_state("active",value);ctx.set_state("moves",ctx.get_state("moves")+1);}
    fn acted(ctx,value){ctx.set_state("actions",ctx.get_state("actions")+1);}
    fn footer(ctx,value){ctx.set_state("footer",ctx.get_state("footer")+1);}
    fn view(ctx){column([
        command::Command(#{key:"commands",label:"Query",query:ctx.get_state("query"),active_value:ctx.get_state("active"),
            appearance:if ctx.get_state("chrome")==0 {"panel"}else{"embedded"},footer_visible:ctx.get_state("chrome")!=2,
            items:[#{value:"a",label:"Alpha",shortcut:"cmd-a"},#{value:"b",label:"Beta"},#{value:"c",label:"Gamma"}],
            on_query_change:Fn("queried"),on_active_change:Fn("moved"),on_action:Fn("acted") __FOOTER__
        }).test_id("command-panel").with_style(style().width(px(560))),
        text(`${ctx.get_state("actions")}/${ctx.get_state("footer")}/${ctx.get_state("moves")}/${ctx.get_state("query")}`).test_id("state")
    ]).with_style(style().width(px(600)).padding(px(20)).gap(px(8)))}
"#;

fn search_chrome(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> GeometryBounds {
    let tree = snapshot(visual, view);
    let input = tree
        .find_by_role_and_name("text_field", "Query")
        .next()
        .unwrap();
    tree.node(input.parent.unwrap())
        .unwrap()
        .geometry
        .unwrap()
        .visual
}

fn close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() < 0.6,
        "{context}: {actual} != {expected}"
    );
}

#[gpui::test]
fn command_chrome_and_viewport_resolve_both_designs_in_rtl_density_and_reading_scale(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for density in ["comfortable", "compact"] {
            for scale in [100, 200] {
                for rtl in [false, true] {
                    let clean = name.starts_with("clean");
                    let compact = density == "compact";
                    let f = f64::from(scale) / 100.0;
                    let pad = if clean { 0.0 } else { 4.0 } * f;
                    let row =
                        (if clean { 36.0 } else { 40.0 } - if compact { 4.0 } else { 0.0 }) * f;
                    let search =
                        (if clean { 48.0 } else { 52.0 } - if compact { 4.0 } else { 0.0 }) * f;
                    let inset =
                        (if clean { 4.0 } else { 6.0 } - if compact { 2.0 } else { 0.0 }) * f;
                    let top = if clean {
                        0.0
                    } else {
                        if compact { 2.0 } else { 4.0 }
                    } * f;
                    let bottom =
                        (if clean { 4.0 } else { 8.0 } - if compact { 2.0 } else { 0.0 }) * f;
                    let footer =
                        (if clean { 40.0 } else { 38.0 } - if compact { 4.0 } else { 0.0 }) * f;
                    let overrides = ThemeTokenOverrides {
                        environment_defaults: [
                            ("density".into(), density.into()),
                            ("scale".into(), scale.to_string()),
                        ]
                        .into(),
                        ..Default::default()
                    };
                    let source = MAIN
                        .replace("__LOCALE__", if rtl { "ar" } else { "en" })
                        .replace("__FOOTER__", "");
                    let (mut visual, view) = Mount::components(source)
                        .theme(theme)
                        .motion(MotionPreference::None)
                        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
                        .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
                        .with(move |view| view.theme_token_overrides(overrides))
                        .id("command-chrome")
                        .mount(cx);
                    settle(&mut visual, Step::Frame, 3);
                    let panel = node_by_test_id(&mut visual, &view, "command-panel")
                        .geometry
                        .unwrap()
                        .visual;
                    let chrome = search_chrome(&mut visual, &view);
                    let field = bounds(&mut visual, &view, "text_field", "Query");
                    let list = bounds(&mut visual, &view, "listbox", "Query");
                    let case = format!("{name}/{density}/{scale}/{rtl}");
                    close(panel.width, 560.0, &case);
                    close(
                        panel.height,
                        2.0 * pad + search + top + 3.0 * row + bottom + footer,
                        &case,
                    );
                    close(chrome.x - panel.x, pad, &case);
                    close(chrome.width, panel.width - 2.0 * pad, &case);
                    close(chrome.y - panel.y, pad, &case);
                    close(chrome.height, search, &case);
                    close(field.height, 20.0 * f, &case);
                    let leading =
                        (if clean { 14.0 } else { 16.0 } - if compact { 4.0 } else { 0.0 }) * f;
                    let icon = (if clean { 16.0 } else { 18.0 }) * f;
                    let gap =
                        (if clean { 12.0 } else { 10.0 }) * f * (if compact { 0.75 } else { 1.0 });
                    let field_start = if rtl {
                        chrome.x + chrome.width - field.x - field.width
                    } else {
                        field.x - chrome.x
                    };
                    close(field_start, leading + icon + gap, &case);
                    close(list.x - panel.x, pad + inset, &case);
                    close(list.width, panel.width - 2.0 * (pad + inset), &case);
                    close(list.y - panel.y, pad + search + top, &case);
                    close(list.height, 3.0 * row, &case);
                    let run = bounds(
                        &mut visual,
                        &view,
                        "text",
                        if rtl { "تشغيل" } else { "Run" },
                    );
                    let footer_pad = 12.0 * f * if compact { 0.75 } else { 1.0 };
                    let end = if rtl {
                        run.x - panel.x
                    } else {
                        panel.x + panel.width - run.x - run.width
                    };
                    close(end, pad + footer_pad, &case);
                    let role = if clean {
                        "command.surface"
                    } else {
                        "command.tray"
                    };
                    let color = visual.update(|_, cx| {
                        view.theme_snapshot(cx)
                            .unwrap()
                            .variant
                            .tokens
                            .color(role)
                            .unwrap()
                            .as_rgba_hex()
                    });
                    let quad = filled(&mut visual, color)
                        .into_iter()
                        .find(|quad| {
                            (f64::from(quad.bounds.x) - panel.x).abs() < 0.6
                                && (f64::from(quad.bounds.y) - panel.y).abs() < 0.6
                                && (f64::from(quad.bounds.height) - panel.height).abs() < 0.6
                        })
                        .expect("the actual outer chrome paints");
                    close(
                        f64::from(quad.corner_radii[0]),
                        (if clean { 12.0 } else { 20.0 }) * f,
                        &case,
                    );
                    assert!(last_error(&mut visual, &view).is_none(), "{case}");
                }
            }
        }
    }
}

#[gpui::test]
fn command_headings_have_natural_extent_and_short_filtered_results_fit_it(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 200] {
            let clean = name.starts_with("clean");
            let f = f64::from(scale) / 100.0;
            let source = MAIN
                .replace("__LOCALE__", "en")
                .replace("__FOOTER__", "")
                .replace(
                    "label:\"Alpha\",shortcut",
                    "label:\"Alpha\",group:\"General\",shortcut",
                )
                .replace("label:\"Beta\"", "label:\"Beta\",group:\"General\"")
                .replace("label:\"Gamma\"", "label:\"Gamma\",group:\"Other\"");
            let overrides = ThemeTokenOverrides {
                environment_defaults: [("scale".into(), scale.to_string())].into(),
                ..Default::default()
            };
            let (mut visual, view) = Mount::components(source)
                .theme(theme)
                .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
                .motion(MotionPreference::None)
                .with(move |view| view.theme_token_overrides(overrides))
                .mount(cx);
            settle(&mut visual, Step::Frame, 8);
            let heading = bounds(&mut visual, &view, "heading", "General");
            let other = bounds(&mut visual, &view, "heading", "Other");
            let list = bounds(&mut visual, &view, "listbox", "Query");
            let group_h = (if clean { 30.0 } else { 28.5 }) * f;
            let row_h = (if clean { 36.0 } else { 40.0 }) * f;
            close(heading.height, group_h, name);
            close(other.height, group_h, name);
            close(list.height, 2.0 * group_h + 3.0 * row_h, name);
            let tree = snapshot(&mut visual, &view);
            let heading_node = tree
                .find_by_role_and_name("heading", "General")
                .next()
                .unwrap();
            let text = tree
                .nodes()
                .find(|node| node.parent == Some(heading_node.id) && node.role == "text")
                .unwrap();
            assert_eq!(text.name, if clean { "GENERAL" } else { "General" });
            let input = bounds(&mut visual, &view, "text_field", "Query");
            click_center(&mut visual, input);
            visual.simulate_input("Beta");
            settle(&mut visual, Step::Frame, 8);
            close(
                bounds(&mut visual, &view, "listbox", "Query").height,
                group_h + row_h,
                name,
            );
            assert!(last_error(&mut visual, &view).is_none(), "{name}");
        }
    }
}

fn all_designs(
    cx: &mut TestAppContext,
    custom_footer: bool,
) -> (VisualTestContext, ScriptViewHandle) {
    all_designs_with_styles(cx, custom_footer, "fn component_styles(){#{}}")
}

fn all_designs_with_styles(
    cx: &mut TestAppContext,
    custom_footer: bool,
    sheet: &str,
) -> (VisualTestContext, ScriptViewHandle) {
    let sheet = sheet.to_owned();
    let footer = if custom_footer {
        r#",footer:button::Button(#{key:"footer-action",text:"Footer action",on_click:Fn("footer")}).test_id("footer-action")"#
    } else {
        ""
    };
    Mount::components(
        MAIN.replace("__LOCALE__", "en")
            .replace("__FOOTER__", footer),
    )
    .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
    .motion(MotionPreference::None)
    .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
    .with(move |view| {
        view.component_styles(sheet).theme_sources(
            gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
                .iter()
                .filter(|(file, _)| *file != "modern_light.rhai")
                .map(|(file, source)| (file.to_string(), source.to_string())),
        )
    })
    .id("command-live")
    .mount(cx)
}

#[gpui::test]
fn command_editor_shapes_the_live_search_role_and_host_override_not_its_fallback_control_role(
    cx: &mut TestAppContext,
) {
    fn caret_x(visual: &mut VisualTestContext) -> f64 {
        f64::from(
            painted_quads(visual)
                .into_iter()
                .find(|quad| {
                    (quad.bounds.width - 1.5).abs() < 0.01 && !quad.background.is_transparent()
                })
                .expect("focused native caret")
                .bounds
                .x,
        )
    }
    for custom in [false, true] {
        let sheet = if custom {
            r#"fn component_styles(){#{"components/command":#{search_input:style().font_size(px(20)).line_height(px(24)).height(px(24))}}}"#
        } else {
            "fn component_styles(){#{}}"
        };
        let (mut visual, view) = all_designs_with_styles(cx, false, sheet);
        visual.update(|window, _| window.activate_window());
        settle(&mut visual, Step::Frame, 3);
        let field = bounds(&mut visual, &view, "text_field", "Query");
        click_center(&mut visual, field);
        visual.simulate_input("MMMM");
        settle(&mut visual, Step::Frame, 2);
        for (family, appearance) in [
            ("Modern", "Light"),
            ("Clean", "Light"),
            ("Modern", "Dark"),
            ("Clean", "Dark"),
        ] {
            visual
                .update(|_, cx| view.select_theme(family, appearance, cx))
                .unwrap();
            settle(&mut visual, Step::Frame, 3);
            visual.simulate_keystrokes("home");
            settle(&mut visual, Step::Frame, 1);
            let start = caret_x(&mut visual);
            visual.simulate_keystrokes("end");
            settle(&mut visual, Step::Frame, 1);
            let width = caret_x(&mut visual) - start;
            let size = if custom {
                20.0
            } else if family == "Clean" {
                13.0
            } else {
                15.0
            };
            let expected = visual.update(|window, _| {
                let mut style = window.text_style().clone();
                style.font_family = "Inter".into();
                style.font_weight = gpui::FontWeight::NORMAL;
                window
                    .text_system()
                    .shape_text("MMMM".into(), px(size), &[style.to_run(4)], None, Some(1))
                    .unwrap()
                    .iter()
                    .map(|line| f64::from(line.width()))
                    .sum::<f64>()
            });
            close(
                width,
                expected,
                &format!("{family}/{appearance}/{custom}: actual editor advances"),
            );
            assert!(last_error(&mut visual, &view).is_none());
        }
    }
}

#[gpui::test]
fn command_search_padding_focuses_the_single_editor_and_retheming_keeps_its_caret(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = all_designs(cx, false);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 3);
    let id = snapshot(&mut visual, &view)
        .find_by_role_and_name("text_field", "Query")
        .next()
        .unwrap()
        .id;
    let chrome = search_chrome(&mut visual, &view);
    #[allow(clippy::cast_possible_truncation)]
    let at = point(
        px((chrome.x + 1.0) as f32),
        px((chrome.y + chrome.height / 2.0) as f32),
    );
    visual.simulate_click(at, Modifiers::none());
    visual.simulate_input("g");
    settle(&mut visual, Step::Frame, 3);
    assert!(text_of(&mut visual, &view, "state").ends_with("/g"));
    let mut query = "g".to_owned();
    for (family, appearance) in [
        ("Clean", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Dark"),
        ("Modern", "Light"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Frame, 3);
        let current = snapshot(&mut visual, &view)
            .find_by_role_and_name("text_field", "Query")
            .next()
            .unwrap()
            .id;
        assert_eq!(
            current, id,
            "same native editor across {family}/{appearance}"
        );
        visual.simulate_input("a");
        query.push('a');
        settle(&mut visual, Step::Frame, 2);
        assert!(text_of(&mut visual, &view, "state").ends_with(&format!("/{query}")));
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn command_appearance_and_footer_changes_do_not_remount_the_active_editor(cx: &mut TestAppContext) {
    let (mut visual, view) = all_designs(cx, false);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 3);
    let input = snapshot(&mut visual, &view)
        .find_by_role_and_name("text_field", "Query")
        .next()
        .unwrap()
        .clone();
    click_center(&mut visual, input.geometry.unwrap().visual);
    visual.simulate_input("g");
    settle(&mut visual, Step::Frame, 2);
    let mut query = "g".to_owned();
    for _ in 0..3 {
        visual
            .update(|window, cx| {
                view.automate(
                    gpui_rhai::AutomationCommand::Action {
                        id: "test.chrome".into(),
                        payload: None,
                    },
                    window,
                    cx,
                )
            })
            .unwrap();
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("text_field", "Query")
                .next()
                .unwrap()
                .id,
            input.id
        );
        visual.simulate_input("a");
        query.push('a');
        settle(&mut visual, Step::Frame, 2);
        assert!(text_of(&mut visual, &view, "state").ends_with(&format!("/{query}")));
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn application_footer_controls_keep_their_styles_semantics_and_own_enter_scope(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = all_designs(cx, true);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 3);
    let footer = node_by_test_id(&mut visual, &view, "footer-action");
    click_center(&mut visual, footer.geometry.unwrap().visual);
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(text_of(&mut visual, &view, "state"), "0/1/0/");
    visual.simulate_mouse_move(point(px(0.), px(0.)), None, Modifiers::none());
    let mut count = 1;
    for (family, appearance) in [
        ("Clean", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Dark"),
        ("Modern", "Light"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Frame, 3);
        let current = node_by_test_id(&mut visual, &view, "footer-action");
        assert_eq!(current.id, footer.id, "slot control identity");
        assert_eq!(
            current.role, "button",
            "the caller's control is not hidden or repainted as a caption"
        );
        close(
            current.geometry.unwrap().visual.height,
            if family == "Clean" { 28.0 } else { 40.0 },
            "fresh slotted Button preset",
        );
        let at = current.geometry.unwrap().visual;
        let theme = visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant);
        let color = |token| gpui::rgba(theme.tokens.color(token).unwrap().as_rgba_hex());
        let matches = |rect: crate::support::Rect| {
            (f64::from(rect.x) - at.x).abs() < 0.6
                && (f64::from(rect.y) - at.y).abs() < 0.6
                && (f64::from(rect.width) - at.width).abs() < 0.6
                && (f64::from(rect.height) - at.height).abs() < 0.6
        };
        let (flat, layered) = visual.update(|window, _| {
            let scale = window.scale_factor();
            let flat_color: gpui::Background = color("button.primary_bg").into();
            window.visual_probe_with_painted_scene(|scene| {
                let (mut flat, mut layered) = (false, false);
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::unit(),
                    &mut |scene, transform| {
                        flat |= scene.quads.iter().any(|quad| {
                            quad.background == flat_color
                                && matches(logical_paint_rect(quad.bounds, transform, scale))
                        });
                        layered |= scene.layered_fills.iter().any(|fill| {
                            matches(logical_paint_rect(fill.bounds, transform, scale))
                                && fill.layers.len() == 2
                                && fill.layers[0].stops.len() == 2
                                && fill.layers[1].stops.len() == 3
                                && fill.layers[0].stops[0].color == color("button.primary_bg")
                                && fill.layers[0].stops[1].color == color("button.primary_bottom")
                        });
                    },
                );
                (flat, layered)
            })
        });
        assert_eq!(
            (flat, layered),
            (family == "Clean", family == "Modern"),
            "{family}/{appearance}: slotted control paints the current design"
        );
        visual.simulate_keystrokes("enter");
        count += 1;
        settle(&mut visual, Step::Frame, 2);
        assert_eq!(
            text_of(&mut visual, &view, "state"),
            format!("0/{count}/0/"),
            "footer Enter must not activate or move a command"
        );
        assert!(last_error(&mut visual, &view).is_none());
    }
}
