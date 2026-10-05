# Gallery and acceptance application

`gpui-rhai gallery` opens the Gallery: the component catalog and the acceptance
application of the design system in [design/](design/). It is an ordinary Rhai
application built from `patterns/app_shell` and the L2 layouts, so it is also
the reference for how a productivity tool is composed. If the Gallery cannot be
built cleanly from the public patterns, the patterns are wrong.

```sh
gpui-rhai gallery
gpui-rhai gallery --page scene.operations --density compact
gpui-rhai gallery --page table --theme default-light --locale zh-CN
gpui-rhai gallery --list
gpui-rhai gallery --story apps/operations --case large
```

From the repository, substitute `cargo run --release -p gpui-rhai-cli --` for
`gpui-rhai`. Unknown pages, densities, themes and locales return an error before
a window opens. The Gallery uses only bundled sources and fixtures: it does not
read or write the launch directory, use the network or run commands.

| Flag | Values | Default |
|---|---|---|
| `--page` | a page ID from `--list` | `button` |
| `--density` | `comfortable`, `compact` | `comfortable` |
| `--theme` | a bundled theme slug: `default-dark`, `default-light`, `catppuccin-latte`, `tokyo-storm`, ... (the theme file name with dashes) | `default-dark` |
| `--locale` | `en`, `zh-CN`, `ar` | `en` |
| `--motion` | `normal`, `reduced`, `none` | `normal` |
| `--story`, `--case` | a development story, see below | — |

The same launch can come from the environment (`GPUI_RHAI_GALLERY=1` with
`GPUI_RHAI_GALLERY_PAGE`, `_DENSITY`, `_THEME`, `_LOCALE`, `_MOTION`, `_STORY`,
`_CASE`), which macOS app bundles use.

## Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│ ●●● GPUI RHAI  Button  [Comfortable|Compact] [Square|Subtle|Round] [Default · Dark▾] [English▾] Commands ⌘K │
├──────────────┬──────────────────────────────────────────────────────┤
│ ⌄ Foundations│ Button                                        Source │
│ ⌄ Markers    │ One solid button per group.                          │
│ ▌ Button     │ VARIANTS                                             │
│   IconButton │ [Deploy] [Export] [Duplicate] Cancel                 │
│   …          │ …                                                    │
├──────────────┴──────────────────────────────────────────────────────┤
│ DENSITY comfortable  THEME Default Dark  CORNERS square  LOCALE en   AUDIT 0  ⌘K │
└─────────────────────────────────────────────────────────────────────┘
```

- **Title bar**: density and corner style (ToggleGroups), the theme (a Select
  of every loaded theme, from `ctx.theme_variants()`), locale (Select) and the
  command palette. On macOS it replaces the platform title bar: the window
  buttons sit at its start and dragging its background moves the window
  (`window_drag`, allowed by the Host). Corner styles are described in
  [design/atoms.md](design/atoms.md#radius-roles-and-the-corner-style); square
  is the design language and the baselines' style.
- **Sidebar**: a Tree of 83 pages in 11 groups: Foundations 4, Markers 8,
  Fields 14, Lists 9, Overlays 6, Containers 11, Display 7, Interaction 7,
  Layouts and patterns 12, Scenes 4, Effects 1.
- **Main**: one Region per page. Each section shows a component in real use,
  not a property sheet; size and density comparisons sit side by side.
- **Source**: the inspector shows the Rhai source of the current page module
  (`Source` or the palette).
- **Status bar**: the launch environment and the live audit count. A page that
  breaks a composition rule shows `AUDIT n` with n > 0.

Keyboard: F6 / Shift+F6 move between sidebar, main and inspector; Tab moves
inside a region; Cmd+K (Ctrl+K) opens the palette, which lists every page and
the Gallery actions (toggle density, light/dark within the theme family, corner
style and source).

The four scenes are small complete tasks rather than specimens:

| Page | Task |
|---|---|
| `scene.operations` | filter hosts, select one by keyboard, act on its detail |
| `scene.data` | a 120-row paginated table with selection and density |
| `scene.form` | fill a form, see validation, submit with Enter |
| `scene.settings` | switch views with Tabs, toggle settings, a disabled managed section |

## Structure

| Path | Content |
|---|---|
| `registry/gallery/main.rhai` | the AppShell, navigation, palette, title controls, page dispatch |
| `registry/gallery/kit.rhai` | page helpers: `page`, `sec`, `line`, `sizes`, `variants`, `densities` |
| `registry/gallery/pages/*.rhai` | one module per sidebar group; each exports `pages()` and `view(ctx, h, v, id)` |
| `crates/gpui-rhai-cli/src/acceptance.rs` | the Host: assembly, launch selection, key bindings, source documents, audit count |

Pages keep their interactive values in one `values` map owned by `main`; page
modules receive handler function pointers (`kit::store`, `kit::put`,
`kit::checked`) instead of owning state. The Host adds only what Rhai cannot do:
the Cmd+K binding, the page sources as native text documents, and the audit
count after each frame.

Adding a component means adding its page. `acceptance::page_ids` reads the
`pages()` lists, and a unit test fails when a bundled component has no page.

## Gates

| Gate | Command |
|---|---|
| every page has zero audit findings in both densities | `cargo test --manifest-path tests/native-keyboard/Cargo.toml --test gallery_acceptance every_gallery_page` |
| the scenes and the palette work by keyboard alone | `cargo test --manifest-path tests/native-keyboard/Cargo.toml --test gallery_acceptance` |
| visual baselines | `scripts/capture-macos-gallery-baselines.sh`, checked by `scripts/audit-visual-baselines.sh` |

The audit gate runs every rule of the productivity profile except
`unresolved-font` (the test platform has no system fonts). Baselines cover four
pages (Button and the operations, form and settings scenes) in comfortable and
compact, light and dark, plus one RTL case (`scene.operations`, Arabic) and two
CJK cases (`table`, `description_list` in Simplified Chinese). They are rendered
offscreen by the real macOS renderer and read back from the GPU texture, so
window managers and other windows never affect them; see
[visual-testing.md](visual-testing.md).

For a review sweep of every page, render them all without writing baselines:

```sh
cd tests/native-keyboard
cargo run --release --bin gallery_baselines -- /tmp/pages --pages compact default-light
```

## Development stories

The stories in `registry/stories/` remain as development fixtures: deterministic
cases for runtime features (charts, motion, the Operations Workbench, Host
slots). `--story` opens one in a standalone window without Gallery chrome:

```sh
gpui-rhai gallery --story charts/catalog
gpui-rhai gallery --story apps/host-embedding
gpui-rhai gallery --story apps/operations --case config-diff
```

`apps/host-embedding` mounts a second resident Rhai view under its own
`ScriptViewHost` and supplies it through `HostSlotRegistry::with_script_view`.
`apps/operations` is a connected application with Rust-owned collections, chart
data, documents and a typed deployment transaction; its cases (`basic`,
`config-diff`, `command-dialog`, `theme-overrides`, `loading`, `empty`,
`failure`, `failure-terminal`, `streaming`, `large`) are listed by `--list`.

`scripts/release-smoke.sh` launches the Gallery (`scene.operations`, compact)
and one story from a fresh temporary directory and checks that neither writes
files there.

## Non-goals

No arbitrary UI zoom, third-party pages, browser build, code editor, terminal,
real command execution, network services or persistence.
