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

The tree retains the last successful mounted/preserved/moved/unmounted report;
failed candidates leave both the tree and report unchanged. Hosts read this
deterministic result through `ScriptViewHandle::reconcile_report`.

No GPUI `Window`, `App`, `Context`, `Div`, or `AnyElement` enters a Rhai
`Dynamic`. Custom Rust primitives are the intentional extension point for
mechanisms that require those types.

The final atomic surface starts with `box(children)` and layout-transparent
`fragment(children)`. `row`, `column`, and `stack` construct Box snapshots.
Fragment may carry only children, source, key; nested fragments are flattened
before GPUI layout, so style/handlers/ref on a fragment are rejected instead of
silently introducing a wrapper.
`stack` establishes relative positioning; children may use typed
`Style.absolute().left(...).top(...)`. The same Style value now maps bounded
flex/grid, four-edge spacing/positioning, wrapping/grow/shrink, gradients,
shadows, opacity, visibility, cursor, typography, truncation, and static paint
translation. NativeSignal translation remains the hot path for playheads and
drag surfaces.

`text("plain")` keeps the cheap uniform path. `text([span(...), ...])` retains
typed inline runs and renders one GPUI `StyledText` with byte-correct highlight
ranges; Span currently exposes color, bold, and italic without splitting text
into layout boxes or invoking Rhai during text layout.

`canvas(canvas_scene([...]))` retains a keyed vector command list. Rect, circle,
line, and arbitrary move/line/quadratic/cubic/close paths validate finite
geometry, positive sizes, safe unique keys, typed solid/two-stop gradient
paint, uniform transform, axis-aligned clip, and Host complexity budgets before
commit. GPUI's Canvas paint closure consumes the accepted Rust scene; it never
calls Rhai during layout, prepaint, or paint.

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
tracked component boundaries. Locale formatter/message/direction reads and
discrete viewport-class reads register the same exact component dependency;
geometry reads already bind to a retained `NodeId`. Locale and viewport changes
therefore rerender only readers, while a separate per-window repaint queue
handles native direction/paint changes that require no Rhai execution. Theme
Style values remain symbolic tokens and resolve in the renderer, so no script
theme-token dependency is necessary. A candidate becomes active only after script
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

The AST interpreter remains the semantic oracle. Named entry/lifecycle/root
calls cross a static execution-backend trait; stored imported callbacks remain
behind the volatile invocation-context adapter. The optional `grain-backend`
feature is a parity harness, not a production switch, and currently records a
diagnostic-format blocker. See [Rhai execution backends](rhai-execution-backends.md).

Formal render functions may declare named effects and typed native signals.
Effects compare immutable `UiValue` dependencies, run old cleanup before new
start, and retain original imported-module invocation context across event turns
and hot reload. Every effect activation receives a distinct Host async scope;
tasks, subscriptions, and image decodes started from that context are cancelled
automatically only after a replacement successfully starts or the effect is
removed. Signals are non-owning component/key/type handles; values live
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
layout geometry plus visual bounds after static/signal/animation translation;
pointer local coordinates invert that visual translation before Canvas hit
testing. Exact geometry reads create component dependencies.
Unmounted refs fail stale instead of rebinding by name. `ctx.focus(ref)` queues
a window-scoped retained command; the Host view owns the corresponding GPUI
`FocusHandle` by `NodeId`, so no window/focus object crosses into Rhai.
Semantic attributes and static text are copied into RetainedNode and projected
as a layout-flattened AccessibilityTree with committed geometry. Host tooling
queries it by role/name or semantic ID; the pinned GPUI platform bridge remains
the final forwarding boundary.
Per-axis `Style.overflow_x_scroll/y_scroll` creates a retained GPUI
`ScrollHandle` for keyed ref nodes. `ctx.scroll_to(ref, x, y)` queues positive
visible offsets and applies them only after transaction commit. Ref descendants
also retain a GPUI `ScrollAnchor` for nearest-ancestor `scroll_into_view`
without assuming they are direct children.

`error_boundary(child, fallback)` catches native subtree rendering failures.
Use `error_boundary_lazy(Fn("child"), Fn("fallback"))` when Rhai construction
itself may fail; the fallback retains a redacted `boundary_error` diagnostic.
Custom primitive lifecycle/render panics are caught and converted to
`PrimitiveError` rather than unwinding through the runtime.
Keyed lifecycle primitives retain their prior normalized instance snapshot and
run `mount -> update(previous, next) -> render -> unmount`; failed update/render
does not replace the retained instance record.

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

Each standalone script window or embedded Host owns one Rust overlay coordinator. Generic Overlay nodes used by Popover, Dropdown, Dialog,
Menu, and Tooltip reserve their portal order during layout and
register measured anchor/panel bounds during prepaint. The coordinator owns
flipping/clamping, the parent-child dismiss stack, outside-click routing,
Escape routing, modal policy, and per-frame cleanup. Rhai supplies only stable
IDs, parent IDs, content, and controlled policy callbacks.

Generic Layer nodes place arbitrary content at a window corner, center, or fill
region with a bounded priority. In an embedded Host they register a view-ID-
namespaced element with one shared deferred portal; standalone views render the
same node directly. Toast is Rhai source over Layer plus component-scoped
declarative one-shot timers. Timer reconciliation, pause/resume, stale
generation rejection, scope disposal, and transaction rollback are generic
runtime behavior rather than a native toast queue. Timer deadlines and
animation sampling use the same Host-injected monotonic `RuntimeClock`, so
automation can advance both deterministically without sleeping.

Generic keyed native mechanisms retain GPUI Entities in element state. Business values
remain controlled Rhai props; only interaction transients such as focus,
selection ranges, open panels, visible months, search text, and scroll offsets
live in the native Entity.

Dropdown is copied Rhai source over public Overlay, Input, semantic events, and
data-backed `virtual_collection`; Select is its scalar Rhai adapter. Validation,
grouping, filtering, keyboard navigation, selection, and presentation remain
inspectable source. Rust contributes only the same generic overlay placement,
text editing, input routing, and virtual measurement available to application
scripts. DatePicker is likewise Rhai composition; checked Gregorian arithmetic
and localized month formatting are generic data APIs rather than a specialized
UI node.

Input and Textarea share a text-editing core for UTF-8/UTF-16 conversion,
grapheme boundaries, selection, clipboard, IME marked ranges, and controlled
reconciliation. The shared core also owns bounded native undo/redo, typing
coalescing, IME-as-one-revision, and external-controlled reset. Their layout elements remain separate: Input is a shaped
single line, while Textarea owns wrapped multiline layout, vertical hit testing,
multi-line selection paint, caret scrolling, and auto-grow measurement.

Table is now a copied Rhai composition over Box/Text and data-backed
`virtual_collection`. Its former native constructor, UiNodeKind, Entity,
fixed-height renderer, private scrollbar fields, eager cell renderers, and Rust
public model were deleted. GPUI layout only queues indices; named/formal Rhai
row renderers execute on the next foreground runtime transaction.

Short-lived render elements never enter persistent runtime state.

Window width is reduced to a configurable `compact`/`regular`/`wide` class.
Crossing a breakpoint reruns `view`; ordinary resizing remains native GPUI
layout and does not drive continuous Rhai evaluation.

## Locale, clocks, and assets

Locale selection retains app/window/subtree precedence and additionally
resolves immutable calendar and number metadata. `YYYY-MM-DD` is the strict
date wire format; locale formatters produce presentation strings. A
`CalendarClock` supplies the current civil date and may be fixed by the Host.
The separate monotonic `RuntimeClock` drives timers and animation and may be
replaced with `ManualRuntimeClock` for deterministic automation. Neither clock
is readable by Rhai.

Component metadata declares small assets. Script preparation registers and
preloads them into `AssetRegistry`; asset-backed image nodes therefore resolve
only cached `AssetId` values during render. Capability-provided image handles
remain supported for dynamic application images.

Theme selection uses the same app/window/subtree precedence and generation
switch. Required semantic colors/spacing/radii remain the base typed schema;
families may add namespaced typed maps such as `charts.series_a` with
color/length/number/string values. Namespace/name/type and finite/non-nested
length rules are validated before an atomic theme generation becomes active.

## Source ownership

The CLI writes editable sources under `ui/` and pristine install baselines under
`.gpui-rhai/baselines/`. Updates compare the baseline, local source, and bundled
registry. A locally modified file is never silently overwritten.

See the security-boundary guide for the resolver, value, effect, diagnostic, and
native-extension trust boundaries.
