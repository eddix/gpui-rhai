# ADR 0015: Atomic UI, events, and native hot state

Status: Accepted

## Context

The first runtime exposed row, column, text, limited Style, click, semantic keys,
and high-level native controls. It could compose the registry but could not
express the custom layout and pointer interactions available in a general UI
binding. Adding a `div` alias alone would not close that gap.

## Decision

Core exposes typed fragment/box/text/span/image/svg/canvas atoms and an
exhaustive-by-default safe mapping of stable GPUI layout, paint, text, and 2D
scene capabilities. Row/column/stack are Box helpers. No GPUI value crosses the
UiNode boundary and no CSS/DOM compatibility is claimed.

Events use capture/target/bubble, ordered Script/Host handler lists, normalized
pointer/window/local/content coordinates, default prevention, propagation
control, and pointer capture. Trusted Rust may register schema-checked
`NativeHandlerRef` values that Rhai binds to ordinary nodes.

The dispatcher snapshots the current handler node's committed visual bounds at
event time. Raw pointer/wheel payloads carry the snapshot as `target`, Script
handlers read it through untracked `ctx.event_target_bounds()`, and trusted Rust
handlers receive `NativeEvent::target`. Click and semantic payload schemas stay
unchanged. The snapshot follows `currentTarget` rather than inferred logical
ancestry; callbacks with no retained-node dispatch receive no target. Tracked
cross-render geometry remains the separate `ElementRef` API.

`NativeSignal<T>` is the only approved high-frequency imperative property path.
It updates typed Style/transform/scroll/Canvas properties without component
rerender. Animation uses the same property-source model and remains native per
frame.

## Consequences

The runtime owns retained hit testing, geometry/ref registries, per-frame event
coalescing, frame budgets, animation clocks, and reduced-motion policy. Raw
pointer flexibility is public to Rhai; custom primitives are an optimization or
domain-native extension, not an expressiveness gate.

This ADR supersedes ADR 0012's rejection of named Host handlers and multiple
handlers. Direct Rust HostCallback remains supported as one handler variant.

Protected by atomic-only visual examples, pure-Rhai Timeline, Rust fast-path
parity, pointer capture/propagation, Canvas hit, a real GPUI click-to-Rhai
geometry comparison, and deterministic animation tests.
