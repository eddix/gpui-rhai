# ADR 0013: Retained tree and incremental formal components

Status: Accepted

## Context

The original runtime evaluated a complete Rhai `view(ctx)` tree after state
changes and converted that snapshot directly into GPUI elements. Keyed component
state survived, but ordinary nodes had no retained identity and Rust diffing
could not avoid Rhai rebuilding the complete tree. This prevents a general
high-frequency UI surface.

## Decision

Rhai and trusted Rust produce the same typed `UiNode` snapshots. A Rust
`RetainedUiTree` owns stable NodeId values, keyed child identity, handlers,
computed property sources, refs, focus, scrolling, capture, animation,
accessibility, and retained primitive instances. A reconciler validates a
complete mutation plan and commits it atomically.

A keyed formal Rhai component is the independent script rerender boundary.
During render, host accessors record state/store/theme/locale/geometry
dependencies. Dirty components rerender their retained invocation recipes and
replace only their subtrees. Helper functions remain part of the owning
component. Whole Map/Array reads create broad dependencies. Explicit bounded
paths track nested Map keys and Array indices; keyed Array selectors resolve by
a stable string field, so a pure reorder preserves the item dependency.

GPUI elements remain immediate values rebuilt from retained state; the runtime
does not cache `AnyElement`.

## Consequences

The runtime must implement identity, diff, transactions, lifecycle, dependency
tracking, and resource budgets. Rust diffing alone is not considered incremental
Rhai execution. Stateful/interactive nodes require explicit keys. Failed
candidates preserve the last-good generation.

Protected by keyed reorder/removal tests, incremental/full-render equivalence,
failure rollback, retained primitive lifecycle, and Timeline performance probes.
