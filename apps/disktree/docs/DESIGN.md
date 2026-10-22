# disktree-rhai design consensus

Agreed 2026-10-01 (design interview on top of three codebase surveys:
tobi/disktree, remorses/gpuix disktree clone, eddix/gpui-rhai).

## Goals

1. Prove gpui-rhai can build a real app of gpuix-disktree's caliber.
2. Stress gpui-rhai in the process; every gap found is fixed upstream on a
   branch → PR → merge, and recorded in the upstream gap report.

## Ground truths

- **tobi/disktree** defines visual + interaction fidelity (functional,
  visual, and blind-operable parity; the title/branding says disktree-rhai).
- **gpuix disktree** defines the v1 feature scope and serves as the
  architecture reference. Its own deviations (frosted light theme, pastel
  palette, WCAG ink) are not copied.
- Data model follows tobi: full tree, ≤96 children per dir in layout with an
  Others tail merge, min_tile 5px — not gpuix's 12-file truncation.

## Architecture

```
Rust host (thin)
 ├─ scanner: st_blocks×512, hardlink (dev,ino) dedup, single filesystem by
 │  mount source (btrfs subvolumes count as one volume), hidden included by
 │  default, symlinks not followed, cooperative cancel, error counts,
 │  classify (9 Category + 9 Reclaim), aggregation
 ├─ capabilities: disktree.scan (subscription, delivery:"latest"),
 │  disktree.open (xdg-open), skin watcher
 └─ skin loading: embedded default ↔ file skins with hot reload

ui/ (the default skin, embedded via include_str!)
 ├─ core.rhai  — squarify/hit-test/formatting shared library; layout runs in
 │               event callbacks/effects and is cached in the store (never
 │               inside view(): 1M-op budget + MutationDuringRender)
 ├─ main.rhai  — the full Explore-screen port
 ├─ components/, theme.rhai, fonts/
 └─ skins are the same shape: a ui/ directory. Skins consume host data +
    action capabilities; they cannot rewrite engine behavior.
```

- Treemap painting: canvas commands (rects/hatch/stroke rings) with built-in
  `canvas_key` hit-testing; labels are absolutely-positioned `text()` nodes
  (≤150), since canvas cannot draw text.
- Layout in rhai (skin-swappable) with an M0 spike gate; fallback is Rust-side
  layout, which costs only the "skins can swap the tiling algorithm" feature.

## Skins (the plus)

- `--export-skin [name]` writes the embedded skin's exact source to
  `$XDG_CONFIG_HOME/disktree-rhai/skins/<name>/`.
- Load order: `--skin <path>` > `--skin <name>` > config default > embedded.
- `--skin-dev <path>` = file-backed with the gpui-rhai watcher (last-good on
  failure). A skin broken at load falls back to the embedded one with an
  error panel.
- `skin_api_version` in app.toml; no cross-version stability promise in v1.
- Ship a minimal second skin (structural reshaping proof) in v1; a radial
  DaisyDisk-style skin follows.

## Themes

v1 ships Tokyo Night (dark) + Flexoki Light (light) token sets, dark by
default. System light/dark follow and Omarchy theme reading come after
(v1.x), with Omarchy strictly degradable (bonus, not a dependency).

## Scope

- **v1 = gpuix feature set + tobi interactions**: scan panel, treemap,
  breadcrumbs, hover node card, sidebar (Selection / Worth a look / Disk),
  help overlay, key bar; keys enter/⌫ u esc (cascade)/arrows hjkl/tab/t/[ ]/
  r/i/d/p/?/q/- = 0/alt←→/</ >/`/` filter (last)/o; mouse select + re-click
  enter, double-click, anchored wheel zoom with zoom-to-enter and ascend at
  bottom, shift-wheel pan, side buttons; UI controls (hidden/apparent
  checkboxes, metric segmented, depth stepper) alongside keys.
- **Not built**: the deletion system. **v1.5**: volumes `v`, whole-disk `g`/
  widening, cross-fs options, growth animation, UI rem scaling.

## Workflow & acceptance

- Independent repo, path dep on a gpui-rhai working branch; every upstream
  fix merges through PR before this repo depends on it.
- Tests: core.rhai unit tests against tobi's expectations; scanner parity vs
  `du`/tobi measurements; manual visual/interaction checklist derived from
  tobi's 36 harness tests; feature matrix + side-by-side screenshots for the
  acceptance trio.

## Milestones

| # | Contents |
|---|----------|
| M0 | scaffold + spikes (rhai squarify op budget, canvas at scale) |
| M1 | scanner (full tobi semantics) + scan capability + scan panel |
| M2 | core.rhai layout + canvas treemap + labels + palette + tooltip |
| M3 | navigation & full interaction set |
| M4 | skin system + minimal skin + `/` filter |
| M5 | themes (light/system/Omarchy-degradable) + acceptance trio + gap report |
