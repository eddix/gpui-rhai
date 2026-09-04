# gpui-rhai CLI

The source-management CLI for GPUI Rhai. It initializes projects, copies
application-owned components/themes/locales/assets, validates compatibility,
performs baseline-aware updates, and generates production embedded sources.

The CLI has no published release yet. Build or install it from the repository
checkout because its bundled registry is maintained at the workspace root:

```text
cargo install --path crates/gpui-rhai-cli
```

Before the runtime's first release, `init` writes the intended future
`version = "0.1"` dependency shape. Point the generated dependency at a local
runtime checkout or an exact accessible Git commit while dogfooding; see the
[repository User Guide](../../USER_GUIDE.md).

`gpui-rhai theme-studio [path]` launches the bundled Theme Studio. It edits only
gpui-rhai `.rhai` themes and renders the canonical official-component specimen
under the live draft.
