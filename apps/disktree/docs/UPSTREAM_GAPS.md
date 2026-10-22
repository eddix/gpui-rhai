# gpui-rhai upstream gap report

Findings from building disktree-rhai on gpui-rhai. Each entry: what we hit,
the evidence, and the disposition (fixed upstream on branch → PR, worked
around locally, or accepted).

This is the record of the standalone repository (eddix/disktree-rhai, up to
2026-10-22). Since the app moved into gpui-rhai's `apps/`, a gap it finds is
a gpui-rhai issue, and an app-side workaround links to it.

## G1 — Script op budget (1M) has no host override; cached compute phases exceed it

- **Hit at**: M0 spike 1 (`cargo run --release --example spike_squarify`).
- **Evidence**: A full-viewport squarified treemap layout at tobi's default
  depth 3 with real pruning rules (min_tile 5px, header ≥44px wide, ≤96
  children/dir + Others) is 83 `squarify` calls / 2082 kept tiles /
  **2,706,703 rhai ops** (release, 78 ms wall). Runtime cost is fine for a
  cached one-shot layout, but `MAX_SCRIPT_OPERATIONS = 1_000_000`
  (engine.rs:550) is a hardcoded const wired into `RuntimeEngine::new()`'s
  on_progress closure — no configuration path for a host that knows a phase
  is one-shot and cacheable.
- **Disposition**: upstream PR **gpui-rhai#96** (branch `host-operation-budget`):
  `RuntimeEngine::set_operation_limit` + `.operation_limit()` on both view
  builders (default unchanged at 1M), applied through the window factory and
  dev-reload candidates. Skins keep the ability to swap layout algorithms;
  disktree-rhai runs at 10M while the PR awaits review/merge.

## G2 — Function expression depth (32) too shallow for algorithmic skin code

- **Hit at**: M0, writing `core.rhai`.
- **Evidence**: rhai's own default function depth is 16; gpui-rhai raises it
  to 32 (engine.rs:2262 `set_max_expr_depths(64, 32)`). The ~90-line
  `squarify` port already needs ~22 — ~30% headroom. Any skin function of
  real algorithm size (recursive layout, filters, analysis) exceeds 32 today
  with a bare `ExprTooDeep` parse error.
- **Disposition**: same upstream PR gpui-rhai#96 — function-body limit raised
  to 128; call-level recursion stays capped at 64 and the op budget remains
  the real guard.

## G3 — rhai `arr[i..j]` cannot be written with computed bounds (parse surprise)

- **Hit at**: M0, `core.rhai`.
- **Evidence**: rhai binds `[]` tighter than `..`, so `areas[start..end]`
  parses as `(areas[start])..end` and fails at runtime with
  `ErrorMismatchDataType("i64", "Range<i64>")`. Range literals in `[]` are
  common in examples, which makes the computed-bound case a quiet trap.
- **Disposition**: documentation-level note for skin authors (our core.rhai
  spells slices with a loop / index-based helpers). Candidate upstream doc
  patch; not a code gap.

## G4 — `WindowOptions` title via `titlebar: Option<TitlebarOptions>`

- **Hit at**: M0 scaffold.
- **Evidence**: `options.title` does not exist; a host sets the title through
  `titlebar: Some(TitlebarOptions { title, .. })`. Minor API surprise, no
  fix needed; noted for other embedders.
- **Disposition**: accepted (documented here).

## G5 — Script errors after prepare() are banner-only; hosts need their own stderr bridge

- **Hit at**: M1, wiring the skin's scan effect.
- **Evidence**: `EmbeddedScriptView::prepare()` can succeed while the
  window mount / first render / effect start later fails; the failure
  surfaces only as the in-window error banner and `ScriptViewHandle::
  last_error`. Nothing reaches stdout/stderr, so headless smoke runs
  (timeouts, CI) show a "clean" app with a blank/error window. The same
  applies to store/capability schema mismatches: an undeclared store
  write kills init with no console trace. This cost a long bisect of a
  healthy-looking app (several distinct causes hid behind the same
  silence).
- **Disposition**: worked around locally — disktree-rhai's host mounts
  through `ScriptViewHost` itself and mirrors `last_error` to stderr from
  its root render. Upstream suggestion: `ScriptApplication` (or an env
  flag like `GPUI_RHAI_STDERR_DIAGNOSTICS=1`) should mirror mount/render
  errors to stderr so unattended runs can fail loudly.
- **Re-checked 2026-10-22 (main)**: still open. Failures are sticky and
  structured now (`last_error` / `last_diagnostic`), but reach no stderr and
  no observer; only fatal startup errors print. Filed as gpui-rhai#185.

## G7 — Node key handlers rejected punctuation keys

- **Hit at**: M3 (navigation). tobi's keymap is `?` help, `/` filter, `[` `]`
  depth, `-` `=` `0` zoom — none of these could be registered: both
  `on_key_value` and `on("key:<name>", …)` validated names as
  `[a-z0-9_]` only, while the dispatcher looks handlers up by the raw gpui
  key string (`?`, `/`, `[`, `-`, `=`).
- **Fix upstream**: PR #97 (branch `key-handler-punctuation`) — key handler
  names accept printable ASCII except the reserved `:` separator; general
  event names keep the snake_case rule; script-boundary tests + a
  "Node key handlers" doc section.
- **Related non-gaps**: modifiers on node key handlers do not exist by
  design (chords are host-bound actions); the disktree host registers
  `alt-left`/`alt-right` history actions through `ScriptViewHost::key_binding`
  + `ctx.register_action`, which works as documented.

## G6 — Rhai-for-Rust-authors traps (documentation)

- **Hit at**: M0/M1 repeatedly.
- **Evidence**: several things a Rust author writes by reflex are invalid
  or behave unexpectedly in rhai 1.26 and fail only at runtime:
  `x as int` is not a cast (`as` only aliases imports; use `to_int()`),
  there is no ternary `?:`, `arr[from..to]` with computed bounds parses as
  `(arr[from])..to`, module-level `const` values are not visible inside
  function bodies when called cross-module, and identifiers may not start
  with `_` (`_i`). Each was found through a runtime error.
- **Disposition**: documented here for skin authors; candidate upstream
  docs page ("Rhai for Rust authors") rather than a code change.

---

# Final status (M5)

## PRs

- **#96** `host-operation-budget` — G1+G2. Open, awaiting upstream agent review.
- **#97** `key-handler-punctuation` — G7. Open, awaiting upstream agent review.
- **#99** `script-theme-query` — G8. Open, awaiting upstream agent review.
- Local integration branch `disktree-integration` (merge of both) is what
  this repo builds against; 440 upstream tests green on it.

## G8 — Scripts cannot observe the resolved theme variant

- **Hit at**: M5 themes. `ctx.set_theme_system("Disktree")` exists and is
  the right primitive for system follow, but there is no query for what
  resolved: scripts can set a family preference, not read the active
  variant/mode. A skin whose palette branches on light vs dark (ours:
  canvas tile fills computed as rgba ints in rhai) cannot know which side
  won, so `--theme system` would recolour chrome (tokens) but leave the
  mosaic palette wrong.
- **Fix upstream**: PR **gpui-rhai#99** (branch `script-theme-query`) —
  `ctx.theme_variant()` returns `#{family, name, mode}` for the context's
  scope, tracked as a theme environment dependency so dependent effects
  re-render on selection or system-appearance changes; `motion_tokens()`
  resolves through the same helper. disktree-rhai now uses it: `--theme
  system` follows the window appearance and the canvas palette reads the
  resolved mode each render. The host fallback (`--theme` > Omarchy > dark)
  still picks the launch default.

## G10 — Manual hosts cannot register their native window for script window commands

- **Hit at**: the skin's `q` key. Two layers:
  1. `ScriptViewHost::new` hardcodes `WindowCommandPolicy::Disabled` and the
     policy-taking constructor was crate-private — fixed upstream by **PR
     #100** (`host-window-policy`, branch pushed, awaiting review).
  2. With the policy enabled, `ctx.close_window("main")` still failed with
     `native window main is unavailable`: `PreparedScriptView::mount`
     builds a private, empty native-window registry. Fixed upstream by
     **PR #103** (`register-native-window`) —
     `ScriptViewHandle::register_native_window(id, window, cx)`, one call
     after mount. disktree-rhai registers in its mount callback; `q`
     now closes the window (blind-tested: focus via Hyprland's new lua
     dispatch API, keypress, process-exit check, zero script errors).

## G9 — rhai data-size limits are cumulative and not host-configurable

- **Hit at**: hover-perf work (M6). A cached paint stored in an app-store
  field failed to write with `Size of array/BLOB too large` at ~10k
  elements — not because any single array was large, but because rhai's
  `calc_data_sizes` counts array elements **cumulatively across the whole
  store value** (maps get their own 100k budget, arrays 10k total). A
  store field holding tiles + a tile cache + paint rows crosses 10k array
  elements easily; hatch slashes alone (hundreds of 4-element coordinate
  rows) were the hidden multiplier.
- **Hit again at (0.2 migration, 2026-10)**: the first real-disk scan after
  the gpui-rhai 0.2 upgrade failed at render with the same error — the
  layout store value alone now carries tiles (2,000) + one crumbs array per
  tile + the cache slot (tiles again) + up to three built mode paints. Two
  more skin-side measures shipped: the paint slots (`paint0`-`paint2`) and
  the tile cache (`cache`) moved out of `layout` into their own app-store
  values (each store value gets its own cumulative budget; the host declares
  them in `DisktreeExtension::configure_runtime`), and the tile budget now
  scales with the crumb depth — `8500 / (root_len + max_depth + 2)`, clamped
  to 250..2000 — so tiles × crumb length stays under the limit at any
  navigation depth.
- **Workaround (skin-side, shipped)**: paint rows are maps (separate
  budget); hatch slash coordinates are not stored at all — the row keeps
  the tile rect and the render regenerates slashes in ~30 loop steps; the
  layout tile cache shrank to one slot; the layout budget caps at 2,000
  tiles. Also documented in `docs/UPSTREAM_GAPS.md` for skin authors:
  **keep arrays out of store fields at scale — use maps or
  regenerate-at-render.**
- **Suggested upstream**: (a) document the cumulative semantics in the
  store/capability docs; (b) like PR #96 did for the op budget, let hosts
  configure (or raise) `max_array_size` for trusted embedded skins.
- **Re-checked 2026-10-22 (main)**: resolved upstream. `docs/rhai.md` (Limits)
  documents the cumulative counting, and a Host raises the limit in
  `ScriptViewExtension::configure_rhai_engine` (`engine.set_max_array_size`):
  200 rows of 60 items fail by default and pass at 50,000, both built locally
  and written to an app store. The skin-side measures above stay; they work.

## G11 — Timer-driven script animation and handle double-clicks

- **Hit at**: the 2026-07 fidelity pass. (a) The navigation growth animation
  steps itself with `timeout(key, 16ms, cb, payload)` — a declared timer is
  one-shot and remembered by signature, so the chain only re-arms because
  the step number rides in the payload; `ctx.motion_duration` supplies the
  step count so reduced motion collapses to zero steps. Workable, but a
  frame-callback or native interpolation of canvas content would not re-run
  the Rhai render ~12 times per navigation. (b) tobi's side panel resets its
  width on a double-click of the drag handle; `components/split_pane`
  exposes no double-click (or click) event on the handle part, so the reset
  is not ported.
- **Suggested upstream**: (a) an animation-frame or native-transition hook
  scripts can attach to; (b) a handle event or `on_double_click` part hook
  on SplitPane.
- **Re-checked 2026-10-22 (main)**: (a) Motion samples in Rust by design and
  never calls Rhai per frame; Canvas primitives take native scale and
  translate, but gpui-pre transforms no subtree, so text labels cannot scale
  with the mosaic. Planned: a custom cubic-bezier easing (gpui-rhai#186), then
  a native canvas zoom with the labels faded in at the end. (b) Still open:
  SplitPane emits only `resize`; deferred, not filed.
- **(a) done 2026-10-22**: the zoom is native Motion on both canvases
  (`scale_x/y` and `translate_x/y` with `cubic_bezier(0.33, 1, 0.68, 1)`,
  the ease-out cubic the Rhai steps used), replayed per layout with
  `motion_replay_key`; the labels wait in one container and fade in by
  keyframes. No Rhai runs per frame.

## G12 — SplitPane panels drop absolutely-positioned content (post-grips runtime)

- **Hit at**: the 2026-10-08 component sync (upstream moved past the resize
  rework, `feat(resize): decorative grips` + renderer interaction fixes).
  The mosaic — canvas plus labels, all absolutely positioned inside the
  pane — silently vanished; panels and flow-positioned children kept
  rendering, and no script error fired. Isolated by bisection with PIL
  pixel checks on offscreen captures: a plain flow box in either SplitPane
  pane renders; an absolutely-positioned box in EITHER pane (start or end)
  does not. Pinning the registry's own pre-grips `split_pane.rhai`
  (351d6ec) against the current runtime reproduces it, so the regression
  is in the runtime's interaction-wrapper/renderer path, not the component
  script.
- **Workaround (skin-side, shipped)**: the side panel split is hand-rolled
  — a fixed-width panel whose 8px left edge drags via window-coordinate
  pointer events (press/drag/release, double-press resets, Escape
  cancels), verified in `key_harness` with real mouse simulation and
  pixel-diffed in the screenshot tool (`SHOT_DRAG=1`). The official
  `split_pane.rhai` copy stays in `ui/components/` unused, to re-adopt
  once upstream fixes this.
- **Suggested upstream**: render absolutely-positioned descendants of
  SplitPane (and any native resize primitive wrapper) correctly, or gate
  the grips rework behind it; a story with an absolute child in each pane
  would have caught it.
- **Re-tested 2026-10-12 (0.2.1)**: #140's fix (2b29cbb) resolved the
  direct case — a bare absolute node in a pane renders. But the mosaic's
  pattern (a wrapper box with flex_grow + relative + overflow_hidden
  containing absolute children) still renders blank. Filed as #156.
  The hand-rolled drag stays until #156 is fixed.
- **Resolved 2026-10-22**: not a runtime bug. The retest ran the copy of
  `split_pane.rhai` installed before #140 (block panes, where `flex_grow`
  does nothing, so the stage was 0px tall and its clip hid the mosaic).
  The official SplitPane is back (disktree-rhai 3647e89). `gpui-rhai
  update` skipped that fix because the component kept version 0.2.0; it
  now compares the installed baseline when versions tie.

## Verified non-gaps (accused, then cleared)

During M3–M5 blind testing these looked like upstream bugs but were
diagnosed as environment or test-injection artifacts:

- **Canvas scene diffing by key** — suspected when an Age-mode recolour
  failed to repaint; actually a stale-crop/scale-2 measurement error, plus
  wtype double-injecting some keys (duplicate `on_key_value`
  registrations in our own skin doubled `r`/`o`). After dedupe, same-key
  canvas colour and geometry updates repaint correctly; zoom
  (geometry-only, key-stable) verified working. No upstream defect.
- **Theme never loading** — `theme_check` against a raw `rhai::Engine`
  fails on expression depth (16), which is not what apps run; against
  `RuntimeEngine` the theme loads, validates, and resolves. The visual
  "default warm grey" observations were the desktop wallpaper sampled at
  logical instead of physical (×2) coordinates.
- **Escape not dispatching** — wtype focus racing on Hyprland; keys
  verified reliable after `hyprctl dispatch focuswindow`. Plain `/`
  (keysym `slash`) never arrives via wtype while shifted `?` does —
  injection-only, physical keyboards unaffected.

## Rhai-for-skin-author notes (carried from G3/G6)

Arrays are values (return them, don't pass out-params); computed slice
bounds are a parse trap; module consts are invisible in fns; `shared` is
a reserved word. All documented in G3/G6 with no further surprises
through M5 — the M4 filter port (Whole/Partial keeps) was written in one
pass against the engine Rust API.
