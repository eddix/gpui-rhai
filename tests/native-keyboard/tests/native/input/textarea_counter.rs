//! The valid maximum is a full counter, not an error, with one part cascade.
use crate::support::{
    Mount, Step, bounds, by_test_id, click_center, dispatch, last_error, settle, snapshot,
};
use gpui::TestAppContext;
use gpui_rhai::{AuditRules, MotionPreference, Rgba8, ThemeTokenOverrides};

#[gpui::test]
fn textarea_counter_uses_graphemes_full_ink_and_host_precedence_without_losing_focus(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for custom in [false, true] {
            let source=r#"
                import "components/textarea" as textarea;
                fn state_schema(){#{fields:#{value:#{schema:#{type:"string"},"default":#{type:"string",value:"👩‍💻"}}}}}
                fn changed(ctx,value){ctx.set_state("value",value);}
                fn fill(ctx,p){ctx.set_state("value","👩‍💻é");}
                fn shorten(ctx,p){ctx.set_state("value","👩‍💻");}
                fn view(ctx){column([
                    textarea::Textarea(#{key:"notes",label:"Notes",value:ctx.get_state("value"),rows:2,
                        max_length:2,show_count:true,on_change:Fn("changed"),part_styles:__PARTS__}),
                    text("Fill").test_id("fill").on_click(Fn("fill")),
                    text("Shorten").test_id("shorten").on_click(Fn("shorten"))
                ]).with_style(style().width(px(420)).background(rgb(0xffffff)))}
            "#.replace("__PARTS__",if custom {"#{counter:style().text_color(rgba(0xfff0f0ff))}"}else{"#{}"});
            let (mut visual, view) = Mount::components(source)
                .theme(theme)
                .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
                .motion(MotionPreference::None)
                .with(|view| {
                    view.theme_token_overrides(ThemeTokenOverrides {
                        colors: [
                            (
                                "field.placeholder".to_owned(),
                                Rgba8::from_rgba_hex(0xfffdfdff),
                            ),
                            (
                                "field.counter_full".to_owned(),
                                Rgba8::from_rgba_hex(0xfffcfcff),
                            ),
                        ]
                        .into(),
                        ..Default::default()
                    })
                })
                .mount(cx);
            settle(&mut visual, Step::Frame, 3);
            visual.update(|window, _| window.activate_window());
            let field = bounds(&mut visual, &view, "text_field", "Notes");
            click_center(&mut visual, field);
            settle(&mut visual, Step::Frame, 2);
            let owner = visual.update(|window, cx| window.focused(cx)).unwrap();
            for (command, text, ink) in [
                (None, "1 / 2", 0xfffdfdff),
                (Some("fill"), "2 / 2", 0xfffcfcff),
                (Some("shorten"), "1 / 2", 0xfffdfdff),
            ] {
                if let Some(command) = command {
                    dispatch(&mut visual, &view, by_test_id(command), "click").unwrap();
                    settle(&mut visual, Step::Frame, 3);
                }
                assert!(
                    snapshot(&mut visual, &view)
                        .find_by_role_and_name("text", text)
                        .next()
                        .is_some(),
                    "{name}/{text}: grapheme counter"
                );
                let findings = visual.update(|_, cx| {
                    view.composition_audit_with(
                        &AuditRules::from_ids(["low-contrast-text"]).unwrap(),
                        cx,
                    )
                    .unwrap()
                });
                let counter = findings
                    .iter()
                    .find(|finding| finding.path.ends_with("/notes/counter/0"))
                    .unwrap_or_else(|| {
                        panic!("{name}/{text}: counter evidence absent: {findings:?}")
                    });
                assert_eq!(
                    counter.contrast.as_ref().unwrap().foreground,
                    Rgba8::from_rgba_hex(if custom { 0xfff0f0ff } else { ink }),
                    "{name}/{text}"
                );
                assert_eq!(
                    visual.update(|window, cx| window.focused(cx)),
                    Some(owner.clone()),
                    "{name}: retained editor focus"
                );
                assert_eq!(last_error(&mut visual, &view), None);
            }
        }
    }
}
