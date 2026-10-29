//! Two windows of the same script stay frame-for-frame identical through
//! scrolling, selection, keyboard focus and hover: window A renders every
//! frame in full, window B replays its cache islands.

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

fn full_frames(mount: Mount) -> Mount {
    mount.config(|config| config.render_islands(false))
}

/// Window B's island hits since the last call; A has none.
fn island_hits(sides: &mut Lockstep) -> u64 {
    let mut hits = Vec::new();
    sides.act(|visual, view| {
        let statistics = visual
            .update(|_, cx| view.take_render_statistics(cx))
            .unwrap();
        assert_eq!(statistics.island_drift, 0);
        hits.push(statistics.island_hits);
    });
    assert_eq!(hits[0], 0, "window A replayed an island");
    hits[1]
}

#[gpui::test]
fn scrolling_and_selecting_a_table_keeps_the_windows_identical(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut sides = Lockstep::mount(
        cx,
        full_frames(table_mount()),
        table_mount(),
        size(px(900.0), px(700.0)),
    );
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
    assert!(island_hits(&mut sides) > 30);
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
    ]).with_style(style().width(px(560)).gap(px(8)).padding(px(16)))
}"#;

#[gpui::test]
fn focus_and_hover_keep_the_windows_identical(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut sides = Lockstep::mount(
        cx,
        full_frames(Mount::components(CONTROLS)),
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

/// Cold rows with hover and focus styles, below a control that commits.
const STYLED_ROWS: &str = r#"fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn bump(ctx,payload){ctx.set_state("count",ctx.get_state("count")+1);}
fn rows(){let result=[];for i in 0..12{result.push(row([text(`Row ${i}`),text("detail")])
    .with_key(`r${i}`).tab_index(i).accessibility_role("listitem").accessibility_label(`Row ${i}`)
    .with_style(style().padding(px(4)).gap(px(8)).hover(style().background(rgba(0x3060ffff)))
        .focus(style().background(rgba(0x30c060ff)))));}result}
fn view(ctx){
    column([
        text(`Count ${ctx.get_state("count")}`).accessibility_role("button").accessibility_label("Bump").on_click(Fn("bump"))
            .with_style(style().hover(style().background(rgba(0xc03030ff)))),
        column(rows()).with_style(style().gap(px(2))),
    ]).with_style(style().width(px(360)).padding(px(8)))
}"#;

#[gpui::test]
fn hover_and_focus_inside_an_island_keep_the_windows_identical(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut sides = Lockstep::mount(
        cx,
        full_frames(Mount::new(STYLED_ROWS)),
        Mount::new(STYLED_ROWS),
        size(px(500.0), px(500.0)),
    );
    sides.assert_same("mounted");
    let _ = island_hits(&mut sides);
    let mut rows = Vec::new();
    sides.act(|visual, view| {
        if rows.is_empty() {
            rows = [1, 4, 7]
                .into_iter()
                .map(|row| center(bounds(visual, view, "listitem", &format!("Row {row}"))))
                .collect();
        }
    });
    for (index, row) in rows.iter().enumerate() {
        sides.act(|visual, _| {
            visual.simulate_mouse_move(*row, None, Modifiers::none());
            settle(visual, Step::Tick, 1);
        });
        sides.assert_same(&format!("hover row {index}"));
    }
    sides.act(|visual, _| {
        visual.update(|window, _| window.activate_window());
        settle(visual, Step::Tick, 1);
    });
    for press in 0..4 {
        sides.act(|visual, _| {
            visual.simulate_keystrokes("tab");
            settle(visual, Step::Tick, 1);
        });
        sides.assert_same(&format!("tab press {press}"));
    }
    sides.act(|visual, view| {
        let bump = bounds(visual, view, "button", "Bump");
        visual.simulate_click(center(bump), Modifiers::none());
        settle(visual, Step::Tick, 2);
    });
    sides.assert_same("committed above the rows");
    let _ = island_hits(&mut sides);
    // Hovering the control above from outside the rows repaints the view
    // around rows whose hover did not change.
    let outside = point(px(450.0), px(450.0));
    for index in 0..2 {
        sides.act(|visual, _| {
            visual.simulate_mouse_move(outside, None, Modifiers::none());
            settle(visual, Step::Tick, 1);
        });
        sides.assert_same(&format!("left the rows {index}"));
        sides.act(|visual, view| {
            let bump = center(bounds(visual, view, "button", "Bump"));
            visual.simulate_mouse_move(bump, None, Modifiers::none());
            settle(visual, Step::Tick, 1);
        });
        sides.assert_same(&format!("hover the control {index}"));
    }
    assert!(island_hits(&mut sides) > 0);
}

/// A scroll container beside a block of cold rows.
const SCROLL_BESIDE_ROWS: &str = r#"fn lines(){let result=[];for i in 0..60{result.push(text(`Line ${i}`).with_key(`l${i}`));}result}
fn rows(){let result=[];for i in 0..40{result.push(row([text(`Row ${i}`),text("detail")]).with_key(`r${i}`));}result}
fn view(ctx){
    column([
        column(lines()).with_key("scroller").accessibility_role("region").accessibility_label("Lines")
            .with_style(style().height(px(160)).overflow_y_scroll()).scrollbars("hidden", "always"),
        column(rows()).with_key("rows"),
    ]).with_style(style().width(px(360)))
}"#;

#[gpui::test]
fn scrolling_a_container_replays_the_islands_beside_it(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut sides = Lockstep::mount(
        cx,
        full_frames(Mount::new(SCROLL_BESIDE_ROWS)),
        Mount::new(SCROLL_BESIDE_ROWS),
        size(px(500.0), px(900.0)),
    );
    sides.assert_same("mounted");
    let _ = island_hits(&mut sides);
    // Over the scrollbar track, where the view's own wheel handler scrolls.
    let mut at = None;
    sides.act(|visual, view| {
        at.get_or_insert_with(|| {
            let region = bounds(visual, view, "region", "Lines");
            point(
                px((region.x + region.width) as f32 - 3.0),
                px((region.y + region.height / 2.0) as f32),
            )
        });
    });
    let at = at.unwrap();
    let mut hits = 0;
    for step in 0..12 {
        sides.act(|visual, _| {
            visual.simulate_event(ScrollWheelEvent {
                position: at,
                delta: ScrollDelta::Pixels(point(px(0.0), px(if step < 9 { -24.0 } else { 40.0 }))),
                ..ScrollWheelEvent::default()
            });
            settle(visual, Step::Tick, 2);
        });
        sides.assert_same(&format!("scroll step {step}"));
        hits += island_hits(&mut sides);
    }
    assert!(hits >= 10, "{hits} island hits over 12 scroll steps");
}

/// An application shell: a fixed sidebar beside a main area that grows into
/// the rest of the row, each holding cold rows, under a control that commits.
const SHELL: &str = r#"fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn bump(ctx,payload){ctx.set_state("count",ctx.get_state("count")+1);}
fn rows(label){let result=[];for i in 0..20{result.push(row([text(`${label} ${i}`),text("detail")]).with_key(`r${i}`));}result}
fn noop(ctx,payload){()}
fn view(ctx){
    column([
        text(`Count ${ctx.get_state("count")}`).accessibility_role("button").accessibility_label("Bump").on_click(Fn("bump"))
            .with_style(style().hover(style().background(rgba(0xc03030ff)))),
        row([
            column([column(rows("Nav")).with_key("nav")]).with_key("sidebar").on("pointer_down",Fn("noop"))
                .with_style(style().width(px(180)).flex_shrink(false)),
            column([column(rows("Item")).with_key("items")]).with_key("main").on("pointer_down",Fn("noop"))
                .with_style(style().flex_grow().min_width(px(0)).padding(px(8))),
        ]).with_key("shell").with_style(style().gap(px(4))),
    ]).with_style(style().width(px(640)))
}"#;

#[gpui::test]
fn a_shell_main_area_holds_islands_and_keeps_the_windows_identical(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut sides = Lockstep::mount(
        cx,
        full_frames(Mount::new(SHELL)),
        Mount::new(SHELL),
        size(px(800.0), px(900.0)),
    );
    sides.assert_same("mounted");
    let _ = island_hits(&mut sides);
    let outside = point(px(760.0), px(880.0));
    let mut hits = 0;
    for index in 0..3 {
        sides.act(|visual, _| {
            visual.simulate_mouse_move(outside, None, Modifiers::none());
            settle(visual, Step::Tick, 1);
        });
        sides.act(|visual, view| {
            let bump = center(bounds(visual, view, "button", "Bump"));
            visual.simulate_mouse_move(bump, None, Modifiers::none());
            settle(visual, Step::Tick, 1);
        });
        sides.assert_same(&format!("hover {index}"));
        hits += island_hits(&mut sides);
    }
    sides.act(|visual, view| {
        let bump = center(bounds(visual, view, "button", "Bump"));
        visual.simulate_click(bump, Modifiers::none());
        settle(visual, Step::Tick, 2);
    });
    sides.assert_same("committed");
    // The sidebar's rows and the main area's rows replay, two islands a frame.
    assert!(hits >= 10, "{hits} island hits over 3 hovers");
}
