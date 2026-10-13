//! `style().hidden()` takes a node out of layout and paint, a `row` or `column` included:
//! their flex direction used to turn display back on.

use gpui::TestAppContext;

use crate::support::{Mount, Step, settle, snapshot};

#[gpui::test]
fn hidden_rows_columns_and_boxes_take_no_room(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(
        r#"
fn probe(name, node) {
    node.with_style(style().width(px(40)).height(px(40)).hidden())
        .accessibility_role("group").accessibility_label(name)
}
fn view(ctx) {
    column([
        probe("box", box([text("box")])),
        probe("row", row([text("row")])),
        probe("column", column([text("column")])),
        text("After").accessibility_role("status").accessibility_label("After"),
    ])
}
"#,
    )
    .id("hidden")
    .mount(cx);
    settle(&mut visual, Step::Frame, 2);
    let tree = snapshot(&mut visual, &view);
    for name in ["box", "row", "column"] {
        let size = tree
            .find_by_role_and_name("group", name)
            .next()
            .and_then(|node| node.geometry)
            .map(|geometry| (geometry.visual.width, geometry.visual.height));
        assert!(
            size.is_none_or(|(width, height)| width == 0.0 && height == 0.0),
            "a hidden {name} takes {size:?}"
        );
    }
    let after = tree
        .find_by_role_and_name("status", "After")
        .next()
        .and_then(|node| node.geometry)
        .unwrap()
        .visual;
    assert_eq!(after.y, 0.0, "nothing hidden pushes the next node down");
}
