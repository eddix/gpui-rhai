# gpui-rhai

The Rust runtime for [GPUI Rhai](https://github.com/eddix/gpui-rhai), a desktop
UI system where editable Rhai source produces a validated declarative `UiNode`
tree and GPUI performs native layout, rendering, input, and retained work.

```toml
[dependencies]
gpui-rhai = "0.1"
```

For a complete application scaffold and editable first-party components:

```text
cargo install gpui-rhai-cli --locked
gpui-rhai init
gpui-rhai add button input form_field
gpui-rhai check
```

## What the runtime owns

- restricted Rhai compilation, lifecycle, typed state, stores, effects, and
  transactional reconciliation;
- a stable `UiNode` boundary instead of exposing GPUI `Window`, `Context`,
  `Div`, or `AnyElement` to scripts;
- native input/textarea, overlays, virtual collections, canvas, animation,
  CodeViewer, DiffViewer, accessibility, themes, locales, and Host extensions;
- `FileScriptView` and `EmbeddedScriptView` preparation for standalone or
  embedded GPUI windows;
- explicit resource budgets and no dependency on `gpui-component`.

Components, themes, locales, and small assets are source-owned by the
application after `gpui-rhai-cli` copies them from the version-matched
`gpui-rhai-registry` package.

Start with the [User Guide](https://github.com/eddix/gpui-rhai/blob/main/USER_GUIDE.md).
Further references cover
[embedding](https://github.com/eddix/gpui-rhai/blob/main/docs/embedding.md),
[component authoring](https://github.com/eddix/gpui-rhai/blob/main/docs/component-authoring-guide.md),
[security](https://github.com/eddix/gpui-rhai/blob/main/docs/security-boundary.md), and
[the component catalog](https://github.com/eddix/gpui-rhai/blob/main/docs/components/catalog.md).
