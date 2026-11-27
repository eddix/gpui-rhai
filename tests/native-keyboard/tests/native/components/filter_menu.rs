//! Native selection, overflow, keyboard routing and real child bounds.
use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{MotionPreference, ScriptViewHandle, UiValue};

use crate::support::{Mount, Step, bounds, click_center, last_error, settle, snapshot, text_of};

const APP: &str = r#"
import "components/filter_menu" as filters;
fn state_schema(){#{fields:#{
    applied:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    calls:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
}}}
fn changed(ctx,v){
    if __ACCEPT__{ctx.set_state("applied",v);}
    ctx.set_state("calls",ctx.get_state("calls")+1);
}
fn groups(){[
    #{id:"a/b",label:"Alpha",chip_prefix:"Alpha ",options:[
        #{id:"same",label:"A fast"},#{id:"locked",label:"Locked",disabled:true},
        #{id:"second",label:"A very long second option"}]},
    #{id:"a:b",label:"Beta",chip_prefix:"Beta ",options:[
        #{id:"same",label:"B fast"},#{id:"second",label:"B very long second option"}]}
]}
fn view(ctx){
    let applied=ctx.get_state("applied");
    if applied==(){applied=__INITIAL__;}
    column([
        filters::FilterMenu(#{key:"filters",title:"Filter test",groups:groups(),value:applied,on_change:Fn("changed")}),
        text(`${ctx.get_state("calls")}:${applied}`).test_id("state"),
    ]).with_style(style().width(relative(1.0)).padding(px(12)));
}
"#;

fn mount(
    cx: &mut TestAppContext,
    theme: &str,
    accept: bool,
    initial: &str,
) -> (VisualTestContext, ScriptViewHandle) {
    let app = APP
        .replace("__ACCEPT__", if accept { "true" } else { "false" })
        .replace("__INITIAL__", initial);
    let add_modern = theme != gpui_rhai_registry::MODERN_DARK_THEME;
    let (mut visual, view) = Mount::components(app)
        .theme(theme)
        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
        .motion(MotionPreference::None)
        .with(move |view| {
            if add_modern {
                view.theme_sources([(
                    "modern_dark.rhai".into(),
                    gpui_rhai_registry::MODERN_DARK_THEME.into(),
                )])
            } else {
                view
            }
        })
        .mount(cx);
    visual.update(|w, _| w.activate_window());
    ready(&mut visual, &view);
    (visual, view)
}

fn ready(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    settle(visual, Step::Tick, 5);
    assert_eq!(last_error(visual, view), None);
}

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let target = bounds(visual, view, role, name);
    click_center(visual, target);
    ready(visual, view);
}

fn key(visual: &mut VisualTestContext, view: &ScriptViewHandle, keys: &str) {
    for step in keys.split_ascii_whitespace() {
        visual.simulate_keystrokes(step);
        ready(visual, view);
    }
}

fn checked(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) -> bool {
    snapshot(visual, view)
        .find_by_role_and_name("checkbox", name)
        .next()
        .unwrap_or_else(|| panic!("missing checkbox {name}"))
        .checked
        == Some(UiValue::Bool(true))
}

#[gpui::test]
fn filter_menu_roves_visible_options_and_qualifies_shared_ids(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        gpui_rhai_registry::MODERN_DARK_THEME,
        true,
        r#"#{ "a/b":["locked"] }"#,
    );
    click(&mut visual, &view, "checkbox", "A fast");
    assert!(checked(&mut visual, &view, "A fast"));
    key(&mut visual, &view, "down space");
    assert!(checked(&mut visual, &view, "A very long second option"));
    key(&mut visual, &view, "left");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "checkbox" && n.name == "A fast")
    );
    key(&mut visual, &view, "down right down enter");
    assert!(
        checked(&mut visual, &view, "B fast"),
        "{}, focus {:?}",
        text_of(&mut visual, &view, "state"),
        visual.update(|w, cx| w.focused(cx))
    );
    let state = text_of(&mut visual, &view, "state");
    assert!(
        state.starts_with("3:") && state.contains("\"a:b\": [\"same\"]"),
        "{state}"
    );
    click(&mut visual, &view, "button", "Remove Alpha A fast");
    assert!(checked(&mut visual, &view, "B fast"));
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Remove Alpha A fast")
    );
    click(&mut visual, &view, "button", "Clear all");
    let state = text_of(&mut visual, &view, "state");
    assert!(
        state.starts_with("5:") && state.contains("\"locked\"") && !state.contains("\"same\""),
        "{state}"
    );
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Clear all" && n.disabled)
    );
}

#[gpui::test]
fn filter_menu_rejected_controlled_edits_preserve_the_presented_selection(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, gpui_rhai_registry::CLEAN_LIGHT_THEME, false, "#{}");
    click(&mut visual, &view, "checkbox", "A fast");
    assert!(!checked(&mut visual, &view, "A fast"));
    let state = text_of(&mut visual, &view, "state");
    assert!(
        state.starts_with("1:"),
        "{state}; geometry: {:?}; focus: {:?}",
        snapshot(&mut visual, &view)
            .find_by_role_and_name("checkbox", "A fast")
            .next()
            .unwrap()
            .geometry,
        visual.update(|w, cx| w.focused(cx))
    );
    key(&mut visual, &view, "enter");
    assert!(!checked(&mut visual, &view, "A fast"));
    assert!(text_of(&mut visual, &view, "state").starts_with("2:"));
}

fn shown_chips(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> (Vec<f64>, usize) {
    let tree = snapshot(visual, view);
    let positions = tree
        .nodes()
        .filter(|n| n.role == "button" && n.name.starts_with("Remove "))
        .map(|n| n.geometry.expect("visible remove geometry").visual.y)
        .collect::<Vec<_>>();
    let summaries = tree
        .nodes()
        .filter_map(|n| n.name.strip_suffix(" more filters")?.parse::<usize>().ok())
        .collect::<Vec<_>>();
    assert!(summaries.len() <= 1, "{summaries:?}");
    (positions, summaries.first().copied().unwrap_or(0))
}

#[gpui::test]
fn filter_menu_clean_counts_real_overflow_while_modern_wraps_the_same_selection(
    cx: &mut TestAppContext,
) {
    let data = r#"#{ "a/b":["same","second"], "a:b":["same","second"] }"#;
    let (mut visual, view) = mount(cx, gpui_rhai_registry::CLEAN_DARK_THEME, true, data);
    visual.simulate_resize(size(px(540.), px(700.)));
    ready(&mut visual, &view);
    let (wide, wide_count) = shown_chips(&mut visual, &view);
    assert_eq!(wide.len() + wide_count, 4);
    visual.simulate_resize(size(px(232.), px(600.)));
    ready(&mut visual, &view);
    let (narrow, count) = shown_chips(&mut visual, &view);
    assert!(
        count > wide_count && narrow.len() + count == 4,
        "{narrow:?}/{count}"
    );
    assert!(narrow.windows(2).all(|y| (y[0] - y[1]).abs() < 1.0));
    visual
        .update(|_, cx| view.select_theme("Modern", "Dark", cx))
        .unwrap();
    ready(&mut visual, &view);
    let (wrapped, count) = shown_chips(&mut visual, &view);
    assert_eq!((wrapped.len(), count), (4, 0));
    assert!(
        wrapped.windows(2).any(|y| (y[0] - y[1]).abs() > 10.0),
        "{wrapped:?}"
    );
    assert!(text_of(&mut visual, &view, "state").starts_with("0:"));
}

#[gpui::test]
fn filter_menu_scrolls_to_focused_option_and_accepts_local_selection(cx: &mut TestAppContext) {
    let app = r#"
        import "components/filter_menu" as filters;
        fn view(ctx){
            let items=[];
            for i in 0..40{items.push(#{id:`${i}`,label:`Choice ${i}`});}
            filters::FilterMenu(#{key:"long",title:"Long filters",hints:false,groups:[
                #{id:"all",label:"Everything",options:items}
            ]})
        }
    "#;
    let (mut visual, view) = Mount::components(app)
        .theme(gpui_rhai_registry::CLEAN_LIGHT_THEME)
        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
        .motion(MotionPreference::None)
        .mount(cx);
    visual.update(|w, _| w.activate_window());
    visual.simulate_resize(size(px(460.), px(590.)));
    ready(&mut visual, &view);
    let start = bounds(&mut visual, &view, "checkbox", "Choice 0");
    let last = bounds(&mut visual, &view, "checkbox", "Choice 39");
    assert!(last.y > start.y + 800.0);
    click(&mut visual, &view, "checkbox", "Choice 0");
    assert!(checked(&mut visual, &view, "Choice 0"));
    key(&mut visual, &view, "end");
    let revealed = bounds(&mut visual, &view, "checkbox", "Choice 39");
    let viewport = bounds(&mut visual, &view, "group", "Everything");
    assert!(
        viewport.height <= 320.5
            && revealed.y >= viewport.y
            && revealed.y + revealed.height <= viewport.y + viewport.height + 1.0,
        "viewport {viewport:?}, revealed {revealed:?}"
    );
    key(&mut visual, &view, "space");
    assert!(checked(&mut visual, &view, "Choice 39"));
    key(&mut visual, &view, "left");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "checkbox")
    );
    key(&mut visual, &view, "right");
    assert!(checked(&mut visual, &view, "Choice 39"));
}
