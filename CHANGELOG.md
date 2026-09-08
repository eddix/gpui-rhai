# Changelog

All notable runtime, CLI, and registry changes are documented here. Version
0.1.0 establishes the first public compatibility baseline; later public API,
component schema, manifest, locale, and generated-source changes follow
semantic versioning from this release.

## 0.1.1 - 2026-09-08

- Runtime identity now distinguishes globally unique program candidates,
  logical component paths, mount incarnations, and per-view presentation
  domains. Multi-window geometry/capture no longer collide, and callbacks,
  timers, and NativeSignal handles from an unmounted same-key component cannot
  target its replacement.
- Runtime transactions now checkpoint the lifecycle tree as well as Engine and
  UI state. Task, subscription, and image-decode cancellation is provisional
  until the outer commit, while rollback restores old delivery authority and
  cancels newly created work. Independent async deliveries commit separately;
  a failed message no longer discards its neighbors.
- Rhai operation accounting now aggregates nested evaluators under semantics
  version 2 and gives delayed callbacks, retained component rerenders, and
  virtual item renderers fresh absolute baselines without resetting their
  enclosing execution budget. The default Engine rejects filesystem imports,
  import extraction walks an unoptimized Rhai AST (including nested template
  interpolation and dead branches), and the pinned Rhai 1.26 constant-container
  assignment panic is rejected during compilation.
- Durable values reject non-finite floats and enforce recursive item/depth/byte
  limits across Rhai, Rust, serde, state, stores, capabilities, handlers, and
  signals. Sensitive diagnostic payloads are redacted before storage, and
  automation returns execution failures instead of reporting dispatch success.
- Component and Store snapshots use copy-on-write state, removing quadratic
  instance mounting and deep-copying of unrelated large stores. No-op viewport
  and unrelated virtual-request paths return before opening a transaction.
- Background tasks use a bounded shared worker pool with panic delivery,
  admission budgets, and optional cooperative cancellation. Lossless receiver
  subscriptions wait for capacity instead of treating backpressure as stream
  termination. Subscription close and capacity waits share one synchronization
  protocol; stale generations discard buffered values, wake producers, and are
  reclaimed immediately.
- Semantic event and action traces retain names and scopes but never their
  payload values. Sensitive state/store values and all event/action payloads are
  therefore absent from retained diagnostics and Inspector snapshots.
- `virtual_collection` is now strictly a presentation/scroll/reveal/sticky
  mechanism and no longer creates a second internal roving row or generic
  background highlight. Command and Combobox remain the sole owners of their
  enabled-item navigation and controlled active style, so Array and
  NativeCollection groups, disabled items, and `active_value` cannot diverge.
  The unused fixed-row `VirtualListSpec`, `VirtualListState`, and
  `VirtualListMetrics` Rust APIs are removed with that duplicate state model.
- CLI updates recompute the complete dependency/asset graph. Multi-file apply
  stages every write, uses a project lock, and rolls back ordinary commit
  failures. A single verification manifest now covers every shipped example,
  while PNG baseline checks fully validate chunks, CRCs, and decoded data.
- Formal components passed through node-valued props now replay their latest
  caller-owned component snapshot when the receiver rerenders instead of an
  initial stale `UiNode`. Receiver rerenders do not re-execute or restart the
  passed component, same-batch dirty owners are order-independent, and caller
  removal still performs normal resource cleanup. Component-owned output is
  separated from typed outer presentation mutations so child updates preserve
  slot styles/handlers/refs/signals/animations without accumulating them. Lazy
  weak-linked snapshots avoid ordinary component clone cost, synchronize later
  virtual realization, and restore their prior value on transaction rollback.
- First-render `element_bounds(ref)` now returns null while retaining a pending
  ref-identity dependency, self-heals after committed prepaint, and follows
  retained node replacement. Event callbacks can resolve another node in their
  formal component with `ctx.element_bounds("local_ref_key")` without retaining
  a Rhai custom value.
- Initial controlled virtual-list reveal no longer pre-scrolls past predecessors
  when the target already fits the configured estimated viewport. Long grouped
  Commands therefore retain their first group heading at the natural scroll
  origin while genuinely offscreen initial targets are still revealed.
- Hosts can retain inactive `ScriptViewHandle` tombstones through explicit
  `suspend`/`resume`. State, native entities, input/scroll/virtual measurements,
  and the last-good tree survive; effects quiesce, subscriptions cancel, timers
  and animations freeze, overlays/focus/capture release, bounded task results
  wait for one atomic resume, and suspended hot reload migrates transactionally.
- Long-lived subscriptions must now be started by declarative component effects,
  making their cleanup, suspension, replacement, reload, and disposal ownership
  explicit.
- CodeViewer and both DiffViewer panes now apply the document typography metrics
  to line-number gutters instead of inheriting a larger ambient font.
- Sticky virtual section headers no longer create a duplicate presentation layer
  at their natural position; sticky indices remain presentation metadata rather
  than an interaction-eligibility channel.
- Command group labels now recede with regular-weight muted typography, gain
  asymmetric breathing room, and visually own indented command rows. Array and
  NativeCollection projections use the same row metric.
- Raw nodes passed through formal component slots now retain the caller's
  callback provenance recursively, including optional, array, map, object, and
  union-shaped Node props.
- Subscription delivery now defaults to a bounded FIFO instead of silently
  overwriting earlier values. Rhai callers pass an explicit options map and may
  opt into latest-only coalescing; full queues return backpressure to Rust.
- Controlled virtual collections synchronously rebuild the retained viewport
  with current Rhai state before committing a rerender, eliminating blank
  frames when Command selection moves beyond the first realized window.
- Embedded Hosts can read and observe each `ScriptViewHandle`'s complete
  effective `ThemeSnapshot`, including colors, spacing, radii, typography,
  namespaced tokens, and system-appearance changes.
- Applications can define validated global formal-component part overrides in
  `ui/styles.rhai`. File, embedded, CLI check/embed, hot reload, rollback, and
  all official component sources use the same typed `ctx.component_style`
  cascade. Explicit instance overrides remain the final application-owned
  layer.

This release intentionally removes the old global
`component_style(props, part, base)` helper. Source components use
`ctx.component_style(part, base)` so retained and deferred renderers share the
same validated stylesheet snapshot.

## 0.1.0 - 2026-09-07

Initial implementation of the stable Rhai `UiNode` boundary, source-owned
component registry, themes/locales/assets, typed capabilities and custom
primitives, file and embedded applications, hot reload, developer inspector,
native input/overlay/animation/virtual-list mechanisms, RTL layout, and the
restricted multi-window lifecycle.

The pre-release Core Runtime v2 expansion adds typed Box/Text/Image/SVG/Canvas
atoms, retained keyed reconciliation, independent formal-component rerendering
and root-dirty bailout, exact store/environment dependencies, transactional
effects/timers, native signals, retained refs/geometry/focus/scroll, atomic
capture/target/bubble events, schema-checked native Rust handlers, Host-owned
trees, generic Overlay/Layer composition, retained automation, and bounded
native collections for large Tables.

Event handlers can consume the current handler node's untracked event-time
visual bounds through `ctx.event_target_bounds()`, raw pointer/wheel
`payload.target`, or `NativeEvent::target`. The application no longer needs a
resize/store channel merely to position native UI after a click.

Automation and mounted accessibility snapshots now project only nodes presented
in the latest GPUI frame. Open Overlay content is locatable, while retained
content from a closed Overlay can no longer receive invisible automation
dispatches.

Transparent formal-component rerenders promote to the nearest replaceable
ancestor instead of failing when a parent and child share one `UiNode` root.
Runtime-error banners are selectable monospace text and can be suppressed by a
Host that surfaces `ScriptViewHandle::last_error` itself.

The generic virtual collection accepts explicit sticky section-header indices,
retains exactly one active header, and pushes it off as the next section enters.
The source Table adds controlled `group_by`/`collapsed_groups` with counts,
toggle events, source parts, Array parity, and cached NativeCollection
sort/group/collapse projection.

Composite component callbacks now cross each formal boundary through declared
events: RadioGroup forwards both pointer and roving-key changes to its caller,
and Dialog explicitly emits `open_change`. Modal Overlay focus is enforced as a
per-frame invariant, including initially-open dialogs and one-off ancestor focus
requests from embedding Hosts, so Escape dismissal remains reachable.
Callback props now carry private module-scope provenance through formal
composition instead of inferring ownership from a function name; entry and
nested component modules may use identical private handler names without
redirecting an event.

The source registry now includes the complete official component specimen,
fifteen bundled theme variants, and Theme Studio for creating, importing,
editing, validating, previewing, and saving gpui-rhai themes.
The same sources ship as the versioned `gpui-rhai-registry` crate consumed by
the publishable CLI, so installed binaries never depend on files outside their
Cargo package.

The public-launch registry expands to 50 official components. Dropdown is
destructively renamed to the strictly controlled Combobox. New source-owned
families include Alert/AlertDialog, Badge, Card/GroupBox/Empty, Kbd/Spinner,
ButtonGroup/InputGroup, Toggle/ToggleGroup, Slider, ScrollArea, ContextMenu,
Sheet, Command/CommandDialog, and application-chrome TitleBar/StatusBar.
Generic Rust mechanisms provide native range
preview with commit-only Rhai delivery, event-coordinate overlay anchors,
logical viewport-edge sheets, themed draggable overlay scrollbars, Canvas
rotation, and NativeCollection fuzzy filtering/navigation. The interactive
`component_gallery` shares Theme Studio's exhaustive specimen and switches all
bundled themes live.

Themes now require eight semantic typography roles. Official source components
and native Input/Textarea shaping consume the same live role values, functional
glyphs use a centered 24px SVG asset system, and the default radius scale is
square. Only elements that are semantically circular retain explicit radii;
Switch tracks and thumbs are rectangular.

TitleBar now accepts string-or-node `title` and `subtitle` content with a
separate required accessibility `label`, allowing structured breadcrumbs to
retain their own styles and pointer/native handlers inside the standard chrome
layout.

The unlocked macOS visual pass tightened responsive specimen wrapping, gave
Theme Studio an independently scrollable token editor, made small Command lists
shrink to their result count, and added deterministic Gallery theme, locale,
category, overlay, compact, regular, and reduced-motion launch states.

Command and CommandDialog now use caller-owned `active_value` and expose a
deduplicated `active_change(string)` event for keyboard and pointer roving
previews while preserving `action` as the explicit confirmation boundary.
Their active item is a controlled virtual-collection reveal target: keyboard
and Host updates follow beyond the visible window, while unchanged targets no
longer reset manual wheel/trackpad scrolling. Generic virtual collections now
distinguish stable-key payload updates from structural changes.

The public registry now includes `CodeViewer` and `DiffViewer` over retained
native document primitives. Direct strings and exact-reader-invalidating
`NativeTextDocument` revisions share background syntax parsing, virtual rows,
continuous source selection, search, wrapping and line navigation. DiffViewer
adds neutral left/right unified and split projections, Unicode-grapheme
intraline emphasis, expandable context, focus-scoped hunk actions, independent
split-pane horizontal scrolling and explicit left-to-right unified-patch copy.
The built-in language pack includes Rhai, Rust, Go, common web/configuration and
scripting languages; every theme materializes syntax/search/diff semantic
colors. Host-configurable resource limits, standalone examples, Gallery/Theme
Studio specimens and a release benchmark cover both document input paths.

The initial theme contract includes explicit `on_accent`, `on_danger`,
`on_warning`, and `on_success` foregrounds. Reduced motion renders looping
indicators at a static midpoint rather than moving them out of view.

Dogfooding API reset before the first published release:

- `ScriptApp` → `FileScriptView`;
- `EmbeddedScriptApp` → `EmbeddedScriptView`;
- `PreparedScriptApp` → `PreparedScriptView`;
- `ScriptAppExtension` → `ScriptViewExtension`;
- `ScriptAppError` → `ScriptViewError`;
- standalone ownership moved to `ScriptApplication`;
- existing GPUI hosts mount isolated views through `ScriptViewHost` and
  `ScriptViewHandle`.

This destructive migration was completed before `0.1.0`; the published
baseline therefore contains no compatibility aliases.

There is no earlier GPUI Rhai release to migrate from.
