# ADR 0018: Retained view suspension

## Status

Accepted for 0.1.1.

## Context

Hosts switch among expensive script-owned panels. Dispose/remount correctly
releases work but destroys local UI state, input history, scroll position, and
native measurements and replays data-fetch effects. Merely removing an element
from Host layout retains those values but also leaves timers, streams, polling,
layers, and callbacks active.

Snapshot/hydrate would preserve only serializable component state and would
still reconstruct native state and replay every effect. It would also create a
second migration and persistence contract before the runtime needs one.

## Decision

`ScriptViewHandle` exposes an explicit three-state lifecycle: `Active`,
`Suspended`, and terminal `Disposed`. Suspension retains the complete mounted
view as a tombstone but removes presentation and execution activity:

- optional `suspend(ctx)` runs, then declarative effects clean up and their exact
  activation scopes cancel;
- subscriptions are required to originate in an effect;
- declarative timers and animations freeze without elapsed-time catch-up;
- overlays and Layers close, focus falls back to the Host, and pointer capture
  clears;
- no UI lifecycle, event, or render Rhai callback runs until resume;
- Host-owned signals, collections, documents, theme, and locale may still change
  and leave accumulated dirty state;
- ordinary in-flight task completions enter a bounded runtime queue, while
  effect-owned work is cancelled.

Resume is transactional. It applies current valid task results, invokes optional
`resume(ctx, elapsed_ms)`, performs one full render/reconcile, then starts new
effect activations. Failure restores the suspended last-good state. Development
file changes are recorded while suspended and compiled on resume; the latest
successful generation migrates in the same transaction and invalidates pending
deliveries from older generations.

Suspended handles reject new layout elements and interaction APIs. Hosts own
LRU policy and use normal disposal to evict a tombstone.

## Consequences

Panel switching can preserve complete UI continuity without background polling.
The runtime avoids a partial snapshot format and keeps a single cleanup owner for
streams. Hosts must explicitly omit suspended elements from rebuilt layouts and
must bound their own tombstone count. External capability side effects remain
outside rollback and still need idempotent Host implementations.
