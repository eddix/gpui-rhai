# CodeViewer

Run the native read-only source viewer:

```sh
cargo run --release -p gpui-rhai --example code_viewer
```

The example uses Rhai syntax highlighting, line virtualization, text selection,
horizontal scrolling, wrapping support, and the focus-scoped `Cmd+F` search UI.
