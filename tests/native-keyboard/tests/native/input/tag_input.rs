//! One native editor, atomically controlled chips, and real text/pointer/key routes.
use gpui::{ClipboardItem, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{MotionPreference, ScriptViewHandle, UiValue};

use crate::support::{Mount, Step, bounds, click_center, last_error, settle, snapshot, text_of};

const APP: &str = r#"
    import "components/tag_input" as tag_input;
    import "components/input" as input;
    fn state_schema(){#{fields:#{
        tags:#{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[
            #{type:"string",value:"seed@example.org"}
        ]}},
        changes:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
    }}}
    fn init(ctx){ctx.set_locale("__LOCALE__");}
    fn changed(ctx,value){
        if __ACCEPT__ {ctx.set_state("tags",value);}
        ctx.set_state("changes",ctx.get_state("changes")+1);
    }
    fn view(ctx){
        let tags=ctx.get_state("tags");
        column([
            tag_input::TagInput(#{key:"tags",label:"Tags",values:tags,max:4,validation:"__VALIDATION__",
                normalize:"lowercase",avatar:true,on_change:Fn("changed")}),
            input::Input(#{key:"following",label:"Following field",value:"",on_change:Fn("noop")}),
            text(`${ctx.get_state("changes")}:${tags}`).test_id("state"),
        ]).with_style(style().width(relative(1.0)).padding(px(12)).gap(px(8)));
    }
    fn noop(ctx,value){}
"#;

fn mount(
    cx: &mut TestAppContext,
    theme: &str,
    accept: bool,
    locale: &str,
    validation: &str,
) -> (VisualTestContext, ScriptViewHandle) {
    let app = APP
        .replace("__ACCEPT__", if accept { "true" } else { "false" })
        .replace("__LOCALE__", locale)
        .replace("__VALIDATION__", validation);
    let (mut visual, view) = Mount::components(app)
        .theme(theme)
        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
        .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
        .motion(MotionPreference::None)
        .mount(cx);
    visual.update(|w, _| w.activate_window());
    ready(&mut visual, &view);
    (visual, view)
}

fn ready(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    settle(visual, Step::Tick, 4);
    assert_eq!(last_error(visual, view), None);
}

fn click_editor(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    let target = bounds(visual, view, "text_field", "Tags");
    click_center(visual, target);
    ready(visual, view);
}

fn draft(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> (String, bool) {
    let tree = snapshot(visual, view);
    let editor = tree
        .find_by_role_and_name("text_field", "Tags")
        .next()
        .unwrap();
    let Some(UiValue::String(value)) = &editor.value else {
        panic!("native editor without text value");
    };
    (value.clone(), editor.invalid)
}

fn paste(visual: &mut VisualTestContext, view: &ScriptViewHandle, value: &str) {
    visual.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string(value.into())));
    visual.simulate_keystrokes("cmd-v");
    ready(visual, view);
}

#[gpui::test]
fn tag_input_paste_is_one_validated_proposal_and_limits_keep_rejected_text(
    cx: &mut TestAppContext,
) {
    for &(preset, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = mount(cx, theme, true, "en", "email");
        click_editor(&mut visual, &view);
        paste(
            &mut visual,
            &view,
            "ALFA@example.org;\r\ninvalid\nBETA@example.org;alfa@example.org",
        );
        assert_eq!(
            text_of(&mut visual, &view, "state"),
            r#"1:["seed@example.org", "alfa@example.org", "beta@example.org"]"#,
            "{preset}",
        );
        assert_eq!(draft(&mut visual, &view), ("invalid".into(), true));
        visual.simulate_keystrokes("escape");
        ready(&mut visual, &view);
        assert_eq!(draft(&mut visual, &view), ("".into(), false));
        visual.simulate_input("third@example.org");
        ready(&mut visual, &view);
        visual.simulate_keystrokes("tab");
        ready(&mut visual, &view);
        assert!(text_of(&mut visual, &view, "state").starts_with("2:"));
        visual.simulate_input("overflow@example.org,");
        ready(&mut visual, &view);
        assert_eq!(
            draft(&mut visual, &view),
            ("overflow@example.org".into(), true)
        );
        visual.simulate_keystrokes("escape backspace");
        ready(&mut visual, &view);
        assert!(text_of(&mut visual, &view, "state").contains("third@example.org"));
        visual.simulate_keystrokes("backspace");
        ready(&mut visual, &view);
        assert_eq!(
            text_of(&mut visual, &view, "state"),
            r#"3:["seed@example.org", "alfa@example.org", "beta@example.org"]"#,
        );
        paste(&mut visual, &view, "alfa@example.org;");
        assert!(text_of(&mut visual, &view, "state").starts_with("3:"));
        assert!(
            snapshot(&mut visual, &view)
                .nodes()
                .any(|n| n.role == "status" && n.name == "alfa@example.org is already present")
        );
    }
}

#[gpui::test]
fn tag_input_rejected_proposal_stays_editable_and_single_paste_preserves_caret(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount(
        cx,
        gpui_rhai_registry::CLEAN_LIGHT_THEME,
        false,
        "en",
        "text",
    );
    click_editor(&mut visual, &view);
    paste(&mut visual, &view, "Alpha;Beta");
    assert_eq!(
        text_of(&mut visual, &view, "state"),
        r#"1:["seed@example.org"]"#
    );
    assert_eq!(draft(&mut visual, &view).0, "Alpha;Beta");
    visual.simulate_keystrokes("escape");
    visual.simulate_input("AlphaBeta");
    ready(&mut visual, &view);
    visual.simulate_keystrokes("home right right right right right");
    paste(&mut visual, &view, ".");
    visual.simulate_input("!");
    ready(&mut visual, &view);
    assert_eq!(draft(&mut visual, &view).0, "Alpha.!Beta");
    assert_eq!(
        text_of(&mut visual, &view, "state"),
        r#"1:["seed@example.org"]"#
    );
}

#[gpui::test]
fn tag_input_preserves_single_editor_under_resize_and_moves_to_chips_in_rtl(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount(cx, gpui_rhai_registry::CLEAN_DARK_THEME, true, "ar", "text");
    visual.simulate_resize(size(px(630.), px(480.)));
    ready(&mut visual, &view);
    click_editor(&mut visual, &view);
    paste(
        &mut visual,
        &view,
        "التصميم الطويل جدًا;المراجعة الطويلة;التنفيذ",
    );
    let before = bounds(&mut visual, &view, "text_field", "Tags");
    visual.simulate_resize(size(px(236.), px(460.)));
    ready(&mut visual, &view);
    let editor = bounds(&mut visual, &view, "text_field", "Tags");
    assert!(
        editor.y > before.y + 10.,
        "before {before:?}, after {editor:?}"
    );
    assert!(editor.width > 0. && editor.x + editor.width <= 236.5);
    visual.simulate_keystrokes("right delete");
    ready(&mut visual, &view);
    let state = text_of(&mut visual, &view, "state");
    assert!(
        state.starts_with("2:") && !state.contains("التنفيذ"),
        "{state}"
    );
    visual.simulate_input("New,");
    ready(&mut visual, &view);
    assert!(text_of(&mut visual, &view, "state").contains("\"new\""));
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn tag_input_ime_preedit_never_commits_a_delimiter(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        gpui_rhai_registry::MODERN_DARK_THEME,
        true,
        "en",
        "text",
    );
    click_editor(&mut visual, &view);
    let mut handler = visual
        .update(|window, _| window.visual_probe_take_input())
        .expect("focused native editor");
    handler.replace_and_mark_text_in_range(None, "中文,", Some(3..3));
    visual.update(|window, _| window.visual_probe_restore_input(handler));
    ready(&mut visual, &view);
    assert!(text_of(&mut visual, &view, "state").starts_with("0:"));
    let mut handler = visual
        .update(|window, _| window.visual_probe_take_input())
        .expect("retained native editor");
    assert_eq!(handler.marked_text_range(), Some(0..3));
    handler.unmark_text();
    visual.update(|window, _| window.visual_probe_restore_input(handler));
    ready(&mut visual, &view);
    assert_eq!(
        text_of(&mut visual, &view, "state"),
        r#"1:["seed@example.org", "中文"]"#
    );
}

#[gpui::test]
fn tag_input_reveals_the_same_editor_when_paste_fills_the_scrolling_well(cx: &mut TestAppContext) {
    let app = APP
        .replace("max:4", "max:128")
        .replace("__ACCEPT__", "true")
        .replace("__LOCALE__", "en")
        .replace("__VALIDATION__", "text");
    let (mut visual, view) = Mount::components(app)
        .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
        .motion(MotionPreference::None)
        .mount(cx);
    visual.update(|w, _| w.activate_window());
    visual.simulate_resize(size(px(302.), px(460.)));
    ready(&mut visual, &view);
    click_editor(&mut visual, &view);
    let batch = (0..30)
        .map(|index| format!("deployment area {index:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    paste(&mut visual, &view, &batch);
    let editor = bounds(&mut visual, &view, "text_field", "Tags");
    let following = bounds(&mut visual, &view, "text_field", "Following field");
    assert!(
        editor.y > 30.0 && editor.y + editor.height <= following.y && following.y < 255.0,
        "editor {editor:?}, following {following:?}"
    );
    visual.simulate_input("still focused,");
    ready(&mut visual, &view);
    let state = text_of(&mut visual, &view, "state");
    assert!(
        state.starts_with("2:") && state.contains("\"still focused\""),
        "{state}"
    );
}
