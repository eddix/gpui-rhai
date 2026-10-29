# Data-backed variable-height virtual collections

`virtual_collection(config, Fn("render_item"))` is the final public
one-dimensional collection path. Data crosses the retained boundary as
`UiValue`; item `UiNode` snapshots do not exist until their viewport window is
requested.

`config.data` may be a Rhai Array of keyed maps or a Rust-owned
`NativeCollection`. Array values remain appropriate for small script-owned
lists. A native collection keeps the complete source out of `Dynamic` and
projects an owned `UiValue` only when the viewport requests that index.

The retained named renderer may curry validated presentation data, including
`UiNode` and scoped `NativeSignal` values nested in arrays or maps. This
supports components such as Command that keep a required textual label in
`config.data` while retaining a separate rich label node, and Table cells that
bind to one column-owned native width signal. Curried values belong only to the
render recipe: nodes are rebound to the active generation, stale signals remain
generation/incarnation checked, and neither enters sortable/comparable
`UiValue` or `NativeCollection` data.

```rhai
fn render_message(ctx, payload) {
    let message = payload.item;
    box([text(message.body)])
        .with_key(payload.key)
        .accessibility_role("listitem")
}

virtual_collection(#{
    key: "messages",
    label: "Messages",
    data: messages, // each map has a stable string `key`
    estimated_height: 48,
    height: 520,
    overdraw_pixels: 240,
    alignment: "bottom",
    follow_tail: true,
}, Fn("render_message"))
```

Every realized renderer payload also contains `previous_key`, `next_key`,
`first_key`, and `last_key` as stable strings or null. These are presentation
neighbors, not numeric reorder authority. Sortable uses them to emit
identity-based insertion anchors for Array and `NativeCollection` data without
materializing offscreen rows in Rhai.

During an in-Host application drag, the virtual policy pins a matching active
source key plus its bounded overdraw halo. The source may leave the painted
viewport while its retained identity and gesture stay alive; target rows still
realize normally as edge auto-scroll advances the destination viewport.

## Controlled reveal targets

Top-aligned selection and navigation components can pass `reveal_key`, using
the same stable key exposed by `config.data`. The runtime reacts only when that
controlled key changes (or after a fresh collection/content session): it keeps a
measured target visible and top-aligns an unmeasured offscreen target so GPUI
can realize it on the next frame.

```rhai
virtual_collection(#{
    key: "results",
    label: "Search results",
    data: results,
    estimated_height: 32,
    height: 320,
    reveal_key: active_result,
}, Fn("render_result"))
```

Ordinary renders, keyed data edits, and viewport realization do not reapply `reveal_key`, so
wheel/trackpad scrolling remains under the user's control until the controlled
target changes. `follow_tail` and `reveal_key` are mutually exclusive because
they represent competing automatic-scroll policies. An empty or unknown reveal
key is rejected at the script boundary.

## Following growing content

`follow_tail: true` enables GPUI's native `FollowMode::Tail`, not an imperative
scroll on every render. Wheel/trackpad input up or a scrollbar drag away from
the end pauses it. Reaching the bottom resumes it. A growing last row taller
than the viewport follows its bottom, not the top of that row. Realization and
payload-only renders never force a paused reader back to the tail.

An optional named `on_follow_change` callback receives `#{ following, unseen }`
after presented native state changes. `unseen` counts distinct new data keys
received while paused, not rerenders, streaming chunks, or row realization.
Deleted unseen keys are removed from the count; following or an explicit tail
jump clears it. The callback is scoped to the declaring component and script
generation and is not emitted on unchanged frames.

```rhai
fn follow_changed(ctx, status) {
    ctx.set_state("following", status.following);
    ctx.set_state("unseen", status.unseen);
}
fn latest(ctx, value) { ctx.scroll_collection_to_end("messages"); }
fn beginning(ctx, value) { ctx.scroll_collection_to_start("messages"); }
```

Both commands address the caller's component-local collection key and execute
only after its transaction succeeds. The end command resumes tail following
when enabled; the start command pauses it. An unmounted collection is ignored.
Commands run after the native view synchronizes newly committed data, keyed
anchors, reveal and follow policy. Thus a callback that prepends and explicitly
jumps to the start cannot have its command overwritten by anchor restoration.
`ctx.collection_follow_status(key)` reads the current native status or null;
it is an event-time query, not a render dependency. Use the callback to store
reactive status. The owning component can bind End/Home to these commands;
the virtualizer does not steal Table/List selection keys or introduce a tab
stop. The Gallery's **Scrolling** page demonstrates this pattern.

Trusted Rust can resolve `ScriptViewHandle::collection_scroll_handle(id, cx)`
(or `VirtualRequestRegistry::scroll_handle(id)`) for `status`, `scroll_to_end`,
`scroll_to_start`, `scroll_by`, and `pause_following`. Handles keep native GPUI state only;
resolve again after unmount/replacement. An ID in another view cannot resolve.

## Rendering ahead and keeping items

Three optional settings shape how far a collection renders ahead and what it
keeps. The collection's own value wins, then the Host's
`VirtualCollectionPolicy`, then the built-in value:

- `overscan_items` (0 to 1000): items rendered beyond each edge of the
  viewport. It replaces `overdraw_pixels`, as `items × estimated_height` up to
  10000 pixels; unset everywhere, `overdraw_pixels` stays the collection's own.
- `batch_items` (1 to 1000, built-in 16): when a frame needs items that are not
  rendered, the realization also renders `batch_items − 1` beyond the farthest
  needed one, on the side the needed items lie. The following frames find their
  items rendered and ask for nothing, so a scroll renders and reconciles once
  per batch instead of once per item. The first render is not batched. When an
  item rendered ahead fails, the realization renders only the needed items and
  reports nothing; a needed item that fails still fails it.
- `retain_items` (0 to 10000, built-in 200): items that leave the realized
  range are kept, least recently released dropped first, and come back without
  running the renderer. They belong to the collection's current render:
  rendering the collection again (new data, props or state of its component)
  drops them, so a kept item never outlives the data and renderer it was
  rendered from. A kept item keeps its read dependencies and the components
  inside it, like a hidden `keep_mounted` panel; a change it read renders the
  collection again. An item whose component owns an effect, a timer or a signal
  is never kept: it is released as before, and its work stops when it leaves.

A new render of the collection keeps the items around the last presented
viewport, not the batch a scroll rendered ahead, so a selection or data change
does not render the batch again.

The Host sets its defaults in `ScriptViewExtension::configure_engine`; they
apply from each collection's next render, clamped to their bounds:

```rust
use gpui_rhai::{RuntimeEngine, ScriptViewExtension, VirtualCollectionPolicy};

struct Lists;
impl ScriptViewExtension for Lists {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine.set_virtual_collection_policy(VirtualCollectionPolicy {
            overscan_items: None,
            batch_items: Some(32),
            retain_items: Some(500),
        });
        Ok(())
    }
}
```

Table passes its own `overscan_rows`, `batch_rows` and `retain_rows` as these
settings.

## Focus and active-item ownership

`virtual_collection` is a presentation and scrolling mechanism. It does not
create an independent focused row, paint a generic active background, or enter
the tab sequence. The source component owns its semantic active key, disabled
policy, keyboard handlers, hover/click behavior, and themed active style; pass
that same key as `reveal_key` when navigation should keep it visible.

This separation is deliberate. Structural rows such as Command group headings
and Table group headers may share the same data projection as interactive rows,
while `sticky_headers` controls only pinned presentation. It is never used as a
proxy for keyboard eligibility. Command and Combobox therefore keep focus on
their input/trigger/panel and expose exactly one component-owned active item,
for both Array and `NativeCollection` data.

Before GPUI's first measurement, a fixed-height collection compares the target
against its estimated initial viewport. A target already expected to be visible
keeps the natural `(item 0, offset 0)` origin and therefore does not hide group
headers or other predecessors. A genuinely offscreen target still receives the
one initial pre-position. Once measured, real item bounds decide reveal.

## Sticky sections

Top-aligned collections may declare sorted, unique item indices as section
headers:

```rhai
virtual_collection(#{
    key: "grouped-items",
    label: "Grouped items",
    data: items,
    estimated_height: 30,
    height: 420,
    sticky_headers: [0, 12, 28],
}, Fn("render_item"))
```

Indices must be in range and cannot be combined with bottom alignment. A
NativeCollection projection may carry the same immutable index set internally;
omitting `sticky_headers` adopts that metadata, while an explicit empty array
disables it.

The active header is the last declared index at or before GPUI's logical top
item. The runtime keeps it in the realization target even after its natural row
is offscreen. When its natural row would also be drawn, a same-height
placeholder preserves ListState measurement and the one retained header is
rendered only in the sticky layer—there are no duplicate element IDs, handlers,
or geometry records. The next measured header pushes the current one upward.
Header surfaces that block clicks should use `occlude_except_scroll()` rather
than `occlude()`, allowing wheel input to continue to the GPUI list behind the
sticky presentation.

The constructor validates every data key and executes only the estimated first
window. GPUI's variable-height `list/ListState` asks for items while laying out,
but that callback never invokes Rhai: it records the indices participating in
one complete prepaint and returns a height-estimated placeholder for a missing
item. After prepaint, the renderer constructs one atomic target containing the
required indices plus already-measured items inside each required index's
overdraw halo. Treating each halo independently is important because GPUI may
also request a disconnected offscreen item through its own list layout state.

The next foreground runtime turn invokes the retained named renderer only for
missing target indices in its original module/component context, renders formal
item components inside a stable per-item scope, `VirtualCollection[key]/Item[<item
key>]`, prunes items outside the target, reconciles once, runs effects, and
notifies GPUI. A component key inside an item only has to be unique within that
item, and a keyless component is named by its item, not by how many items the
batch rendered before it.
Every commit uses the complete target. A frame that needs no unrendered item
asks for nothing, so items that left the halo are pruned with the next frame
that needs one (see [Rendering ahead and keeping items](#rendering-ahead-and-keeping-items)). Retained
rows preserve state, incarnation, dependencies, effects, timers, signals and
refs; removed rows release those resources together. A failed candidate restores
the last successful node and resource manifest.
The structural collection scope preserves the real caller's callback and read
owner in both initial and delayed realization. Reads from a raw row renderer
invalidate the owning root/formal component; reads inside a formal row component
remain that component's dependencies.
When the same target is already realized, no request is queued and the frame
poll does not notify, so cached overdraw cannot create an evaluation or repaint
loop.

Items that leave the realized window are kept or released (see below); a released
item's subtree is replaced and cleans its component
state/tasks/effects. The GPUI `ListState` is not reset when only the realized map
or item payload changes while ordered stable keys stay identical, preserving
measured heights and the user's scroll anchor. Keyed insertions, deletions and
appends splice only native changed ranges and restore the surviving reader's
key and intra-row offset. Reordering keeps a maximal ordered subsequence of
native rows; moved rows are remeasured. Theme/environment changes
invalidate measurements without resetting the reader or the native follow
state. Replacing every key is treated as new content and resets naturally.
Data and realized item counts have separate Host budgets.

The separate deterministic `VariableListState` policy core uses a Fenwick prefix tree to
compute pixel-overdraw windows, preserve measured heights by key across
reorder/filter, return anchor scroll correction after remeasurement, and
compute bottom/tail offsets. It does not drive the rendered collection or add
a competing position engine. Rendered collections use one GPUI `ListState`.
The initial native scrollbar extent uses `estimated_height`; actual native
measurements replace hints as rows are encountered. GPUI 0.3.7's splice API does
not accept per-item size hints. After structural insertions/moves the adapter
therefore re-seeds the native hint tree: already measured native dimensions
remain exact height hints, unknown/moved rows use the estimate, and only
encountered viewport/overdraw rows are remeasured. Deletion-only edits keep
native measured entries. The first/changed-width native layout clears hints,
so they are seeded once after that pass, not on every frame. A minimum one-pixel
native measurement sentinel (also when configured overdraw is zero) prevents
an unknown, provisionally zero-height suffix from looking like the true tail.
No SDK height cache or competing follow/position engine is involved. Per-item
size hints and transfer of private native measured objects for moved rows are
not exposed by the pinned GPUI API.
Tests cover 10,000 policy items, formal component
state cleanup, off-layout realization, and the 2,000-item Chat acceptance app.

`UiRuntimeState::virtual_requests.inspect()` and the development Inspector expose
one transaction-aware `VirtualCollectionMetrics` record per retained
collection: item/realized/requested counts and ranges, GPUI visible range,
viewport height, logical top item/offset, scrolled state, alignment, and
follow-tail policy, native `is_following_tail` and `unseen_count`, plus the active sticky-header index. Frame reports derive
visible range from measured GPUI item bounds when available and otherwise use
the pre-realized window; scroll changes update it immediately from
`ListScrollEvent`.
Pending requested metrics clear when the foreground realization batch drains,
and failed transactions restore the prior metric snapshot.

The old eager `virtual_list` Rhai constructor, `UiNodeKind`, and unused
fixed-row `VirtualListSpec/State/Metrics` Rust APIs have been deleted. Table,
Combobox, and Select consume only the same public data-backed
`virtual_collection` API available to application Rhai; the retained
`VariableListState` remains the measured-height policy core.
