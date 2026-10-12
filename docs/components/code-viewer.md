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
    height: "fill",     // fill | content
    max_height: (),     // with content: pixels or a Length
    on_location_activate: Fn("open_location"),
})
```

The Viewer supports virtualized rows, horizontal scrolling, viewport/fixed
column wrapping, syntax highlighting, continuous cross-line selection, copying
the original source, and component-local search. `Cmd+F` opens search;
`Cmd+G`/`Shift+Cmd+G` moves between matches. Search supports case, whole-word,
and regular-expression policies.

By default (`height: "fill"`) the Viewer fills the box its parent gives it, so
it needs a definite height; in a scrolling Region it would collapse. With
`height: "content"` it is as tall as its rows: the native view counts its
display rows at layout time from the current width, wrapped rows included, and
multiplies them by the row height, so a code sample in a scrolling Region takes
exactly its height and a narrow window adds rows instead of scrolling inside
the Viewer. `max_height` (logical pixels, or a Length such as
`theme_length("metrics.row") * 10`) caps it; a longer document scrolls inside
as in `fill`. Content mode wraps every line at each new width, so it is meant
for samples and short files; large documents belong in `fill`.

```rhai
code_viewer::CodeViewer(#{ key: "sample", source: sample, language: "toml",
    wrap: "viewport", height: "content", max_height: 400 })
```

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
