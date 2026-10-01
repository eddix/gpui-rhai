# ADR 0022: Unified two-dimensional Interaction Runtime

- Status: Accepted
- Target: 0.1.8
- Date: 2026-09-28

## Context

GPUI Rhai 0.1.7 has a generic retained event router, per-view pointer capture,
committed geometry, native signals, Motion, Resizable and SplitPane. It also has
specialized native interactions for Table column resize, Slider, ScrollArea and
Chart. These capabilities prove the required mechanisms, but they grew as
separate state machines.

The 0.1.8 component scope adds Draggable, DragSource, DropZone, Sortable,
PanZoom, SelectionArea, Rotatable, RangeSlider and Tree. Implementing them as
new isolated primitives would duplicate gesture ownership, coordinate
conversion, constraints, preview/commit state and lifecycle cleanup. The audit
in `docs/audits/2026-09-28-interaction-foundation/` records the existing
duplication and migration boundaries.

## Decision

### One interaction domain per ScriptViewHost

`ScriptViewHost` owns a `WindowInteractionCoordinator` alongside its overlay
coordinator. It is the single window/Host-domain authority for active pointer
sessions, application drag payloads, eligible drop targets and cancellation.
It installs one move/up/cancel route per frame rather than one complete set of
window listeners per handle.

Views register frame-local targets with stable ownership:

- Host/window domain and view ID;
- retained/native node identity, whose allocation changes across a logical
  unmount/remount even when the local component key is reused;
- pointer ID, interaction kind and target priority;
- presented geometry and supported operations.

Unmount, suspend, view removal, Host removal, window close, disable, stale
generation and lost pointer ownership all cancel through the same path. Drop
sessions may cross views only inside the same Host interaction domain. OS file
drops and cross-window drag remain explicit Host/platform integrations.

Primitive suspend/resume/compensation runs inside the owning ScriptHostView
update. During that lifecycle lease, signal reads and writes use the runtime
state directly instead of re-reading or updating the same GPUI Entity. This
single phase boundary covers idle controls as well as active cancellation.

A retained primitive has one canonical instance identity: primitive type,
presented retained key and retained `NodeId`. Constructor-local keys remain
configuration data; mount, render, focus, accessibility, Automation and
unmount never reconstruct identity from a different key surface.

### One gesture state machine

The internal `GestureSession` owns the common pointer lifecycle:

`idle -> armed -> active -> finished`, with cancellation from every
non-terminal state. It records the start/current/previous pointer sample,
threshold and structured owner. A successful script rerender invalidates an
older active native gesture before another pointer sample can commit it; native
controls additionally compare the source/constraint contract they captured.
The invalidation mark is produced by the common successful render-commit path,
including node, native, timer, task and subscription entry points. Internal
virtual realization is presentation work and does not invalidate a business
gesture by itself.

Behaviors supply pure policy rather than another event loop:

- transform a normalized pointer sample into a preview;
- clamp/snap against typed constraints;
- produce one semantic proposal at completion;
- clear or restore preview on cancellation or controlled rejection.

Draggable, existing Resizable, SplitPane, Table column resize, Sortable,
SelectionArea and Rotatable use this lifecycle. Slider/RangeSlider and
Scrollbar reuse the pointer/session and axis math while retaining their own
value/scroll semantics. Chart reuses normalized gesture input and affine
mapping but keeps its coordinate-typed viewport acknowledgement and link model.

### Atomic native preview

Native preview is a typed `PreviewPatch`, not a sequence of unrelated signal
writes. A patch validates every signal/value first, then changes all values and
requests at most one repaint. A four-field rectangle therefore cannot expose an
intermediate x/y/width/height combination or perform four Entity updates.

Controlled components keep preview state separate from the source value and
emit one deferred semantic proposal. A source/constraint rerender cancels the
captured gesture generation and restores the new controlled source. Components
whose proposal can be rejected without changing the source (for example Table
column width) clear the native override after proposal dispatch. A stale
gesture therefore cannot overwrite a newer Host value, and a rejected proposal
cannot remain painted as though it was accepted.

An uncontrolled Table column keeps its accepted native width separately from
the current gesture rollback point. Cancellation restores the value at gesture
start rather than erasing an earlier accepted resize.

Pointer, keyboard and autofit variants of one control share proposal cleanup.
Native Automation policies return a schema-checked `PrimitiveSemanticProposal`
to the ScriptView boundary; Automation executes it synchronously and reports
the real callback success or failure rather than merely confirming that work
was queued.

### Explicit coordinate and transform model

The runtime adds one finite `Affine2D` representation with composition,
forward/inverse point mapping and transformed bounds. Presented geometry joins:

- local bounds;
- layout and visual window bounds;
- local-to-window and window-to-local transforms;
- clip and scroll ancestry;
- stable owner identity.

Canvas paint/hit testing, Motion affine samples, PanZoom, Rotatable,
SelectionArea and drag/drop target resolution consume this model. Chart domain
scales remain separate and explicitly convert at their boundary. UI scale,
content zoom, DPI, RTL and logical axes are never treated as aliases.
Rotatable reads the actual styled Canvas node, and a geometry revision change
during a gesture cancels against current geometry. Degenerate marquee input is
decided in window logical pixels; content-space predicates use relative numeric
tolerance rather than a fixed area in arbitrary Canvas units.
The GeometryRegistry separately records the border/layout box and the actual
inner Canvas drawable bounds measured by GPUI, including snapped element
offsets. Canvas paint, inverse hit mapping and pivot compensation share that
drawable rectangle. Area and intersection predicates use translated edge
vectors so content origin and supported zoom do not change selection.

### Typed primitive boundary

`PrimitiveEventEmitter` currently emits events, writes signals and reads
geometry. It becomes a focused `PrimitiveContext` passed to native handlers.
The context owns typed prop access, atomic preview writes, deferred semantic
proposals, geometry reads and interaction registration. Repeated local
`number_prop`, `string_prop`, `bool_prop`, `signal_prop`, `ref_prop` and
`style_prop` helpers are removed after callers migrate.

This is a deliberate pre-1.0 Rust API cleanup. Rhai receives no GPUI object or
interaction-runtime handle.

### Drag and collection boundaries

Position movement and data transfer remain different public concepts:

- `Draggable` changes one controlled position;
- `DragSource` begins a bounded typed application drag session;
- `DropZone` accepts declared payload types/operations;
- `Sortable` proposes keyed order changes using source and insertion-anchor
  identity, never stale numeric indices.

Sortable edge auto-scroll uses existing scroll handles. Every list has a
component-scoped collection identity distinct from its public local key.
Virtualized sorting carries the source projection index in the native drag
session plus a bounded source snapshot. The owning virtual collection renews a
logical source lease while the same member remains valid, even when its row is
offscreen; actual drop targets still require presented hitboxes. Variable-list
`ListState` and ordinary `ScrollHandle` both implement the same native edge
auto-scroll boundary. Auto-scroll belongs to the accepting destination and its
scroll ancestry, not to the source collection. Unrelated lists are not scanned.
Drop preview never mutates the canonical collection.
Virtual destinations carry a retained container identity. Auto-scroll follows
the accepting target's real ancestor chain and can continue through row gaps
only while the validated container hitbox remains visible and unoccluded.
Stationary scrolling uses one bounded native tick after a presented frame;
freshly registered hitboxes are not queried before that frame is available.

Escape is intercepted at the Host/window interaction domain while a session is
active. It cancels the actual pointer, application-drag or wheel owner even if
its source row is offscreen or another control has keyboard focus; with no
active owner, Escape continues to overlays and application shortcuts. Window
deactivation uses the same cancellation boundary.

Tree uses a shared flattened-outline projection with stable key, parent, depth,
expanded/loading/disabled/navigation metadata, including its nearest enabled
ancestor. It reuses virtual collection,
reveal and selection mechanisms rather than adding another list renderer.
Table, Command and Combobox may reuse common flat grouping/order helpers, but
their product-specific projections remain distinct.

### Rhai and foreground boundary

Pointer move, wheel, auto-scroll, target resolution, geometry sampling and
preview stay in Rust on the foreground thread. Rhai receives bounded proposals
such as move, drop, reorder, transform, selection or range committed. No
per-move Rhai callback or complete-tree evaluation is introduced.

Stored Rhai callbacks remain behind `ScriptInvocationContext`. Delayed/retained
dispatch starts from a fresh call-depth baseline while synchronous nested calls
retain the real recursion guard; this closes issue #80 without raising the
global call-level limit.

Virtual collection scopes are structural namespaces only. They may own item
identity, but callback props retain the real formal/root component owner and
incarnation. Effect deliveries additionally carry an activation lease checked
immediately before invocation, so a message drained before cleanup cannot run
after that activation is replaced.
Initial and delayed virtual realization share the same structural owner rules.
Their commit manifest covers all target rows, retaining unchanged component
resources and releasing removed rows even when there is no new row to execute.
Raw row reads belong to the caller's executable render boundary; formal row
components own their own resources and dependencies. Cancelled async scopes are
discarded before callback-owner checks, while explicit stale callbacks remain
errors. Debounce and explicit wheel completion both release Escape ownership.

Task and subscription payloads are recursively checked against the same
string/array/map limits configured on the Rhai Engine before schema traversal
or callback execution starts.
Online delivery schema validation stops at the first bounded diagnostic;
offline tooling may still collect complete issue lists. Each successful item
in an async batch records its committed interaction-contract invalidation even
when a later independent item fails.
An undeliverable success becomes one bounded error delivery; arbitrary callback
failures are never retried after user or Host side effects may have occurred.

## Public 0.1.8 surface

The target additions are Draggable, DragSource, DropZone, Sortable, PanZoom,
SelectionArea, Rotatable, RangeSlider and Tree. Resizable and SplitPane migrate
to the shared runtime without changing their distinct public responsibilities.

Only one canonical name is exported for each concept. There are no Droppable,
Reorderable, Pannable or Zoomable aliases. Drag preview is a slot/presentation
policy, not a separate component.

## Deferred boundaries

- DockLayout is a later layout-tree/workspace system.
- Cross-window and OS/file drag require Host/platform authority.
- 3D remains a separate feature/version.
- Breadcrumb, Sidebar, HoverCard, Drawer and Menubar remain ordinary
  compositions unless a future requirement proves missing semantics.

## Acceptance

Before new components ship, existing Resizable, SplitPane, Table resize,
Slider, ScrollArea and Chart interaction tests must pass through the shared
mechanisms. Acceptance covers pointer/keyboard parity, threshold, grab offset,
constraints, snapping, cancellation, controlled acceptance/rejection, nested
targets, scrolling, virtualization, RTL, transforms, suspend/resume, unmount,
window close, stale generations and bounded work. The Gallery contains one real
integrated scene that composes move, resize, rotate, pan/zoom, selection,
drag/drop and sorting rather than isolated callback demonstrations.
