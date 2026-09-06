# CodeViewer

`CodeViewer` is a native, read-only source document surface. It is not a code
editor: there is no editable caret, undo history, completion, diagnostics, LSP,
minimap, symbol navigation, or syntax-tree folding.

```rhai
import "components/code_viewer" as code_viewer;

code_viewer::CodeViewer(#{
    key: "script",
    source: rhai_source, // string or NativeTextDocument
    label: "Rhai script",
    file_name: "main.rhai",
    language: "rhai",
    show_line_numbers: true,
    wrap: "none",       // none | viewport | column
    wrap_column: 100,
    tab_size: 4,
    on_location_activate: Fn("open_location"),
})
```

The Viewer supports virtualized rows, horizontal scrolling, viewport/fixed
column wrapping, syntax highlighting, continuous cross-line selection, copying
the original source, and component-local search. `Cmd+F` opens search;
`Cmd+G`/`Shift+Cmd+G` moves between matches. Search supports case, whole-word,
and regular-expression policies.

Double-click emits `location_activate(#{ line, column })` with one-based source
coordinates. Hosts can dispatch `RevealDocumentLine { side: None, line }` while
focus remains inside this Viewer. Ordinary selection and scrolling never invoke
Rhai.

Parts are `root`, `gutter`, `line`, `text`, `loading`, `error`, and `search`.
Line rendering remains native; there is intentionally no per-line Rhai callback.
`text` and `gutter` start with the resolved platform monospace family, and
`part_styles` can override either family.

Run the standalone example:

```text
cargo run --release -p gpui-rhai --example code_viewer
```
