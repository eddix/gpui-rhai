//! Array-backed Table and Combobox in `multiple` mode: the row data stays linear in the rows
//! however many of them are selected, and a click (or Enter) still reports the next
//! selection. Both shared the precomputed per-row selection of round-4 U2 (List).

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext};
use gpui_rhai::*;

use crate::support::{Mount, Step, bounds, center, last_error, text_of};

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(source).id("table").mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Tick, 1);
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    text_of(visual, view, "log").trim().to_owned()
}

fn press(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let position = center(bounds(visual, view, role, name));
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::none());
    settle(visual);
}

/// 500 rows r0.. with two columns in `multiple` mode, the selection in state starting at the
/// first `initial` rows; "Many" selects r0..r399. The log records each next selection's size,
/// whether it holds r2, and its last key.
fn script(initial: usize) -> String {
    format!(
        r#"import "components/table" as table;
fn state_schema(){{#{{fields:#{{
    log:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}},
    selected:#{{schema:#{{type:"optional",value:#{{type:"array",max_items:10000,items:#{{type:"string"}}}}}},
        "default":#{{type:"null"}}}}}}}}}}
fn note(ctx,line){{ctx.set_state("log",`${{ctx.get_state("log")}} ${{line}}`);}}
fn first(count){{let keys=[];for i in 0..count{{keys.push(`r${{i}}`);}}keys}}
fn selected(ctx,keys){{ctx.set_state("selected",keys);
    note(ctx,`sel:${{keys.len}}:${{"r2" in keys}}:${{if keys.len==0 {{"-"}} else {{keys[keys.len-1]}}}}`);}}
fn many(ctx,payload){{ctx.set_state("selected",first(400));note(ctx,"many");}}
fn view(ctx){{
    let rows=[];
    for i in 0..500{{rows.push(#{{id:`r${{i}}`,label:`Row ${{i}}`,group:"g"}});}}
    let chosen=ctx.get_state("selected");
    column([
        text("Many").accessibility_role("button").accessibility_label("Many").on_click(Fn("many"))
            .with_style(style().width(px(80)).height(px(24))),
        table::Table(#{{key:"rows",label:"Rows",row_key:"id",height:200,rows:rows,
            columns:[#{{key:"label",title:"Label",width:#{{kind:"fixed",value:160}}}},
                #{{key:"group",title:"Group",width:#{{kind:"fixed",value:80}}}}],
            selection_mode:"multiple",selected_keys:if chosen == () {{ first({initial}) }} else {{ chosen }},
            on_selection_change:Fn("selected")}}),
        text(ctx.get_state("log")).test_id("log"),
    ]).with_style(style().width(px(400)))
}}"#
    )
}

#[gpui::test]
fn table_500_unselected_rows_control(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(0));
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "{error:?}");
}

#[gpui::test]
fn table_500_rows_with_20_selected_still_renders(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(20));
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "{error:?}");
}

#[gpui::test]
fn table_500_selected_rows_still_render(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(500));
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "{error:?}");
}

#[gpui::test]
fn table_clicks_build_the_next_selection_from_few_to_many_selected_rows(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(1));
    press(&mut visual, &view, "row", "Rows row 2");
    assert_eq!(log(&mut visual, &view), "sel:2:false:r1");
    press(&mut visual, &view, "button", "Many");
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "400 of 500 rows selected: {error:?}");
    assert_eq!(log(&mut visual, &view), "sel:2:false:r1 many");
    // A click on a selected row drops it; a second click appends it again.
    press(&mut visual, &view, "row", "Rows row 3");
    press(&mut visual, &view, "row", "Rows row 3");
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "{error:?}");
    assert_eq!(
        log(&mut visual, &view),
        "sel:2:false:r1 many sel:399:false:r399 sel:400:true:r2"
    );
}

/// 500 options o0.. in an open `multiple` Combobox, the selection in state starting at the
/// first `initial` options; "Many" selects o0..o399. The log records each next selection's
/// size, whether it holds o2, and its last value.
fn combobox_script(initial: usize) -> String {
    format!(
        r#"import "components/combobox" as combobox;
fn state_schema(){{#{{fields:#{{
    log:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}},
    selected:#{{schema:#{{type:"optional",value:#{{type:"array",max_items:10000,items:#{{type:"string"}}}}}},
        "default":#{{type:"null"}}}}}}}}}}
fn note(ctx,line){{ctx.set_state("log",`${{ctx.get_state("log")}} ${{line}}`);}}
fn first(count){{let values=[];for i in 0..count{{values.push(`o${{i}}`);}}values}}
fn changed(ctx,values){{ctx.set_state("selected",values);
    note(ctx,`sel:${{values.len}}:${{"o2" in values}}:${{if values.len==0 {{"-"}} else {{values[values.len-1]}}}}`);}}
fn many(ctx,payload){{ctx.set_state("selected",first(400));note(ctx,"many");}}
fn view(ctx){{
    let options=[];
    for i in 0..500{{options.push(#{{value:`o${{i}}`,label:`Option ${{i}}`}});}}
    let chosen=ctx.get_state("selected");
    column([
        text("Many").accessibility_role("button").accessibility_label("Many").on_click(Fn("many"))
            .with_style(style().width(px(80)).height(px(24))),
        combobox::Combobox(#{{key:"many",label:"Options",mode:"multiple",options:options,
            selected:if chosen == () {{ first({initial}) }} else {{ chosen }},
            open:true,query:"",on_change:Fn("changed")}}),
        text(ctx.get_state("log")).test_id("log"),
    ]).with_style(style().width(px(400)).height(px(400)))
}}"#
    )
}

#[gpui::test]
fn combobox_500_options_with_20_selected_still_renders(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, combobox_script(20));
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "{error:?}");
}

#[gpui::test]
fn combobox_picks_build_the_next_selection_from_few_to_many_selected_options(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount(cx, combobox_script(1));
    press(&mut visual, &view, "option", "Option 1");
    assert_eq!(log(&mut visual, &view), "sel:2:false:o1");
    press(&mut visual, &view, "button", "Many");
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "400 of 500 options selected: {error:?}");
    assert_eq!(log(&mut visual, &view), "sel:2:false:o1 many");
    // A pick of a selected option drops it; Enter on the active option (the pick made o2
    // active and left focus in the panel) appends it again.
    press(&mut visual, &view, "option", "Option 2");
    visual.simulate_keystrokes("enter");
    settle(&mut visual);
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "{error:?}");
    assert_eq!(
        log(&mut visual, &view),
        "sel:2:false:o1 many sel:399:false:o399 sel:400:true:o2"
    );
}
