//! Two windows of the same script stay frame-for-frame identical through
//! scrolling, selection, keyboard focus and hover. Both run the same render
//! path today; a faster path runs in window B against the plain path in A.

use gpui::{Modifiers, ScrollDelta, ScrollWheelEvent, TestAppContext, point, px, size};

use crate::support::{Lockstep, Mount, Step, bounds, center, settle};

/// A 400-row Table followed by 330 rows of plain filler.
const TABLE_AND_FILLER: &str = r#"import "components/table" as table;
fn state_schema(){#{fields:#{selected:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}}
fn select(ctx,payload){ctx.set_state("selected","r3");}
fn rows(){let result=[];for index in 0..400{result.push(#{id:`r${index}`,label:`Row ${index}`});}result}
fn fillers(){let result=[];for i in 0..330{result.push(row([text(`Filler ${i}`),text("a"),text("b")]).with_key(`f${i}`));}result}
fn view(ctx){
    let selected=ctx.get_state("selected");
    column([
        table::Table(#{key:"rows",label:"Rows",row_key:"id",height:320,rows:rows(),
            columns:[#{key:"label",title:"Label",width:#{kind:"fixed",value:200}}],
            selection_mode:"single",selected_keys:if selected=="" {[]} else {[selected]}}),
        text("Select").accessibility_role("button").accessibility_label("Select").on_click(Fn("select")),
        column(fillers()),
    ]).with_style(style().width(px(400)))
}"#;

fn table_mount() -> Mount {
    Mount::new(TABLE_AND_FILLER)
        .registry("components/table")
        .registry("components/badge")
        .bundled_assets()
}

#[gpui::test]
fn scrolling_and_selecting_a_table_keeps_the_windows_identical(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut sides = Lockstep::mount(cx, table_mount(), table_mount(), size(px(900.0), px(700.0)));
    sides.assert_same("mounted");
    let mut at = None;
    sides.act(|visual, view| {
        at.get_or_insert_with(|| center(bounds(visual, view, "table", "Rows")));
    });
    let at = at.unwrap();
    for step in 0..40 {
        sides.act(|visual, _| {
            visual.simulate_event(ScrollWheelEvent {
                position: at,
                delta: ScrollDelta::Pixels(point(
                    px(0.0),
                    px(if step < 30 { -32.0 } else { 48.0 }),
                )),
                ..ScrollWheelEvent::default()
            });
            settle(visual, Step::Tick, 2);
        });
        sides.assert_same(&format!("scroll step {step}"));
    }
    sides.act(|visual, view| {
        let select = bounds(visual, view, "button", "Select");
        visual.simulate_click(center(select), Modifiers::none());
        settle(visual, Step::Tick, 2);
    });
    sides.assert_same("selected a row");
}

const CONTROLS: &str = r#"import "components/button" as button;
import "components/checkbox" as checkbox;
import "components/tabs" as tabs;
fn state_schema(){#{fields:#{tab:#{schema:#{type:"string"},"default":#{type:"string",value:"one"}}}}}
fn noop(ctx,payload){()}
fn pick(ctx,payload){ctx.set_state("tab",payload.value);}
fn view(ctx){
    column([
        button::Button(#{key:"save",text:"Save",variant:"primary",on_click:Fn("noop")}),
        button::Button(#{key:"open",text:"Open",variant:"ghost",on_click:Fn("noop")}),
        checkbox::Checkbox(#{checked:false,label:"Agree",on_change:Fn("noop")}),
        tabs::Tabs(#{key:"tabs",label:"Sections",value:ctx.get_state("tab"),on_change:Fn("pick"),
            tabs:[#{value:"one",label:"One",content:text("First")},#{value:"two",label:"Two",content:text("Second")}]}),
    ]).with_style(style().gap(px(8)).padding(px(16)))
}"#;

#[gpui::test]
fn focus_and_hover_keep_the_windows_identical(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut sides = Lockstep::mount(
        cx,
        Mount::components(CONTROLS),
        Mount::components(CONTROLS),
        size(px(600.0), px(400.0)),
    );
    sides.assert_same("mounted");
    sides.act(|visual, _| {
        visual.update(|window, _| window.activate_window());
        settle(visual, Step::Tick, 1);
    });
    for press in 0..5 {
        sides.act(|visual, _| {
            visual.simulate_keystrokes("tab");
            settle(visual, Step::Tick, 1);
        });
        sides.assert_same(&format!("tab press {press}"));
    }
    let mut targets = Vec::new();
    sides.act(|visual, view| {
        if targets.is_empty() {
            targets = ["Save", "Open"]
                .into_iter()
                .map(|name| center(bounds(visual, view, "button", name)))
                .collect();
        }
    });
    for (index, target) in targets.iter().enumerate() {
        sides.act(|visual, _| {
            visual.simulate_mouse_move(*target, None, Modifiers::none());
            settle(visual, Step::Tick, 1);
        });
        sides.assert_same(&format!("hover {index}"));
    }
    sides.act(|visual, view| {
        let tab = bounds(visual, view, "tab", "Two");
        visual.simulate_click(center(tab), Modifiers::none());
        settle(visual, Step::Tick, 2);
    });
    sides.assert_same("picked a tab");
}
