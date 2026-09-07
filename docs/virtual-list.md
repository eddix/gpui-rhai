# Data-backed variable-height virtual collections

`virtual_collection(config, Fn("render_item"))` is the final public
one-dimensional collection path. Data crosses the retained boundary as
`UiValue`; item `UiNode` snapshots do not exist until their viewport window is
requested.

`config.data` may be a Rhai Array of keyed maps or a Rust-owned
`NativeCollection`. Array values remain appropriate for small script-owned
lists. A native collection keeps the complete source out of `Dynamic` and
projects an owned `UiValue` only when the viewport requests that index.

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

## Controlled reveal targets

Top-aligned selection and navigation components can pass `reveal_key`, using
the same stable key exposed by `config.data`. The runtime reacts only when that
controlled key changes (or when the ordered key set itself changes): it keeps a
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

Ordinary renders and viewport realization do not reapply `reveal_key`, so
wheel/trackpad scrolling remains under the user's control until the controlled
target changes. `follow_tail` and `reveal_key` are mutually exclusive because
they represent competing automatic-scroll policies. An empty or unknown reveal
key is rejected at the script boundary.

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
also request a disconnected offscreen focused item.

The next foreground runtime turn invokes the retained named renderer only for
missing target indices in its original module/component context, renders formal
item components inside a stable `VirtualCollection[key]` state scope, prunes
items outside the target, reconciles once, runs effects, and notifies GPUI.
When the same target is already realized, no request is queued and the frame
poll does not notify, so cached overdraw cannot create an evaluation or repaint
loop.

Realized windows replace offscreen item subtrees and clean their component
state/tasks/effects. The GPUI `ListState` is not reset when only the realized map
or item payload changes while ordered stable keys stay identical, preserving
measured heights and the user's scroll anchor. A key-order or estimated-height
change resets measurements; the controlled reveal policy is then applied once.
Data and realized item counts have separate Host budgets.

The deterministic `VariableListState` policy core uses a Fenwick prefix tree to
compute pixel-overdraw windows, preserve measured heights by key across
reorder/filter, return anchor scroll correction after remeasurement, and
compute bottom/tail offsets. Tests cover 10,000 policy items, formal component
state cleanup, off-layout realization, and the 2,000-item Chat acceptance app.

`UiRuntimeState::virtual_requests.inspect()` and the development Inspector expose
one transaction-aware `VirtualCollectionMetrics` record per retained
collection: item/realized/requested counts and ranges, GPUI visible range,
viewport height, logical top item/offset, scrolled state, alignment, and
follow-tail policy, plus the active sticky-header index. Frame reports derive
visible range from measured GPUI item bounds when available and otherwise use
the pre-realized window; scroll changes update it immediately from
`ListScrollEvent`.
Pending requested metrics clear when the foreground realization batch drains,
and failed transactions restore the prior metric snapshot.

The old eager `virtual_list` Rhai constructor and `UiNodeKind` have been deleted.
The remaining fixed-range policy types are internal helpers of the generic
`virtual_collection` element; Table, Combobox, and Select now consume only the
same public data-backed API available to application Rhai.
