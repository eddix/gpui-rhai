# Virtual lists and Table realization

`virtual_list(config)` is the general one-dimensional native behavior primitive.
It accepts stable-key item nodes, one equal `row_height`, a bounded viewport,
an accessibility label, and overscan.

```rhai
virtual_list(#{
    key: "files", label: "Files", row_height: 28, height: 420, overscan: 2,
    items: [
        #{ key: "readme", node: text("README.md") },
        #{ key: "cargo", node: text("Cargo.toml") },
    ],
}).on_change(Fn("focused"))
```

GPUI `uniform_list` creates elements only for the requested visible range. A
keyed Entity retains scroll and focus identity through ordinary renders,
reorder, and filtering. Up/Down/Home/End move logical focus and scroll it into
view.

The general primitive still receives prebuilt Rhai item nodes: GPUI element
realization is bounded, but Rhai node construction is eager. Its documentation,
tests, and performance claims must preserve that distinction.

Table reuses the same fixed-height range, overscan, stable-key, scroll-handle,
and focus logic through a data-driven native specialization. Scalar row maps
stay as data and are converted directly only for visible rows. Explicit custom
cell renderers are evaluated for all rows during the complete Rhai view
transaction, after which their GPUI elements remain viewport-bounded. Calling
Rhai lazily from scroll/layout callbacks is forbidden.

Neither primitive supports variable-height realization or arbitrary 2D
spreadsheet virtualization. Table's horizontal column layout and shared header/
body scroll are independent from the vertical range calculation.

The existing `native_virtual_list_smoke` retains the 5,000-node evidence. The
`data_table` example combines controlled paging, sorting, selection, locale
formatting, and a custom cell; native metrics separately enforce the 10,000-row
scalar realization bound.
