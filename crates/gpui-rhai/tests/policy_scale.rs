//! Retained-tree and virtual-list policy at scale: a 100,000-row variable-height list
//! realizes only the rows around its viewport, and reversing 2,000 keyed rows keeps every
//! node. For the timings, run `cargo test --release -p gpui-rhai --test policy_scale --
//! --nocapture`.

use std::time::Instant;

use gpui_rhai::{RetainedUiTree, UiNode, VariableListSpec, VariableListState};

#[test]
fn a_large_variable_list_realizes_only_its_viewport() {
    let started = Instant::now();
    let spec = VariableListSpec::new(32.0, 256.0).unwrap();
    let mut list = VariableListState::default();
    list.set_keys(
        (0..100_000).map(|index| format!("row-{index}")).collect(),
        spec,
    )
    .unwrap();
    for index in (0..100_000).step_by(17) {
        list.measure(
            &format!("row-{index}"),
            24.0 + f64::from(index % 5) * 8.0,
            Some("row-50000"),
        )
        .unwrap();
    }
    let window = list.window(spec, 1_600_000.0, 720.0).unwrap();
    println!(
        "variable policy: {:?}, realized {}, total {:.1}",
        started.elapsed(),
        window.range.len(),
        window.total
    );
    assert!(
        !window.range.is_empty() && window.range.len() < 100,
        "realized {} rows",
        window.range.len()
    );
}

#[test]
fn reversing_keyed_rows_preserves_every_node() {
    let rows = |order: Vec<usize>| {
        UiNode::box_node(
            order
                .into_iter()
                .map(|index| UiNode::text(format!("Row {index}")).with_key(format!("row-{index}")))
                .collect(),
        )
    };
    let mut tree = RetainedUiTree::new();
    tree.reconcile(rows((0..2_000).collect())).unwrap();
    let started = Instant::now();
    let report = tree.reconcile(rows((0..2_000).rev().collect())).unwrap();
    println!(
        "retained reorder: {:?}, preserved {}, moved {}",
        started.elapsed(),
        report.preserved.len(),
        report.moved.len()
    );
    // The root box and its 2,000 rows survive; every row moves.
    assert_eq!(report.preserved.len(), 2_001);
    assert_eq!(report.moved.len(), 2_000);
}
