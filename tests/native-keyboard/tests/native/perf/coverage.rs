//! Cache islands reach the cold content of the containers applications fill:
//! a cold wrapper that sets its own height (a `flex_grow` pane, a clipped
//! body) does not take its content out of the islands, and the panes of a
//! horizontal SplitPane anchor their widths. Each case runs in two windows,
//! one rendering every frame in full, and their frames stay identical.

use gpui::{TestAppContext, px, size};

use crate::support::{Lockstep, Mount, Step, by_test_id, dispatch, settle};

/// A count above `body`, so a click renders the view whole while `body`
/// renders the same.
fn script(imports: &str, body: &str) -> String {
    format!(
        r#"{imports}
fn state_schema(){{#{{fields:#{{
    count:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}},
    size:#{{schema:#{{type:"float"}},"default":#{{type:"float",value:0.4}}}}}}}}}}
fn bump(ctx,payload){{ctx.set_state("count",ctx.get_state("count")+1);}}
fn widen(ctx,payload){{ctx.set_state("size",0.6);}}
fn noop(ctx,payload){{()}}
fn rows(prefix){{
    let result=[];
    for i in 0..20{{result.push(row([text(`${{prefix}} ${{i}}`),text("detail")]).with_key(`${{prefix}}${{i}}`));}}
    result
}}
fn view(ctx){{
    column([
        text(`Count ${{ctx.get_state("count")}}`).test_id("count"),
        text("Bump").test_id("bump").on_click(Fn("bump")),
        text("Widen").test_id("widen").on_click(Fn("widen")),
        {body},
    ]).with_style(style().width(relative(1.0)).height(relative(1.0)))
}}"#
    )
}

fn sides(cx: &mut TestAppContext, mount: impl Fn() -> Mount) -> Lockstep {
    cx.update(gpui_rhai::install);
    let host_sized = |mount: Mount| mount.config(|config| config.host_sets_width(true));
    Lockstep::mount(
        cx,
        host_sized(mount()).config(|config| config.render_islands(false)),
        host_sized(mount()),
        size(px(900.0), px(700.0)),
    )
}

/// Clicks `target` in both windows and returns window B's statistics; the
/// windows must draw the same.
fn click(sides: &mut Lockstep, target: &str) -> gpui_rhai::ScriptViewRenderStatistics {
    let mut statistics = Vec::new();
    sides.act(|visual, view| {
        let _ = visual.update(|_, cx| view.take_render_statistics(cx));
        dispatch(visual, view, by_test_id(target), "click").unwrap();
        settle(visual, Step::Tick, 2);
        statistics.push(
            visual
                .update(|_, cx| view.take_render_statistics(cx))
                .unwrap(),
        );
    });
    sides.assert_same(&format!("after {target}"));
    let b = statistics.pop().unwrap();
    assert_eq!(b.island_drift, 0, "{target}: {b:?}");
    b
}

#[track_caller]
fn assert_replays(sides: &mut Lockstep, islands: u64) {
    click(sides, "bump");
    let statistics = click(sides, "bump");
    assert!(
        statistics.island_hits >= islands,
        "fewer than {islands} islands replay: {statistics:?}"
    );
}

#[gpui::test]
fn a_cold_growing_wrapper_keeps_its_rows_in_an_island(cx: &mut TestAppContext) {
    let source = script(
        "",
        r#"column([column(rows("Row")).with_key("rows")]).with_key("fill")
            .with_style(style().flex_grow().min_height(px(0)))"#,
    );
    let mut sides = sides(cx, || Mount::new(source.clone()));
    sides.assert_same("mounted");
    assert_replays(&mut sides, 1);
}

#[gpui::test]
fn a_cold_clipped_wrapper_keeps_its_rows_in_an_island(cx: &mut TestAppContext) {
    let source = script(
        "",
        r#"column([column(rows("Row")).with_key("rows")]).with_key("body")
            .with_style(style().flex_grow().min_height(px(0)).clip())"#,
    );
    let mut sides = sides(cx, || Mount::new(source.clone()));
    sides.assert_same("mounted");
    assert_replays(&mut sides, 1);
}

#[gpui::test]
fn both_panes_of_a_horizontal_split_pane_hold_islands(cx: &mut TestAppContext) {
    let source = script(
        r#"import "components/split_pane" as split_pane;"#,
        r#"split_pane::SplitPane(#{key:"split",label:"Panes",orientation:"horizontal",
            size:ctx.get_state("size"),start_key:"start",end_key:"end",
            start:column(rows("Start")).with_key("start-rows"),
            end:column(rows("End")).with_key("end-rows"),on_resize:Fn("noop")})
            .with_style(style().flex_grow().min_height(px(0)))"#,
    );
    let mut sides = sides(cx, || Mount::components(source.clone()));
    sides.assert_same("mounted");
    assert_replays(&mut sides, 2);
    // A new size commits the panes at other widths: they measure again and
    // replay at those.
    click(&mut sides, "widen");
    assert_replays(&mut sides, 2);
}

#[gpui::test]
fn an_auto_grid_settles_its_tracks_before_its_island_is_pinned(cx: &mut TestAppContext) {
    // A fixed 600px grid lays out at an estimated width (the viewport's) on
    // its first frame and at its own the next: the island measures the second.
    let source = script(
        "",
        r#"box(rows("Tile")).with_key("grid")
            .with_style(style().width(px(600)).auto_grid_columns(128, 160, 224))"#,
    );
    let mut sides = sides(cx, || Mount::new(source.clone()));
    sides.assert_same("mounted");
    sides.act(|visual, view| {
        let mounted = visual
            .update(|_, cx| view.take_render_statistics(cx))
            .unwrap();
        assert_eq!(mounted.island_drift, 0, "mounted: {mounted:?}");
    });
    assert_replays(&mut sides, 1);
}
