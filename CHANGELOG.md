# Changelog

All notable runtime, CLI, and registry changes are documented here. No public
release baseline exists yet: dogfooding changes target the best final API and do
not retain compatibility shims. Once the maintainer gives an explicit release
signal, semantic-version and migration notes start from that published
baseline.

## 0.1.0 - unreleased

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

The source registry now includes the complete official component specimen,
fifteen bundled theme variants, and Theme Studio for creating, importing,
editing, validating, previewing, and saving gpui-rhai themes.

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

No compatibility aliases are retained because `0.1.0` has not been published.

There is no earlier GPUI Rhai release to migrate from.
