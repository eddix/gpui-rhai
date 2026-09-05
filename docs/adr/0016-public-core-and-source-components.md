# ADR 0016: Public core and source-only compositional components

Status: Accepted

## Context

The registry is intended to follow source ownership, but several complex
components depended on specialized native nodes unavailable to ordinary Rhai
authors. That made the project appear like a component library without a
complete underlying UI platform.

## Decision

The Rust core/runtime and copied registry are separate product layers. Every
compositional official component uses public atomic and headless mechanisms.
Native code remains only for generic platform mechanisms: text editing/IME,
scrolling, focus/accessibility, deferred layers, generic virtualization,
Canvas, animation/signals, input routing, assets/fonts, and retained primitive
lifecycle.

Table, DatePicker, Select/Combobox, Menu, Tabs, Toast presentation, Pagination,
and other high-level components are Rhai source. Generic virtualization supports
data-backed variable-height item components plus explicit sticky section-header
indices, single-instance presentation, and push-off. Table grouping/collapse,
counts, events, semantics, and styling remain source policy; NativeCollection
may cache its flattened sort/group/collapse data order without becoming a
private native Table API.

Dogfooding migration is destructive: old component declarations, Style/atomic
APIs, specialized native UI nodes, and compatibility shims are deleted. Version
remains 0.1.0 and Runtime API remains 1 until an explicit release signal.

## Consequences

Registry migration is a completeness test for core. Official components cannot
receive hidden layout, event, focus, scroll, overlay, or virtualization
privileges. CLI source baselines, metadata, docs, examples, and tests move
together.

Protected by registry API-surface audit, copied-source examples, CLI snapshots,
and final component/visual/interaction matrices.
