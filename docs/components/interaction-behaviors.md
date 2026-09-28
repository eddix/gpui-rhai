# Direct manipulation and layout behaviors

This is the maintained responsibility and design contract for interactions that
move, resize, reorder, select, or arrange content. These behaviors compose with
ordinary components; they do not prescribe a separate visual skin or Runtime.

Implementation status was updated on the 0.1.7 acceptance branch on 2026-09-28.
**Planned entries below are design requirements, not callable Rhai APIs.** Their
labels identify capabilities; exact exports, props, events, and style parts must
be specified alongside implementation. They do not increase the implemented
component count in the [catalog](catalog.md). Repository implementation status
does not imply availability in an already published crate.

## Capability map

| Capability | Responsibility | Typical use | Current status |
|---|---|---|---|
| Draggable | Change one object's position in a declared coordinate space | Floating card, canvas node, movable overlay | Implemented as `components/draggable::Draggable` |
| DragSource | Begin an in-app typed payload drag without changing accepted object position | Resource tile, command item, transferable card | Implemented as `components/drag_source::DragSource` |
| Resizable | Change one object's size or bounds using edges/handles | Floating card, image frame, adjustable content container | Implemented as `components/resizable::Resizable`; independent of SplitPane |
| SplitPane | Redistribute a shared layout region between panels | Navigation/content, preview/source, horizontal or vertical split | Implemented as `components/split_pane::SplitPane` |
| DropZone | Accept a typed drag payload and propose a domain operation | Kanban column, object container, in-app resource drop target | Implemented as `components/drop_zone::DropZone` |
| Sortable | Propose a new order for a keyed collection | List, Tab order, toolbar items | Implemented for bounded source-owned items; virtual adapter remains a 0.1.8 completion item |
| PanZoom | Change the viewing transform while preserving content coordinates | Map, image viewer, node canvas | Implemented for Canvas over shared affine geometry; Chart keeps its domain viewport |
| SelectionArea | Maintain object selection by click, modifiers, range, or marquee | File grid, canvas objects, multi-selection surface | Planned for 0.1.8; Table/text selection remain distinct |
| Rotatable | Change an object's angle around an explicit pivot | Drawing or design tools | Implemented for Canvas with atomic pivot compensation |
| RangeSlider | Select one ordered numeric interval with two thumbs | Filters, time/value windows | Implemented with shared Slider axis/RTL/step math |
| Tree | Navigate a flattened hierarchical outline | Files, settings, object hierarchy | Planned for 0.1.8 over shared outline projection |
| Dockable / DockLayout | Arrange panels through docking, grouping, splitting, or floating | IDE/tool workspaces | Deferred higher-level layout system |

## Choose by the state being changed

- Moving a floating card changes its **position**: Draggable.
- Pulling the card's corner changes its **bounds**: Resizable.
- Pulling the separator between two panels changes their **space allocation**:
  SplitPane.
- Dropping a card into another column changes **ownership or domain state**:
  a drag source plus DropZone.
- Moving the third item before the first changes **key order**: Sortable.
- Moving or magnifying the view changes the **viewport transform**, not the
  objects' stored positions: PanZoom.
- Drawing a rectangle over objects changes the **selection set**: SelectionArea.

Draggable and Resizable can be attached to the same in-window card. SplitPane
does not implement free-floating resizing. Independent operating-system window
movement/resizing belongs to the Rust Host and native Window APIs, not to these
in-window behaviors.

## Draggable — implemented

The caller owns the accepted position. The behavior defines its coordinate
space, movable axes, optional boundary constraints, and a drag handle or eligible
surface. A threshold distinguishes an intended drag from an ordinary click.

The preview must account for the pointer's initial grab offset, parent scrolling,
and supported transforms, so an object does not jump to place its origin under
the pointer. Bounds and optional snapping are evaluated in the declared space.
Pointer-following preview remains native; release proposes a final position and
rejection restores the controlled value.

Dragging a panel by its title region must not steal input from buttons, text
selection, native editors, or scrollbars inside it. Define cancellation and a
keyboard alternative for moving the object. Free positioning alone neither
transfers data to another container nor changes collection order.

| Props / event | Current meaning |
|---|---|
| `key`, `label` | Stable controlled identity and accessible interaction label |
| `position` | Required controlled `{x,y}` in local logical pixels |
| `content` | Required positioned object content |
| `handle` | Optional dedicated handle node; absent means the whole object is eligible |
| `axes` | `both`, `horizontal`, or `vertical` |
| `contain` | Clamp the object to the local boundary; true by default |
| `threshold` | Movement before drag activation; default 4 logical pixels |
| `snap_x`, `snap_y` | Optional positive per-axis snap steps |
| `keyboard_step` | Arrow-key step; Shift multiplies by four |
| `disabled` | Suppress pointer and keyboard manipulation |
| `on_move` / `move({x,y})` | One final controlled-position proposal |

The native primitive preserves the initial grab offset because preview derives
from total pointer delta, not the pointer's absolute origin. It cancels when the
boundary changes during a gesture and restores the controlled position on
Escape, pointer loss, disable, unmount or Host rejection.

Source: [draggable.rhai](../../registry/components/draggable.rhai).
Runnable story: `gpui-rhai gallery --story components/draggable`.

## Resizable — implemented

`components/resizable` exports a controlled single-rectangle composition. The
caller owns the accepted `rect`; native pointer movement previews four optional
signals and release emits one complete proposal. It neither reallocates sibling
space nor changes SplitPane state.

| Props / event | Current meaning |
|---|---|
| `key`, `label` | Required stable identity and accessible handle label prefix |
| `rect` | Required controlled `{x,y,width,height}` in local logical pixels |
| `content` | Required node rendered inside the controlled rectangle |
| `handles` | Unique subset of physical `n/s/e/w/ne/nw/se/sw`; all eight by default |
| `min_width`, `min_height` | Positive lower dimensions; default 24 |
| `max_width`, `max_height` | Positive upper dimensions; default 16,384 |
| `aspect_ratio` | Optional positive width/height ratio |
| `contain` | Constrain to the component boundary; true by default |
| `keyboard_step` | Arrow-key logical-pixel step; default 8, Shift multiplies by four |
| `disabled` | Suppress pointer and keyboard manipulation |
| `on_resize` / `resize(rect+handle)` | One final controlled rectangle proposal |

Dragging west/north changes the origin so the opposite edge remains fixed;
east/south preserves the origin. Corner aspect locking selects the closer
pointer-derived axis, then atomically clamps the ratio against dimension and
boundary constraints. Edge-only aspect locking adjusts the perpendicular
dimension around its center. When constraints exceed available space, the
effective boundary maximum wins deterministically; sizes never become negative.

The constraint policy must define how bounds, min/max, and aspect ratio interact
and how an unsatisfiable request is handled. Reject invalid/non-finite geometry;
never allow negative sizes. Container changes during a drag require a defined
rebase or cancellation policy. Rotation combined with resize requires an
explicit supported transform model rather than mixing window and local axes.

Style parts are `root`, `surface`, `content`, and `handle`. Physical handles do
not reverse in RTL; they remain attached to physical rectangle edges. Pointer
loss cancels preview, and Host rejection restores the controlled rectangle.
Clicking a handle focuses the same native identity used for keyboard resizing.
The visual mark is smaller than its hit region and uses theme border/accent and
focus roles.

Source: [resizable.rhai](../../registry/components/resizable.rhai).
Runnable story: `gpui-rhai gallery --story components/resizable`.
Native acceptance covers pointer preview/no Rhai rerender, west+north opposite
corner preservation, one release proposal, controlled rejection/restore, and
keyboard parity.

## SplitPane — implemented

`components/split_pane` exports a controlled, nestable **two-panel layout
component**. The separator redistributes a shared region; it is not a generic
Resizable wrapper. The current source contract includes:

| Props / event | Current meaning |
|---|---|
| `key`, `label` | Required stable identity and accessible separator name |
| `orientation` | `horizontal` by default, or `vertical` |
| `start`, `end` | Required panel nodes |
| `start_key`, `end_key` | Stable panel keys; default to `start` and `end` |
| `size` | Required controlled start-panel ratio in `0..1` |
| `min_start`, `min_end`, `max_start` | Logical-pixel panel constraints; no current `max_end` prop |
| `start_collapsed`, `end_collapsed` | Controlled collapse flags; both true is rejected |
| `keyboard_step` | Positive logical-pixel step; defaults to 8 |
| `disabled` | Suppress separator interaction |
| `on_resize` / `resize(number)` | Propose the next start-panel ratio |

Style parts are `root`, `start`, `handle`, and `end`. Pointer movement updates a
native preview signal; release emits a ratio proposal. The caller accepts it by
updating `size`; rejection restores the controlled value. Directional keyboard
steps use the proposal path. Horizontal interaction follows logical start/end
in RTL. Separator accessibility orientation is perpendicular to panel layout.

The divider occupies real layout space. Constraints and the remaining space
affect the effective panel size; callers must not treat `size` as an absolute
pixel width. Nested splits compose additional panels. Free-floating bounds,
drag-to-reorder panels, docking, and persistence are not part of this component.

Source: [split_pane.rhai](../../registry/components/split_pane.rhai).
Runnable story: `gpui-rhai gallery --story components/split-pane`.

## DragSource and DropZone — implemented

`DragSource` does not imply free positioning. Pair it with a `DropZone` that
declares accepted payload types and
operations. Separate source identity, payload, current target, eligibility,
preview, and committed result. Hovering a target is not a data mutation.

Define deterministic target selection for nested or overlapping targets, and
visible valid/invalid feedback. A successful drop proposes one domain operation;
rejecting or cancelling it leaves committed state unchanged. A move between
containers must update ownership atomically instead of independently deleting
from the source and inserting into the destination. Copy and move are distinct
operations when offered.

Payloads are bounded typed application data, not arbitrary executable callbacks
or retained GPUI objects. OS file drops and cross-window/platform drag sessions
need explicit Host integration and permissions; an in-app DropZone must not
silently grant script filesystem access. Provide a keyboard-equivalent operation.

`DragSource` requires stable `key`, `label`, `source_id`, `payload_type`, bounded
`payload`, `operation`, and `content`. It supports threshold, disabled state,
optional `keyboard_target`, and `drag_end({accepted,target_id,operation,cancelled})`.
`DropZone` requires stable `key`, `label`, `target_id`, accepted
`payload_types`, accepted `operations`, and content; optional priority resolves
nested/overlapping targets before the smaller-area tie-break. Its
`drop({source_id,target_id,payload_type,payload,operation,x,y})` event is the
single controlled domain proposal.

The coordinator retains no Rhai callback in the pointer-move hot path. It owns
only the bounded typed payload and frame-local target registrations. Escape,
pointer loss, source/target unmount, view suspend and Host teardown clear
transient feedback. Enter/Space on a focused source with `keyboard_target`
executes the same type/operation acceptance and target callback.

Sources: [drag_source.rhai](../../registry/components/drag_source.rhai) and
[drop_zone.rhai](../../registry/components/drop_zone.rhai).
Runnable story: `gpui-rhai gallery --story components/drag-drop`.

## Sortable — bounded implementation complete

The caller owns the ordered stable keys. The interaction chooses an insertion
position and previews the resulting order, then proposes a move/order on commit.
Previewing a new position must not repeatedly mutate the canonical collection.

Specify vertical, horizontal, or grid ordering; disabled/non-movable items;
keyboard moves; edge auto-scroll; and behavior when data changes during a drag.
Virtualized collections must preserve dragged identity even when the source row
leaves the realized range. Move proposals should identify keys and insertion
anchors so stale indices cannot reorder the wrong item.

The existing [Motion ReorderList](../../registry/motion/reorder_list.rhai)
accepts ordered keyed labels, animates their layout changes, and emits no events.
It does **not** currently provide dragging, insertion targets, or reorder events.
Reuse its animation capability where appropriate, without confusing animation
with the interaction and controlled-order model.

The current `components/sortable::Sortable` accepts at most 512 source-owned
items with unique stable keys. Each item has an independent focusable grip so
interactive content is not covered by a drag overlay. Pointer release emits one
`reorder({source_key,anchor_key,placement,x,y})`; self and adjacent no-op moves
emit nothing. Option/Alt + Arrow, Home, and End use the same registered
before/after targets. Target registrations carry their own scroll ancestry, so
edge movement scrolls the destination container rather than the source lane.

Source: [sortable.rhai](../../registry/components/sortable.rhai).
Runnable story: `gpui-rhai gallery --story components/sortable`.

The bounded component is not presented as the promised virtualized mode. That
remaining adapter must pin the active key and a bounded set of neighboring
anchors over `virtual_collection` without materializing all rows before 0.1.8
is considered complete.

## PanZoom — Canvas implementation

The caller owns a viewport transform with defined pan coordinates, scale limits,
and reset/fit semantics. Zoom anchored at a pointer or viewport point preserves
the content point under that anchor. Pan changes the view, not the domain data.

Display DPI, application UI zoom, and content zoom are distinct. The surface must
share transforms between painting, hit testing, selection, and overlays. Define
which wheel/modifier/gesture combinations it consumes and how unused input
reaches scrollable ancestors; normal page scrolling must not accidentally zoom
the content. Support a keyboard alternative and reduced-motion policy.

Chart viewport interactions remain governed by [charts.md](../charts.md),
including coordinate-specific semantics and linked logical windows. A future
generic PanZoom must integrate with those contracts rather than introducing a
second authoritative Chart camera.

`components/pan_zoom::PanZoom` accepts a controlled `{x,y,scale}` transform and
one Canvas node. Drag panning uses the shared Host gesture coordinator. Wheel
zoom keeps the content coordinate below the pointer stationary, coalesces
precise phase-less events, respects explicit touch phases, and emits one final
proposal. Ordinary wheel input bubbles under the default `modifier` policy.
Arrow keys pan; `+`/`-` zoom around the viewport center; `0` resets.

The transform hot lane is one four-signal atomic patch. Canvas paint and
hit-testing use the same `Affine2D`-backed scale/rotation snapshot; the component
does not claim support for arbitrary GPUI subtrees, native editors, or Chart
domain cameras. Source: [pan_zoom.rhai](../../registry/components/pan_zoom.rhai).
Runnable story: `gpui-rhai gallery --story components/pan-zoom`.

## SelectionArea — planned

The caller owns selected object keys, with a distinct active/cursor key and range
anchor where needed. Define single, additive/toggle, range, and marquee selection
as separate operations; platform modifiers and keyboard behavior are explicit.

Marquee selection must state whether an object must be fully enclosed or merely
intersect the rectangle. Account for scrolling, transformed coordinates,
virtualization, disabled objects, and removal/replacement during the gesture.
Bound work by the data model rather than materializing every item for hit tests.

Object selection is distinct from text selection, focus, and dragging an already
selected object. Table, Command, and text viewers keep their existing controlled
selection models. A new selection surface must not consume native editing keys
or pointer gestures merely because it is an ancestor.

## Rotatable — Canvas implementation

The caller owns an angle and explicit pivot. Define units, angle wrapping,
optional snapping, and how rotation composes with movement and resizing.
Painting, clipping, hit testing, and coordinate conversion must agree.

Declare supported surfaces before exposing an API. Canvas/object rotation does
not establish support for arbitrary native input widgets or GPUI subtrees.
Provide keyboard increments, cancellation, and a non-drag way to inspect or set
the angle. This is a later editor-oriented capability, not a prerequisite for
ordinary Gallery layouts.

`components/rotatable::Rotatable` now implements that contract for Canvas. The
caller supplies the local pivot and controlled angle; optional snap is shared
by pointer and keyboard input. The native preview writes angle and pivot
translation as one atomic signal patch, so Canvas painting and hit testing use
the same affine result. A separate optional handle keeps rotation composable
with PanZoom or SelectionArea instead of claiming the whole viewport.

Source: [rotatable.rhai](../../registry/components/rotatable.rhai).
Runnable story: `gpui-rhai gallery --story components/rotatable`.

## Dockable / DockLayout — deferred composition

Docking owns a layout tree: stable panel identities, split groups, Tab groups,
active panels, and eligible drop regions. It composes drag/drop and split layout
but additionally owns panel rearrangement and focus restoration.

Floating inside one window and detaching into another native window are separate
capabilities. Cross-window ownership, close policy, persistence format and
versioning, and recovery from invalid saved layouts require their own design.
Do not grow SplitPane into this system through unrelated optional props.

## Shared interaction contract

These are implementation requirements for planned behaviors and review criteria
for existing mechanisms, not a claim that every mechanism already satisfies
every case.

- Reuse pointer capture, geometry, native preview, retained identity, and Host
  lifecycle infrastructure. Do not create parallel focus, theme, or event models.
- Separate accepted application state from temporary interaction state. A
  completed discrete manipulation emits one final proposal; high-frequency
  previews must not require Rhai callbacks or whole-tree evaluation per move.
  Continuous viewport controls need an explicit native update/commit policy.
- Escape, pointer cancellation, disable, unmount, suspend, owner/window closure,
  and stale generations must release transient resources and reject late work.
  An invalid start or failed candidate must preserve last-good state.
- Coordinate spaces and units are explicit. Content pan, UI scale, DPI, RTL,
  and logical start/end are not interchangeable transformations.
- One gesture has one declared owner. Resolve dragging, resizing, selection,
  scrolling, and native text editing without double activation or axis remapping.
- Accepted geometry/order must respect constraints; no-op interactions must not
  produce spurious change events. Host updates during preview follow a documented
  rebase/cancel policy rather than silently overwriting newer state.
- Reuse the [visual system](../registry-design-system.md#direct-manipulation-surfaces):
  handles and previews follow theme roles, visible focus, and restrained feedback.

## Acceptance and implementation order

Every implemented behavior needs a discoverable Gallery story, runnable source,
and actual pointer/keyboard tests. Check preview, acceptance, rejection, cancel,
no-op, boundaries, nested containers, RTL where relevant, different DPI/viewport
sizes, theme overrides, lifecycle interruption, and data changes mid-gesture.
Assert final geometry/order/selection and resource cleanup, not only a callback
count or a nonempty screenshot.

Resizable and SplitPane have distinct floating-card and shared-layout scenes.
For 0.1.8, first migrate them and existing native controls to the shared
Interaction Runtime, then add Draggable, DragSource/DropZone, Sortable, PanZoom,
SelectionArea and Rotatable. RangeSlider and Tree join the same release through
the shared axis and collection foundations. DockLayout remains later work.

When a planned entry lands, replace its status with the exact public schema,
supported surfaces and limitations, story, and test references. Keep component
exports/counts derived from implemented registry sources; do not ship empty
placeholders or aliases merely to fill this map.
