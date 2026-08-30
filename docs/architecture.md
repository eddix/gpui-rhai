# Architecture

GPUI Rhai keeps a hard boundary between script-owned declarations and
platform-owned rendering mechanisms.

## Runtime layers

1. Rhai source produces `UiNode`, `Style`, semantic events, and typed values.
2. The Rust runtime validates schemas, owns state, resolves modules and themes,
   and binds callbacks to a script generation.
3. A per-view `RetainedUiTree` validates and reconciles snapshots into stable
   `NodeId` identity; the GPUI renderer then creates short-lived native
   elements from the accepted tree.

No GPUI `Window`, `App`, `Context`, `Div`, or `AnyElement` enters a Rhai
`Dynamic`. Custom Rust primitives are the intentional extension point for
mechanisms that require those types.

The final atomic surface starts with `box(children)` and layout-transparent
`fragment(children)`. `row`, `column`, and `stack` construct Box snapshots.
Fragment may carry only children, source, key; nested fragments are flattened
before GPUI layout, so style/handlers/ref on a fragment are rejected instead of
silently introducing a wrapper.

`text("plain")` keeps the cheap uniform path. `text([span(...), ...])` retains
typed inline runs and renders one GPUI `StyledText` with byte-correct highlight
ranges; Span currently exposes color, bold, and italic without splitting text
into layout boxes or invoking Rhai during text layout.

`canvas(canvas_scene([...]))` retains a keyed vector command list. Rect, circle,
and stroked-line commands validate finite geometry, positive sizes, safe unique
keys, typed colors, and Host command budgets before commit. GPUI's Canvas paint
closure consumes the accepted Rust scene; it never calls Rhai during layout,
prepaint, or paint.

A trusted Rust Host may also build a tree directly and attach `HostCallback`
closures. Structural node data remains declarative, but this Host-augmented
subset contains opaque foreground event behavior and is not serializable pure
data. Rhai cannot construct or receive Host callbacks.

Schemas validate public values before native construction. The complex-control
line adds bounded numeric schemas, `one_of`, `Length`, and a schema that accepts
only values convertible to `UiValue`; it does not add an unrestricted Dynamic
escape hatch.

## Rendering transaction

`view(ctx)` returns a declarative snapshot. Formal component calls record
generation-scoped invocation recipes, and state/store writes dirty only their
tracked component boundaries. A candidate becomes active only after script
evaluation, retained validation, staged component-state commit, effect
transitions, signal reconciliation, and animation reconciliation succeed.
Failed renders and reloads restore one runtime/Engine checkpoint and keep:

- the previous AST generation;
- callback validity;
- the last-good node tree;
- component and store state;
- component definitions/render functions and retained invocation recipes;
- effect and native-signal ownership.

State schema changes are reconciled by stable component keys. Compatible fields
survive; incompatible fields reset to their declared defaults.

Formal Rhai modules register once with `define_component`; their PascalCase
constructors call `render_component(id, props)`. A render-local stack derives
nested instance paths and stages scoped state, event, effect, signal, and
invocation updates. Returned handlers carry component path, declared event
schema, generation, and an internal Rhai module-call context so later callbacks
resolve in their source module.

Formal render functions may declare named effects and typed native signals.
Effects compare immutable `UiValue` dependencies, run old cleanup before new
start, and retain original imported-module invocation context across event turns
and hot reload. Signals are non-owning component/key/type handles; values live
in Rust, writes never dirty a Rhai component, and approved style bindings are
sampled by the GPUI renderer without per-frame Rhai execution.

`UiEventHandler` joins event targets only at the node/primitive boundary.
Each event has ordered capture/target/bubble bindings. Responses independently
control default behavior, propagation, immediate propagation, and pointer
capture intent. Script handlers keep generation, component scope, Rhai context,
transactions, and runtime traces. Host callbacks execute their labeled Rust
closure directly. A schema-checked `NativeHandlerRef` lets Rhai attach a Host-
registered Rust fast path to the same ordinary node and outer transaction.
Because GPUI 0.2.2 has no direct pointer-capture API, the runtime retains
`pointer_id -> NodeId` ownership and reroutes window-capture move/up events to
the captured node until release, pointer-up, or unmount.

Pointer down/up/move and wheel input are normalized at the GPUI boundary into
stable `UiValue` maps with logical window/local/content coordinates, buttons,
modifiers, click count, precise delta, and a monotonic timestamp. Retained
`ElementRef` declarations bind to stable `NodeId`; prepaint reports committed
layout/visual geometry and exact geometry reads create component dependencies.
Unmounted refs fail stale instead of rebinding by name. `ctx.focus(ref)` queues
a window-scoped retained command; the Host view owns the corresponding GPUI
`FocusHandle` by `NodeId`, so no window/focus object crosses into Rhai.
Per-axis `Style.overflow_x_scroll/y_scroll` creates a retained GPUI
`ScrollHandle` for keyed ref nodes. `ctx.scroll_to(ref, x, y)` queues positive
visible offsets and applies them only after transaction commit.

`error_boundary(child, fallback)` catches native subtree rendering failures.
Use `error_boundary_lazy(Fn("child"), Fn("fallback"))` when Rhai construction
itself may fail; the fallback retains a redacted `boundary_error` diagnostic.
Custom primitive lifecycle/render panics are caught and converted to
`PrimitiveError` rather than unwinding through the runtime.

Foreground events, semantic actions, close callbacks, and async deliveries use
one outer transaction spanning callback, semantic dispatch, effect processing,
incremental render, validation, and reconciliation. A failure restores
component/store state, theme/locale selection, dirty sets, effects, signals,
queued UI events/actions, Engine generation, and component recipes. External
capability side effects are intentionally outside this transaction and must be
designed idempotently by the host; task, subscription, and image-decode handles
created by a failed callback are canceled before their results can deliver.

## Script lifecycle

The only required function is `view(ctx)`. `init(ctx)` and `dispose(ctx)` are
optional. Effectful APIs reject calls made during `view`.

During development, a polling filesystem watcher feeds a dependency graph.
Changed modules and their transitive dependants are compiled transactionally.
The GPUI Entity is updated only on the foreground thread.

## Capabilities

External effects are accessed through versioned Rust capabilities declared in
`ui/app.toml`. Inputs and outputs both use schema-checked `UiValue`; arbitrary
Rust values cannot cross this boundary. Async task and subscription handles are
implemented on top of the same registry; completions are delivered on the GPUI
foreground thread and stale script generations are rejected.

## View and window mechanisms

`FileScriptView` and `EmbeddedScriptView` prepare the core unit of execution.
Each mounted view owns an independent Engine, Runtime, Lifecycle, component
root, task/subscription scopes, and diagnostics. `ScriptApplication` is an
optional adapter that owns `Application` and native windows; existing GPUI
applications mount views directly.

Sibling views in one native window share a `ScriptViewHost`. The Host is an
interaction domain, not a state container: it owns overlay/tooltip/toast order,
absolute window-level placement, outside-click and Escape routing, focus
fallback, and approved key bindings. Local IDs are namespaced by `view_id`.
The view's automatically measured content bounds drive its responsive class,
while the independent Host overlay viewport normally covers the whole window.

Each native window runs the same compiled entry in a separate lifecycle,
component-state root, renderer path, and animation namespace. The runtime and
app stores are shared. Window stores, theme/locale overrides, overlays, and
window/component async scopes are released together on close. Rhai submits
validated commands and never receives a GPUI window handle.

Each standalone script window or embedded Host owns one Rust overlay coordinator. Popover, Dropdown, Dialog,
Menu, Tooltip, and Toast elements reserve their portal order during layout and
register measured anchor/panel bounds during prepaint. The coordinator owns
flipping/clamping, the parent-child dismiss stack, outside-click routing,
Escape routing, modal policy, and per-frame cleanup. Rhai supplies only stable
IDs, parent IDs, content, and controlled policy callbacks.

Keyed native controls retain GPUI Entities in element state. Business values
remain controlled Rhai props; only interaction transients such as focus,
selection ranges, open panels, visible months, search text, and scroll offsets
live in the native Entity.

Dropdown and Select share a private listbox core for validation, grouping,
filtering, keyboard navigation, and viewport-bounded option realization. Their
public value and composition semantics remain separate. DatePicker reuses the
same host overlay coordinator but owns a Gregorian calendar state machine.

Input and Textarea share a text-editing core for UTF-8/UTF-16 conversion,
grapheme boundaries, selection, clipboard, IME marked ranges, and controlled
reconciliation. Their layout elements remain separate: Input is a shaped
single line, while Textarea owns wrapped multiline layout, vertical hit testing,
multi-line selection paint, caret scrolling, and auto-grow measurement.

The current legacy Table path is a data-driven fixed-height native element. It
is scheduled for removal by Core Runtime v2 registry migration and is not a
privilege available to final registry components. Its scalar row maps are not
expanded into Rhai cell nodes before scrolling; GPUI `uniform_list` requests
only visible rows. A column with an explicit Rhai `cell_renderer` is the stated
exception: the callback runs for all rows during the complete view transaction,
then native element creation remains viewport-bounded. The runtime never calls
Rhai from a scroll/layout closure.

Short-lived render elements never enter persistent runtime state.

Window width is reduced to a configurable `compact`/`regular`/`wide` class.
Crossing a breakpoint reruns `view`; ordinary resizing remains native GPUI
layout and does not drive continuous Rhai evaluation.

## Locale, clock, and assets

Locale selection retains app/window/subtree precedence and additionally
resolves immutable calendar and number metadata. `YYYY-MM-DD` is the strict
date wire format; locale formatters produce presentation strings. A Runtime
Clock supplies the system-local current date, and hosts may inject a fixed Clock
without exposing wall-clock APIs to Rhai.

Component metadata declares small assets. Script preparation registers and
preloads them into `AssetRegistry`; asset-backed image nodes therefore resolve
only cached `AssetId` values during render. Capability-provided image handles
remain supported for dynamic application images.

## Source ownership

The CLI writes editable sources under `ui/` and pristine install baselines under
`.gpui-rhai/baselines/`. Updates compare the baseline, local source, and bundled
registry. A locally modified file is never silently overwritten.

See the security-boundary guide for the resolver, value, effect, diagnostic, and
native-extension trust boundaries.
