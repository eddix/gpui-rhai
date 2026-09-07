# GPUI Rhai

Start with the [User Guide](USER_GUIDE.md) for the complete application and
agent workflow.

GPUI Rhai is a desktop UI system in which editable Rhai source builds a stable
declarative `UiNode` tree and a thin Rust runtime renders that tree with GPUI.

This is not a traditional opaque component crate. The runtime is a Cargo
dependency, while components, themes, the typed component stylesheet, locale bundles, and small assets are
copied into the application repository and owned by the application developer.

The project is under active implementation. The authoritative product contract
is in [INTENT.md](INTENT.md), and the dependency-ordered implementation plan is
in [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md).

The implemented complex-control line is specified under
[docs/components](docs/components/) for DatePicker, Select, Table, Pagination,
Textarea, CodeViewer, and DiffViewer.

## Quick start

```text
cargo install gpui-rhai-cli --locked
gpui-rhai --root /path/to/app init
gpui-rhai --root /path/to/app add button label input icon divider popover combobox dialog
gpui-rhai --root /path/to/app check
gpui-rhai --root /path/to/app metadata
```

`init` adds the runtime dependency and creates the minimal Rust host, Rhai entry,
theme, and manifests without overwriting an existing `main.rs`. `add` copies
editable source plus a committed update baseline.

The generated Host uses the crates.io `gpui-rhai = "0.1"` runtime dependency.
For repository development, invoke the matching workspace CLI with
`cargo run -p gpui-rhai-cli -- ...`.

`update` performs an offline three-way merge between the installed baseline,
the application-owned source, and the bundled registry. Conflicts never
overwrite local source; inspect them under `.gpui-rhai/conflicts/`.

`metadata` compiles the installed Rhai component sources and writes
`.gpui-rhai/editor/components.json` plus basic editor snippets derived from the
actual exported prop, event, slot, and part schemas. It also writes
`.gpui-rhai/editor/definitions/gpui_rhai.d.rhai` using Rhai 1.26's official
`Engine::definitions()` format for the Rhai Language Server. Hosts that register
extensions can call `RuntimeEngine::definition_source()` after configuration to
emit the same format including their custom APIs.

`theme-studio [path]` opens the first-party semantic theme editor and canonical
all-component specimen. It creates, opens, imports-as-copy, validates, previews,
and saves gpui-rhai `.rhai` themes; see [Theme Studio](docs/theme-studio.md) and
the [bundled theme catalog](docs/bundled-themes.md).

Run the repository example with:

```text
cargo run -p gpui-rhai --features dev-reload --example hello_world
cargo run -p gpui-rhai --example settings_panel
cargo run -p gpui-rhai --example dashboard_layout
cargo run -p gpui-rhai --example form_showcase
cargo run -p gpui-rhai --example data_table
cargo run -p gpui-rhai --example component_gallery
cargo run --release -p gpui-rhai --example code_viewer
cargo run --release -p gpui-rhai --example diff_viewer
cargo run --release -p gpui-rhai --example table_1000
cargo run -p gpui-rhai-cli -- theme-studio
cargo run -p gpui-rhai --example extension_host
cargo run -p gpui-rhai --example host_owned_tree
cargo run -p gpui-rhai --example multi_window
cargo run -p gpui-rhai --example embedded_views
```

Standalone applications explicitly adapt a prepared view into a window-owning
application:

```rust
let view = gpui_rhai::FileScriptView::new("ui/main.rhai").prepare()?;
gpui_rhai::ScriptApplication::new(view).run()?;
```

Existing GPUI applications instead mount one or more isolated views through a
shared `ScriptViewHost`; see the [embedding guide](docs/embedding.md).

Hosts that already own a plain-data UI tree can render it without Rhai and
attach trusted Rust event closures. The `host_owned_tree` example demonstrates
`HostCallback`, callback-typed primitive props, a worker channel, and controlled
`StaticUiView::set_root` updates without a script lifecycle or capability bridge.

See the [documentation index](docs/README.md) or the
[Simplified Chinese quick start](docs/quick-start.zh-CN.md).

## License

GPUI Rhai is licensed under either Apache-2.0 or MIT, at your option.
