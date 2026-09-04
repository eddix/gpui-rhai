# Theme Studio

Theme Studio is the first-party gpui-rhai theme editor and canonical component
specimen. It intentionally understands only gpui-rhai `.rhai` themes.

```text
gpui-rhai theme-studio
gpui-rhai theme-studio ui/theme.rhai
```

When running from this workspace:

```text
cargo run -p gpui-rhai-cli -- theme-studio
```

## Workflow

- `New` starts an unsaved theme using Default Dark as a sound baseline.
- `Open` loads the path field and saves back to that file.
- `Import` reads the path into an unsaved copy and never changes the source.
- `Save` writes the current document to the explicit `.rhai` path.
- `Derive` recalculates secondary surfaces, muted/disabled text, borders,
  accent hover, focus, and filled-state foregrounds from the current anchors.
- The bundled-theme selector imports any built-in variant as a new copy.

All edits update the active preview ThemeVariant without recompiling Rhai or
discarding component state. Invalid colors remain visible in the editor with a
diagnostic; the last valid preview stays active. Contrast failures are warnings,
not hidden color rewrites.

Theme Studio saves a simple canonical `theme() -> map`. Opening a complex valid
theme is supported, but saving intentionally normalizes its implementation.
Leading `//` attribution comments are preserved.

## Component specimen

The right pane renders every official component from the same source shipped to
applications. It includes action variants and sizes, form controls, choices,
status and loading states, navigation, grouped/sticky virtualized Table,
Pagination, assets, and interactive Menu, Tooltip, Popover, Dialog, DatePicker,
Dropdown, Select, and Toast surfaces.

Product examples remain responsible for realistic application composition;
Theme Studio is the single exhaustive component/theme contract.
