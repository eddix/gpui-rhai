# Changelog

All notable runtime, CLI, and registry changes are documented here. The project
uses semantic versions; during `0.x`, every breaking public API or component
schema change must include a migration note in this file.

## 0.1.0 - unreleased

Initial implementation of the stable Rhai `UiNode` boundary, source-owned
component registry, themes/locales/assets, typed capabilities and custom
primitives, file and embedded applications, hot reload, developer inspector,
native input/overlay/animation/virtual-list mechanisms, RTL layout, and the
restricted multi-window lifecycle.

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
