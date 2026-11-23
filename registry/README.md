# gpui-rhai-registry

The versioned first-party source bundle for
[GPUI Rhai](https://github.com/eddix/gpui-rhai). It contains the design
system's token base, the editable Rhai components, layouts and patterns, the
application profiles, optional chart and motion source packs, themes (palettes),
locales, the Gallery and Theme Studio sources, small SVG assets copied into
application repositories by `gpui-rhai-cli`, and the agent skills
`gpui-rhai skills` writes (`BUNDLED_SKILL_FILES`, paths below a skills
directory).

Most application authors should install the CLI rather than depend on this
crate directly:

```text
cargo install gpui-rhai-cli --locked
cargo new my-app && cd my-app
gpui-rhai init --profile productivity
gpui-rhai add button input form_field
```

The crate intentionally exports immutable source constants so alternative
installers and build tooling can consume the exact same release snapshot. It
contains no runtime engine and grants no component privileged access to GPUI.

`BUNDLED_COMPONENT_SOURCES_BY_ID` exposes canonical `(module_id, source)`
pairs. `BUNDLED_ASSET_SOURCES` uses the exact provider-relative paths declared
by component metadata. The legacy source-only component slice and individual
constants remain available for consumers that do not need the named catalogs.
`BUNDLED_CHART_SOURCES_BY_ID` exposes the optional `charts/chart` adapter;
`BUNDLED_LAYOUT_SOURCES_BY_ID` and `BUNDLED_PATTERN_SOURCES_BY_ID` the `layouts/`
and `patterns/` modules; `TOKEN_BASE_SOURCE` the token base that official
components need (a Host passes it to `token_base`); `BUNDLED_PROFILES` the
application profiles.

Shared declaration contracts are authored in `contracts/*.json` and expanded by
`node scripts/sync-component-contracts.mjs` into marked literal blocks. Use its
`--check` mode to detect drift. Command and CommandDialog share the item
contract this way: their installed files stay standalone, and module-init
purity does not need runtime helper calls or mutable exported schema maps.
The SDK's registry tests compare both evaluated literal blocks with the bundled
`COMMAND_ITEMS_CONTRACT_JSON` source.
