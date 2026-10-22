# Interaction checklist

Derived from tobi/disktree's 36-test harness (crates/disktree-app/src/tests.rs)
plus our skin-system surface. Automated entries were exercised blind via
`wtype`/`hyprctl`/`grim` on Hyprland (scale-2, wayland) against
`/home/eddix` and `/home/eddix/Codes` scans; each marked ✅ when the
screenshot/state evidence confirmed the tobi-expected behaviour.

## Scan lifecycle

- ✅ launch scans PATH (default `$HOME`), progress panel with files/dirs/
  measured/unreadable stats
- ✅ esc while scanning cancels; "Scan again" restarts
- ✅ `r` rescans; `i` toggles hidden (rescan); `d` toggles apparent (rescan)
- ✅ du parity: allocated bytes match `du -s` (M1 test, bit-exact on home)

## Treemap

- ✅ squarified tiles fill viewport; labels on tiles ≥54×15px, ≤150, by area
- ✅ header bands on subdivided directories; closed tiles below min area
- ✅ Others tile merges children beyond 96 (`+N more`)
- ✅ `[`/`]` depth 1–6 stepper + keys; tiles appear/disappear per level
- ✅ `t` cycles Size → Files → Age (areas re-rank; Age recolours by mtime
  buckets, verified by pixel histogram: bright-blue ≤7 d bucket dominates)

## Navigation

- ✅ enter opens selected directory; enter on a file opens its parent;
  enter with nothing selected opens the largest entry
- ✅ `u`/⌫ ascends; crumb bar reflects path; selection follows
- ✅ hjkl/arrows move by tobi's same-depth scoring (gap + 2.5×offset),
  off-the-edge left/up selects the parent-of-root
- ✅ tab cycles siblings next-largest by metric
- ✅ breadcrumb segment click jumps (`go_to`), root segments render
- ✅ alt←→ history back/forward (host-bound actions); `<`/`>` buttons
  enabled exactly when stacks are non-empty; side mouse buttons navigate
- ✅ wheel zoom anchors at pointer; `-`/`=`/`0` zoom centred; `-` at floor
  with descend=false is a no-op (pixel-identical frame, like tobi)
- ✅ shift-wheel pans vertically, clamped ≥0
- ✅ click selects; re-click on selected dir enters; double-click enters
- ✅ esc cascade: applied filter → selection → ascend → (at root) nothing

## Find / filter

- ✅ `/` (and `s`) opens the three-key field; typing updates live match
  count via the tree capability (`cargo` → 42 matches · 2.9 MiB)
- ✅ backspace edits; enter applies — mosaic keeps only matches at true
  relative sizes, largest selected; esc clears back to the full view
- ✅ typing with 0 matches does not apply (needle mismatch case verified)

## Panels & help

- ✅ hover tooltip follows cursor with edge flip; name + size + kind + age
- ✅ sidebar Selection: name/path/share/Files/Last write/Kind + Open/Reveal
- ✅ sidebar Worth a look: real findings (node modules, stale python
  packages, build artifacts), click selects
- ✅ sidebar Disk: free/used bar from statvfs
- ✅ `p` toggles the sidebar (pixel-diff verified)
- ✅ `?` overlay with full key list; esc/`?`/q closes; keys ignored while
  open (except those three)
- ✅ `o` reveals selection/hover/current dir via xdg-open parent
- ✅ `q` quits (close_window)

## Skins & themes

- ✅ `--export-skin NAME` writes app.toml, main.rhai, core.rhai, tokens.rhai,
  components/, layouts/, patterns/, assets/, theme.rhai,
  themes/flexoki_light.rhai to `$XDG_CONFIG_HOME/disktree-rhai/skins/NAME/`
  (byte-identical to `ui/`, verified by diff)
- ✅ editing the export and running `--skin NAME` loads the edited copy
  (verified: header text change visible)
- ✅ `--dev`/`--skin-dev` hot-reload edits within ≤3 s; a broken edit
  surfaces the compile error without stranding the view
- ✅ `--skin ./skins/minimal` runs the second skin: slice-and-dice tiling,
  grayscale depth palette, no sidebar/controls — keys unchanged
- ✅ `--theme light` switches the Disktree family to Flexoki Light
  (paper #FFFCF0 surface, Flexoki blue accent, pale tile tints);
  default resolution: `--theme` > Omarchy `theme/colors.toml` mode > dark

## Known deviations (v1)

- shift+tab reverse cycle — node key handlers carry no modifiers (G7/G8)
- double-click on the panel edge resets its width — SplitPane exposes no
  double-click on the handle (gpui-rhai gap, see UPSTREAM_GAPS)
- find input cannot type uppercase or non-Latin characters — node key
  events carry no key_char (G7/G8)
- widening scan (`g`, dim crumbs above the root) — v1.5 by design

2026-10-07 later: the platform title bar is replaced by the official
TitleBar component (window drag + double-press zoom, traffic-light inset;
`window_drag_areas(true)` on the mount config — off by default upstream),
theme switching rebuilds the mosaic paint so tiles follow the new palette,
and `docs/screenshots/` is refreshed from the new offscreen screenshot
example. The theme list's families match the bundled palettes exactly
("Tokyo Night", "Retro 82" — with spaces).

Shipped 2026-10-07 (was open): the crumb ▾ sibling menu, dim-while-typing
(tile fills, not just labels), the colour legend + scan totals row,
right-click reveal, sidebar drag-resize with responsive hide, git state in
the selection panel, the navigation growth animation, 17-palette theme
switching, and the end-to-end key harness (`cargo run --example
key_harness`).


## 2026-10-10 — gpui-rhai 0.2 validation complete

All core contracts exercised and passing. One open upstream issue
(G12/#140: SplitPane drops absolute content) with a hand-rolled drag
workaround; filed at github.com/eddix/gpui-rhai/issues/140. Remaining
minor gaps: G7/G8 (no key_char for uppercase/IME), G11 partial (no
double-click on split handle). Acceptance trio: skin_check (3-source
prepare), key_harness (real-window all-keys + mouse drag), screenshot
(offscreen PIL-verified).


## 2026-10-22 — moved into gpui-rhai `apps/`

The app now lives in gpui-rhai's repository as a reference application
and builds against that checkout by path. The registry components were
brought up to date with `gpui-rhai update` (7 components whose content had
changed under unchanged versions), and the unmodified copies of the token
base and the 15 bundled palettes were refreshed: the current Button needs
`control.*` tokens the 0.2.1 token base lacked, which failed the file skin.
skin_check is now `tests/skins.rs`, so `cargo test` prepares all three
skins; key_harness and screenshot build on every platform and run on
macOS. Verified on macOS: `cargo test` (unit tests + 3 skins),
key_harness, and a screenshot of a scanned tree with the official
SplitPane.

2026-10-22 later: key_harness drags the side panel's separator where the
frame draws it (found through the accessibility snapshot) and asserts it
moved 150px; the old fixed point pressed the mosaic and tested nothing. The
first real drag showed the sidebar kept its fixed 368px inside the wider
pane, leaving an empty strip, and its left border doubled SplitPane's own
line: the sidebar now fills the pane without a border, and the unused
`panel_width` state is gone. `SHOT_DRAG` presses the separator the same way.

2026-10-22, native zoom: the navigation zoom had not run since
`apply_navigation` started storing the new crumbs before the layout
recompute — the recompute compared the crumbs with themselves, so neither
the entering nor the ascending branch ever fired. The departing crumbs now
travel as `layout.came_from`, `layout.gen` counts across recomputes, and the
zoom is native Motion on the canvases (G11 (a)) instead of the 16 ms
`anim_tick` timer, which is gone. Verified with `SHOT_FRAMES` (frames timed
after the last key) and `+ms` waits in `SHOT_KEYS`: 20 ms after `enter` and
after `u`, the frame is zoomed on the tile with the labels hidden; the
settled frame is the identity layout with labels.
