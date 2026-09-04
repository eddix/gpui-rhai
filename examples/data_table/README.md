# data_table

An independently runnable embedded-release data table combining Table,
Pagination, Select page-size control, scalar cells, controlled
sorting/selection/grouping/collapse, loading state, and horizontal tracks. Its
intentionally wide columns expose Table's visible horizontal scrollbar;
vertical wheel input remains on the row list while horizontal gestures and
thumb dragging move the shared header/body track. **Group status** exercises
counted group rows and the generic sticky-section layer in the grouped fixture:

```sh
GPUI_RHAI_VISUAL_STATE=grouped cargo run -p gpui-rhai --example data_table
```

```sh
cargo run -p gpui-rhai --example data_table
```

The deterministic source models 500 total rows but generates only the current
controlled page, matching a server-paged caller. Runtime model tests separately
exercise the 10,000-row scalar realization bound.
