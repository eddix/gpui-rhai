# ADR 0012: Host callbacks on Host-owned UiNode trees

Status: Superseded in part by ADR 0015

## Context

Some trusted GPUI hosts already own a plain-data UI protocol and translate each
frame into `UiNode`. Display-only trees can use the renderer directly, but every
interactive handler previously required a generation-bound Rhai
`ScriptCallback`. That forced these hosts to add a mechanical Rhai template,
frame subscription, JSON expansion, and event capability even when no UI logic
belonged in gpui-rhai.

## Decision

`UiEventHandler` is the event target stored by `UiNode` and callback-typed
primitive props. It has independent `Script(ScriptCallback)` and
`Host(HostCallback)` variants. `ScriptCallback` retains generation, component
scope, native Rhai context, transactional dispatch, and stale-callback checks.

`HostCallback` contains a required diagnostic label and an opaque `Rc` closure:

```text
Fn(UiValue, &mut Window, &mut App) -> EventPropagation
```

It is constructed only through Rust APIs, executes synchronously on the GPUI
foreground thread, follows ordinary Rust closure lifetime rules, and is invoked
directly by the renderer. The Host owns blocking, errors, side effects,
observability, stale work, and reference-cycle prevention. Callback equality is
the label plus `Rc` pointer identity; cloning a tree preserves identity, while
rebuilding a closure creates a new handler.

Primitive event payloads still pass through the descriptor's `EventSchema`
before either handler kind executes. Rhai `ValueSchema::Callback` conversion can
construct only the Script variant. Host callbacks never cross `UiValue`,
capability, serialization, or Rhai `Dynamic` boundaries.

No Host runtime, View type, registry, provenance flag, generation manager,
async callback, automatic trace, or Rust component-builder layer is added.
Existing `GpuiNodeRenderer` and `StaticUiView::set_root` become interactive for
Host-built trees without assuming ownership of Host state or frame delivery.

## Consequences

The Script-produced subset of `UiNode` remains declarative and auditable. A
trusted Rust Host may augment a tree with opaque foreground event closures, so
the type as a whole is no longer absolute pure data. Debug and Inspector output
show only `host:<label>` and never closure addresses or captures.

A callback stored in a tree must not strongly capture the Entity that owns that
tree. Hosts should capture channels or `WeakEntity`; gpui-rhai cannot infer or
repair arbitrary Rust ownership cycles.

## Rejected alternatives at the time

- Adding a Host variant to `ScriptCallback`, which would blur generation and
  transactional semantics.
- A callback registry or generated IDs, which would duplicate Host lifecycle
  and cleanup work.
- Named Host handlers referenced from Rhai, which does not remove the adapter
  script in the motivating integration.
- Host-built/script-built provenance tracking, which adds no sandbox protection
  beyond restricting construction to Rust.
- Multiple handlers per node/event, which introduces ordering and propagation
  ambiguity.
- Async Host callbacks or automatic worker/root management, which would create
  a second task and state runtime.

Protected by Host-only pointer/keyboard tests, callback-typed TextInput and
custom-primitive tests, drop/identity/Inspector tests, existing Script callback
generation regressions, and the `host_owned_tree` release example.

ADR 0015 later adopts ordered multi-handlers and schema-checked named native
handlers as part of the general atomic event platform. The direct Rust-owned
`HostCallback` variant and its closure ownership rules remain valid.
