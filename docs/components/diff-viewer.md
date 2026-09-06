# DiffViewer

`DiffViewer` compares two neutral document descriptors. Left and right do not
mean old/new, correct/incorrect, or local/remote.

```rhai
import "components/diff_viewer" as diff_viewer;

diff_viewer::DiffViewer(#{
    key: "servers",
    left: #{ source: server_a, label: "Server A", file_name: "app.toml" },
    right: #{ source: server_b, label: "Server B", file_name: "app.toml" },
    mode: "split",            // split | unified
    whitespace: "exact",      // exact | ignore_changes | ignore_all
    context_lines: 3,          // 0..100 | "all"
    show_line_numbers: true,
    wrap: "none",
    wrap_column: 100,
    tab_size: 4,
    on_location_activate: Fn("open_location"),
})
```

Rust computes stable line-level hunks with Unicode-grapheme intraline
refinement. Split mode uses one aligned row model and one vertical scroll
position; each pane retains its own document coordinates and selection. Unified
mode shows directional left/right rows without changing the public neutral
semantics. CRLF and LF delimit equivalent lines; missing terminal newlines
remain visible differences.

Large equal regions collapse to the requested context. Clicking a fold expands
it; search and `RevealDocumentLine` expand a containing fold before navigation.
`Option-Up`/`Option-Down` selects the previous/next hunk. `Shift+Cmd+E` expands
all, `Shift+Cmd+C` collapses all, and `Cmd+F` searches both complete documents,
including currently folded text.

Selection stays within one side and ordinary copy returns original source text.
`Option+Cmd+C` explicitly interprets left as the source and right as the target
for that command only, then copies a standard unified patch using the supplied
labels. Double-click emits one-based
`location_activate(#{ side: "left" | "right", line, column })`.

Parts are `root`, `header`, `gutter`, `line`, `text`, `fold`, `loading`,
`error`, `search`, and `status`. Diff rows cannot be rendered through Rhai.

Run the standalone example:

```text
cargo run --release -p gpui-rhai --example diff_viewer
```
