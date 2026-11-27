//! One measured, virtualized list owns fit-to-content and bounded scrolling.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{AutomationCommand, MotionPreference, ScriptViewHandle, UiValue};

use crate::support::{Mount, Step, bounds, last_error, settle, snapshot};

const MAIN: &str = r#"
    fn state_schema(){#{fields:#{mode:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
    fn init(ctx){ctx.register_action("test.mode",Fn("mode"));}
    fn mode(ctx,value){ctx.set_state("mode",value);}
    fn row_view(ctx,payload){
        if payload.index==0 {return text("A heading that wraps at the actual available width")
            .with_style(style().font_size(px(12)).line_height(px(20)).padding_y(px(5)))
            .test_id("heading");}
        text(`Row ${payload.index}`).with_style(style().height(px(30)))
            .test_id(`row-${payload.index}`)
    }
    fn view(ctx){
        let mode=ctx.get_state("mode");
        let count=if mode==1 {5000}else if mode==2 {0}else{3};
        let rows=[];for n in 0..count {rows.push(#{key:`row-${n}`});}
        let padding=if mode==4 {18}else{0};
        column([
            virtual_collection(#{key:"bounded",label:"Bounded",data:rows,estimated_height:32,
                max_height:px(240),overdraw_pixels:64,content_padding_top:padding,
                content_padding_bottom:if mode==4 {20}else{0}},Fn("row_view"))
                .accessibility_role("list").accessibility_label("Bounded"),
            text("Footer").test_id("footer")
        ]).with_style(style().width(px(if mode==3 {100}else{300})).padding(px(10)).gap(px(0)))
    }
"#;

fn action(visual: &mut VisualTestContext, view: &ScriptViewHandle, mode: i64) {
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Action {
                    id: "test.mode".into(),
                    payload: Some(UiValue::Integer(mode)),
                },
                window,
                cx,
            )
        })
        .unwrap();
    settle(visual, Step::Frame, 8);
    assert!(last_error(visual, view).is_none());
}

#[gpui::test]
fn bounded_content_padding_fits_the_native_rows_without_creating_items(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::components(MAIN)
        .motion(MotionPreference::None)
        .mount(cx);
    settle(&mut v, Step::Frame, 8);
    let baseline = assert_fits(&mut v, &view);
    action(&mut v, &view, 4);
    let list = bounds(&mut v, &view, "list", "Bounded");
    let tree = snapshot(&mut v, &view);
    let geometry = |id| {
        tree.find_by_test_id(id)
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    };
    let first = geometry("heading");
    let last = geometry("row-2");
    assert!((list.height - baseline - 38.).abs() < 0.6, "{list:?}");
    assert!((first.y - list.y - 18.).abs() < 0.6, "{first:?}, {list:?}");
    assert!((list.y + list.height - last.y - last.height - 20.).abs() < 0.6);
    let metrics = v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
    assert_eq!(metrics.virtual_collections[0].item_count, 3);
    action(&mut v, &view, 0);
    assert!((assert_fits(&mut v, &view) - baseline).abs() < 0.6);
}

fn assert_fits(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> f64 {
    let tree = snapshot(visual, view);
    let heading = tree
        .find_by_test_id("heading")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let last = tree
        .find_by_test_id("row-2")
        .next()
        .and_then(|node| node.geometry)
        .unwrap_or_else(|| {
            panic!(
                "unpresented last row: {:?}",
                tree.nodes()
                    .map(|node| (&node.role, &node.name, &node.test_id, node.geometry))
                    .collect::<Vec<_>>()
            )
        })
        .visual;
    let footer = tree
        .find_by_test_id("footer")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let list = bounds(visual, view, "list", "Bounded");
    let total = last.y + last.height - heading.y;
    assert!(
        (list.height - total).abs() < 0.6,
        "exact measured total: {list:?}, {total}"
    );
    assert!(
        (footer.y - list.y - list.height).abs() < 0.6,
        "the following sibling follows the fitted viewport"
    );
    list.height
}

#[gpui::test]
fn bounded_content_fits_real_rows_reflows_and_returns_from_large_and_empty_data(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = Mount::components(MAIN)
        .motion(MotionPreference::None)
        .mount(cx);
    settle(&mut visual, Step::Frame, 8);
    let wide = assert_fits(&mut visual, &view);
    assert!(wide > 60.0 && wide < 240.0);
    action(&mut visual, &view, 3);
    let narrow = assert_fits(&mut visual, &view);
    assert!(narrow > wide, "the actual text wraps after narrowing");
    action(&mut visual, &view, 1);
    let list = bounds(&mut visual, &view, "list", "Bounded");
    assert!((list.height - 240.0).abs() < 0.6);
    let tree = snapshot(&mut visual, &view);
    assert!(
        tree.nodes().count() < 160,
        "the long collection stays virtualized, not eagerly measured"
    );
    let id = visual.update(|_, cx| {
        view.take_performance_snapshot(cx)
            .unwrap()
            .virtual_collections[0]
            .id
            .clone()
    });
    visual.update(|_, cx| {
        view.collection_scroll_handle(&id, cx)
            .unwrap()
            .unwrap()
            .scroll_to_end(cx)
    });
    settle(&mut visual, Step::Frame, 8);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_test_id("row-4999")
            .next()
            .and_then(|node| node.geometry)
            .is_some(),
        "the ordinary native scroll owner reaches the tail"
    );
    action(&mut visual, &view, 0);
    assert!(
        (assert_fits(&mut visual, &view) - wide).abs() < 0.6,
        "shrinking from a scrolled tail fits the whole short list"
    );
    action(&mut visual, &view, 2);
    assert!(
        bounds(&mut visual, &view, "list", "Bounded").height < 0.6,
        "empty content fits zero, not the cap"
    );
    action(&mut visual, &view, 0);
    assert!((assert_fits(&mut visual, &view) - wide).abs() < 0.6);
}
