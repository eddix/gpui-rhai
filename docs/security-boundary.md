# Security boundary

GPUI Rhai treats scripts as application-owned UI code, not as a general-purpose
system automation environment. Rhai can construct validated nodes, read declared
state, and request explicitly registered services. It cannot receive GPUI
contexts, arbitrary Rust values, filesystem paths, URLs, sockets, or process APIs.

## Enforced boundaries

- Imports resolve only through `ScriptSource`; absolute paths, parent traversal,
  undeclared modules, and cycles are rejected.
- `UiValue` permits scalar data, arrays, string-keyed maps, and typed opaque
  handles. GPUI elements and arbitrary `Dynamic` Rust values cannot cross it.
- Capability inputs and outputs are validated against their declared schemas.
- Mutations and effect execution are rejected during `view(ctx)`; formal render
  may only emit pure named effect, timer, signal, ref, and node descriptors.
- Callbacks, tasks, subscriptions, declarative timers, and image decodes bind to an AST generation;
  stale work is discarded after hot reload.
- Script operation, expression-depth, call-depth, array, and map limits are set
  by the runtime.
- Host-configurable `RuntimeBudgets` independently reject an otherwise valid
  candidate before commit when retained nodes, event handlers, formal
  components, effects, timers, signals, element refs, generic Layers, Canvas
  scenes/commands, or virtual data/realization counts exceed their independent
  limits.
- Window commands accept bounded sizes and validated stable IDs. Scripts cannot
  access native window handles.
- Diagnostics redact fields marked `sensitive`; capability payloads should be
  treated as sensitive unless a host explicitly decides otherwise.
- `HostCallback` can be constructed only by trusted Rust code. It cannot enter
  Rhai, `UiValue`, capabilities, serialization, or script callback schemas.
- `NativeHandlerRef` is resolvable by Rhai only after trusted Rust registers a
  namespaced descriptor; declared event payload schemas are checked before the
  Rust handler runs.

## Host responsibilities

Only register the capabilities a particular application needs. Keep credentials
inside Rust handlers, return the smallest useful `UiValue`, validate opaque
handle ownership, and perform authorization again at the service boundary.
Application-owned Rhai files have the same trust level as that application's
Rust source; do not load unreviewed remote scripts at runtime.

The runtime uses safe Rust. A custom primitive is native Rust code and therefore
belongs to the host trust boundary, not the script sandbox.

A Host-augmented `UiNode` may contain an opaque Rust event closure. The Host
owns its blocking behavior, side effects, stale references, and retain cycles;
scripts retain the existing generation-bound and transactional callback model.
