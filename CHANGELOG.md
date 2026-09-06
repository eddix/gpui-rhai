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

No compatibility aliases are retained because `0.1.0` has not been published.

There is no earlier GPUI Rhai release to migrate from.
