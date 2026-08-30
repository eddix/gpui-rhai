# Core Runtime v2 completion audit

This is the evidence ledger for `INTENT.md` and `IMPLEMENTATION_PLAN.md`. A
green current test suite is necessary but does not mark an incomplete
workstream complete. Every row becomes complete only when its implementation,
negative cases, acceptance application, and required platform gate exist.

Last updated: 2026-08-31.

| Workstream | Status | Evidence present | Blocking gaps |
| --- | --- | --- | --- |
| A. Rhai boundary | Partial | Rhai is pinned to 1.26.0; volatile native call context is isolated in `invocation.rs`; retained callback closure/curry/import tests exist | operation accounting across every callback path; stale effect/reload matrix; backend trait and optional Grain parity harness |
| B. Retained tree | Partial | monotonic `NodeId`; atomic keyed reconciliation; group-local key identity; duplicate/type/reorder rollback tests; lifecycle/renderer consume `RetainedUiTree` | retained nodes do not yet own handlers, refs, signals, geometry, capture, accessibility, or primitive handles; no host resource budget, diff metrics, or Entity cycle gate |
| C. Components/dependencies/effects | Partial | `define_component`/`render_component`; closed structural prop union; retained invocation recipes; topmost dirty rerender; field/store invalidation; declarative named effects with dependency restart, imported-context cleanup, failure rollback, and 64-transition budget | theme/locale/viewport/geometry dependency graph; broad Map/Array path APIs; effect-owned async scopes and Inspector data; full-render equivalence/property tests; module-init purity audit |
| D. Atomic visual surface | Missing | current text/image/container/style subset | final fragment/box/span/svg/canvas atoms; exhaustive typed layout/paint/text; font system; namespaced themes; rich text/selection/search; Canvas retained scene |
| E. Signals/animation | Partial | component/key/type-scoped `NativeSignal`; Rhai and Rust foreground writes; rollback and stale-handle behavior; approved width/height/opacity/translation/color bindings sampled without Rhai rerender; compatible identity retention tests; legacy node animation runtime and deterministic unit tests | generalized property-source ownership; transitions/springs/keyframes/derived sources on final atoms; scroll/Canvas bindings; writer/clock Inspector data; enter/exit/layout/shared-layout behavior |
| F. Events/refs/focus/scroll/a11y | Partial | ordered capture/target/bubble binding model; independent response controls; real GPUI pointer down/up/move/wheel normalization; retained mouse capture registry rerouting window move/up to `NodeId` with release/unmount cleanup; direct Host callback and schema-checked `NativeHandlerRef` through one outer transaction; retained `ElementRef` to `NodeId`; stale/key/duplicate validation; prepaint geometry and exact read invalidation; window-scoped ref focus commands backed by Host-owned `FocusHandle` | transformed local/content coordinates and central hit path for non-mouse/multi-pointer input; cancel/coalescing/frame accounting; focus scopes/traps/restore/roving/directional behavior; scroll ref commands and nested scroll router; public layer behavior; retained accessibility tree |
| G. Native mechanisms | Missing | mount/render/unmount custom primitive registry; fixed-height uniform list; native text input/area | update lifecycle; retained Entity/resource ownership; variable-height collection; text undo/redo and document infrastructure |
| H. Registry migration | Missing | registry components are inspectable Rhai modules using the final component registration names | registry still calls privileged table/dropdown/date-picker/toast/overlay nodes; those node kinds and renderers remain; atomic/headless replacements and source audits are absent |
| I. Tooling/apps | Missing | manifest/component checks, Inspector baseline, current examples | final schema/definition emission and LSP; automation IDs/query/actions/screenshots; artistic showcase; pure-Rhai Mini Timeline; variable-height chat; Rust fast-path parity app |
| J. Certification | Missing | format, all-target Clippy with `dev-reload`, and current workspace tests pass | final API/node/style/event matrices, screenshot/accessibility/IME/hot-reload/resource/property tests, 120 Hz performance evidence, and final manual macOS gate |

## Current automated baseline

Run from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --features dev-reload -- -D warnings
cargo test --workspace --all-targets
```

The audit is complete only when every row is complete and the acceptance gates
in section 13 of `IMPLEMENTATION_PLAN.md` pass without exclusions.
