# ADR 0014: Rhai invocation, retained values, and effects

Status: Accepted

## Context

Rhai 1.26 FnPtr values can carry curry and captured environments. Imported
callbacks require module call context; gpui-rhai currently stores that context
through a volatile `internals` API. Arbitrary Dynamic values and captured
closures cannot provide durable equality, rollback, hot reload, or thread
safety.

## Decision

One internal `ScriptInvocationContext` adapter is the only code allowed to use
Rhai evaluator internals. It stores generation-bound named FnPtr/module context
and invokes component, event, effect, and timer functions. The crate pins Rhai exactly
and revalidates characterization tests before upgrades.

Durable data is schema-checked `UiValue`. Component invocations use a closed
`ComponentPropValue` union for generation-scoped nodes, styles, callbacks,
slots, refs, signals, and approved handles. Retained callbacks/effects/timers must be
named functions with UiValue-convertible curry; captured anonymous closures
cannot escape a synchronous evaluation.

Formal modules register their render function once with `define_component`.
The restricted resolver audits their top-level AST before evaluation: only
imports, literal constants, exports, and one direct literal
`define_component` declaration are accepted. They cannot hold mutable global UI
state or run arbitrary module-init calls. Effects and one-shot timers are pure
render descriptors reconciled after successful commit and cleaned on
replacement/unmount/reload.

## Consequences

The runtime, not Rhai, supplies transactions and dependency tracking. Engine,
Dynamic, FnPtr, and stored contexts remain foreground-only without the Rhai
`sync` feature. AST interpretation is the semantic oracle; optional Grain use
requires parity and performance evidence.

Protected by imported callback/component/effect tests, closure rejection,
generation cleanup, field/broad dependency tests, and AST/Grain parity probes.
