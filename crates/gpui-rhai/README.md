# gpui-rhai

The Rust runtime for [GPUI Rhai](https://github.com/eddix/gpui-rhai), a desktop
UI system where editable Rhai source produces a validated declarative `UiNode`
tree and GPUI performs native layout, rendering, input, and retained work.

```toml
[dependencies]
gpui-rhai = "0.2"
```

For a complete application scaffold and editable first-party components:

```text
cargo install gpui-rhai-cli --locked
cargo new my-app && cd my-app
gpui-rhai init --profile productivity
gpui-rhai add button input form_field
gpui-rhai check
```

## What the runtime owns

- restricted Rhai compilation, lifecycle, typed state, stores, effects, and
  transactional reconciliation;
- a stable `UiNode` boundary instead of exposing GPUI `Window`, `Context`,
  `Div`, or `AnyElement` to scripts;
- native input/textarea, overlays, virtual collections, Canvas, motion,
  CodeViewer, DiffViewer, accessibility, themes and the token environment
  (density, size, corners), locales, the composition audit, and Host
  extensions;
- optional native composable charts with typed/streaming data, Geo2D,
  interaction, motion, Rust extensions, and SVG/PNG export;
- `FileScriptView` and `EmbeddedScriptView` preparation for standalone or
  embedded GPUI windows;
- explicit resource budgets and no dependency on `gpui-component`.

Components, layouts, themes, the token base, the typed component stylesheet,
locales, and small assets are source-owned by the application after
`gpui-rhai-cli` copies them from the version-matched `gpui-rhai-registry`
package. The runtime itself has no visual opinions; the official design system
is specified in [docs/design](https://github.com/eddix/gpui-rhai/tree/main/docs/design).

Start with the [User Guide](https://github.com/eddix/gpui-rhai/blob/main/USER_GUIDE.md).
Further references cover
[embedding](https://github.com/eddix/gpui-rhai/blob/main/docs/embedding.md),
[component authoring](https://github.com/eddix/gpui-rhai/blob/main/docs/component-authoring-guide.md),
[security](https://github.com/eddix/gpui-rhai/blob/main/docs/security-boundary.md), and
[the component catalog](https://github.com/eddix/gpui-rhai/blob/main/docs/components/catalog.md).
