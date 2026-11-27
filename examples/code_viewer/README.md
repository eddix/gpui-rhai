# CodeViewer

Run the native read-only source viewer:

```sh
cargo run --release -p gpui-rhai --example code_viewer
```

The example uses Rhai syntax highlighting, line virtualization, text selection,
horizontal scrolling, wrapping support, and the focus-scoped `Cmd+F` search UI.

The application header and spacing use the selected design's typography and
tokens. All four Modern/Clean presets and bundled fonts are registered. Native
integration tests mount the executable's actual `view()` factory and retain the
same document owner through design changes.
