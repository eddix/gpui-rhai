# DiffViewer

Compare two neutral configuration sources:

```sh
cargo run --release -p gpui-rhai --example diff_viewer
```

The example switches between unified and split layouts. Use `Option-Up` and
`Option-Down` to navigate changes and `Cmd+F` to search both sides.

The layout picker is the official controlled `ToggleGroup`, with pointer and
arrow/Space operation, not a separately painted text button. Native integration
tests mount the same `view()` factory, switch all four Modern/Clean presets and
keep the native document owner while changing the comparison layout.
