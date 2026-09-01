# gpui-rhai CLI

The source-management CLI for GPUI Rhai. It initializes projects, copies
application-owned components/themes/locales/assets, validates compatibility,
performs baseline-aware updates, and generates production embedded sources.

The CLI binary is currently distributed from the repository release rather than
as a crates.io package because its bundled registry is maintained at the
workspace root.

`gpui-rhai theme-studio [path]` launches the bundled Theme Studio. It edits only
gpui-rhai `.rhai` themes and renders the canonical official-component specimen
under the live draft.
