# gpui-rhai CLI

The source-management CLI for
[GPUI Rhai](https://github.com/eddix/gpui-rhai). It initializes applications,
copies editable first-party components/themes/locales/assets, validates the
result with the real runtime, performs baseline-aware updates, takes offscreen
screenshots, and bundles macOS applications.

```text
cargo install gpui-rhai-cli --locked

cargo new my-app && cd my-app
gpui-rhai init --profile productivity
gpui-rhai add button input form_field
gpui-rhai add chart
gpui-rhai check
gpui-rhai dev
```

Important commands:

- `init` creates the Rust Host (mounting `gpui_rhai::script_view!()`), a
  `build.rs` that embeds `ui/` for release builds, the `examples/shot.rs`
  that `shot` runs, and the application-owned `ui/` source tree: the
  palette, the token base `tokens.rhai`, `styles.rhai` component overrides and,
  with `--profile`, the design rules `check` reports;
- `add`, `diff`, and `update` install and three-way merge official source;
- `check` compiles and executes a validated headless first frame;
- `dev` runs the application; `dev --bundle` runs it inside a macOS
  application bundle, for system notifications and the Dock badge;
- `shot` renders the view offscreen on macOS and saves PNGs, after steps
  such as `--theme`, `--click`, `--key` and `--wait`;
- `bundle` builds a signed macOS `.app` from
  `[package.metadata.gpui-rhai.bundle]`;
- `metadata` emits schemas, editor snippets, and Rhai definitions;
- `skills <directory>` writes the agent skills for this version, for example
  into `.claude/skills` or `.agents/skills`;
- `theme-studio [path]` opens the live semantic theme editor and complete
  component specimen;
- `gallery` opens the Gallery: every official component in both densities,
  with its source.

The CLI consumes the version-matched `gpui-rhai-registry` package internally.
Applications depend on `gpui-rhai`; they do not need to add the registry crate
themselves.

See the complete
[User Guide](https://github.com/eddix/gpui-rhai/blob/main/USER_GUIDE.md) and
[quick start](https://github.com/eddix/gpui-rhai/blob/main/docs/quick-start.zh-CN.md).
