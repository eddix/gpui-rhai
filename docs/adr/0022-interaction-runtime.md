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

### One gesture state machine

The internal `GestureSession` owns the common pointer lifecycle:

`idle -> armed -> active -> finished`, with cancellation from every
non-terminal state. It records the start/current/previous pointer sample,
threshold and structured owner. A successful script rerender invalidates an
older active native gesture before another pointer sample can commit it; native
controls additionally compare the source/constraint contract they captured.

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
session, so only the owning collection pins the active key; unrelated lists
are not scanned. Drop preview never mutates the canonical collection.

Tree uses a shared flattened-outline projection with stable key, parent, depth,
expanded/loading/disabled/navigation metadata. It reuses virtual collection,
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

Task and subscription payloads are recursively checked against the same
string/array/map limits configured on the Rhai Engine before a callback starts.
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
