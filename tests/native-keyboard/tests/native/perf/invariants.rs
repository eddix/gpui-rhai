//! Counters that keep a frame's cost tied to what changed: a frame after no
//! commit does not walk the committed tree.

use gpui::{ScrollDelta, ScrollWheelEvent, TestAppContext, point, px, size};

use crate::support::{Mount, Step, bounds, center, settle};

/// A 400-row Table followed by 330 rows of plain filler.
const TABLE_AND_FILLER: &str = r#"import "components/table" as table;
fn rows(){let result=[];for index in 0..400{result.push(#{id:`r${index}`,label:`Row ${index}`});}result}
fn fillers(){let result=[];for i in 0..330{result.push(row([text(`Filler ${i}`),text("a"),text("b")]).with_key(`f${i}`));}result}
fn view(ctx){
    column([
        table::Table(#{key:"rows",label:"Rows",row_key:"id",height:320,rows:rows(),
            columns:[#{key:"label",title:"Label",width:#{kind:"fixed",value:200}}]}),
        column(fillers()),
    ]).with_style(style().width(px(400)))
}"#;

#[gpui::test]
fn only_frames_after_a_commit_walk_the_committed_tree(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (mut visual, view) = Mount::new(TABLE_AND_FILLER)
        .registry("components/table")
        .registry("components/badge")
        .bundled_assets()
        .mount(cx);
    visual.simulate_resize(size(px(900.0), px(700.0)));
    settle(&mut visual, Step::Tick, 2);
    let at = center(bounds(&mut visual, &view, "table", "Rows"));
    let _ = visual
        .update(|_, cx| view.take_render_statistics(cx))
        .unwrap();
    let (mut quiet, mut committing) = (0, 0);
    for step in 0..40 {
        visual.simulate_event(ScrollWheelEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-32.0))),
            ..ScrollWheelEvent::default()
        });
        settle(&mut visual, Step::Tick, 2);
        let statistics = visual
            .update(|_, cx| view.take_render_statistics(cx))
            .unwrap();
        assert!(statistics.host_renders > 0, "step {step} rendered no frame");
        if statistics.commits == 0 {
            quiet += 1;
            assert_eq!(
                statistics.tree_walks, 0,
                "step {step} walked the tree without a commit"
            );
        } else {
            committing += 1;
            assert!(
                statistics.tree_walks > 0,
                "step {step} kept walks of an older tree"
            );
        }
    }
    assert!(
        quiet >= 30 && committing >= 1,
        "{quiet} quiet and {committing} committing steps"
    );
}

#[gpui::test]
fn quiet_scroll_frames_replay_the_cold_filler(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (mut visual, view) = Mount::new(TABLE_AND_FILLER)
        .registry("components/table")
        .registry("components/badge")
        .bundled_assets()
        .mount(cx);
    visual.simulate_resize(size(px(900.0), px(700.0)));
    settle(&mut visual, Step::Tick, 2);
    let at = center(bounds(&mut visual, &view, "table", "Rows"));
    let _ = visual
        .update(|_, cx| view.take_render_statistics(cx))
        .unwrap();
    let mut quiet = 0;
    for step in 0..20 {
        visual.simulate_event(ScrollWheelEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-32.0))),
            ..ScrollWheelEvent::default()
        });
        settle(&mut visual, Step::Tick, 2);
        let statistics = visual
            .update(|_, cx| view.take_render_statistics(cx))
            .unwrap();
        assert_eq!(statistics.island_drift, 0, "step {step}");
        if statistics.commits == 0 {
            quiet += 1;
            assert!(
                statistics.island_hits >= statistics.host_renders
                    && statistics.island_misses == 0
                    && statistics.island_uncached == 0,
                "step {step} rendered the filler again: {statistics:?}"
            );
        }
    }
    assert!(quiet >= 15, "{quiet} quiet steps");
}
