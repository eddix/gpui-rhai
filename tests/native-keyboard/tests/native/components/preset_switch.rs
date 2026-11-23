//! Switch settings rows share the source geometry, logical placement and one focus owner.
use crate::support::{
    Mount, Step, bounds, by_test_id, click_center, dispatch, filled, last_error, settle, snapshot,
    text_of,
};
use gpui::{Modifiers, TestAppContext, point, px};
use gpui_rhai::{MotionPreference, ThemeTokenOverrides, UiValue};

fn close(actual: f64, expected: f64, case: &str) {
    assert!(
        (actual - expected).abs() < 0.6,
        "{case}: {actual} != {expected}"
    );
}

#[gpui::test]
fn switch_descriptions_and_logical_end_placement_keep_first_line_control_geometry(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for rtl in [false, true] {
            for scale in [100, 200] {
                for end in [false, true] {
                    for small in [false, true] {
                        let factor = f64::from(scale) / 100.;
                        let overrides = ThemeTokenOverrides {
                            environment_defaults: [("scale".into(), scale.to_string())].into(),
                            ..Default::default()
                        };
                        let source = format!(
                            r#"
                        import "components/switch" as sw;
                        fn state_schema(){{#{{fields:#{{checked:#{{schema:#{{type:"bool"}},"default":#{{type:"bool",value:false}}}}}}}}}}
                        fn init(ctx){{ctx.set_locale("{}");}}
                        fn changed(ctx,value){{ctx.set_state("checked",value);}}
                        fn view(ctx){{column([
                            sw::Switch(#{{key:"setting",label:"Background sync",description:"Keep this device up to date.",placement:"{}",size:"{}",
                                checked:ctx.get_state("checked"),on_change:Fn("changed"),part_styles:#{{control:style().background(rgb(0x00ff00)),description:style().background(rgb(0xff00ff))}}}})
                                .with_style(style().width(px(380))),
                            text(`${{ctx.get_state("checked")}}`).test_id("state")
                        ]).with_style(style().padding(px(20)).gap(px(12)))}}
                    "#,
                            if rtl { "ar" } else { "en" },
                            if end { "end" } else { "start" },
                            if small { "sm" } else { "md" }
                        );
                        let (mut visual, view) = Mount::components(source)
                            .theme(theme)
                            .motion(MotionPreference::None)
                            .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
                            .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
                            .with(move |view| view.theme_token_overrides(overrides))
                            .mount(cx);
                        settle(&mut visual, Step::Refresh, 4);
                        let root = bounds(&mut visual, &view, "switch", "Background sync");
                        let label = bounds(&mut visual, &view, "text", "Background sync");
                        let control = filled(&mut visual, 0x00ff00ff);
                        let desc = filled(&mut visual, 0xff00ffff);
                        assert_eq!(control.len(), 1, "{name}");
                        assert_eq!(desc.len(), 1, "{name}");
                        close(root.width, 380., name);
                        close(
                            f64::from(control[0].bounds.width),
                            if small { 26. } else { 32. } * factor,
                            name,
                        );
                        close(f64::from(control[0].bounds.height), 20. * factor, name);
                        close(f64::from(control[0].bounds.y), root.y, name);
                        close(label.y, root.y, name);
                        close(f64::from(desc[0].bounds.y), label.y + label.height, name);
                        close(
                            root.height,
                            label.height + f64::from(desc[0].bounds.height),
                            name,
                        );
                        if end {
                            close(
                                if rtl {
                                    f64::from(control[0].bounds.x)
                                } else {
                                    f64::from(control[0].bounds.right())
                                },
                                if rtl { root.x } else { root.x + root.width },
                                name,
                            );
                        }
                        let tree = snapshot(&mut visual, &view);
                        let owner = tree
                            .find_by_role_and_name("switch", "Background sync")
                            .next()
                            .unwrap();
                        let id = owner.id;
                        assert_eq!(owner.description, "Keep this device up to date.");
                        let desc_rect = desc[0].bounds;
                        visual.simulate_click(
                            point(
                                px(desc_rect.x + desc_rect.width / 2.),
                                px(desc_rect.y + desc_rect.height / 2.),
                            ),
                            Modifiers::none(),
                        );
                        settle(&mut visual, Step::Refresh, 4);
                        assert_eq!(text_of(&mut visual, &view, "state"), "true");
                        let tree = snapshot(&mut visual, &view);
                        let owner = tree
                            .find_by_role_and_name("switch", "Background sync")
                            .next()
                            .unwrap();
                        assert_eq!(owner.id, id);
                        assert_eq!(owner.checked, Some(UiValue::Bool(true)));
                        assert_eq!(last_error(&mut visual, &view), None);
                    }
                }
            }
        }
    }
}

#[gpui::test]
fn switch_placement_and_description_updates_preserve_focus_and_host_part_overrides(
    cx: &mut TestAppContext,
) {
    let source = r#"
        import "components/switch" as sw;
        fn state_schema(){#{fields:#{end:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},checked:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn moved(ctx,payload){ctx.set_state("end",!ctx.get_state("end"));}
        fn changed(ctx,value){ctx.set_state("checked",value);}
        fn view(ctx){column([
            sw::Switch(#{key:"setting",label:"Setting",description:if ctx.get_state("end"){"Extra context"}else{""},placement:if ctx.get_state("end"){"end"}else{"start"},
                checked:ctx.get_state("checked"),on_change:Fn("changed"),part_styles:#{root:style().justify_start().gap(px(3)),label:style().font_weight(600)}})
                .with_style(style().width(px(320))),
            text("Move").test_id("move").on_click(Fn("moved")),text(`${ctx.get_state("checked")}`).test_id("state")
        ]).with_style(style().padding(px(20)).gap(px(12)))}
    "#;
    let (mut visual, view) = Mount::components(source).mount(cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Refresh, 3);
    let area = bounds(&mut visual, &view, "switch", "Setting");
    click_center(&mut visual, area);
    settle(&mut visual, Step::Refresh, 3);
    let focus = visual.update(|window, cx| window.focused(cx));
    assert!(focus.is_some());
    let id = snapshot(&mut visual, &view)
        .find_by_role_and_name("switch", "Setting")
        .next()
        .unwrap()
        .id;
    dispatch(&mut visual, &view, by_test_id("move"), "click").unwrap();
    settle(&mut visual, Step::Refresh, 4);
    assert_eq!(visual.update(|window, cx| window.focused(cx)), focus);
    assert_eq!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("switch", "Setting")
            .next()
            .unwrap()
            .id,
        id
    );
    visual.simulate_keystrokes("space");
    settle(&mut visual, Step::Refresh, 3);
    assert_eq!(text_of(&mut visual, &view, "state"), "false");
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let sw = match root.kind() {
        gpui_rhai::UiNodeKind::Box { children } => &children[0],
        _ => panic!("root column"),
    };
    assert_eq!(sw.style().base.justify, Some(gpui_rhai::Justify::Start));
    assert_eq!(sw.style().base.gap, Some(gpui_rhai::Length::Pixels(3.)));
    assert_eq!(last_error(&mut visual, &view), None);
}
