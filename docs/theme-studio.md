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

Theme Studio owns semantic theme tokens, including spacing, radii, and
typography. Application-wide Button/Input/etc. overrides are a separate typed
contract in `ui/styles.rhai`; see [component stylesheets](component-styles.md).
Keeping that file separate lets one component skin consume any light or dark
theme variant through symbolic tokens.

## Component specimen

The right pane renders all 51 official components from the same source shipped
to applications. It begins with all eight live typography roles, then includes
action variants and sizes, form controls, choices,
status and loading states, navigation, grouped/sticky virtualized Table,
Pagination, themed ScrollArea, Slider, Command, ContextMenu, Sheet, assets, and
the complete overlay family, CodeViewer, and unified/split DiffViewer. Loaded
themes materialize their own syntax/search/diff palette from semantic anchors;
explicit namespaced overrides remain editable data in the canonical theme.

Product examples remain responsible for realistic application composition;
Theme Studio is the focused theme editor. The formal Gallery reuses its
curated specimen source and adds category navigation, exact source display,
viewport/Motion controls, and live switching across bundled themes:

```text
cargo run --release -p gpui-rhai-cli -- gallery --story components/catalog
```

The editor and specimen panes scroll independently. Gallery launches accept
`--theme`, `--locale`, `--story`, and `--case`; viewport and Motion policy are
changed without recompiling or resetting story state.
