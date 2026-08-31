# Data-backed variable-height virtual collections

`virtual_collection(config, Fn("render_item"))` is the final public
one-dimensional collection path. Data crosses the retained boundary as
`UiValue`; item `UiNode` snapshots do not exist until their viewport window is
requested.

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

The constructor validates every data key and executes only the estimated first
window. GPUI's variable-height `list/ListState` requests indices while laying
out, but that callback never invokes Rhai: it queues indices and returns a
height-estimated placeholder for a missing item. The next foreground runtime
turn invokes the retained named renderer in its original module/component
context, renders formal item components inside a stable
`VirtualCollection[key]` state scope, reconciles the realized window, runs
effects, and notifies GPUI. Existing requested nodes are reused when no index is
missing, so ordinary layout cannot create an evaluation loop.

Realized windows replace offscreen item subtrees and clean their component
state/tasks/effects. The GPUI `ListState` is not reset when only the realized map
changes, preserving measured heights and anchor state. Data and realized item
counts have separate Host budgets.

The deterministic `VariableListState` policy core uses a Fenwick prefix tree to
compute pixel-overdraw windows, preserve measured heights by key across
reorder/filter, return anchor scroll correction after remeasurement, and
compute bottom/tail offsets. Tests cover 10,000 policy items, formal component
state cleanup, off-layout realization, and the 2,000-item Chat acceptance app.

`UiRuntimeState::virtual_requests.inspect()` and the development Inspector expose
one transaction-aware `VirtualCollectionMetrics` record per retained
collection: item/realized/requested counts and ranges, GPUI visible range,
viewport height, logical top item/offset, scrolled state, alignment, and
follow-tail policy. Before GPUI emits its first scroll event, visible range uses
the pre-realized window; later reports come directly from `ListScrollEvent`.
Pending requested metrics clear when the foreground realization batch drains,
and failed transactions restore the prior metric snapshot.

The old eager `virtual_list` Rhai constructor and `UiNodeKind` have been deleted.
The remaining fixed-range policy types are internal helpers of the generic
`virtual_collection` element; Table, Dropdown, and Select now consume only the
same public data-backed API available to application Rhai.
