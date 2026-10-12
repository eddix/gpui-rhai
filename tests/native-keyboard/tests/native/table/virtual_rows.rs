//! Table rows in the native virtual list: grouped headers stick, and the 1000-row workload
//! tracks a resized window viewport.

use std::time::Duration;

use super::{mount_example, mount_refreshed, table, table_1000_example};
use crate::support::{dispatch_script_button, wait_for_view_text};
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

    scroll(&mut visual, -300.0);
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(16));
    wait_for_view_text(
        &mut visual,
        &view,
        "row-15",
        "the next virtual target must settle before crossing the group",
    );
    scroll(&mut visual, -300.0);
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
    scroll(&mut visual, -200.0);
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
    dispatch_script_button(&mut visual, &view, "Reverse 1,000 rows");
    advance(&mut visual, 4);
    wait_for_view_text(
        &mut visual,
        &view,
        "Account 0999",
        "native Table sorting should project the descending Rust order",
    );
    dispatch_script_button(&mut visual, &view, "Reverse 1,000 rows");
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
    advance(&mut visual, 3);
    let compact = node_bounds(&mut visual, "table", "One thousand deterministic accounts");
    let compact_email = node_bounds(&mut visual, "columnheader", "Email");
    let compact_root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let (_, compact_realized) = virtual_realized_window(&compact_root).unwrap();

    visual.simulate_resize(size(px(1_180.0), px(820.0)));
    advance(&mut visual, 3);
    let expanded = node_bounds(&mut visual, "table", "One thousand deterministic accounts");
    let expanded_email = node_bounds(&mut visual, "columnheader", "Email");
    let expanded_root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let (expanded_range, expanded_realized) = virtual_realized_window(&expanded_root).unwrap();

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
        expanded_realized > compact_realized + 2,
        "the virtual row window did not follow the taller viewport: compact={compact_realized}, expanded={expanded_realized}"
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
