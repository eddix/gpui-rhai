# ADR 0001: UiNode boundary

Status: Accepted

Rhai returns a runtime-owned `UiNode` tree. GPUI elements, windows, contexts,
and lifetimes never enter `Dynamic`. Rust converts nodes to short-lived GPUI
elements after validation.

This adds an explicit renderer layer but isolates GPUI API churn and enables
schema checks, source diagnostics, reconciliation, snapshots, and devtools.

Protected by the node, renderer, official registry, and custom primitive tests.
