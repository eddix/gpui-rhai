# Reference applications

Complete programs built on gpui-rhai and kept in this repository. Each one
builds against the crates and the registry of this checkout, so a runtime or
registry change is tried on real applications in the same pull request,
instead of surfacing later as a downstream report that nobody can reproduce.

| Application | What it is |
| --- | --- |
| [`disktree`](disktree/) | Disk usage treemap, a port of [tobi/disktree](https://github.com/tobi/disktree) whose entire UI is a reshapable Rhai skin |

## Where things go

| Kind | Lives in | Holds |
| --- | --- | --- |
| Tutorial | `examples/`, `crates/gpui-rhai/examples/` | one integration pattern, small enough to copy |
| Component acceptance | the Gallery (`registry/stories`) | one official component, theme or motion feature per page |
| Test harness | `tests/` | tests only |
| Reference application | `apps/` | a working program with its own purpose, Host extension, dependencies and tests; not meant to be copied whole |

Rule of thumb: what a reader copies is an example; what checks one component
is a Gallery page; what is useful software in its own right is an app.

## Rules

- **Path dependencies.** An app depends on `crates/gpui-rhai` and `registry`
  by path, in this separate workspace (`apps/Cargo.toml`, its own
  `Cargo.lock`), so its dependencies stay out of the published crates'
  graph. Every app is `publish = false`.
- **Installed like a downstream project.** An app installs registry
  components with the CLI, as any application does, and keeps them current:
  `scripts/check-app-components.sh` fails CI when one differs from the
  registry. After a registry change run
  `cargo run -p gpui-rhai-cli -- --root apps/<name> update`.
- **Checks run under `cargo test`.** Whatever needs no window runs as a test
  (disktree prepares every skin in `tests/skins.rs`), so CI covers it on
  Linux without app-specific steps. Harnesses that need the macOS renderer
  run in the release checklist's macOS gates.
- **Say why an app changed.** When a runtime or registry pull request also
  edits an app, it states whether the app adapts to an intentional change or
  works around a bug. A workaround links its gpui-rhai issue, so a
  regression is never hidden by patching the app until CI passes.
- **Own license.** An app keeps its own `LICENSE`, with the attribution an
  app it ports requires.
- **Small assets.** Screenshots are downscaled and quantized; regenerate
  them with the app's own tooling instead of committing raw captures.

## Commands

```sh
cargo test   --manifest-path apps/Cargo.toml --locked
cargo clippy --manifest-path apps/Cargo.toml --locked --all-targets -- -D warnings
cargo fmt    --manifest-path apps/Cargo.toml --all -- --check
bash scripts/check-app-components.sh
```

## Adding an app

1. Create `apps/<name>/` with `publish = false` and path dependencies, and
   add it to `members` in `apps/Cargo.toml`.
2. From the repository root, set up its skin and components with
   `cargo run -p gpui-rhai-cli -- --root apps/<name> init`, then
   `… add <component>…`.
3. Add a row to the table above.
