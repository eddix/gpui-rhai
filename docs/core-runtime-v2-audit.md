# Core Runtime v2 completion audit

This is the evidence ledger for `INTENT.md` and `IMPLEMENTATION_PLAN.md`. A
green current test suite is necessary but does not mark an incomplete
workstream complete. Every row becomes complete only when its implementation,
negative cases, acceptance application, and required platform gate exist.

Last updated: 2026-08-31.

| Workstream | Status | Evidence present | Blocking gaps |
| --- | --- | --- | --- |
| A. Rhai boundary | Partial | Rhai is pinned to 1.26.0; volatile native call context is isolated in `invocation.rs`; retained callback closure/curry/import tests exist | operation accounting across every callback path; stale effect/reload matrix; backend trait and optional Grain parity harness |
| B. Retained tree | Partial | monotonic `NodeId`; atomic keyed reconciliation; group-local key identity; duplicate/type/reorder rollback tests; lifecycle/renderer consume `RetainedUiTree`; retained nodes own event bindings/ref/capture/scroll metadata; Host-configurable pre-commit budgets for nodes/handlers/components/effects/signals/refs | retained nodes do not yet own accessibility or primitive Entity handles; Canvas/layer/virtual-data budget classes, diff metrics, and Entity cycle gate remain |
| C. Components/dependencies/effects | Partial | `define_component`/`render_component`; closed structural prop union; retained invocation recipes; topmost dirty rerender; field/store invalidation; declarative named effects with dependency restart, imported-context cleanup, failure rollback, and 64-transition budget | theme/locale/viewport/geometry dependency graph; broad Map/Array path APIs; effect-owned async scopes and Inspector data; full-render equivalence/property tests; module-init purity audit |
| D. Atomic visual surface | Partial | final `Box` node kind; nested layout-transparent `Fragment` flattening with decoration rejection; row/column/stack Box helpers; plain Text plus typed inline Span runs rendered by one GPUI `StyledText`; retained keyed Canvas scene with validated rect/circle/line commands, typed colors, paint-only Rust execution, and command budget; required semantic theme schema plus validated namespaced color/length/number/string tokens under atomic app/window/subtree selection; current image/style subset | full wrapping/alignment/ellipsis/clamp/link/selection/search text contract; svg atom; exhaustive typed layout/paint and namespaced length/data consumption APIs; font system; Canvas path/gradient/transform/clip/hit/events/a11y/signals |
| E. Signals/animation | Partial | component/key/type-scoped `NativeSignal`; Rhai and Rust foreground writes; rollback and stale-handle behavior; approved width/height/opacity/translation/color bindings sampled without Rhai rerender; compatible identity retention tests; legacy node animation runtime and deterministic unit tests | generalized property-source ownership; transitions/springs/keyframes/derived sources on final atoms; scroll/Canvas bindings; writer/clock Inspector data; enter/exit/layout/shared-layout behavior |
| F. Events/refs/focus/scroll/a11y | Partial | ordered capture/target/bubble binding model; independent response controls; real GPUI pointer down/up/move/wheel normalization; retained mouse capture registry rerouting window move/up to `NodeId` with release/unmount cleanup; direct Host callback and schema-checked `NativeHandlerRef` through one outer transaction; retained `ElementRef` to `NodeId`; stale/key/duplicate validation; prepaint geometry and exact read invalidation; window-scoped ref focus commands backed by Host-owned `FocusHandle`; per-axis scroll Style, retained `ScrollHandle`, and transactional ref scroll offsets | transformed local/content coordinates and central hit path for non-mouse/multi-pointer input; cancel/coalescing/frame accounting; focus scopes/traps/restore/roving/directional behavior; nested scroll boundary/containment/scrollIntoView/signals; public layer behavior; retained accessibility tree |
| G. Native mechanisms | Partial | keyed custom primitive mount/update(previous,next)/render/unmount with retained normalized snapshots and panic boundaries; variable-height GPUI `list/ListState` production element; deterministic Fenwick measurement/overdraw/anchor/reorder/tail core with 10k-item tests; native text input/area | explicit task/subscription/capture/resource ownership on primitive instances and Entity-cycle gate; data-backed formal item execution is still eager; follow-tail/measurement metrics wiring and no-empty-frame benchmark; text undo/redo and document infrastructure |
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
