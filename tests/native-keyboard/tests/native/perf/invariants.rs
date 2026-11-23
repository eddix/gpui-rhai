//! Counters that keep a frame's cost tied to what changed: a frame after no
//! commit does not walk the committed tree.

use gpui::{ScrollDelta, ScrollEvent, TestAppContext, point, px, size};

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
        .registry("components/selectable_list")
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
        visual.simulate_event(ScrollEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-32.0))),
            ..ScrollEvent::default()
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
#[cfg_attr(feature = "gpui-fast", ignore = "cache islands are off on gpui-fast")]
fn quiet_scroll_frames_replay_the_cold_filler(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (mut visual, view) = Mount::new(TABLE_AND_FILLER)
        .registry("components/table")
        .registry("components/selectable_list")
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
        visual.simulate_event(ScrollEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-32.0))),
            ..ScrollEvent::default()
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
#[cfg(feature = "gpui-fast")]
#[gpui::test]
fn repeating_fill_period_changes_the_promoted_tiles_without_moving_the_scroll(
    cx: &mut gpui::TestAppContext,
) {
    use gpui::{
        BackgroundLayer, BackgroundStop, Context, InteractiveElement, IntoElement,
        LayeredBackground, ParentElement, Render, ScrollDelta, ScrollEvent, ScrollHandle,
        StatefulInteractiveElement, Styled, TouchPhase, VisualTestContext, Window, div, point, px,
        rgb, rgba, size,
    };

    struct RepeatPage {
        period: f32,
        scroll: ScrollHandle,
    }
    impl Render for RepeatPage {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let mut sheet = div().w(px(220.)).h(px(1600.)).flex_shrink_0();
            let layer = BackgroundLayer::new(
                135.,
                vec![
                    BackgroundStop {
                        offset: 0.,
                        color: rgba(0xc0607099),
                    },
                    BackgroundStop {
                        offset: 1.,
                        color: rgba(0x2bad78cc),
                    },
                ],
            )
            .repeating(Some(self.period));
            sheet.style().background = Some(LayeredBackground::new(vec![layer]).unwrap().into());
            div()
                .id("repeat-scroller")
                .w(px(220.))
                .h(px(180.))
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .bg(rgb(0x202025))
                .child(sheet)
        }
    }
    let scroll = ScrollHandle::new();
    let handle = scroll.clone();
    let window = cx.add_window(move |_, _| RepeatPage {
        period: 7.,
        scroll: handle,
    });
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(280.), px(240.)));
    settle(&mut visual, Step::Frame, 2);

    let layer = |v: &mut VisualTestContext| {
        v.update(|window, _| {
            window
                .painted_scene()
                .layers
                .frames
                .iter()
                .find(|frame| {
                    frame.tile_size > 0
                        && frame.content.scenes().any(|scene| {
                            scene.layered_fills.iter().any(|fill| {
                                fill.layers
                                    .iter()
                                    .any(|layer| layer.repeat_period.is_some())
                            })
                        })
                })
                .map(|frame| (frame.key, frame.generation, frame.dirty_tiles.len()))
        })
    };
    for _ in 0..8 {
        visual.simulate_event(ScrollEvent {
            position: point(px(40.), px(40.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-20.))),
            touch_phase: TouchPhase::Moved,
            ..ScrollEvent::default()
        });
        settle(&mut visual, Step::Frame, 1);
        if layer(&mut visual).is_some() {
            break;
        }
    }
    let initial = layer(&mut visual).expect("real Fast scroll promotes the repeating fill");
    let offset = scroll.offset();
    assert!(offset.y < px(0.));
    let repaint = |visual: &mut VisualTestContext, period: f32| {
        window
            .update(visual, |page, _, cx| {
                page.period = period;
                cx.notify();
            })
            .unwrap();
        settle(visual, Step::Frame, 1);
        layer(visual).expect("scroll layer remains promoted after repaint")
    };
    let same = repaint(&mut visual, 7.);
    assert_eq!(initial.0, same.0);
    assert!(
        same.1 == initial.1 || same.2 == 0,
        "equal paint has no new dirty tiles"
    );
    let changed = repaint(&mut visual, 17.);
    assert_eq!(same.0, changed.0);
    assert!(
        changed.1 > same.1 && changed.2 > 0,
        "changing only the repeated distance dirties tiles"
    );
    assert_eq!(scroll.offset(), offset);
}
