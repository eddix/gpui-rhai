# disktree-rhai

Disk space as a squarified treemap — a faithful port of
[tobi/disktree](https://github.com/tobi/disktree) whose entire UI is a
[Rhai](https://rhai.rs) skin running on gpui-rhai. It is one of
gpui-rhai's [reference applications](../README.md): it builds against the
crates and the registry of this checkout, so a change to either is tried on
it in the same pull request.

![](docs/screenshots/side-by-side.png)

The default skin, dark and Flexoki Light (`--theme light`), exploring
gpui-rhai's own source tree.

```sh
cd apps/disktree
cargo run --release -- /some/path
```

## The idea

- The **Rust side** owns the engine: du-faithful scanning (allocated
  blocks, hard links counted once, single filesystem, symlinks not
  followed), window plumbing, and the skin loader. Unit tests,
  including `du` parity.
- **Everything you see is the default skin**: plain Rhai source under
  `ui/`, embedded into the binary — the tiling algorithm included.
  Export it, reshape it, load it back:

```sh
disktree-rhai --export-skin my-skin   # writes the live UI source to
                                       # $XDG_CONFIG_HOME/disktree-rhai/skins/my-skin/
$EDITOR ~/.config/disktree-rhai/skins/my-skin/main.rhai
disktree-rhai --skin my-skin           # run your skin
disktree-rhai --skin-dev my-skin       # hot reload while editing
disktree-rhai --skin ./skins/minimal   # the bundled second skin
```

- `--dev` runs the built-in `ui/` from source with hot reload — the
  dogfood loop the default skin itself is developed in.

A skin directory is `app.toml`, `main.rhai`, `core.rhai`, `tokens.rhai`
(the gpui-rhai 0.2 token base), `theme.rhai` plus optional extra theme
variants under `themes/` (palettes own colors only — typography, spacing
and metrics come from the token base), and the official 0.2 components the
skin mounts under `components/`, `layouts/`, and `patterns/` (with their
declared SVGs under `assets/`). Skins own the tiling (`core.rhai`), the
palette, the panels, and the keymap — everything above the scan
capabilities.

## Keys

`enter` open · `u`/⌫ up · `h j k l`/arrows move · `tab` next sibling ·
`/` filter · `t` size·files·age · `[ ]` depth · `- = 0` zoom · wheel
zoom / shift-wheel pan · `alt`←→ history · `i` hidden · `d` apparent ·
`r` rescan · `p` sidebar · `o` reveal · `?` help · `q` quit · `esc`
close/clear/ascend.

## CLI

```
disktree-rhai [PATH] [-a] [-H] [-d 1-6] [--metric files|bytes]
              [--skin NAME|PATH] [--skin-dev NAME|PATH] [--dev]
              [--export-skin NAME] [--theme dark|light|system]
```

`--theme` resolution: explicit flag → Omarchy theme
(`~/.local/state/omarchy/current/theme/colors.toml`, degradable) →
dark. The family ships two variants — Tokyo Night and Flexoki Light —
and `system` follows the window appearance live, mosaic palette
included.

## Acceptance

- Feature matrix against tobi and gpuix: [docs/FEATURES.md](docs/FEATURES.md)
- Blind-exercised interaction checklist: [docs/CHECKLIST.md](docs/CHECKLIST.md)
- Design consensus: [docs/DESIGN.md](docs/DESIGN.md)
- What building this found in gpui-rhai (three upstream PRs):
  [docs/UPSTREAM_GAPS.md](docs/UPSTREAM_GAPS.md)

## Development

Run these from `apps/disktree`. The app depends on `crates/gpui-rhai` and
`registry` by path, in the `apps/` workspace.

```sh
cargo test                           # unit tests, and every skin prepares
cargo run -- --dev                   # built-in skin from source, hot reload
cargo run --example spike_squarify   # M0 spike: op budget of the layout port
cargo run --example spike_canvas     # M0 spike: retained canvas at scale
cargo run --example du_parity        # engine vs du
cargo run --example rhai_check -- ui/main.rhai
```

On macOS, two harnesses drive the real renderer offscreen:

```sh
cargo run --example key_harness      # every key through a real window
cargo run --example screenshot -- PATH   # PNG of the frame; SHOT_OUT,
                                         # SHOT_THEME, SHOT_KEYS, SHOT_W
```

`ui/components/`, `ui/layouts/` and `ui/patterns/` are installed from the
registry and recorded in `.gpui-rhai/manifest.toml`. When the registry
changes, update them from the repository root:

```sh
cargo run -p gpui-rhai-cli -- --root apps/disktree update
```

The same command merges `ui/tokens.rhai` and the bundled palettes in
`ui/themes/` from `registry/tokens.rhai` and `registry/themes/`; `cargo test`
fails when a component needs a token the copy lacks.

## Status

M0–M5 complete. v1.5 candidates: crumb sibling menus, growth animation,
volumes view, the DaisyDisk-style radial skin the `skins/` directory is
reserved for.
