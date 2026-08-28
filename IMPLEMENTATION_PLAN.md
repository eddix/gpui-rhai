# GPUI Rhai Implementation Plan

## 1. Purpose

This plan turns the product contract in `INTENT.md` into dependency-ordered,
testable delivery milestones. It deliberately prioritizes end-to-end risk
reduction over component count.

The plan contains no calendar estimates. Work advances when an exit gate is met,
not when a nominal date arrives.

## 2. Planning principles

1. Build one executable vertical slice before broadening APIs.
2. Keep GPUI types behind the `UiNode` rendering boundary.
3. Stabilize identity, state, errors, and schemas before writing many components.
4. Put native mechanisms in Rust and component policy in Rhai.
5. Exercise every public extension point in an example or integration test.
6. Treat keyboard accessibility and diagnostics as architecture, not polish.
7. Add dependencies only when their responsibility and feature placement are
   explicit.
8. Do not begin M2 breadth work while M1 mechanism risks remain unresolved.

## 3. Target workspace

```text
gpui-rhai/
├── Cargo.toml
├── crates/
│   ├── gpui-rhai/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── app/
│   │       ├── assets/
│   │       ├── capability/
│   │       ├── diagnostics/
│   │       ├── engine/
│   │       ├── event/
│   │       ├── node/
│   │       ├── overlay/
│   │       ├── primitive/
│   │       ├── schema/
│   │       ├── state/
│   │       ├── style/
│   │       ├── theme/
│   │       └── devtools/
│   └── gpui-rhai-cli/
├── registry/
│   ├── components/
│   ├── themes/
│   ├── locales/
│   └── assets/
├── examples/
│   ├── hello_world/
│   ├── settings_panel/
│   ├── dashboard_layout/
│   └── form_showcase/
├── docs/
└── tests/
```

Module boundaries may be consolidated while code is small, but public concepts
must not be collapsed merely to reduce file count.

## 4. Phase 0: feasibility and foundations

Phase 0 should produce disposable or narrowly scoped probes. Only validated
findings become public API.

### P0.1 Bootstrap the workspace

- Create the Cargo workspace, runtime crate, and CLI crate.
- Pin a GPUI revision and a compatible Rhai release.
- Re-export GPUI from the runtime crate.
- Declare stable Rust policy, edition, and initial MSRV.
- Add formatting, lint, unit-test, and macOS CI jobs.
- Add `MIT`, `Apache-2.0`, contribution, and third-party attribution files.

**Acceptance:** both crates build on macOS; CI runs formatting, linting, and tests;
the dependency graph contains no `gpui-component` package.

### P0.2 Probe `UiNode` rendering

- Define a temporary minimal node enum containing text and a block container.
- Evaluate one Rhai function returning a nested node tree.
- Render the tree into a GPUI window.
- Confirm that GPUI contexts and elements never enter Rhai values.
- Measure conversion and script execution for a representative small tree.

**Acceptance:** a Rhai-authored tree renders in GPUI and the boundary can be
expressed without unsafe lifetime workarounds.

### P0.3 Probe Rhai callbacks and reload invalidation

- Register a normalized click callback represented by a Rhai function pointer.
- Invoke it from GPUI on the foreground thread.
- Compile a replacement AST and prove callbacks from the old generation are
  rejected.
- Record the error and backtrace information Rhai can retain across modules.

**Acceptance:** a click updates observable Rust-owned state, and stale callbacks
cannot execute after a generation change.

### P0.4 Probe source locations and module resolution

- Implement a restricted resolver rooted at an explicit script source.
- Reject absolute paths, parent traversal, undeclared roots, and cycles.
- Confirm error spans survive imports and point to the correct file, line, and
  column.
- Disable arbitrary eval and unbounded dynamic module loading.

**Acceptance:** an imported syntax/runtime error produces an actionable source
diagnostic and no module can escape its allowed roots.

### Phase 0 exit gate

- The core architecture is feasible without exposing GPUI types to Rhai.
- Callback generation invalidation is demonstrated.
- Source diagnostics and restricted imports are demonstrated.
- Any deviation from `INTENT.md` is captured as an explicit design decision
  before M0 begins.

## 5. M0: executable foundation

M0 ends with a small but real application created by the CLI, authored in Rhai,
hot-reloaded during development, and embedded for release.

### M0.1 Core value and schema model

- Implement `UiValue` primitives, arrays, maps, and registered opaque handles.
- Add schema definitions for scalar, enum, optional, array, map, node, callback,
  handle, and object values.
- Implement path-aware validation errors and unknown-field rejection.
- Define component schemas for props, state, events, slots, parts, dependencies,
  capabilities, component version, and runtime API range.
- Add schema serialization for CLI/editor tooling.

**Tests:** validation success/failure matrices, nested error paths, unknown fields,
handle type mismatch, schema serialization snapshots.

### M0.2 Stable node and style model

- Define the initial `UiNode`, `NodeKind`, `NodeKey`, source metadata, props, and
  children model.
- Define typed length, color, edge, alignment, typography, and layout values.
- Implement a deliberately small `Style` builder and deterministic merge rules.
- Implement pseudo-state style declarations for hover, active, focus, and
  disabled.
- Expose initial layout/text primitives to Rhai.

**Tests:** node snapshots, style merge precedence, invalid style values, source
metadata propagation, pseudo-state resolution.

### M0.3 Component declarations and module API

- Implement `export_component`.
- Define and parse the structured component header metadata block.
- Cross-check header identity, compatibility, dependencies, and capabilities
  against the registry and exported schema.
- Require explicit aliases and PascalCase exports.
- Distinguish formal component instances from render helper functions.
- Validate component props/state/events/slots/parts against the schema.
- Generate component instance paths from parents and keys.

**Tests:** duplicate export, missing key for stateful component, duplicate sibling
keys, invalid slot/event, component path stability.

### M0.4 State, stores, and render transactions

- Implement Rust-owned component state with schema-declared keys.
- Implement typed application and window stores.
- Track store reads and invalidate only dependent consumers.
- Batch mutations until the current callback completes.
- Reconcile keyed instances and clean unreachable state only after a successful
  render.
- Preserve compatible state and locally reset incompatible state on reload.

**Tests:** controlled/uncontrolled behavior, keyed reorder, selective
invalidation, batched updates, successful/failed reload cleanup, schema-change
reset.

### M0.5 Event and action core

- Implement restricted `UiContext` state/store/event APIs.
- Implement normalized payloads and `handled`/`propagate` bubbling.
- Implement declared event output validation.
- Implement namespaced action registration and dispatch.
- Add a small configurable keybinding adapter.

**Tests:** event order, propagation stop, payload validation, action dispatch,
disabled action, keyboard-to-action mapping.

### M0.6 Script application lifecycle

- Implement required `view(ctx)` and optional `init(ctx)` / `dispose(ctx)`.
- Enforce the no-side-effects render phase in capability APIs.
- Add AST and module dependency caching.
- Add operation/depth/data limits and slow-script diagnostics.
- Keep all Rhai evaluation on the foreground thread.

**Tests:** lifecycle order, render-phase capability rejection, limit errors,
dependency cache reuse, callback generation checks.

### M0.7 Theme foundation

- Define the initial semantic token schema.
- Load theme source from Rhai and validate type/completeness.
- Implement `ThemeFamily`, variants, and app/window/subtree scopes.
- Implement `system` selection and runtime switching.
- Ensure theme changes invalidate consumers without AST recompilation or state
  loss.
- Create project-owned Default Light and Default Dark.

**Tests:** missing/wrong token diagnostics, scope precedence, live switch, system
variant selection, state preservation.

### M0.8 Hot reload and error recovery

- Add file watching behind a development feature.
- Recompile only changed modules and transitive dependants.
- Retain the last good AST/tree/state after compile or render failure.
- Implement structured diagnostics with script stack, source span, component
  path, and key.
- Add root error fallback and the first `ErrorBoundary` node.

**Tests:** leaf and dependency reload, compile failure recovery, render failure
boundary, stale callbacks, state compatibility.

### M0.9 Custom primitive registration

- Define the public namespaced primitive registration API.
- Register props schemas and a renderer.
- Normalize primitive events into `UiValue` payloads.
- Add optional instance lifecycle and state hooks.
- Build one example custom primitive outside the runtime module to prove the
  public interface.

**Tests:** registration conflicts, invalid props, event conversion, lifecycle,
external-crate usage test.

### M0.10 CLI and registry skeleton

- Define registry metadata, component dependency graph, and local manifest.
- Implement `init`, `add`, `check`, and `--dry-run` foundations.
- Bundle the registry snapshot in the CLI.
- Resolve transitive dependencies and runtime API ranges.
- Create `.gpui-rhai/manifest.toml` and pristine baselines.
- Generate a minimal host without overwriting existing project files.

**Tests:** fresh project fixture, existing-entry fixture, dependency resolution,
cycle detection, incompatible runtime, dry-run with zero writes.

### M0.11 File and embedded script sources

- Define a shared `ScriptSource` interface.
- Implement file-backed development loading.
- Generate an embedded Rust source manifest from the CLI.
- Prove import, theme, locale, and asset identifiers resolve identically in both
  modes.

**Tests:** run identical tree snapshots against file and embedded sources;
content-hash stability; missing embedded resource errors.

### M0.12 First source components and example

- Author Button with sizes, all declared variants, disabled/loading behavior,
  prefix/suffix icon slots, style, and part styles.
- Author Label with required marker, description, and accessibility semantics.
- Build `hello_world` using only public downstream APIs and copied sources.
- Add component headers, schemas, tests, screenshots, and usage documentation.

### M0 exit gate

- `gpui-rhai init` creates a runnable application without overwriting user files.
- `gpui-rhai add button label` installs reproducible editable source.
- `gpui-rhai dev` renders and hot-reloads `hello_world`.
- A failed edit retains the last good UI and reports the correct source span.
- Runtime theme switching works without state loss.
- A custom Rust primitive renders and emits a Rhai event.
- A release-mode example runs entirely from embedded sources.
- Button and Label pass logic, node snapshot, screenshot, keyboard, and
  accessibility gates.

## 6. M1: mechanism and interaction validation

M1 addresses the areas most likely to force architectural changes: native text
input, focus, accessibility, overlays, animation, asynchronous work, assets,
large lists, and developer inspection.

### M1.1 Accessibility and focus infrastructure

- Define runtime semantic properties supported by the pinned GPUI version.
- Implement focus handles/scopes, deterministic traversal, and restoration.
- Add type-ahead helper behavior and disabled-state consistency.
- Surface known upstream accessibility gaps in diagnostics and documentation.
- Add reduced-motion environment handling.

**Acceptance:** all M1 controls are operable by keyboard, focus is visible, and
semantic assertions pass where GPUI exposes the required platform API.

### M1.2 Text input primitive

- Implement editing buffer, cursor, selection, clipboard, IME composition,
  placeholder, disabled/read-only, and focus behavior in Rust.
- Normalize change, submit, focus, and blur payloads.
- Keep validation and visual policy in Rhai.
- Test Latin, CJK IME, selection replacement, clipboard, and shortcut behavior
  on macOS.

**Acceptance:** the primitive survives real IME composition and focus changes
without corrupting text or exposing GPUI objects to Rhai.

### M1.3 Overlay manager

- Implement per-window portal layers and ordering.
- Implement anchored placement, flipping, clamping, and viewport avoidance.
- Implement dismiss hierarchy, outside click, Escape, modal masks, focus trap,
  and focus restoration.
- Implement tooltip delays and toast queue regions.
- Validate nested overlays and parent/child dismissal.

**Acceptance:** Popover inside Dialog and nested Menu scenarios have deterministic
focus and dismissal behavior.

### M1.4 Animation runtime

- Implement transition/spring specifications and Rust-side frame scheduling.
- Support opacity, transform, size/clip, and the minimum properties required by
  M1 components.
- Cancel or retarget animations when keyed nodes change.
- Apply reduced-motion policy centrally.

**Acceptance:** no per-frame Rhai execution occurs; Accordion/Popover-style
transitions remain correct across interruption and reload.

### M1.5 Tasks, subscriptions, and capabilities

- Implement versioned capability registration and manifest validation.
- Implement `TaskHandle` completion, cancellation, and error payloads.
- Implement `SubscriptionHandle` delivery, throttling hooks, and cleanup.
- Reject effectful capability calls during `view`.
- Drop callbacks from obsolete AST generations.

**Acceptance:** a demo capability loads data asynchronously, a subscription
updates it repeatedly, and both cancel correctly on component removal/reload.

### M1.6 Assets and images

- Implement `AssetId`, `ImageHandle`, and provider registration.
- Add SVG loading, semantic color inheritance, cache keys, and fallback behavior.
- Add asynchronous raster decode and cancellation.
- Package the minimal official icon set with attribution.

**Acceptance:** Icon and Avatar source can switch live without direct script path
or URL access.

### M1.7 Virtual list

- Implement one-dimensional viewport-driven realization.
- Require item count, stable key, predictable height, and a pure item renderer.
- Preserve focus and state through scroll, reorder, and filtering.
- Add overscan and performance diagnostics.

**Acceptance:** a several-thousand-item list maintains bounded realized nodes and
correct keyboard selection on macOS.

### M1.8 Locale bundles

- Define locale bundle schemas and fallback rules.
- Add runtime app/window/subtree locale selection if required by real component
  composition.
- Ship English and Simplified Chinese bundles.
- Ensure components do not hard-code internal user-visible phrases.

**Acceptance:** an example switches locales live without losing component state.

### M1.9 Developer tools

- Add a development-only inspector overlay.
- Display the component/node tree, keys, source spans, props, parts, computed
  style, theme tokens, state/store values, and invalidation reasons.
- Add recent event/action/capability traces and script timing.
- Redact schema-marked sensitive values.

**Acceptance:** a developer can locate a rendered node back to its Rhai source
and explain why it rerendered.

### M1.10 CLI development and update workflow

- Complete `gpui-rhai dev` orchestration and diagnostic output.
- Complete `diff` and three-way `update` using committed baselines.
- Refuse silent overwrite and preserve conflict artifacts for inspection.
- Emit schema-driven editor metadata and basic snippets.
- Keep a full LSP and formatter out of scope.

**Acceptance:** a locally modified component can be updated offline with clean
changes merged and conflicts explicitly reported.

### M1.11 Themes

- Refine semantic tokens against real interactive components.
- Add two Tokyo Night variants.
- Add Catppuccin Latte and Mocha.
- Preserve upstream names/attribution while mapping to project semantic tokens.
- Add representative multi-theme visual regression coverage.

### M1.12 Risk-validation components

Author and fully test:

- Input;
- Icon;
- Divider;
- Popover;
- Dropdown, including single/multi-select, search, keyboard navigation, outside
  dismissal, controlled/uncontrolled use, slots, and large-option virtualization;
- Dialog, including modal focus, Escape, focus restoration, slots, and actions.

Build `settings_panel` from installed source components and public APIs.

### M1 exit gate

- CJK IME and clipboard behavior pass macOS integration tests.
- Nested overlays, modal focus, and keyboard dismissal pass interaction tests.
- Async task/subscription cleanup passes removal and hot-reload tests.
- A 5,000-item virtual list has bounded node realization.
- Default, Tokyo Night, and Catppuccin themes hot-switch without recompilation or
  state loss.
- English and Simplified Chinese internal strings switch live.
- Devtools explains source, state, computed styling, and invalidation.
- The modified-source three-way update workflow is proven end to end.
- Every M1 component passes all quality gates from `INTENT.md`.

## 7. M2: component product line and release hardening

### M2.1 Remaining primitives

Author and fully test:

- Checkbox with checked, unchecked, and indeterminate states;
- Radio and RadioGroup with roving keyboard focus;
- Switch with loading state;
- Tag with variants and close event;
- Avatar with initials fallback and presence indicator;
- Progress with determinate and indeterminate animation;
- Skeleton with reduced-motion behavior.

### M2.2 Remaining composites

Author and fully test:

- Tooltip;
- Accordion with single/multiple modes and animation;
- Collapsible;
- Tabs with horizontal/vertical orientation and keyboard navigation;
- Menu with nested submenus, separators, action state, and shortcut hints;
- Toast with queue, automatic expiry, pause behavior, and manual dismissal;
- FormField with label/control/description/error associations.

### M2.3 Locale and RTL validation

- Validate logical spacing, alignment, ordering, and directional icons under RTL.
- Add at least one RTL test locale for layout validation.
- Complete screenshot and keyboard-navigation coverage in both directions.
- Document components or upstream GPUI APIs that cannot yet meet the contract.

### M2.4 Example completion

- Expand and finalize `settings_panel`.
- Build `dashboard_layout`.
- Build `form_showcase` with explicit Rhai state and validation.
- Keep every example independently runnable and embedded-release compatible.
- Add an extension example showing a custom Rust capability and primitive.

### M2.5 Multi-window API

- Validate the app/window/component lifetime model built earlier.
- Add a restricted window command API using the validated lifetime model.
- Define app-store sharing, per-window stores, themes, locales, task ownership,
  close confirmation, and focus behavior.
- Add integration coverage for opening, communicating with, and closing multiple
  windows without leaking state, overlays, tasks, or subscriptions.

### M2.6 Documentation

- Write architecture and security-boundary documentation.
- Write the component authoring guide using `export_component` and schemas.
- Write theming, locale, assets, capabilities, custom primitives, hot reload,
  production embedding, and source-update guides.
- Document every known GPUI accessibility/platform gap.
- Provide a concise English quick start and optional Simplified Chinese guide.

### M2.7 Release engineering

- Audit public API visibility and SemVer exposure.
- Verify component/runtime/registry compatibility checks.
- Audit dependency licenses and copied asset attribution.
- Test MSRV and latest stable Rust.
- Produce macOS release-mode smoke tests for all examples.
- Verify that release artifacts contain no unintended devtools, source paths, or
  sensitive diagnostic values.
- Publish migration notes for every breaking `0.x` runtime API change.

### M2 exit gate

- The complete component list in `INTENT.md` meets all quality gates.
- All four examples are independently runnable and documented.
- App/window state, theme, locale, task, and overlay lifetimes pass multi-window
  integration tests.
- Representative RTL layouts pass visual and keyboard-navigation tests.
- CLI init/add/check/dev/diff/update workflows pass clean and modified-project
  fixtures.
- File-backed development and embedded production produce equivalent behavior.
- macOS keyboard, focus, accessibility, theme, and visual suites pass.
- Documentation is sufficient to author a component, capability, custom
  primitive, theme, and production application without reading runtime internals.

## 8. Cross-cutting test strategy

### Unit tests

- schemas, values, styles, tokens, state, reconciliation, actions, manifests;
- capability lifecycle and generation invalidation;
- module resolution and compatibility ranges.

### Rhai contract tests

- component defaults and validation;
- declared state and event behavior;
- import graph and component export semantics;
- file/embedded source equivalence.

### Snapshot tests

- normalized `UiNode` trees;
- schemas and editor metadata;
- diagnostics and CLI dry-run plans;
- manifests and generated embedding source.

### macOS integration tests

- real GPUI rendering and input;
- keyboard/focus/IME/clipboard;
- overlays and animation;
- hot reload and last-good recovery;
- task/subscription cleanup.

### Visual regression tests

- representative themes, sizes, variants, and states;
- deterministic fonts, scale factor, viewport, and animation state;
- documented tolerance and intentional-baseline update process.

### Performance checks

- script compile and cached-render timings;
- node conversion and reconciliation cost;
- store invalidation fan-out;
- virtual-list realized-node bounds;
- warnings for slow view/handler/capability delivery.

Initial numeric budgets should be recorded after Phase 0 measurements rather
than invented before a working GPUI baseline exists.

## 9. Decision records required during implementation

Create short ADRs before committing to public APIs for:

1. `UiNode` representation and GPUI conversion ownership;
2. Rhai engine/thread model and callback generation identity;
3. component schema format and editor metadata representation;
4. state path/key reconciliation and cleanup;
5. script source/module resolver abstraction;
6. capability/task/subscription ABI;
7. custom primitive registration API;
8. style and semantic token type systems;
9. overlay manager and focus hierarchy;
10. registry metadata, baseline storage, and three-way update behavior.

An ADR records the chosen design, rejected alternatives, consequences, and the
tests that protect the decision. It does not restate general project intent.

## 10. First implementation sequence

The recommended first pull requests are deliberately small:

1. Workspace, licenses, CI, pinned dependencies, and dependency guard.
2. Minimal `UiNode` Rhai-to-GPUI feasibility probe.
3. Callback generation and restricted module resolver probes.
4. `UiValue` and schema core.
5. Stable `UiNode`/`Style` core and renderer.
6. Component declaration, identity, state, and event transaction path.
7. Minimal script app lifecycle and error recovery.
8. Default theme contract and live switch.
9. External custom primitive proof.
10. CLI init/add/check skeleton and registry format.
11. File/embedded source equivalence.
12. Button, Label, and `hello_world` M0 gate.

Do not start broad component authoring before item 12 passes. Component breadth
must validate a stable runtime rather than become test data for an unstable one.
