//! Markdown tables lay out by their content (feedback #3, #222): a table of at most 1,000
//! cells lays out in full, each row as tall as its wrapped cells, and grows with the
//! document; a column fits its widest cell on one line within 64 to 360 logical pixels and
//! the table does not stretch to the reader. A larger table keeps a viewport of measured
//! rows, `table_max_height` high.

use gpui::{TestAppContext, VisualTestContext};

use crate::support::{Mount, Step, last_error, settle};

const COMPONENT: &str = include_str!("../../../../../registry/components/markdown_viewer.rhai");

fn mount(cx: &mut TestAppContext, source: &str, extra: &str) -> VisualTestContext {
    let main = format!(
        r#"
import "components/markdown_viewer" as markdown;
fn view(ctx) {{ markdown::MarkdownViewer(#{{key: "tables", label: "Tables",
    source: {source:?}, height: "content", {extra}}}).with_style(style().width(px(800))) }}
"#
    );
    let (mut visual, view) = Mount::new(main)
        .module("components/markdown_viewer", COMPONENT)
        .id("markdown-tables")
        .mount(cx);
    settle(&mut visual, Step::Frame, 5);
    assert_eq!(last_error(&mut visual, &view), None);
    visual
}

fn cell(visual: &mut VisualTestContext, row: usize, column: usize) -> gpui::Bounds<gpui::Pixels> {
    let selector: &'static str =
        Box::leak(format!("markdown-cell:0:{row}:{column}").into_boxed_str());
    visual
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("no cell {row}:{column}"))
}

#[gpui::test]
fn a_small_table_grows_with_wrapped_rows_and_fits_its_columns(cx: &mut TestAppContext) {
    let long =
        "a cell that is much longer than the widest column allows, so it wraps onto more lines";
    let mut source = "| id | name | note |\n| --- | --- | --- |\n".to_owned();
    for index in 0..10 {
        let note = if index == 3 {
            long.to_owned()
        } else {
            format!("note {index}")
        };
        source.push_str(&format!("| {index} | host-{index}.example | {note} |\n"));
    }
    let mut visual = mount(cx, &source, "");
    let table = visual.debug_bounds("markdown-table:0").unwrap();
    // Columns: the short id column at the minimum, the long note column at the maximum,
    // the name column in between; the table is their sum, not the reader's width.
    let (id, name, note) = (
        cell(&mut visual, 0, 0),
        cell(&mut visual, 0, 1),
        cell(&mut visual, 0, 2),
    );
    assert_eq!(id.size.width, gpui::px(64.0), "{id:?}");
    assert_eq!(note.size.width, gpui::px(360.0), "{note:?}");
    assert!(
        name.size.width > gpui::px(64.0) && name.size.width < gpui::px(360.0),
        "{name:?}"
    );
    assert!(table.size.width >= id.size.width + name.size.width + note.size.width - gpui::px(1.0));
    // The wrapped row is taller than the others, and every row is laid out inside the
    // table: nothing hides in an inner scroll.
    let plain = cell(&mut visual, 2, 2);
    let wrapped = cell(&mut visual, 4, 2);
    assert!(
        wrapped.size.height > plain.size.height + gpui::px(10.0),
        "{plain:?} {wrapped:?}"
    );
    let last = cell(&mut visual, 10, 0);
    assert!(
        last.bottom() <= table.bottom() + gpui::px(0.5),
        "{last:?} in {table:?}"
    );
}

#[gpui::test]
fn a_table_over_a_thousand_cells_keeps_a_viewport_of_measured_rows(cx: &mut TestAppContext) {
    let mut source = "| a | b | c | d |\n| --- | --- | --- | --- |\n".to_owned();
    for index in 0..300 {
        let d = if index == 1 {
            "a cell that is much longer than the widest column allows, so it wraps onto more lines"
                .to_owned()
        } else {
            format!("d{index}")
        };
        source.push_str(&format!("| a{index} | b{index} | c{index} | {d} |\n"));
    }
    let mut visual = mount(cx, &source, "table_max_height: 240,");
    let table = visual.debug_bounds("markdown-table:0").unwrap();
    assert!(
        (f32::from(table.size.height) - 240.0).abs() < 1.0,
        "the viewport takes table_max_height: {table:?}"
    );
    let plain = cell(&mut visual, 1, 3);
    let wrapped = cell(&mut visual, 2, 3);
    assert!(
        wrapped.size.height > plain.size.height + gpui::px(10.0),
        "{plain:?} {wrapped:?}"
    );
    assert!(
        visual.debug_bounds("markdown-cell:0:299:0").is_none(),
        "the rows past the viewport are not laid out"
    );
}
