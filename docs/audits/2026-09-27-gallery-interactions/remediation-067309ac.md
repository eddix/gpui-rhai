# Gallery interaction review remediation

Remediation baseline: `067309ac`. Original review baseline: `502c9fe2`.

## Status

The source/runtime findings in the review are closed by `eb7be6c0`,
`6fcf7b23`, and `067309ac`:

- Ordinary single-axis Rhai and native containers opt out of GPUI's implicit
  cross-axis wheel conversion. Two-axis surfaces retain explicit 2D behavior.
- Focus pseudo styles now create an interaction wrapper, and native Input and
  Textarea share one focus identity with that wrapper. The wrapper observes
  focus without becoming a second tab stop; explicit `tab_stop(false)` remains
  authoritative.
- Every Gallery shell action is keyboard reachable and has focus-visible
  paint. The complete shell test exercises mode, theme, locale, search, and
  both pane splitters.
- Operations deployment is a schema-checked Rust transaction carrying target,
  channel, expected revision, and failure intent. Success advances only the
  target `NativeTextDocument`; cancel, failure, and stale revision preserve the
  committed model.
- Explore and Operations are real top-level modes. Workbench no longer renders
  inside story metadata/source chrome. Explore navigation and source panes are
  collapsible, pointer-resizable, and keyboard-adjustable.
- Public `components/split_pane` provides controlled horizontal/vertical
  layout, stable panel keys, min/max constraints, deterministic
  over-constraint handling, controlled collapse, RTL pointer behavior, native
  preview, one release commit, and separator keyboard/semantic behavior.
- Component-module search now creates a direct navigation row into the catalog
  case that presents the matching component. SplitPane also has its own story.

The original characterization probes now fail in the expected direction:
Rhai cross-axis scroll remains stationary, the real editor focus paints the
focus-ring token, and an `audit-canary` deployment changes only
`operations_config_edge-01` from revision 1 to revision 2.

## Verification

- `cargo test --workspace --all-features`: pass (439 core unit tests plus all
  workspace integration/CLI/registry/doc tests).
- Independent native suite: 119 tests pass (30 Chart, 9 control visual, 15
  Gallery, 62 keyboard/interaction, 3 primitive lifecycle).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass.
- Full release smoke: pass, including Gallery large-data launch from an empty
  temporary directory.
- Release artifact audit and existing 67-PNG structural baseline audit: pass.
- `gpui-rhai` and `gpui-rhai-registry` package assembly: pass. CLI package
  assembly remains sequenced after publishing its exact `gpui-rhai 0.1.6`
  dependency to crates.io.

## Remaining platform gate

The new behavior changes Gallery composition, so the affected Gallery PNGs
must be visually re-recorded rather than treating the old structurally valid
files as current evidence. The release `.app` was built and launched, but the
macOS session locked before screenshot inspection. This is the only remaining
gate in this remediation record.
