# data_table

An independently runnable embedded-release data table combining Table,
Pagination, Select page-size control, locale-aware number/date cells, controlled
sorting and selection, loading state, horizontal tracks, a custom Tag cell, and
a keyboard-focusable custom action cell. Its intentionally wide columns expose
Table's visible horizontal scrollbar; vertical wheel input remains on the row
list while horizontal gestures and thumb dragging move the shared header/body
track.

```sh
cargo run -p gpui-rhai --example data_table
```

The deterministic source models 500 total rows but generates only the current
controlled page, matching a server-paged caller. Runtime model tests separately
exercise the 10,000-row scalar realization bound.
