//! Table column widths: flex weights, native resize preview and commit, resize handles on
//! the boundaries, native autofit, and an auto-min-width host column.

use super::{mount_example, mount_refreshed, table, table_1000_example};
use crate::support::{
    bounds, by_role, center, dispatch, last_error, palette_texts, wait_for_view_text,
};
use gpui::{
    Context, IntoElement, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, ParentElement,
    Render, ScrollDelta, ScrollWheelEvent, Styled, TestAppContext, VisualTestContext, Window, div,
    point, px, size,
};
use gpui_rhai::{ExecutionOperation, ScriptViewConfig, ScriptViewHandle, ScriptViewHost};

fn simulate_double_click(visual: &mut VisualTestContext, position: gpui::Point<gpui::Pixels>) {
    visual.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Left,
        click_count: 2,
        ..MouseDownEvent::default()
    });
    visual.simulate_event(MouseUpEvent {
        position,
        button: MouseButton::Left,
        click_count: 2,
        ..MouseUpEvent::default()
    });
}

struct AutoMinWidthTableHost {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for AutoMinWidthTableHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(
            div()
                .flex()
                .size_full()
                .child(self.view.flex_item().unwrap())
                .child(div().w(px(200.0)).h_full()),
        )
    }
}

#[gpui::test]
fn table_flex_columns_distribute_remaining_width_by_weight(cx: &mut TestAppContext) {
    let main = r#"
                    import "components/table" as table;
                    fn view(ctx) {
                        table::Table(#{
                            key: "weighted", label: "Weighted columns", row_key: "id",
                            rows: [#{ id: "row", fixed: "Fixed", percent: "Percent",
                                one: "One", two: "Two" }],
                            columns: [
                                #{ key: "fixed", title: "Fixed", width: #{ kind: "fixed", value: 120 } },
                                #{ key: "percent", title: "Percent", width: #{ kind: "percent", value: 25 } },
                                #{ key: "one", title: "Flex one", width: #{ kind: "flex", value: 1 } },
                                #{ key: "two", title: "Flex two", width: #{ kind: "flex", value: 2 } },
                            ],
                            height: 140,
                        })
                    }
                "#;
    let (window, view) = mount_refreshed(
        cx,
        table(main).ids("weighted-table-window", "weighted-table-view"),
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    let widths = |visual: &mut VisualTestContext, view: &ScriptViewHandle| {
        ["Fixed", "Percent", "Flex one", "Flex two"]
            .map(|name| bounds(visual, view, "columnheader", name).width)
    };

    for viewport in [800.0, 1_200.0] {
        visual.simulate_resize(size(px(viewport), px(420.0)));
        visual.run_until_parked();
        let [fixed, percent, one, two] = widths(&mut visual, &view);
        assert!((fixed - 120.0).abs() < 1.0, "fixed={fixed}");
        assert!(percent > fixed, "percent={percent} fixed={fixed}");
        assert!(one > 40.0, "flex one collapsed: {one}");
        assert!(
            (1.9..=2.1).contains(&(two / one)),
            "flex weights did not produce 1:2 widths: one={one} two={two}"
        );
    }
}

#[gpui::test]
fn table_column_resize_previews_natively_and_emits_once_on_commit(cx: &mut TestAppContext) {
    let main = r#"
                    import "components/table" as table;
                    fn state_schema() { #{ fields: #{
                        resize_count: #{ schema: #{ type: "integer", min: 0 },
                            "default": #{ type: "integer", value: 0 } },
                        resize_width: #{ schema: #{ type: "number", min: 0.0 },
                            "default": #{ type: "float", value: 0.0 } },
                        declared_width: #{ schema: #{ type: "number", min: 1.0 },
                            "default": #{ type: "float", value: 160.0 } },
                    } } }
                    fn resized(ctx, payload) {
                        ctx.set_state("resize_count", ctx.get_state("resize_count") + 1);
                        ctx.set_state("resize_width", payload.width.value);
                        ctx.set_state("declared_width", payload.width.value);
                    }
                    fn reset_width(ctx, payload) { ctx.set_state("declared_width", 160.0); }
                    fn view(ctx) {
                        column([
                            text(`resize:${ctx.get_state("resize_count")}:${ctx.get_state("resize_width")}`),
                            text("Reset width").on_click(Fn("reset_width"))
                                .accessibility_role("button").accessibility_label("Reset width"),
                            table::Table(#{
                                key: "resizable", label: "Resizable columns", row_key: "id",
                                rows: [#{ id: "row", left: "Left", right: "Right" }],
                                columns: [
                                    #{ key: "left", title: "Left", width: #{ kind: "fixed",
                                        value: ctx.get_state("declared_width") }, max_width: 240 },
                                    #{ key: "right", title: "Right", width: #{ kind: "flex", value: 1 } },
                                ],
                                height: 140, resizable_columns: true,
                                on_column_resize: Fn("resized"),
                            })
                        ]).with_style(style().width(relative(1.0)).gap(px(8)))
                    }
                "#;
    let (window, view) = mount_refreshed(
        cx,
        table(main).ids("resizable-table-window", "resizable-table-view"),
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(800.0), px(420.0)));
    visual.run_until_parked();
    let header = bounds(&mut visual, &view, "columnheader", "Left");
    let separator = bounds(&mut visual, &view, "separator", "Resize Left column");
    assert!((header.width - 160.0).abs() < 1.0, "header={header:?}");
    let start = center(separator);
    let end = point(start.x + px(120.0), start.y + px(40.0));

    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let preview = bounds(&mut visual, &view, "columnheader", "Left").width;
    assert!((preview - 240.0).abs() < 1.0, "preview={preview}");
    let preview_texts = palette_texts(&mut visual, &view);
    assert!(
        preview_texts
            .iter()
            .any(|text| text.starts_with("resize:0:")),
        "pointer-move preview must not invoke Rhai: {preview_texts:?}"
    );

    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let committed_texts = palette_texts(&mut visual, &view);
    assert!(
        committed_texts
            .iter()
            .any(|text| text == "resize:1:240" || text == "resize:1:240.0"),
        "mouse-up must emit exactly one semantic resize event: {committed_texts:?}"
    );

    dispatch(
        &mut visual,
        &view,
        by_role("button", "Reset width"),
        "click",
    )
    .unwrap();
    visual.run_until_parked();
    let reset_width = bounds(&mut visual, &view, "columnheader", "Left").width;
    assert!((reset_width - 160.0).abs() < 1.0, "reset={reset_width}");

    visual.simulate_event(MouseDownEvent {
        position: start,
        button: MouseButton::Left,
        click_count: 1,
        ..MouseDownEvent::default()
    });
    visual.simulate_event(MouseUpEvent {
        position: start,
        button: MouseButton::Left,
        click_count: 1,
        ..MouseUpEvent::default()
    });
    visual.run_until_parked();
    let single_click_texts = palette_texts(&mut visual, &view);
    assert!(
        single_click_texts
            .iter()
            .any(|text| text == "resize:1:240" || text == "resize:1:240.0"),
        "a zero-motion separator click must not emit resize: {single_click_texts:?}"
    );

    simulate_double_click(&mut visual, start);
    visual.run_until_parked();
    let auto_fit = bounds(&mut visual, &view, "columnheader", "Left").width;
    assert!((auto_fit - 58.0).abs() < 1.0, "auto_fit={auto_fit}");
    let auto_fit_texts = palette_texts(&mut visual, &view);
    assert!(
        auto_fit_texts
            .iter()
            .any(|text| text == "resize:2:58" || text == "resize:2:58.0"),
        "double-click auto-fit must emit one semantic resize: {auto_fit_texts:?}"
    );

    dispatch(
        &mut visual,
        &view,
        by_role("separator", "Resize Left column"),
        "key:right",
    )
    .unwrap();
    visual.run_until_parked();
    let keyboard_texts = palette_texts(&mut visual, &view);
    assert!(
        keyboard_texts
            .iter()
            .any(|text| text == "resize:3:66" || text == "resize:3:66.0"),
        "focused separators must support logical arrow-key resizing: {keyboard_texts:?}"
    );
}

#[gpui::test]
fn table_resize_handles_sit_on_column_boundaries_including_the_last_column(
    cx: &mut TestAppContext,
) {
    let main = r#"
                    import "components/table" as table;
                    fn view(ctx) {
                        table::Table(#{
                            key: "boundaries", label: "Boundaries", row_key: "id",
                            rows: [#{ id: "row", name: "Ada", score: "42", status: "Active" }],
                            columns: [
                                #{ key: "name", title: "Name", width: #{ kind: "fixed", value: 120 } },
                                #{ key: "score", title: "Score", width: #{ kind: "fixed", value: 140 },
                                    max_width: 400 },
                                #{ key: "status", title: "Status", width: #{ kind: "fixed", value: 160 },
                                    max_width: 400 },
                            ],
                            height: 140, resizable_columns: true,
                        })
                    }
                "#;
    let (window, view) = mount_refreshed(
        cx,
        table(main).ids("boundary-table-window", "boundary-table-view"),
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(800.0), px(420.0)));
    visual.run_until_parked();
    // (header x, header right edge, handle x, handle right edge) per column title.
    let geometry = |visual: &mut VisualTestContext, title: &str| {
        let header = bounds(visual, &view, "columnheader", title);
        let handle = bounds(
            visual,
            &view,
            "separator",
            &format!("Resize {title} column"),
        );
        (
            header.x,
            header.x + header.width,
            handle.x,
            handle.x + handle.width,
        )
    };

    // Inner columns: the handle (and the line painted at its center) straddles the boundary.
    for title in ["Name", "Score"] {
        let (_, boundary, handle_x, handle_right) = geometry(&mut visual, title);
        let center = (handle_x + handle_right) / 2.0;
        assert!(
            (center - boundary).abs() < 0.5,
            "{title}: handle center {center} must sit on the column boundary {boundary}"
        );
    }
    // The last column's handle ends at the table's right edge instead of overhanging it.
    let (_, status_right, _, status_handle_right) = geometry(&mut visual, "Status");
    assert!(
        (status_handle_right - status_right).abs() < 0.5,
        "Status: handle right {status_handle_right} must meet the header right edge {status_right}"
    );

    let drag = |visual: &mut VisualTestContext, x: f64, y: f64| {
        let start = point(px(x as f32), px(y as f32));
        let end = point(start.x + px(40.0), start.y);
        visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
        visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        visual.run_until_parked();
    };
    let header = bounds(&mut visual, &view, "columnheader", "Score");
    let header_y = header.y + header.height / 2.0;

    // Grabbing just right of the boundary, over the next header cell, resizes the left column.
    let (score_x, score_right, _, _) = geometry(&mut visual, "Score");
    drag(&mut visual, score_right + 2.0, header_y);
    let (_, resized_right, _, _) = geometry(&mut visual, "Score");
    assert!(
        (resized_right - score_x - 180.0).abs() < 1.0,
        "Score must grow from 140 to 180, got {}",
        resized_right - score_x
    );

    // The last column resizes from the table's right edge.
    let (status_x, status_right, _, _) = geometry(&mut visual, "Status");
    drag(&mut visual, status_right - 2.0, header_y);
    let (_, resized_right, _, _) = geometry(&mut visual, "Status");
    assert!(
        (resized_right - status_x - 200.0).abs() < 1.0,
        "Status must grow from 160 to 200, got {}",
        resized_right - status_x
    );
}

#[gpui::test]
fn native_collection_table_autofit_stays_on_the_realized_rust_path(cx: &mut TestAppContext) {
    let (window, view) = mount_example(
        cx,
        table_1000_example::table_1000_resizable_view(),
        "native-autofit-window",
        "native-autofit-view",
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    wait_for_view_text(
        &mut visual,
        &view,
        "Account 0000",
        "native rows must realize before auto-fit",
    );
    let header = bounds(&mut visual, &view, "columnheader", "Row");
    let separator = bounds(&mut visual, &view, "separator", "Resize Row column");
    let _ = visual.update(|_, cx| view.take_performance_snapshot(cx));
    simulate_double_click(&mut visual, center(separator));
    visual.run_until_parked();
    let fitted = bounds(&mut visual, &view, "columnheader", "Row").width;
    assert!(
        fitted >= 48.0 && fitted < header.width - 8.0,
        "native realized content did not auto-fit: before={header:?}, after={fitted}"
    );
    let performance = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    assert!(
        performance.timings.iter().all(|timing| !matches!(
            timing.operation,
            ExecutionOperation::Render | ExecutionOperation::VirtualCollection(_)
        )),
        "auto-fit must not materialize NativeCollection rows or rerender Rhai: {performance:?}"
    );
    assert!(
        performance
            .timings
            .iter()
            .all(|timing| !matches!(timing.operation, ExecutionOperation::Callback(_))),
        "an unobserved auto-fit should stay entirely in Rust: {performance:?}"
    );

    visual.simulate_event(ScrollWheelEvent {
        position: point(
            px((header.x + header.width / 2.0) as f32),
            px((header.y + 120.0) as f32),
        ),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-600.0))),
        ..ScrollWheelEvent::default()
    });
    for _ in 0..4 {
        cx.background_executor
            .advance_clock(std::time::Duration::from_millis(16));
        visual.run_until_parked();
    }
    wait_for_view_text(
        &mut visual,
        &view,
        "Account 0030",
        "retained virtual rows must replay their scoped width signals after auto-fit",
    );
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn table_does_not_expand_an_auto_min_width_host_flex_column_across_frames(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let main = r#"
                    import "components/table" as table;
                    fn row_clicked(ctx, key) { () }
                    fn view(ctx) {
                        table::Table(#{
                            key: "runaway", label: "Runaway probe", row_key: "id",
                            rows: [#{ id: "row-1", left: "Left", middle: "Middle", right: "Right" }],
                            columns: [
                                #{ key: "left", title: "Left", width: #{ kind: "fixed", value: 320 } },
                                #{ key: "middle", title: "Middle", width: #{ kind: "fixed", value: 320 } },
                                #{ key: "right", title: "Right", width: #{ kind: "fixed", value: 320 } },
                            ],
                            height: 240, selection_mode: "single", on_row_click: Fn("row_clicked"),
                        })
                    }
                "#;
    let prepared = table(main).prepare().unwrap();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("auto-min-table-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("auto-min-table-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        AutoMinWidthTableHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(1_000.0), px(600.0)));
    visual.run_until_parked();
    let mut widths = Vec::new();
    for _ in 0..8 {
        cx.refresh().unwrap();
        visual.run_until_parked();
        widths.push(
            visual
                .debug_bounds("gpui-rhai-flex-item:auto-min-table-view")
                .unwrap()
                .size
                .width,
        );
    }
    let first = widths[0];
    assert!(
        (799.0..=801.0).contains(&f32::from(first)),
        "Table min-content escaped the 800px host flex allocation: {widths:?}"
    );
    assert!(
        widths.iter().all(|width| (*width - first).abs() < px(0.1)),
        "Table fed its bordered min-content width back into the host flex column: {widths:?}"
    );
}
