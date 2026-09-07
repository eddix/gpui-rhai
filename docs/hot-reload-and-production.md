# Hot reload and production embedding

## Development

Run `gpui-rhai dev` or a file-backed `FileScriptView` with the `dev-reload` feature.
The watcher tracks Rhai modules, themes, the component stylesheet, locale bundles, and image assets. Script candidates are
compiled transactionally. A reverse dependency graph recompiles changed modules
and their transitive dependants while the resolver reuses content-matching ASTs;
the entry is then compiled self-contained for atomic commit. Failed candidates keep the last-good AST, tree,
callbacks, state, and component metadata. Successful reloads preserve compatible
state and invalidate callbacks from the previous generation.

A suspended `ScriptViewHandle` continues to collect changed filesystem paths
but does not compile modules or invoke Rhai in the background. `resume` processes
the latest batch first. The latest successful program candidate migrates through
the same atomic resume transaction; a failed edit keeps the prior generation
and appears through normal last-good diagnostics. Pending async deliveries from
an older generation are discarded rather than replayed into migrated code.

Mounted views surface the latest failed callback/render/reload above the
last-good tree as selectable monospace text. Embedded Hosts that render their
own error UI may set `ScriptViewConfig::show_error_banner(false)` and continue
reading `ScriptViewHandle::last_error`; standalone adapters expose the matching
`ScriptApplication` option.

The inspector is available only in development and opens with Command-Option-I
or F12. It shows source locations, redacted state, computed semantics, traces,
and execution timings.

## Production

Run:

```text
gpui-rhai check
gpui-rhai embed
cargo build --release
```

`embed` deterministically generates `src/gpui_rhai_embedded.rs` using
`include_str!`/`include_bytes!` for the entry, installed components, component
stylesheet, locales, themes, and assets, plus an `app_manifest()` constructor. Construct
`EmbeddedScriptView` from those generated sources and pass
`.component_styles(generated::COMPONENT_STYLES_SOURCE)` and
`.manifest(generated::app_manifest())`.
Pass `.asset_sources(generated::asset_sources())` so embedded `app/...` image
IDs resolve exactly like file-backed assets.
Call `.prepare()` and either mount the result through a `ScriptViewHost` or pass
it to `ScriptApplication::new`.
Production needs no filesystem watcher or source-path access, and file-backed
and embedded sources use the same restricted module-resolution contract.

Do not edit the generated Rust module. Edit files under `ui/`, run `check`, then
regenerate it. Keep `dev-reload` disabled in release builds unless a product has
an explicit, reviewed reason to ship source watching.
