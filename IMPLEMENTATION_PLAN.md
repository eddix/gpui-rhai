# Core Runtime v2 implementation plan

## 1. Execution policy

This plan implements the final Core Runtime v2 described by `INTENT.md`.
Sections are internal dependency order, not preview products. Every merged API
must belong to the final design. The maintainer is asked to review only the
complete delivery.

Constraints that apply to every section:

- keep crate/component version `0.1.0` and `RUNTIME_API_VERSION` 1;
- accept destructive Rust/Rhai/schema/registry migration;
- add no compatibility shim, deprecated alias, dual path, or downstream app
  adapter;
- keep zero dependency on `gpui-component` and GPUIX;
- pin Rhai exactly and isolate volatile internals;
- preserve view-first embedding and Host-owned UiNode rendering;
- retain last-good UI on every failed candidate;
- run tests and benchmarks continuously even though there is no intermediate
  user acceptance request.

## 2. Final invariants

The implementation is not complete until all invariants hold together:

1. Rhai and Rust snapshots reconcile into one RetainedUiTree.
2. Stable keyed identity owns focus, scroll, capture, signals, animation,
   accessibility, and retained primitive Entities.
3. Formal components rerender independently from tracked dependencies.
4. Rhai and Host/native handlers use one ordered event propagation contract.
5. NativeSignal updates approved hot properties without Rhai rerender.
6. Registry components have no private layout/interaction primitives.
7. Public atoms can build the artistic showcase and pure-Rhai Mini Timeline.
8. All failure, hot reload, and resource-limit paths retain an internally
   consistent last-good generation.

## 3. Workstream A: Rhai characterization and invocation boundary

### A1. Pin and adapter

- Confirm the locked Rhai 1.26.0 feature graph in tests.
- Create one `ScriptInvocationContext` module as the only user of
  `NativeCallContextStore`, `GlobalRuntimeState`, or other Rhai internals.
- Move existing ScriptCallback native-context storage and invocation behind it.
- Keep Engine/Dynamic/FnPtr foreground-only and reject cross-thread leakage at
  type/API boundaries.
- Keep `OptimizationLevel::Simple` and the restricted ScriptSource resolver.

### A2. Characterization matrix

Add version-pinned tests for:

- imported named callbacks, nested module helpers, curry, and source traces;
- anonymous/captured FnPtr rejection at retained callback boundaries;
- independent imported component invocation;
- effect start/cleanup in original module context;
- operation accounting across callback paths;
- old-generation stale delivery and cleanup;
- verified Rhai hazards relied on by the security design.

### A3. Script backend boundary

- Define AST-interpreter backend traits without changing script semantics.
- Add optional Grain parity harness behind a development/test feature.
- Compare UiNode snapshots, events, errors, imports, limits, and diagnostics.
- Record residual coverage and benchmark results before any release-default
  decision. AST remains the semantic oracle and fallback.

**A gate:** no volatile Rhai type escapes the adapter; existing module callback,
hot reload, component, and capability tests pass through it.

## 4. Workstream B: retained tree and reconciliation

### B1. Core model

- Add opaque monotonic `NodeId` and explicit node-kind identifiers.
- Separate snapshot `UiNode` from `RetainedNode` runtime state.
- Implement closed `ComponentPropValue` for data and structural props.
- Store style, semantics, handlers, refs, sources, property bindings, and custom
  primitive handles in retained nodes.
- Add Host-configurable retained resource budgets.

### B2. Keyed diff

- Reconcile children by parent/key/kind with deterministic positional fallback
  only for display-only unkeyed nodes.
- Reject duplicate keys and missing keys on retained-state nodes.
- Produce atomic create/update/move/remove plans before mutation.
- Preserve retained state on compatible updates and invoke explicit unmount for
  removals.
- Implement last-good rollback when validation or retained update fails.
- Add diff metrics and Inspector mutation traces.

### B3. GPUI ownership

- Make one per-view GPUI Entity own the runtime tree and frame scheduler.
- Use WeakEntity in retained closures/tasks and avoid nested Entity updates.
- Batch one `cx.notify()` after committed mutation/frame work.
- Keep GPUI `AnyElement` ephemeral; create retained Entities only for mechanisms
  that require state.

**B gate:** reorder/removal/type-change tests prove identity, state cleanup, no
Entity retain cycles, and atomic failure behavior for Rust- and Rhai-built trees.

## 5. Workstream C: formal components, dependencies, and effects

### C1. Component format

- Replace `export_component/component_render` with
  `define_component/render_component`.
- Register metadata, schema, render invocation context, and source location once
  per module generation.
- Reject mutable module-global UI state and effectful ModuleInit calls.
- Validate data/node/style/callback/slot/ref/signal prop categories.
- Retain normalized invocation recipes for keyed formal components.

### C2. Dependency tracking

- Track component state fields, exact store paths, keyed collection items,
  theme/locale tokens, viewport conditions, and geometry reads.
- Treat whole Map/Array reads as broad dependencies.
- Version values at the Host UiValue/domain layer; equal writes remain clean.
- Build dirty sets and rerender only formal component subtrees.
- Batch one GPUI turn of mutations into one atomic reconcile commit.

### C3. Effects and transactions

- Add declarative named effect descriptors with UiValue dependencies.
- Start only after successful subtree commit; cleanup before replacement and
  exactly once on unmount/reload.
- Extend Host snapshots/rollback to signal writes, dirty sets, effects, and
  queued work.
- Keep external Host side effects explicitly outside rollback.
- Add effect/state feedback-loop budgets and diagnostics.

**C gate:** incremental output is observationally equal to full render across
nested components, state/store/theme/locale changes, failure, and hot reload;
performance probes demonstrate unchanged subtrees do not execute Rhai.

## 6. Workstream D: final atomic visual surface

### D1. Node API

- Implement final fragment/box/text/span/image/svg/canvas atoms.
- Make row/column/stack helpers return Box snapshots.
- Add immutable decoration composition for style, semantics, refs, handlers,
  and headless behaviors without wrapper nodes.
- Remove old constructors and node kinds after registry migration.

### D2. Style values

- Replace globally nonnegative Length with property-validated signed length,
  dimension/intrinsic/grid/angle types.
- Map every stable, cross-platform, safely representable GPUI layout and paint
  field: flex/grid/wrap/tracks, sizing, spacing, position/insets, overflow,
  border/radius/shadow/gradient/alpha, cursor/hit testing, typography,
  selection, clipping, stacking, and 2D transforms.
- Implement explicit merge/provenance and limited documented inheritance.
- Apply runtime state refinements in the final fixed order.
- Reject unsupported or invalid combinations before GPUI build.

### D3. Colors, themes, assets, fonts

- Implement one strict ColorValue parser and typed constructors.
- Replace fixed theme struct with namespaced typed token schemas and atomic
  app/window/subtree switching.
- Add declared font assets, aliases, fallback stacks, weights/styles/features,
  hot reload, and text cache invalidation.
- Add validated bounded inline SVG; retain AssetId/provider and no path/URL
  script policy.
- Extend image fit/alignment/tint/grayscale where GPUI supports them.

### D4. Text and Canvas

- Implement inline Span runs, wrapping/alignment/ellipsis/clamp, continuous
  cross-node selection/copy, subtree search/highlight, links, and automation
  geometry.
- Establish shared document-text infrastructure for future Markdown/Code/Diff.
- Implement retained keyed vector scenes, path building, fill/stroke/gradient,
  transform/clip, hit testing, events, accessibility, signals, and budgets.
- Ensure no Rhai call occurs during GPUI layout/prepaint/paint.

**D gate:** atomic-only scripts reproduce the artistic showcase and Canvas
interaction probes across theme/font hot changes with no private registry API.

## 7. Workstream E: property sources, signals, and animation

### E1. NativeSignal

- Implement component/key/type-scoped typed signal registry.
- Support Rhai, Host, task, and native-handler foreground updates.
- Bind approved Style/transform/scroll/Canvas properties without component
  invalidation.
- Make state/signal synchronization explicit and transactional.
- Track writers, bindings, values, and timing in Inspector/automation.

### E2. PropertySource and animation

- Give each animatable property exactly one literal/signal/transition/spring/
  keyframes/derived source.
- Implement delay/easing, repeat/reverse, retargeting, native clock, and reduced
  motion.
- Implement enter/exit ghosts and committed-geometry layout transitions.
- Implement unique shared layout IDs within compatible Host layers.
- Keep hit testing and visual bounds aligned with transformed animated geometry.

**E gate:** deterministic clock tests cover interruption, state-style target
changes, reduced motion, exit/remount, and layout transitions without Rhai
per-frame execution.

## 8. Workstream F: event, ref, focus, scroll, overlay, accessibility

### F1. Event routing

- Define normalized pointer/wheel/focus/key/layout/scroll/outside event payloads.
- Build transformed window/local/content coordinate conversion.
- Implement capture/target/bubble, ordered multi-handlers, default prevention,
  stop/stop-immediate, and pointer capture/release.
- Add per-frame coalescing with ordered mandatory event queues.
- Generalize HostCallback and add schema-checked named NativeHandlerRef.
- Add frame-level script/reconcile accounting independent of Rhai counters.

### F2. ElementRef and geometry

- Bind component-scoped refs to retained NodeId and reject stale refs.
- Expose focus, scroll, capture, layout/visual bounds, clip, and exact committed
  geometry dependencies.
- Add native conditional Style and configurable breakpoint/container queries.
- Detect measurement/style feedback loops.

### F3. Focus, scroll, layers, accessibility

- Implement public focus scopes/traps/restore, roving and directional/grid
  behavior, tab order, focus-visible, and pointer-focus defaults.
- Implement one per-axis nested scroll router with boundary chaining,
  containment, visible offsets, scrollIntoView, and synchronized signals.
- Generalize the Host overlay coordinator into public layer behavior.
- Build retained accessibility nodes and semantic actions for normal/Canvas/
  virtualized content.

**F gate:** pure-Rhai drag/resize/pan/zoom, nested scrolling, overlay, keyboard,
focus, Canvas hit, and accessibility tests pass through production input paths.

## 9. Workstream G: retained native mechanisms

### G1. Custom primitive lifecycle

- Replace render-only handlers with mount/update/render/unmount retained API.
- Preserve keyed GPUI Entities and validated prop/style/event diffs.
- Bind tasks/subscriptions/capture/resources to instance cleanup.
- Make downstream and built-in primitives use one contract.

### G2. Virtualization

- Implement public data-backed variable-height one-dimensional collection.
- Render only viewport plus overdraw formal item components outside layout/paint.
- Add estimates, measurement cache, remeasure, anchoring, follow-tail,
  bottom alignment, focus retention, and programmatic scroll.
- Expose row/window metrics and prove no empty frames during rapid scrolling.

### G3. Text editor

- Consolidate Input/Textarea editing core into retained TextEditor.
- Add controlled/uncontrolled revisions, optimistic native paint, rejection
  rollback, selection, IME, clipboard, undo/redo, autoscroll, wrapping, and
  auto-grow.
- Keep rich/code editor explicitly separate.

**G gate:** variable-height chat, large list, controlled/uncontrolled editor,
IME, selection, undo/redo, and retained primitive lifecycle tests pass.

## 10. Workstream H: registry migration and removal

- Rewrite every compositional component using public atoms and headless behavior.
- Rebuild Table on generic virtualization/scroll/focus/Canvas or atoms; remove
  the native Table node.
- Rebuild DatePicker, Select/Combobox, Menu, Tabs, Toast presentation, and
  overlays without private native UI nodes.
- Keep native TextEditor, generic virtual collection, layer/focus/scroll/input
  routing, Canvas, animation, and accessibility as generic mechanisms only.
- Convert component source to `define_component/render_component`.
- Migrate themes to namespaced token schemas and assets/fonts to declarations.
- Update bundled registry, CLI metadata, baselines, embedded sources, examples,
  snapshots, docs, and third-party attribution.
- Delete old Style, node/component declarations, specialized native UI nodes,
  obsolete tests, and compatibility code in the same final change series.

**H gate:** official registry audit proves no private constructor, style field,
event, focus, scroll, overlay, or virtualization privilege remains.

## 11. Workstream I: tooling, automation, and acceptance applications

### I1. Definitions and check

- Emit `.d.rhai` definitions for core, extensions, components, themes, and
  capabilities.
- Add known-call/arity AST lint and real headless view execution to check.
- Feed definitions to editor/LSP integration and CLI diagnostics.

### I2. Inspector and automation

- Extend Inspector with retained identity, provenance, dependencies, signals,
  effects, event paths, geometry, focus/scroll/capture, accessibility, and frame
  budgets.
- Add role/name/text/test-ID locators and production-path input commands.
- Add deterministic animation/signal/timer clock and GPU screenshots.
- Add in-process Rust API and opt-in language-neutral stdio/JSON protocol.

### I3. Final applications

- Artistic showcase: declared custom font, blurred window, translucent/
  gradient/shadow cards, animated Tabs, responsive grid, normal/reduced motion.
- Pure-Rhai Mini Timeline: clips, trim handles, snapping, marquee, playhead,
  pointer-centered zoom, two-axis pan, frozen panes, culling, and Canvas detail.
- Variable-height Chat/List: follow-tail, streaming remeasure, Markdown/Code/Diff
  text infrastructure where complete.
- Rust fast-path Timeline: identical UI/event semantics using NativeHandlerRef
  and NativeSignal for comparison.
- Keep product examples and Theme Studio independently runnable.

**I gate:** automation drives every final app without manual input and records
deterministic state/geometry/performance assertions plus curated screenshots.

## 12. Workstream J: final certification

Run and pass:

- unit/property/snapshot tests for values, schemas, diff, state, animation,
  layout, text, Canvas, and event propagation;
- Rhai AST and optional Grain characterization/parity tests;
- native GPUI entity, input, layout, focus, scroll, overlay, IME, accessibility,
  and screenshot tests;
- registry metadata/dependency/source/asset/font/theme/locale audits;
- CLI init/add/check/dev/diff/update/embed dry-run and artifact tests;
- debug/release strict Clippy, rustdoc, release build and smoke tests;
- macOS visual matrix over themes, locale/RTL, resize, scale, reduced motion,
  overlays, selection, and window appearance;
- performance matrix including total Rhai + reconcile + GPUI build/layout/paint.

Reference performance acceptance:

- pure-Rhai Mini Timeline targets 120 Hz on the reference M-series Mac;
- script + reconcile p95 <= 4 ms and total interactive frame p95 <= 8.33 ms;
- other supported baseline environments maintain at least 60 Hz;
- no benchmark may substitute a native primitive for the pure-Rhai acceptance
  path or omit GPUI flush/layout/paint.

Final cleanup requirements:

- no temporary prompt files;
- no old API or compatibility shim;
- no unnecessary native high-level component;
- no unbounded foreground native function;
- no leaked strong Entity cycle;
- no undocumented platform fallback;
- no unresolved manual gate on the reference macOS session.

Only after every gate passes is the complete delivery presented for maintainer
review.

## 13. Public-launch P0 component completion

This is one final delivery, not a sequence of preview releases. Keep version
`0.1.0` and runtime API 1 while accepting destructive source/API migration.

### P0.1 Public taxonomy

- Rename `Dropdown`/`components/dropdown`/Overlay kind `dropdown` to
  `Combobox`/`components/combobox`/`combobox` with no alias or compatibility
  path.
- Keep `Select` for scalar choice, `Combobox` for searchable single/multiple
  choice, `Menu` for action menus, and `ContextMenu` for pointer-anchored action
  menus.

### P0.2 Generic runtime mechanisms

- Add generic range-input behavior for single-value Slider: pointer capture,
  keyboard stepping, min/max/step normalization, native hot interaction, and
  range accessibility semantics without invoking Rhai for every pointer move.
- Extend generic Overlay with validated event-coordinate anchors and modal
  viewport-edge placement used by ContextMenu and Sheet.
- Add themeable overlay scrollbars to generic retained scrolling, including
  per-axis `auto`/`always`/`hidden`, thumb dragging, track paging, wheel and
  touchpad coexistence, RTL, and scroll refs/signals.
- Add normal-node rotation as a generic native animation/property source for
  Spinner and application-owned Rhai components.
- Add missing generic semantic attributes such as pressed and range values.
- Add no private high-level component constructor or node kind.

### P0.3 Official Rhai source components

- Add Alert, AlertDialog, Badge, Card, GroupBox, Empty, Kbd, Spinner,
  ButtonGroup, InputGroup, Toggle, ToggleGroup, Slider, ContextMenu, Sheet,
  ScrollArea, Command, CommandDialog, TitleBar, and StatusBar.
- Keep application values and open/query state strictly controlled. Retain only
  transient pointer geometry, drag state, and roving active state in generic
  native/runtime mechanisms.
- Command owns deterministic fuzzy matching/ranking for label and keywords;
  Array data stays appropriate for small command sets and NativeCollection owns
  filtering/projection for large sets.
- Command is an embeddable searchable action list. CommandDialog composes
  Command with Dialog and never registers a global shortcut.
- ContextMenu reuses Menu item schemas, nesting, roving focus, typeahead, and
  action events; only activation and anchor policy differ.
- Sheet is a temporary controlled modal on logical start/end or physical
  top/bottom, never a persistent Sidebar.
- Slider is single-value only; a future dual-value control is a distinct
  RangeSlider.

### P0.4 Distribution and macOS certification

- Register every component and transitive dependency in the CLI, metadata,
  definitions, copied-source update path, and Theme Studio.
- Add one polished `component_gallery` example that interactively exercises all
  48 official components, category navigation, all theme hot switches, and
  important controlled states. Theme Studio remains the exhaustive theme-state
  contract; the gallery is the user-facing experience demo.
- Update User Guide, component authoring/API docs, examples, release notes,
  source audit, and component count assertions.
- Certify pointer, keyboard, focus, accessibility, RTL, reduced motion, resize,
  and every bundled theme on macOS. Linux and Windows are explicitly outside
  the first public release gate until hardware or public CI is available.

**P0 gate:** no component is experimental; the full workspace, CLI, native
macOS interaction, registry, Theme Studio, gallery preparation/smoke, Clippy,
rustdoc, package, release build, and checked-in visual baseline audits pass.
