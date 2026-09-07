# ADR 0019: Component ownership across node props

## Status

Accepted for 0.1.1.

## Context

Rhai evaluates formal component constructors eagerly. A component passed as a
`Node` prop is therefore already rendered and mounted before the receiving
component decides where to present it. Its `UiNode` is not plain immutable
content: it represents a caller-owned component instance with state, a retained
invocation context, callbacks, effects, timers, signals, refs, virtual
collections, and a last accepted component-owned visual snapshot.

Incremental rerender originally stored the first node value in the receiver's
invocation recipe. If the passed component later rerendered independently, the
visible tree changed but the receiver recipe remained stale. Rerendering the
receiver replayed that old value, making current state-backed UI appear to reset
even though the caller-owned state and effects were still mounted.

Copying the current subtree back out of the accepted visual tree is also wrong.
That tree may contain styles, handlers, refs, signals, or animations added by
the receiver or a higher presentation owner. Replaying it through the receiver
would apply those mutations twice.

## Decision

Construction ownership and presentation ownership remain separate:

- The component path and all lifecycle resources belong to the context that
  constructed the node. Passing a node never reparents or adopts the component.
- Every `ComponentInvocationRecipe` owns a lightweight lazy snapshot handle.
  Passing the component through a node prop or applying the first outer
  presentation mutation activates an `Rc<UiNode>` containing its latest
  successful output after its own part styles, scope, and component-root marker
  are applied, but before any caller/receiver mutation. Ordinary components pay
  no deep-snapshot cost and keep using the accepted-tree bailout path.
- Completing another formal component seals its root: mutations accumulated
  while composing its children become part of that new component's owned
  snapshot.
- Fluent mutations made after a component-root boundary are recorded as typed
  presentation operations while also updating the effective node. The tracked
  surface includes keys, style/part style, attributes and handler payloads,
  handlers, signals, refs, and animations.
- Replaying node-valued props recursively hydrates every formal subtree from
  the latest generation-matching recipe snapshot across direct, optional,
  array, map, object, and union-shaped Dynamic values and every retained child
  family. Existing prop-level presentation operations are reapplied once; the
  receiver then executes and adds its own current presentation.
- Independently replacing a component subtree transfers the accepted node's
  outer presentation operations onto the new component-owned result. Child
  state updates therefore do not erase slot styles or callbacks.
- Later native virtual-item realization updates an active owning snapshot in the
  same transaction, so replay does not regress into repeated realization.
- Active snapshot values participate in `RuntimeEngineCheckpoint`; rejected
  render, retained-budget, effect, or hot-reload candidates restore the prior
  `Rc<UiNode>` value before any receiver can hydrate from the handle.
- Node-valued props remain conservatively unequal for component bailout. The
  receiver must rerun when its caller reconstructs the prop, but receiver-local
  rerenders do not execute the passed component again.

Node-prop construction is eager. If a receiver temporarily omits a passed node
from presentation, the caller-owned component remains mounted. A caller that
wants unmount cleanup must stop constructing/passing that component. This is the
same retained-content policy used by closed overlays.

## Invariant matrix

| Execution path | Component-owned snapshot | Presentation operations | State/resources |
| --- | --- | --- | --- |
| Initial/full render | Install lazy owner; activate only for node transport/presentation | Apply caller/receiver layers once | Mount under construction path |
| Component bailout | Replay active recipe-owned output, otherwise accepted subtree | Caller executes and reapplies current layers | Retain invocation closure unchanged |
| Passed component dirty | Replace with new owned output | Transfer current outer layers | Preserve state; reconcile only its declarations |
| Receiver dirty | Hydrate node props before invoking receiver | Start from prop layers, then add receiver layers once | Do not execute/restart passed component |
| Child and receiver dirty together | Later operation observes earlier recipe update; later child replacement preserves layers | Order-independent final presentation | One state/resource transition per dirty owner |
| Nested Node containers/kinds | Recursive hydration | Preserve layers at every component boundary | Preserve original construction paths |
| Caller removes component | No snapshot enters the successful active graph | Removed with its presentation | Cleanup state/effects/tasks/timers/signals/refs normally |
| Failed rerender | Do not publish candidate recipe snapshot | Keep last-good accepted layers | Restore runtime/Engine checkpoint |
| Hot reload | Rebuild snapshots in candidate generation | Reconstruct from candidate render | Never hydrate from an old generation |
| Suspend/resume | Retain snapshots while no Rhai executes | Retain with the tombstoned tree | Existing suspend/resume resource policy applies |

## Rhai and thread boundary

This is a Rust host-runtime feature, not Rhai reactivity. Rhai remains pinned to
1.26 without `sync`; `UiNode`, `Dynamic`, `FnPtr`, recipe snapshots, and stored
call contexts stay on the GPUI foreground thread. Nodes carry only `Weak`
snapshot references, recipes own the corresponding `Rc`, and active snapshot
values are shared through `Rc<UiNode>`. This avoids cycles and prevents
transaction checkpoints/reuse plans from deep-copying every subtree.

An alternating 30-sample release A/B on Macmini9,1 kept the final lazy design in
the current-main noise range: unchanged rerender p95 5.06ms versus
4.96–5.10ms, reverse 11.25ms versus 11.10–11.15ms, selection 20.40ms versus
20.14–20.88ms, and native resize 12.74ms versus 13.22–13.29ms. Every unchanged
sample retained 26 realized rows and executed zero virtual Rhai work. A rejected
mandatory-snapshot prototype measured a repeatable 13–17% selection regression
and was not retained.

## Rejected alternatives

- Forcing every node-prop interaction through a full root render is correct but
  discards component-level incremental execution and makes hot widgets costly.
- Treating structural `UiNode` equality as an ownership proof cannot establish
  callback context, generation, or resource identity.
- Copying accepted visual subtrees contaminates component output with old outer
  presentation and can duplicate handlers/styles.
- Reparenting a component when it enters a receiver would require migrating
  state and every retained resource identity after Rhai has already executed.
- Serializing/hydrating state would preserve neither invocation context nor the
  native/resource graph and creates a second lifecycle model.
