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

Root-view rerenders perform component-level bailout before invoking a formal
component's Rhai render. Equal normalized props, no dirty descendant, and an
unchanged script/theme/locale/calendar environment reuse the component's
recipe-owned `UiNode` snapshot together with all retained component-owned
runtime declarations and dependency edges. Node-valued props compare
conservatively as unequal. Receiver-local rerenders hydrate them recursively
from the passed components' latest owned snapshots; typed presentation
mutations remain outside those snapshots and survive child replacement without
being applied twice. ADR 0019 defines that ownership split and its matrix.
Components that sample an untracked native signal during render are not
reusable; signal-bound native properties remain the intended hot path.

Some source adapters intentionally return and decorate another formal
component's root node, so a single `component_root` marker cannot represent both
boundaries. If a dirty child has no independently replaceable address in the
accepted snapshot, incremental dispatch promotes it to the nearest active
ancestor that does. Rerendering that ancestor replays its decoration and the
dirty descendant while preserving unrelated siblings; falling back to the root
remains the final safe case.

GPUI elements remain immediate values rebuilt from retained state; the runtime
does not cache `AnyElement`.

## Consequences

The runtime must implement identity, diff, transactions, lifecycle, dependency
tracking, and resource budgets. Rust diffing alone is not considered incremental
Rhai execution. Stateful/interactive nodes require explicit keys. Failed
candidates preserve the last-good generation.

Protected by keyed reorder/removal tests, incremental/full-render equivalence,
root-dirty nested bailout and transparent Select/Combobox promotion tests,
node-prop replay/state/resource/presentation tests, retained
dependency/resource tests, failure rollback, retained primitive lifecycle, and
end-to-end performance probes.
