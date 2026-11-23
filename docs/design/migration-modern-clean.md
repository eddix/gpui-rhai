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

This migration is a review candidate. Its renderer uses GPUI capabilities
that are not available in the published backend versions currently pinned
by this repository. Verify the required backend changes on both GPUI
variants and use supported public releases before merging or publishing
the SDK. Keep temporary dependency overrides out of product manifests
and release lockfiles.

Verify native mounting and interaction, live preset changes without state
loss, and packaged consumers on both backends. The real macOS VoiceOver
workflow and IME candidate window positioning still require OS-level
acceptance; an accessibility tree or synthetic input test does not
replace them. Preserve known contrast warnings where the authored preset
colors do not meet the audit threshold.
