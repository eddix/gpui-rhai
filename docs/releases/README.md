# Release notes

| Version | Focus | Integration impact |
|---|---|---|
| [0.3.3](0.3.3.md) | Frame replay: the subtrees of a view that did not change replay as cached GPUI views instead of building, laying out and painting again, and a commit does work in proportion to what changed; GPUI 0.3.9 | Compatible in Rhai and Rust; a Host depending on GPUI directly pins `=0.3.9`; a Host that mounts a view filling its window or a pane it sizes adds `host_sets_width(true)`; `render_islands(false)` or `GPUI_RHAI_RENDER_ISLANDS=off` renders every frame in full |
| [0.3.2](0.3.2.md) | Virtual collections render ahead in batches and keep the items a scroll leaves, with values set per collection, per Table or as a Host default | Compatible; the new defaults apply without a source update (`batch_items: 1, retain_items: 0` restores the old behavior); `gpui-rhai update` brings Table |
| [0.3.1](0.3.1.md) | Carousel and SlidingPanel; Tabs panels that slide in from the side of the tab they replace; disabled icons take the disabled ink; scroll containers stop at their last row | Compatible; `gpui-rhai update` brings Tabs (with the new SlidingPanel), Button, IconButton and AnimatedTabs; `gpui-rhai add components/carousel` for the pager |
| [0.3.0](0.3.0.md) | Tabs in three looks (track, underline, bar) with counts, closing, adding, reordering, slots and an all-tabs menu; TabOverflowMenu; Dracula, Molokai and Absolutely themes; the Gallery grouped by design chapter; module-scoped callbacks, Host error observer, responsive grid tracks, cubic-Bézier easings | **Not compatible:** TabBar is removed (use `Tabs` with `variant: "bar"`, `on_close` gets `#{ value, next }`); Tabs renders inside a mounted view; a hand-edited token base needs the counter lines; `MotionEasing` gains a variant; update with the 0.3.0 CLI |
| [0.2.3](0.2.3.md) | Retained pages and navigation stacks, shared scrollbars and tail-following, ColumnBrowser, Composer, selectable cards, calendars, time ranges, timelines, native Markdown, recalibrated palettes, and a CLI that keeps copied sources current | **Not fully compatible:** add `owner: None` to hand-built `PendingEvent`/`ActionInvocation`; sorted Array Tables sort in Rust; a hand-edited token base needs the `control.*` lines; update with the 0.2.3 CLI |
| [0.2.2](0.2.2.md) | Toast stacked layout and activation, a Host hook for the Rhai engine, keyboard focus events for scripts, `one_of` error paths, `hidden()` on rows and columns | Compatible; `gpui-rhai update` brings Toast; check rows and columns that call `hidden()` |
| [0.2.1](0.2.1.md) | Fixes after the 0.2.0 upgrade (modal focus, keys in layers, Table/List focus, footers, contrast) and small additive APIs (Host overlay queries, capability invocation context, `check` and Host extensions, CodeViewer content height, List loading) | Compatible; `gpui-rhai update` brings eleven changed modules; apply two token-base lines by hand |
| [0.2.0](0.2.0.md) | Design system for productivity tools: neutral token runtime, token base, 64 components (62 rebuilt, List and TabBar new), layouts and patterns, composition audit and profiles, the Gallery as acceptance application | Runtime API 3; add the token base for official components; review removed props and changed defaults |
| [0.1.8](0.1.8.md) | Unified Interaction Runtime, direct-manipulation components, Host policy/theme queries and Table/Ref convergence | Upgrade all three crates and copied Table source together; review Rust/key breaking changes; Runtime API stays 2 |
| [0.1.7](0.1.7.md) | Formal Gallery, Operations Workbench, source-backed acceptance stories | Replace removed Gallery examples; adopt optional Host environment APIs and Table search/page props |
| [0.1.6](0.1.6.md) | GPUI package-family migration and native AccessKit projection | Rust Host/MSRV migration to the exact gpui-pre 0.3.7 family |
| [0.1.5](0.1.5.md) | Native Chart Runtime and complete theme-driven UI | Add `spacing.xxs` to custom themes; typed chart viewport recommended |
| [0.1.4](0.1.4.md) | SVG/image pipeline, Host slots, adaptive overlays | No Runtime API migration |
| [0.1.3](0.1.3.md) | Motion Runtime 2 | Runtime API 2; migrate removed animation prototypes |
| [0.1.2](0.1.2.md) | Frozen 51-component foundation, Table resizing, accessibility | Add explicit labels required by eleven components |
| [0.1.1](0.1.1.md) | Runtime identity, transactions, async ownership, component styles | Replace the removed global `component_style` helper |
| [0.1.0](0.1.0.md) | First public runtime, CLI, registry, themes and 50 components | Initial adoption |

All three crates use the same version. Update the Rust dependency, installed
registry snapshot and CLI together so component schemas and runtime behavior do
not drift.

```bash
cargo update -p gpui-rhai
cargo install gpui-rhai-cli --version <version> --locked --force
gpui-rhai update
gpui-rhai check
```

Runtime API and crate semver are separate contracts. 0.1.3 introduced Runtime
API 2; versions 0.1.4 through 0.1.8 retain it. 0.2.0 introduces Runtime API 3.
