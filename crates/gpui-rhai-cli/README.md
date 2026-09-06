# gpui-rhai CLI

The source-management CLI for
[GPUI Rhai](https://github.com/eddix/gpui-rhai). It initializes applications,
copies editable first-party components/themes/locales/assets, validates the
result with the real runtime, performs baseline-aware updates, and generates
production embedded sources.

```text
cargo install gpui-rhai-cli --locked

gpui-rhai init
gpui-rhai add button input form_field
gpui-rhai check
gpui-rhai dev
```

Important commands:

- `init` creates the Rust Host and application-owned `ui/` source tree;
- `add`, `diff`, and `update` install and three-way merge official source;
- `check` compiles and executes a validated headless first frame;
- `metadata` emits schemas, editor snippets, and Rhai definitions;
- `embed` generates production `include_str!`/`include_bytes!` wiring;
- `theme-studio [path]` opens the live semantic theme editor and complete
  component specimen.

The CLI consumes the version-matched `gpui-rhai-registry` package internally.
Applications depend on `gpui-rhai`; they do not need to add the registry crate
themselves.

See the complete
[User Guide](https://github.com/eddix/gpui-rhai/blob/main/USER_GUIDE.md) and
[quick start](https://github.com/eddix/gpui-rhai/blob/main/docs/quick-start.zh-CN.md).
