use std::time::Instant;

use gpui_rhai::{RetainedUiTree, UiNode, VariableListSpec, VariableListState};

fn main() {
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
    let policy_elapsed = started.elapsed();

    let mut tree = RetainedUiTree::new();
    let first = UiNode::box_node(
        (0..2_000)
            .map(|index| UiNode::text(format!("Row {index}")).with_key(format!("row-{index}")))
            .collect(),
    );
    tree.reconcile(first).unwrap();
    let diff_started = Instant::now();
    let reordered = UiNode::box_node(
        (0..2_000)
            .rev()
            .map(|index| UiNode::text(format!("Row {index}")).with_key(format!("row-{index}")))
            .collect(),
    );
    let report = tree.reconcile(reordered).unwrap();

    println!(
        "variable policy: {:?}, realized {}, total {:.1}; retained reorder: {:?}, preserved {}, moved {}",
        policy_elapsed,
        window.range.len(),
        window.total,
        diff_started.elapsed(),
        report.preserved.len(),
        report.moved.len(),
    );
}
