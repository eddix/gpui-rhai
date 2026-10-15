//! Table rows in the native virtual list: fill-height respects a minimum inside a scroll
//! container, grouped headers stick, and the 1000-row workload tracks a resized window.

use std::time::Duration;

use super::{mount_example, mount_refreshed, table, table_1000_example};
use crate::support::{
    ReplaceOnce, Step, dispatch_script_button, settle, snapshot, wait_for_view_text,
};
use gpui::{ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::{ExecutionOperation, UiNode, UiValue};

/// Advances the clock 16ms and runs until parked, `times` times.
fn advance(visual: &mut VisualTestContext, times: usize) {
    for _ in 0..times {
        visual.executor().advance_clock(Duration::from_millis(16));
        visual.run_until_parked();
    }
}

fn virtual_realized_window(node: &UiNode) -> Option<(std::ops::Range<usize>, usize)> {
    match node.kind() {
        gpui_rhai::UiNodeKind::Box { children } | gpui_rhai::UiNodeKind::Fragment { children } => {
            children.iter().find_map(virtual_realized_window)
        }
        gpui_rhai::UiNodeKind::Overlay {
            trigger, content, ..
        } => virtual_realized_window(trigger).or_else(|| virtual_realized_window(content)),
        gpui_rhai::UiNodeKind::Layer { content, .. } => virtual_realized_window(content),
        gpui_rhai::UiNodeKind::ErrorBoundary { child, fallback } => {
            virtual_realized_window(child).or_else(|| virtual_realized_window(fallback))
        }
        gpui_rhai::UiNodeKind::VirtualCollection { spec } => {
            let mut indices = spec.realized.keys().copied();
            let Some(first) = indices.next() else {
                return Some((0..0, 0));
            };
            let (min, max) = indices.fold((first, first), |(min, max), index| {
                (min.min(index), max.max(index))
            });
            Some((min..max.saturating_add(1), spec.realized.len()))
        }
        _ => None,
    }
}

fn virtual_spec(node: &UiNode) -> Option<&gpui_rhai::VirtualCollectionNodeSpec> {
    match node.kind() {
        gpui_rhai::UiNodeKind::VirtualCollection { spec } => Some(spec),
        gpui_rhai::UiNodeKind::Box { children } | gpui_rhai::UiNodeKind::Fragment { children } => {
            children.iter().find_map(virtual_spec)
        }
        gpui_rhai::UiNodeKind::Overlay {
            trigger, content, ..
        } => virtual_spec(trigger).or_else(|| virtual_spec(content)),
        gpui_rhai::UiNodeKind::Layer { content, .. } => virtual_spec(content),
        gpui_rhai::UiNodeKind::ErrorBoundary { child, fallback } => {
            virtual_spec(child).or_else(|| virtual_spec(fallback))
        }
        _ => None,
    }
}

#[gpui::test]
fn fill_height_table_honors_min_height_inside_scroll_fallback(cx: &mut TestAppContext) {
    // Below the minimum the parent scrolls; above it the Table fills the available height.
    // Both must measure a nonzero body viewport and actually present rows below the header.
    for parent_height in [180, 360] {
        let main = r#"
            import "components/table" as table;
            fn view(ctx) {
                let rows = [];
                for index in 0..15 {
                    rows.push(#{ id: `row-${index}`, value: `Value ${index}` });
                }
                column([
                    table::Table(#{
                        key: "constrained", label: "Constrained table", row_key: "id",
                        rows: rows, fill_height: true,
                        columns: [
                            #{ key: "id", title: "ID",
                                width: #{ kind: "fixed", value: 120 } },
                            #{ key: "value", title: "Value",
                                width: #{ kind: "flex", value: 1 } },
                        ],
                    }).with_style(style().min_height(px(240)))
                ]).accessibility_role("group").accessibility_label("Scroll fallback")
                    .with_style(style().width(px(480)).height(px(PARENT_HEIGHT))
                        .min_height(px(0)).overflow_y_scroll())
            }
        "#
        .replace_once("PARENT_HEIGHT", &parent_height.to_string());
        let id = format!("constrained-table-{parent_height}");
        let (window, view) = mount_refreshed(cx, table(main).id(&id));
        let mut visual = VisualTestContext::from_window(*window, cx);
        advance(&mut visual, 4);
        wait_for_view_text(
            &mut visual,
            &view,
            "Value 0",
            "a min-height fill Table inside a scroll fallback must realize its rows",
        );
        settle(&mut visual, Step::Frame, 2);

        let tree = snapshot(&mut visual, &view);
        // Compare layout boxes, not visual boxes with border/paint offsets.
        let layout = |role, name| {
            tree.find_by_role_and_name(role, name)
                .next()
                .expect("the Table and its first row must be accessible")
                .geometry
                .expect("the node must have committed layout geometry")
                .layout
        };
        let parent = layout("group", "Scroll fallback");
        let table = layout("table", "Constrained table");
        // Header cells are centered inside a bordered row and can have a half-pixel
        // offset; the header row itself is what consumes the Table's height.
        let header = tree
            .nodes()
            .find(|node| node.role == "row" && node.row_index == Some(1))
            .expect("the Table must present its header row")
            .geometry
            .expect("the header row must have committed layout geometry")
            .layout;
        let row = layout("row", "Constrained table row 1");
        let viewport = visual
            .debug_bounds("virtual-list:constrained-body")
            .expect("the fill-height Table body must have a native viewport");
        assert!(
            (parent.height - f64::from(parent_height)).abs() < 0.5,
            "the scroll parent must keep its declared height: {parent:?}"
        );
        assert!(
            (table.height - f64::from(parent_height.max(240))).abs() < 0.5,
            "Table must honor its minimum or fill the taller parent: {table:?}"
        );
        assert!(
            viewport.size.height > px(0.0)
                && (f64::from(viewport.origin.y) - (header.y + header.height)).abs() < 0.5
                && (f64::from(viewport.origin.y + viewport.size.height) - (table.y + table.height))
                    .abs()
                    < 0.5,
            "the body must fill the Table below its header: {viewport:?}, {header:?}, {table:?}"
        );
        assert!(
            row.height > 0.0
                && row.y >= f64::from(viewport.origin.y) - 0.5
                && row.y + row.height <= f64::from(viewport.origin.y + viewport.size.height) + 0.5,
            "the first row must be presented inside the body: {row:?}, {viewport:?}"
        );
        let performance = visual
            .update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap();
        let metrics = performance.virtual_collections.first().unwrap();
        assert_eq!(metrics.item_count, 15);
        assert!(metrics.realized_count > 0);
        assert!(
            (metrics.viewport_height - f64::from(viewport.size.height)).abs() < 0.5,
            "virtualization must use the laid-out body, not the parent: {metrics:?}"
        );
    }
}

#[gpui::test]
fn grouped_table_headers_stick_through_the_native_virtual_list(cx: &mut TestAppContext) {
    let main = r#"
                    import "components/table" as table;
                    fn toggled(ctx, group) { () }
                    fn view(ctx) {
                        let rows = [];
                        for index in 0..40 {
                            rows.push(#{
                                id: `row-${index}`,
                                track: if index < 20 { "alpha" } else { "beta" },
                                state: if index % 2 == 0 { "Ready" } else { "Drift" }
                            });
                        }
                        table::Table(#{
                            key: "groups", label: "Grouped clusters", row_key: "id",
                            height: 150, rows: rows, group_by: "track",
                            columns: [
                                #{ key: "id", title: "ID",
                                    width: #{ kind: "fixed", value: 100 } },
                                #{ key: "track", title: "Track",
                                    width: #{ kind: "fixed", value: 180 } },
                                #{ key: "state", title: "State",
                                    width: #{ kind: "fixed", value: 120 } }
                            ],
                            on_group_toggle: Fn("toggled")
                        })
                    }
                "#;
    let (window, view) = mount_refreshed(
        cx,
        table(main).ids("sticky-table-window", "sticky-table-view"),
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    advance(&mut visual, 4);
    let viewport = visual
        .debug_bounds("virtual-list:groups-body")
        .expect("grouped Table virtual viewport");
    let bounds_for_group = |visual: &mut VisualTestContext, name: &str| {
        let result = visual
            .update(|window, cx| {
                view.automate(
                    gpui_rhai::AutomationCommand::Query {
                        locator: gpui_rhai::AutomationLocator::RoleName {
                            role: "rowheader".to_owned(),
                            name: name.to_owned(),
                        },
                    },
                    window,
                    cx,
                )
            })
            .unwrap();
        let gpui_rhai::AutomationResult::Node { node } = result else {
            panic!("expected grouped Table rowheader {name}");
        };
        node.bounds
            .expect("sticky group header has committed bounds")
    };
    let scroll = |visual: &mut VisualTestContext, delta: f32| {
        visual.simulate_event(ScrollWheelEvent {
            position: point(
                viewport.origin.x + viewport.size.width / 2.0,
                viewport.origin.y + viewport.size.height / 2.0,
            ),
            delta: ScrollDelta::Pixels(point(px(0.0), px(delta))),
            ..ScrollWheelEvent::default()
        });
    };

    assert!(
        visual
            .debug_bounds("virtual-list-sticky:groups-body")
            .is_none(),
        "a group header at its natural position must not be duplicated in the sticky layer"
    );
    scroll(&mut visual, -60.0);
    advance(&mut visual, 1);
    let sticky_bounds = visual
        .debug_bounds("virtual-list-sticky:groups-body")
        .expect("the group header must enter the sticky layer after crossing the viewport top");
    visual.simulate_event(ScrollWheelEvent {
        position: point(
            sticky_bounds.origin.x + sticky_bounds.size.width / 2.0,
            sticky_bounds.origin.y + sticky_bounds.size.height / 2.0,
        ),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-240.0))),
        ..ScrollWheelEvent::default()
    });
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(16));
    wait_for_view_text(
        &mut visual,
        &view,
        "row-10",
        "the first scroll target must realize before the next wheel event",
    );
    let alpha = bounds_for_group(&mut visual, "alpha, 20");
    assert!(
        (alpha.y - f64::from(viewport.origin.y)).abs() < 1.0,
        "first group did not remain pinned: header={alpha:?}, viewport={viewport:?}"
    );

    // Native height hints expose the complete extent from the first frame.
    // Do not depend on the old unmeasured suffix clamping away part of each
    // wheel step: these intermediate distances deliberately traverse rows
    // 15, 19 and 25 before the final beta-header geometry assertion.
    scroll(&mut visual, -200.0);
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(16));
    wait_for_view_text(
        &mut visual,
        &view,
        "row-15",
        "the next virtual target must settle before crossing the group",
    );
    scroll(&mut visual, -140.0);
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(16));
    wait_for_view_text(
        &mut visual,
        &view,
        "row-19",
        "the last row of the first group must enter the realized window",
    );
    scroll(&mut visual, -100.0);
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(16));
    wait_for_view_text(
        &mut visual,
        &view,
        "beta",
        "the second group must enter the realized virtual window",
    );
    scroll(&mut visual, -120.0);
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(16));
    wait_for_view_text(
        &mut visual,
        &view,
        "row-25",
        "rows inside the second group must settle before asserting its sticky position",
    );
    scroll(&mut visual, -60.0);
    advance(&mut visual, 4);
    let beta = bounds_for_group(&mut visual, "beta, 20");
    assert!(
        (beta.y - f64::from(viewport.origin.y)).abs() < 1.0,
        "next group did not replace the sticky header: header={beta:?}, viewport={viewport:?}"
    );
    assert!(
        visual
            .debug_bounds("virtual-list-sticky:groups-body")
            .is_some(),
        "the native virtual list must own one sticky presentation layer"
    );
    let metrics = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    assert_eq!(metrics.virtual_collections[0].sticky_header, Some(21));
}

#[gpui::test]
fn table_1000_tracks_the_resized_window_viewport(cx: &mut TestAppContext) {
    let (window, view) = mount_example(
        cx,
        table_1000_example::table_1000_view(),
        "table-1000-resize-window",
        "table-1000-resize-view",
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    advance(&mut visual, 4);
    wait_for_view_text(
        &mut visual,
        &view,
        "Account 0000",
        "the complete virtual target window should settle with visible rows",
    );
    let settled = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    assert!(!settled.pending_virtual_requests);
    let virtual_operations = settled
        .timings
        .iter()
        .filter_map(|timing| {
            matches!(timing.operation, ExecutionOperation::VirtualCollection(_))
                .then_some(timing.operations)
        })
        .collect::<Vec<_>>();
    assert!(!virtual_operations.is_empty());
    assert!(
        virtual_operations
            .iter()
            .all(|operations| (1..10_000).contains(operations)),
        "stored Rhai callback contexts must report per-invocation operation deltas: {virtual_operations:?}"
    );
    let settled_ranges = settled
        .virtual_collections
        .iter()
        .map(|collection| collection.realized_range.clone())
        .collect::<Vec<_>>();
    for _ in 0..2 {
        advance(&mut visual, 1);
        let next = visual
            .update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap();
        assert!(!next.pending_virtual_requests);
        assert_eq!(
            next.virtual_collections
                .iter()
                .map(|collection| collection.realized_range.clone())
                .collect::<Vec<_>>(),
            settled_ranges
        );
        assert!(
            next.timings.iter().all(|timing| !matches!(
                timing.operation,
                ExecutionOperation::VirtualCollection(_)
            )),
            "stable frames must not keep invoking virtual item Rhai renderers"
        );
    }
    // Read from the middle so a full reorder has room on both sides to retain
    // the reader, rather than hitting a natural start/end clamp.
    let collection = settled.virtual_collections[0].id.clone();
    visual.update(|_, cx| {
        view.collection_scroll_handle(&collection, cx)
            .unwrap()
            .unwrap()
            .scroll_by(px(15_000.0), cx)
    });
    settle(&mut visual, Step::Tick, 8);
    let reader_metrics = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
        .virtual_collections[0]
        .clone();
    let reader_root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let reader_key = virtual_spec(&reader_root)
        .unwrap()
        .data
        .key(reader_metrics.scroll_item)
        .unwrap();
    let reader_name = format!("Account {}", reader_key.strip_prefix("row-").unwrap());
    let reader = snapshot(&mut visual, &view)
        .nodes()
        .find(|node| node.name == reader_name)
        .expect("the actual native reader anchor has presented account text")
        .clone();
    let reader_y = reader.geometry.unwrap().visual.y;
    dispatch_script_button(&mut visual, &view, "Reverse 1,000 rows");
    settle(&mut visual, Step::Tick, 8);
    let descending = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let descending = &virtual_spec(&descending).unwrap().data;
    assert_eq!(
        descending.key(0),
        Some("row-0999"),
        "the full native projection really reordered"
    );
    assert_eq!(descending.key(descending.len() - 1), Some("row-0000"));
    let after = snapshot(&mut visual, &view)
        .nodes()
        .find(|node| node.name == reader.name)
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    assert!(
        (after.y - reader_y).abs() < 1.0,
        "sorting preserves the surviving reader's presented coordinate"
    );
    dispatch_script_button(&mut visual, &view, "Reverse 1,000 rows");
    settle(&mut visual, Step::Tick, 8);
    let ascending = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let ascending = &virtual_spec(&ascending).unwrap().data;
    assert_eq!(ascending.key(0), Some("row-0000"));
    assert_eq!(ascending.key(ascending.len() - 1), Some("row-0999"));
    let after = snapshot(&mut visual, &view)
        .nodes()
        .find(|node| node.name == reader.name)
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    assert!(
        (after.y - reader_y).abs() < 1.0,
        "restoring order also preserves the reader"
    );
    // Explicit navigation, not a sort/reset side effect, returns to the start.
    visual.update(|_, cx| {
        view.collection_scroll_handle(&collection, cx)
            .unwrap()
            .unwrap()
            .scroll_to_start(cx)
    });
    advance(&mut visual, 4);
    wait_for_view_text(
        &mut visual,
        &view,
        "Account 0000",
        "native Table sorting should restore ascending order",
    );
    let node_bounds = |visual: &mut VisualTestContext, role: &str, name: &str| {
        let result = visual
            .update(|window, cx| {
                view.automate(
                    gpui_rhai::AutomationCommand::Query {
                        locator: gpui_rhai::AutomationLocator::RoleName {
                            role: role.to_owned(),
                            name: name.to_owned(),
                        },
                    },
                    window,
                    cx,
                )
            })
            .unwrap();
        let gpui_rhai::AutomationResult::Node { node } = result else {
            panic!("expected {role} node named {name}");
        };
        node.bounds.expect("node has committed bounds")
    };
    visual.simulate_resize(size(px(820.0), px(620.0)));
    settle(&mut visual, Step::Tick, 8);
    let compact = node_bounds(&mut visual, "table", "One thousand deterministic accounts");
    let compact_email = node_bounds(&mut visual, "columnheader", "Email");
    let compact_root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let (_, compact_realized) = virtual_realized_window(&compact_root).unwrap();
    let compact_metrics = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
        .virtual_collections[0]
        .clone();

    visual.simulate_resize(size(px(1_180.0), px(820.0)));
    settle(&mut visual, Step::Tick, 8);
    let expanded = node_bounds(&mut visual, "table", "One thousand deterministic accounts");
    let expanded_email = node_bounds(&mut visual, "columnheader", "Email");
    let expanded_root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let (expanded_range, expanded_realized) = virtual_realized_window(&expanded_root).unwrap();
    let expanded_metrics = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
        .virtual_collections[0]
        .clone();

    assert!(
        expanded.width > compact.width + 300.0,
        "table width did not follow the window resize: compact={compact:?}, expanded={expanded:?}"
    );
    assert!(
        expanded.height > compact.height + 150.0,
        "table height did not follow the window resize: compact={compact:?}, expanded={expanded:?}"
    );
    assert!(
        expanded_email.width > compact_email.width + 90.0,
        "percentage column did not relayout after resize: compact={compact_email:?}, expanded={expanded_email:?}"
    );
    assert!(
        expanded_metrics.visible_range.len() > compact_metrics.visible_range.len() + 2
            && expanded_realized >= expanded_metrics.visible_range.len(),
        "presented rows must follow the taller measured viewport (cached halos may differ): compact={compact_metrics:?}/{compact_realized}, expanded={expanded_metrics:?}/{expanded_realized}"
    );

    let coordinate = |value: f64| px(value.to_string().parse::<f32>().unwrap());
    visual.simulate_event(ScrollWheelEvent {
        position: point(
            coordinate(expanded.x + expanded.width / 2.0),
            coordinate(expanded.y + expanded.height / 2.0),
        ),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-600.0))),
        ..ScrollWheelEvent::default()
    });
    advance(&mut visual, 4);
    let scrolled_root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let (scrolled_range, scrolled_realized) = virtual_realized_window(&scrolled_root).unwrap();
    assert!(
        scrolled_realized < 64,
        "scrolling must keep realization bounded, got {scrolled_realized} rows"
    );
    let scrolled = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    let scrolled_metrics = scrolled.virtual_collections.first().unwrap();
    assert!(
        scrolled_metrics.scroll_item > 0 || scrolled_metrics.visible_range.start > 0,
        "vertical wheel input did not advance the virtual viewport: before={expanded_range:?}, after={scrolled_range:?}, metrics={scrolled_metrics:?}"
    );

    let replacement = gpui_rhai::NativeCollection::new(
        "id",
        [std::collections::BTreeMap::from([
            (
                "id".to_owned(),
                UiValue::String("row-replacement".to_owned()),
            ),
            (
                "account".to_owned(),
                UiValue::String("Replacement account".to_owned()),
            ),
        ])],
    )
    .unwrap();
    assert!(
        visual
            .update(|_, cx| view.replace_native_collection("accounts", replacement, cx))
            .unwrap()
    );
    advance(&mut visual, 4);
    wait_for_view_text(
        &mut visual,
        &view,
        "Replacement account",
        "host collection replacement should invalidate its subscribed Table",
    );
}
