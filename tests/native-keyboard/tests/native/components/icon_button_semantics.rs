//! Ordinary icon actions and explicit controlled toggles have distinct semantics.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{AutomationCommand, MotionPreference, UiValue};

use crate::support::{Mount, Step, click_center, last_error, node_by_test_id, settle, text_of};

const SOURCE: &str = r#"
    import "components/icon_button" as icon_button;
    fn state_schema(){#{fields:#{
        mode:#{schema:#{type:"string",allowed:["action","off","on","unset"]},"default":#{type:"string",value:"action"}},
        clicks:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
    }}}
    fn init(ctx){ctx.register_action("test.icon_intent",Fn("set_intent"));}
    fn set_intent(ctx,value){ctx.set_state("mode",value);}
    fn activated(ctx,event){ctx.set_state("clicks",ctx.get_state("clicks")+1);}
    fn view(ctx){
        let props=#{key:"action",icon:text("+"),label:"Intent",on_click:Fn("activated")};
        let mode=ctx.get_state("mode");
        if mode=="off"{props.selected=false;}
        if mode=="on"{props.selected=true;}
        if mode=="unset"{props.selected=();}
        column([icon_button::IconButton(props).test_id("intent"),
            text(`${ctx.get_state("clicks")}`).test_id("clicks")])
    }
"#;

fn mode(visual: &mut VisualTestContext, view: &gpui_rhai::ScriptViewHandle, mode: &str) {
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Action {
                    id: "test.icon_intent".into(),
                    payload: Some(UiValue::String(mode.into())),
                },
                window,
                cx,
            )
        })
        .unwrap();
    settle(visual, Step::Frame, 3);
}

#[gpui::test]
fn optional_toggle_intent_changes_semantics_without_replacing_the_icon_owner(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = Mount::components(SOURCE)
        .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
        .motion(MotionPreference::None)
        .with(|view| {
            view.theme_sources([
                (
                    "modern_dark.rhai".to_owned(),
                    gpui_rhai_registry::MODERN_DARK_THEME.to_owned(),
                ),
                (
                    "clean_light.rhai".to_owned(),
                    gpui_rhai_registry::CLEAN_LIGHT_THEME.to_owned(),
                ),
                (
                    "clean_dark.rhai".to_owned(),
                    gpui_rhai_registry::CLEAN_DARK_THEME.to_owned(),
                ),
            ])
        })
        .mount(cx);
    visual.update(|window, _| window.activate_window());
    let owner = node_by_test_id(&mut visual, &view, "intent").id;
    let mut clicks = 0;
    for (family, appearance) in [
        ("Modern", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Clean", "Dark"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Frame, 3);
        for (intent, pressed) in [
            ("action", None),
            ("off", Some(false)),
            ("on", Some(true)),
            ("unset", None),
        ] {
            mode(&mut visual, &view, intent);
            let node = node_by_test_id(&mut visual, &view, "intent");
            assert_eq!(node.id, owner);
            assert_eq!(node.role, "button");
            assert_eq!(node.pressed, pressed, "{family}/{appearance}/{intent}");
            click_center(&mut visual, node.geometry.unwrap().visual);
            settle(&mut visual, Step::Frame, 2);
            visual.simulate_keystrokes("space");
            settle(&mut visual, Step::Frame, 2);
            clicks += 2;
            assert_eq!(text_of(&mut visual, &view, "clicks"), clicks.to_string());
            assert_eq!(
                node_by_test_id(&mut visual, &view, "intent").pressed,
                pressed,
                "callback-only updates do not invent a new controlled selection"
            );
            assert_eq!(last_error(&mut visual, &view), None);
        }
    }
}
