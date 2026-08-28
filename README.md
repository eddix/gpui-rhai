# GPUI Rhai

GPUI Rhai is a desktop UI system in which editable Rhai source builds a stable
declarative `UiNode` tree and a thin Rust runtime renders that tree with GPUI.

This is not a traditional opaque component crate. The runtime is a Cargo
dependency, while components, themes, locale bundles, and small assets are
copied into the application repository and owned by the application developer.

The project is under active implementation. The authoritative product contract
is in [INTENT.md](INTENT.md), and the dependency-ordered implementation plan is
in [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md).

## Quick start

```text
cargo run -p gpui-rhai-cli -- --root /path/to/app init
cargo run -p gpui-rhai-cli -- --root /path/to/app add button label input icon divider popover dropdown dialog
cargo run -p gpui-rhai-cli -- --root /path/to/app check
cargo run -p gpui-rhai-cli -- --root /path/to/app metadata
```

`init` adds the runtime dependency and creates the minimal Rust host, Rhai entry,
theme, and manifests without overwriting an existing `main.rs`. `add` copies
editable source plus a committed update baseline.

`update` performs an offline three-way merge between the installed baseline,
the application-owned source, and the bundled registry. Conflicts never
overwrite local source; inspect them under `.gpui-rhai/conflicts/`.

`metadata` compiles the installed Rhai component sources and writes
`.gpui-rhai/editor/components.json` plus basic editor snippets derived from the
actual exported prop, event, slot, and part schemas.

Run the repository example with:

```text
cargo run -p gpui-rhai --features dev-reload --example hello_world
cargo run -p gpui-rhai --example settings_panel
cargo run -p gpui-rhai --example dashboard_layout
cargo run -p gpui-rhai --example form_showcase
cargo run -p gpui-rhai --example component_gallery
cargo run -p gpui-rhai --example extension_host
cargo run -p gpui-rhai --example multi_window
```

See the [documentation index](docs/README.md) or the
[Simplified Chinese quick start](docs/quick-start.zh-CN.md).

## License

GPUI Rhai is licensed under either Apache-2.0 or MIT, at your option.
