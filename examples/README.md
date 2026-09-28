# GPUI Rhai examples

This is the single index for examples maintained by the repository. Start with
the formal acceptance application when you want to explore the public surface:

```sh
cargo run --release -p gpui-rhai-cli -- gallery
```

Gallery stories are the authoritative interactive examples for all official
components, Motion, Charts, themes, locales, and the Operations Workbench. The
source pane displays the same bundled Rhai file that is running.

## Copyable tutorials

These Cargo examples demonstrate Host integration patterns worth copying:

| Target | Purpose | Companion material |
| --- | --- | --- |
| `hello_world` | Minimal file-backed Rhai application | `examples/hello_world/` |
| `embedded_hello_world` | Small embedded-source application | — |
| `embedded_views` | Several independently mounted script views | `examples/embedded_views/` |
| `extension_host` | Typed capabilities, subscription, primitive, and HostSlot | `examples/extension_host/` |
| `multi_window` | Host-owned multi-window composition | `examples/multi_window/` |
| `host_owned_tree` | Rust-owned retained tree and hot-value updates | `examples/host_owned_tree/` |
| `dashboard_layout` | Responsive dashboard composition | `examples/dashboard_layout/` |
| `form_showcase` | Form controls and validation | `examples/form_showcase/` |
| `data_table` | Table integration and visual states | `examples/data_table/` |
| `settings_panel` | Settings composition | `examples/settings_panel/` |
| `code_viewer` | Native read-only code surface | `examples/code_viewer/` |
| `diff_viewer` | Neutral text comparison surface | `examples/diff_viewer/` |
| `variable_height_chat` | Variable-height virtual collection | — |
| `artistic_showcase` | Canvas, SVG, type, and visual primitives | — |
| `mini_timeline` | Compact timeline composition | — |

Run one with:

```sh
cargo run -p gpui-rhai --example extension_host
```

## Internal verification targets

The following retain stable Cargo target names for CI and benchmark scripts,
but live under `crates/gpui-rhai/examples/internal/` because they are not API
tutorials:

- `performance_probe` — retained diff and virtual-list policy micro-probe.
- `table_1000` — end-to-end 1,000-row Table performance baseline.
- `native_overlay_smoke` — native overlay/focus launch smoke.
- `native_virtual_list_smoke` — native virtual-collection launch smoke.
- `phase0_probe` — raw phase-zero renderer launch smoke.

The canonical target inventory is `scripts/verification-manifest.json`; the
verification script resolves Cargo metadata, so explicitly located internal
targets cannot silently fall out of CI.
