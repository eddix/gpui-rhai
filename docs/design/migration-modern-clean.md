# Modern / Clean integration status

Modern and Clean each provide light and dark presets. Use the matching
SDK, CLI and registry from the same revision. Copying the new registry
to an older SDK does not provide the same rendering or behavior.

- The [visual specification](modern-clean-spec.md) defines the shared
  geometry, typography, paint and state rules.
- The [component inventory](component-coverage.md) covers the official
  modules. Gallery is the interactive entry point for the four presets.
- Theme Studio, the bundled examples and the in-repository DiskTree skins
  use the same public theme and component mechanisms.

## Before integration

The default [public profile](../gpui-profiles.md) uses released dependencies
and explicit rendering fallbacks. The alternate macOS vendor profile uses
the repository's optical and layout extensions without changing the
published dependency graph.

Verify native mounting and interaction, live preset changes without state
loss, and packaged consumers on both backends. The real macOS VoiceOver
workflow and IME candidate window positioning still require OS-level
acceptance; an accessibility tree or synthetic input test does not
replace them. Preserve known contrast warnings where the authored preset
colors do not meet the audit threshold.
