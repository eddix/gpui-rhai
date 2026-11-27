//! Container-width classification is scoped, discrete and independent of window class.

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ExecutionOperation, MotionPreference, ScriptViewHandle};

use crate::support::{
    Mount, Step, bounds, click_center, filled_including_rounded_clips, last_error, settle,
    snapshot, text_of,
};

const MAIN: &str = r#"
    import "components/command" as command;
    fn state_schema(){#{fields:#{
        query:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        active:#{schema:#{type:"string"},"default":#{type:"string",value:"alpha"}},
        action:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}
    }}}
    fn init(ctx){ctx.set_locale("__LOCALE__");}
    fn query(ctx,value){ctx.set_state("query",value);}
    fn active(ctx,value){ctx.set_state("active",value);}
    fn action(ctx,value){ctx.set_state("action",value);}
    fn view(ctx){column([
        command::Command(#{key:"commands",label:"Commands",query:ctx.get_state("query"),active_value:ctx.get_state("active"),
            items:[#{value:"alpha",label:"Alpha",hint:"Remote host",shortcut:"cmd-k"},#{value:"beta",label:"Beta"}],
            on_query_change:Fn("query"),on_active_change:Fn("active"),on_action:Fn("action"),
            part_styles:#{shortcut:style().background(rgb(0xd34cb8))}
        }).with_style(style().width(relative(1.0))),
        text(`${ctx.get_state("action")}/${ctx.get_state("query")}`).test_id("state")
    ]).with_style(style().width(relative(1.0)).padding(px(20)))}
"#;

fn resize(visual: &mut VisualTestContext, width: f32) {
    visual.simulate_resize(size(px(width), px(850.0)));
    // Geometry dirties enter the existing Host controller on its 16ms poll.
    // Native frame simulation alone does not advance the test executor clock.
    settle(visual, Step::Tick, 10);
}

fn script_runs(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> usize {
    visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
        .timings
        .iter()
        .filter(|timing| {
            matches!(
                timing.operation,
                ExecutionOperation::Render | ExecutionOperation::VirtualCollection(_)
            )
        })
        .count()
}

#[gpui::test]
fn command_container_crossings_keep_focus_and_only_boundary_changes_run_rhai(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for rtl in [false, true] {
            let clean = name.starts_with("clean");
            let (mut visual, view) =
                Mount::components(MAIN.replace("__LOCALE__", if rtl { "ar" } else { "en" }))
                    .theme(theme)
                    .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
                    .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
                    .motion(MotionPreference::None)
                    .mount(cx);
            visual.update(|window, _| window.activate_window());
            resize(&mut visual, 640.0);
            let input = snapshot(&mut visual, &view)
                .find_by_role_and_name("text_field", "Commands")
                .next()
                .unwrap()
                .clone();
            click_center(&mut visual, input.geometry.unwrap().visual);
            visual.simulate_input("a");
            settle(&mut visual, Step::Tick, 8);
            script_runs(&mut visual, &view);
            let before_width = bounds(&mut visual, &view, "option", "Alpha").width;
            resize(&mut visual, 630.0);
            assert!(
                bounds(&mut visual, &view, "option", "Alpha").width < before_width - 5.0,
                "the fixture must deliver an actual viewport/layout change"
            );
            assert_eq!(
                script_runs(&mut visual, &view),
                0,
                "{name}/{rtl}: in-class native resizing must not run Rhai"
            );
            assert_eq!(
                bounds(&mut visual, &view, "option", "Alpha").height,
                if clean { 36.0 } else { 40.0 }
            );
            assert!(!filled_including_rounded_clips(&mut visual, 0xd34cb8ff).is_empty());
            resize(&mut visual, 440.0);
            assert_eq!(
                bounds(&mut visual, &view, "option", "Alpha").height,
                if clean { 40.0 } else { 44.0 }
            );
            assert_eq!(
                snapshot(&mut visual, &view)
                    .find_by_role_and_name("text", "Remote host")
                    .count(),
                0
            );
            assert!(filled_including_rounded_clips(&mut visual, 0xd34cb8ff).is_empty());
            assert_eq!(
                snapshot(&mut visual, &view)
                    .find_by_role_and_name("text", "Navigate")
                    .count(),
                usize::from(!clean && !rtl)
            );
            let current = snapshot(&mut visual, &view)
                .find_by_role_and_name("text_field", "Commands")
                .next()
                .unwrap()
                .clone();
            assert_eq!(current.id, input.id);
            visual.simulate_input("l");
            settle(&mut visual, Step::Frame, 8);
            visual.simulate_keystrokes("enter");
            settle(&mut visual, Step::Frame, 8);
            assert_eq!(
                text_of(&mut visual, &view, "state"),
                "alpha/al",
                "{name}/{rtl}: the real editor still owns focus and Enter"
            );
            resize(&mut visual, 640.0);
            assert_eq!(
                snapshot(&mut visual, &view)
                    .find_by_role_and_name("text_field", "Commands")
                    .next()
                    .unwrap()
                    .id,
                input.id
            );
            assert_eq!(
                snapshot(&mut visual, &view)
                    .find_by_role_and_name("text", "Remote host")
                    .count(),
                1
            );
            script_runs(&mut visual, &view);
            settle(&mut visual, Step::Tick, 8);
            assert_eq!(
                script_runs(&mut visual, &view),
                0,
                "responsive height changes must reach quiescence"
            );
            assert!(last_error(&mut visual, &view).is_none());
        }
    }
}

#[gpui::test]
fn commands_in_the_same_wide_window_use_their_own_card_widths(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let clean = name.starts_with("clean");
        let (mut visual,view)=Mount::components(r#"
            import "components/command" as command;
            fn panel(key,label,width){command::Command(#{key:key,label:label,query:"",active_value:"a",
                items:[#{value:"a",label:`${label} action`,hint:`${label} hint`,shortcut:"cmd-k"}]})
                .with_style(style().width(px(width)).flex_shrink(false))}
            fn view(ctx){row([panel("small","Small",360),panel("large","Large",560)])
                .with_style(style().width(relative(1.0)).padding(px(20)).gap(px(20)))}
        "#).theme(theme).motion(MotionPreference::None).mount(cx);
        resize(&mut visual, 1200.0);
        assert_eq!(
            bounds(&mut visual, &view, "option", "Small action").height,
            if clean { 40.0 } else { 44.0 }
        );
        assert_eq!(
            bounds(&mut visual, &view, "option", "Large action").height,
            if clean { 36.0 } else { 40.0 }
        );
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("text", "Small hint")
                .count(),
            0
        );
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("text", "Large hint")
                .count(),
            1
        );
        assert!(last_error(&mut visual, &view).is_none(), "{name}");
    }
}

#[gpui::test]
fn command_dialog_forwards_the_explicit_card_boundary(cx: &mut TestAppContext) {
    let (mut visual,view)=Mount::components(r#"
        import "components/command_dialog" as command_dialog;
        fn view(ctx){command_dialog::CommandDialog(#{key:"dialog",label:"Commands",open:true,width:px(600),narrow_at:700.0,
            query:"",active_value:"a",items:[#{value:"a",label:"Alpha",hint:"Remote host",shortcut:"cmd-k"}]})}
    "#).motion(MotionPreference::None).mount(cx);
    resize(&mut visual, 1200.0);
    assert_eq!(bounds(&mut visual, &view, "option", "Alpha").height, 44.0);
    assert_eq!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("text", "Remote host")
            .count(),
        0
    );
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn narrow_clean_command_keeps_a_custom_footer_and_its_own_enter_scope(cx: &mut TestAppContext) {
    let (mut visual,view)=Mount::components(r#"
        import "components/command" as command;import "components/button" as button;
        fn state_schema(){#{fields:#{
            commands:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
            footers:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
        }}}
        fn command_action(ctx,value){ctx.set_state("commands",ctx.get_state("commands")+1);}
        fn footer(ctx,value){ctx.set_state("footers",ctx.get_state("footers")+1);}
        fn view(ctx){column([
            command::Command(#{key:"commands",label:"Commands",query:"",active_value:"a",items:[#{value:"a",label:"Alpha"}],
                part_styles:#{item:style().height(px(70)).min_height(px(70))},
                on_action:Fn("command_action"),footer:button::Button(#{key:"footer",text:"Footer action",on_click:Fn("footer")})})
                .with_style(style().width(px(360))),
            text(`${ctx.get_state("commands")}/${ctx.get_state("footers")}`).test_id("state")
        ]).with_style(style().padding(px(20)))}
    "#).theme(gpui_rhai_registry::CLEAN_LIGHT_THEME).motion(MotionPreference::None).mount(cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 8);
    assert_eq!(
        bounds(&mut visual, &view, "option", "Alpha").height,
        70.0,
        "a common instance override remains above the narrow-state preset"
    );
    let footer = bounds(&mut visual, &view, "button", "Footer action");
    click_center(&mut visual, footer);
    settle(&mut visual, Step::Frame, 8);
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Frame, 8);
    assert_eq!(text_of(&mut visual, &view, "state"), "0/2");
    assert!(last_error(&mut visual, &view).is_none());
}
