# Variable-height virtual collections

`virtual_list(config)` is the current public one-dimensional retained mechanism.
Its source contract is variable-height and destructive relative to the old
fixed-row API:

```rhai
virtual_list(#{
    key: "files",
    label: "Files",
    estimated_height: 28,
    height: 420,
    overdraw_pixels: 112,
    alignment: "top",       // or "bottom" for chat/log layout
    follow_tail: false,
    items: [
        #{ key: "readme", node: text("README.md") },
        #{ key: "cargo", node: text([span("Cargo").bold(), span(".toml")]) },
    ],
}).on_change(Fn("focused"))
```

The GPUI production element uses `list/ListState`, not `uniform_list`. GPUI
measures realized rows, keeps a keyed Entity and variable-height scroll state,
and renders only the requested range. Up/Down/Home/End retain logical focus and
scroll the keyed item into view. Bottom alignment and follow-tail are explicit.

The standalone `VariableListState` is the deterministic policy core used for
tests and future Inspector/automation metrics. It uses a Fenwick prefix tree to
compute pixel-overdraw windows, preserves measured heights by key through
reorder/filter, returns anchor scroll correction after remeasurement, and
computes bottom/tail offsets. The 10,000-item tests prove bounded realization
and anchor preservation.

One gap remains: `VirtualListNodeSpec` still receives prebuilt item `UiNode`
snapshots. GPUI realization is lazy, but Rhai item construction is eager. The
final data-backed formal item-component API must move item execution before
layout/prepaint and outside the full root render; no implementation may call
Rhai from a GPUI list callback.

The old native Table remains a separate fixed-height privileged specialization
and is scheduled for deletion after its Rhai/headless replacement uses the
public variable collection. No final architecture claim should infer 2D
spreadsheet virtualization from this one-dimensional mechanism.
